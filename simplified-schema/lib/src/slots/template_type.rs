//! Replacing template types ([`TypeExpr::Slot`]) with the definitions their
//! slots returned.
//!
//! A returned value is parsed once as a type definition
//! ([`definition_from_value`]) and expanded as a named type of the slot's
//! defining file used at the slot's position, so its imports, the written
//! suffix, union flattening, and every merge rule are the static import
//! engine's own. Each occurrence expands twice: first bare, so the checks
//! see only what the returned data introduced (no active slot reachable
//! through its imports, every pattern passing the slot-pattern rules), then
//! with the written suffix, which gives the definition the schema uses.
//! Every read the expansions cause passes the host's
//! [`super::TemplateReads`] first.

use serde_json::Value;

use super::substitute::{ExpansionSite, LateDependencies, SlotValues, check_slot_pattern, json_kind, slot_error};
use crate::errors::{SchemaError, SlotRelated, SlotResultKind};
use crate::resolve::{expand_template_definition, pattern_key_source_name, rehome_returned_examples};
use crate::simplified::{
    Constraint, DefKey, DefinitionTable, FunctionParameters, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaRoot,
    SchemaShape, SchemaSourcePath, Slot, TupleType, TypeExpr, parse_type_definition_value,
};

/// Replaces every template type in `root` and `definitions` whose slot has
/// a value in `values` with the definition the value holds, expanded from
/// `site`. A template type without a value is left for lowering to report.
///
/// ## Errors
///
/// [`SchemaError::SlotResult`] at the slot, naming its property, when the
/// value is not a usable definition; [`SchemaError::SlotReadRefused`] when a
/// read its expansion needs was refused; the import engine's own error
/// (a missing file, an import cycle) otherwise.
pub(crate) fn substitute_types(
    root: &mut SchemaRoot,
    definitions: &mut DefinitionTable,
    values: &SlotValues,
    site: &ExpansionSite<'_>,
) -> Result<LateDependencies, SchemaError> {
    let mut run = TypeSubstitution { values, site: *site, published: DefinitionTable::default(), late: LateDependencies::default() };
    for (key, body) in definitions.entries_mut() {
        run.def(body, &SchemaSourcePath::root().property(key.name.as_str()))?;
    }
    match root {
        SchemaRoot::Single(shape) => run.shape(shape, &SchemaSourcePath::root())?,
        SchemaRoot::Union(arms) => {
            for (index, arm) in arms.iter_mut().enumerate() {
                if let SchemaArm::Inline(shape) = arm {
                    run.shape(shape, &SchemaSourcePath::root().union_arm(index))?;
                }
            }
        }
    }
    // The returned definitions' stored types join the site's table; one
    // already there (the same type imported statically) keeps its entry.
    definitions.absorb(&run.published);
    Ok(run.late)
}

/// Parses the value a template type's slot returned as a type definition,
/// and refuses what returned data may never introduce: an active slot of
/// its own, or a local type reference. This is the one validator of a
/// returned value's shape.
///
/// - a string is a definition (`"null"` is the null type, and a bracket list
///   such as `"[string, number]"` is the union of its items);
/// - an array is a union of definitions, each element converted the same
///   way, so a number element is the definition parser's error, never
///   dropped;
/// - `null`, a mapping, a number, or a boolean is no definition.
///
/// ## Errors
///
/// [`SchemaError::SlotResult`] at `slot` for `property`:
/// [`SlotResultKind::NotATypeDefinition`] for a wrong shape or a definition
/// the parser rejects (its error kept), [`SlotResultKind::NestedSlot`], or
/// [`SlotResultKind::LocalReference`].
pub(crate) fn definition_from_value(slot: &Slot, property: &str, value: &Value) -> Result<PropertyDef, SchemaError> {
    let fail = |kind| located(slot, property, kind, Vec::new());
    let yaml = match value {
        Value::String(text) => serde_yaml_ng::Value::String(text.clone()),
        Value::Array(_) => serde_yaml_ng::to_value(value).map_err(|error| {
            fail(SlotResultKind::NotATypeDefinition(std::sync::Arc::new(SchemaError::Grammar {
                property: property.to_string(),
                message: format!("the array is not a list of definitions: {error}"),
                span: 0..0,
                related: Vec::new(),
            })))
        })?,
        other => {
            return Err(fail(SlotResultKind::NotATypeDefinition(std::sync::Arc::new(SchemaError::Grammar {
                property: property.to_string(),
                message: format!("expected a type definition (a string or an array), found {}", json_kind(other)),
                span: 0..0,
                related: Vec::new(),
            }))));
        }
    };
    let def = parse_type_definition_value(property, &yaml)
        .map_err(|error| fail(SlotResultKind::NotATypeDefinition(std::sync::Arc::new(error))))?;
    if !super::definition_slots(&def).is_empty() {
        return Err(fail(SlotResultKind::NestedSlot));
    }
    if let Some(name) = local_reference(&def) {
        return Err(fail(SlotResultKind::LocalReference(name)));
    }
    Ok(def)
}

/// A union's arms after its template-type arms were replaced.
struct Spliced {
    arms: Vec<PropertyAtom>,
    /// The property markers the replaced arms' postfixes hoisted.
    markers: Vec<Constraint>,
    /// The first slot replaced.
    first: Option<Slot>,
}

struct TypeSubstitution<'r> {
    values: &'r SlotValues,
    site: ExpansionSite<'r>,
    /// The stored definitions the returned definitions reach.
    published: DefinitionTable,
    late: LateDependencies,
}

impl TypeSubstitution<'_> {
    fn shape(&mut self, shape: &mut SchemaShape, path: &SchemaSourcePath) -> Result<(), SchemaError> {
        for (name, def) in &mut shape.properties {
            self.def(def, &path.property(name.as_str()))?;
        }
        for pattern in &mut shape.pattern_keys {
            self.def(&mut pattern.def, &path.property(pattern_key_source_name(&pattern.key)))?;
        }
        Ok(())
    }

    /// Replaces the template types of `def` and of everything nested in it.
    /// Returns the first slot replaced in `def`'s own union structure (its
    /// top level or a grouped union at any depth of its arms), after
    /// checking the union invariants `def` must keep with it. Objects and
    /// tuples nested in `def` check their own definitions.
    fn def(&mut self, def: &mut PropertyDef, path: &SchemaSourcePath) -> Result<Option<Slot>, SchemaError> {
        let replaced = match def {
            PropertyDef::Single(atom) => match &atom.ty {
                TypeExpr::Slot(slot) if self.values.get(slot).is_some() => {
                    let (slot, atom) = (slot.clone(), atom.clone());
                    *def = self.expand(&atom, false, path)?;
                    Some(slot)
                }
                _ => self.atom(atom, path)?,
            },
            PropertyDef::Union(arms) => {
                let mut inner = None;
                for (index, arm) in arms.iter_mut().enumerate() {
                    if let Some(slot) = self.atom(arm, &path.union_arm(index))? {
                        inner.get_or_insert(slot);
                    }
                }
                let Spliced { arms: spliced, markers, first } = self.splice(std::mem::take(arms), path)?;
                *def = if markers.is_empty() {
                    PropertyDef::Union(spliced)
                } else {
                    // As the import engine builds a union whose postfixed arm
                    // hoisted property markers.
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Union(spliced),
                        is_array: false,
                        constraints: markers,
                        array_constraints: Vec::new(),
                        description: None,
                    })
                };
                first.or(inner)
            }
        };
        if let Some(slot) = &replaced {
            def.check_union_invariants(&path.to_string())
                .map_err(|error| located(slot, &path.to_string(), SlotResultKind::InvalidExpansion(grammar_message(error)), Vec::new()))?;
        }
        Ok(replaced)
    }

    /// Replaces the template types inside `atom`, which is not itself a
    /// template type. A grouped union (`({{ … }} | string)[]`) is
    /// substituted, not only traversed, at every depth: its own arms first,
    /// then its template-type arms spliced. Returns the first slot replaced
    /// in such a group, for the enclosing definition's invariant check.
    fn atom(&mut self, atom: &mut PropertyAtom, path: &SchemaSourcePath) -> Result<Option<Slot>, SchemaError> {
        match &mut atom.ty {
            TypeExpr::InlineObject(shape) => self.shape(shape, path).map(|()| None),
            TypeExpr::Union(arms) => {
                let mut inner = None;
                for (index, arm) in arms.iter_mut().enumerate() {
                    if let Some(slot) = self.atom(arm, &path.union_arm(index))? {
                        inner.get_or_insert(slot);
                    }
                }
                Ok(self.atom_union(atom, path)?.or(inner))
            }
            TypeExpr::Tuple(tuple) => {
                for (index, element) in tuple.elements.iter_mut().enumerate() {
                    let element_path = path.union_arm(index);
                    if let Some(slot) = self.def(&mut element.def, &element_path)? {
                        TupleType::check_element(&element.def).map_err(|message| {
                            located(&slot, &element_path.to_string(), SlotResultKind::InvalidExpansion(message), Vec::new())
                        })?;
                    }
                }
                Ok(None)
            }
            // The grammar refuses a template type in a signature, a spread,
            // and a `property(...)` declaration; a template-type arm of a
            // union is spliced by the union's own pass.
            TypeExpr::Function(_)
            | TypeExpr::Primitive(_)
            | TypeExpr::Imported { .. }
            | TypeExpr::Ref(_)
            | TypeExpr::Slot(_) => Ok(None),
        }
    }

    /// Splices the returned definitions of the template-type arms of a
    /// grouped union atom (`({{ … }} | string)[]`) into it; the markers a
    /// postfixed arm hoists join the atom's own list (its array list for an
    /// array), as expansion places them.
    fn atom_union(&mut self, atom: &mut PropertyAtom, path: &SchemaSourcePath) -> Result<Option<Slot>, SchemaError> {
        let TypeExpr::Union(arms) = &mut atom.ty else {
            return Ok(None);
        };
        let Spliced { arms: spliced, markers, first: replaced } = self.splice(std::mem::take(arms), path)?;
        atom.ty = TypeExpr::Union(spliced);
        let list = if atom.is_array { &mut atom.array_constraints } else { &mut atom.constraints };
        for marker in markers {
            if !list.contains(&marker) {
                list.push(marker);
            }
        }
        Ok(replaced)
    }

    /// The arms of a union with every template-type arm replaced by the arms
    /// of its returned definition, the property markers those hoisted, and
    /// the first slot replaced.
    fn splice(&mut self, arms: Vec<PropertyAtom>, path: &SchemaSourcePath) -> Result<Spliced, SchemaError> {
        let mut spliced = Vec::with_capacity(arms.len());
        let mut markers: Vec<Constraint> = Vec::new();
        let mut first = None;
        for (index, arm) in arms.into_iter().enumerate() {
            let TypeExpr::Slot(slot) = &arm.ty else {
                spliced.push(arm);
                continue;
            };
            if self.values.get(slot).is_none() {
                spliced.push(arm);
                continue;
            }
            first.get_or_insert_with(|| slot.clone());
            match self.expand(&arm, true, &path.union_arm(index))? {
                PropertyDef::Union(returned) => spliced.extend(returned),
                PropertyDef::Single(PropertyAtom { ty: TypeExpr::Union(returned), is_array: false, constraints, .. }) => {
                    spliced.extend(returned);
                    for marker in constraints {
                        if !markers.contains(&marker) {
                            markers.push(marker);
                        }
                    }
                }
                PropertyDef::Single(atom) => spliced.push(atom),
            }
        }
        Ok(Spliced { arms: spliced, markers, first })
    }

    /// The definition the template type `atom` stands for at `path`.
    fn expand(&mut self, atom: &PropertyAtom, as_arm: bool, path: &SchemaSourcePath) -> Result<PropertyDef, SchemaError> {
        let TypeExpr::Slot(slot) = &atom.ty else {
            unreachable!("only a template type is expanded")
        };
        let value = self.values.get(slot).expect("the caller checked the slot's value");
        let property = path.to_string();
        let mut returned = definition_from_value(slot, &property, value)?;
        let site = self.site;
        refusal_first(rehome_returned_examples(&mut returned, slot, &site, &mut self.late.searched))?;

        // The bare expansion: only what the returned data introduced.
        let mut reached = DefinitionTable::default();
        let bare = refusal_first(expand_template_definition(&returned, atom, true, as_arm, slot, &site, &mut reached))?;
        self.late.imports.extend(bare.imports);
        self.late.searched.extend(bare.searched);
        check_introduced(slot, &property, &bare.def, &reached)?;
        lowers(&bare.def, &reached, &property)
            .map_err(|error| located(slot, &property, SlotResultKind::NotATypeDefinition(std::sync::Arc::new(error)), Vec::new()))?;

        // The use: the written suffix merged by the named-type rules.
        let used = refusal_first(expand_template_definition(&returned, atom, false, as_arm, slot, &site, &mut self.published))
            .map_err(|error| suffix_error(slot, &property, error))?;
        self.late.imports.extend(used.imports);
        self.late.searched.extend(used.searched);
        // A suffix constraint the returned type cannot take fails at the
        // slot, not later as a broken schema. A template constraint left in
        // the suffix is judged once the constraint pass fills it.
        if super::definition_slots(&used.def).is_empty() {
            lowers(&used.def, &self.published, &property).map_err(|error| suffix_error(slot, &property, error))?;
        }
        Ok(used.def)
    }
}

/// `result`, unless a read was refused while it was produced: the refusal
/// then stands whatever the engine made of it (its discovery swallows
/// resolution errors), so a denied read never yields a definition.
fn refusal_first<T>(result: Result<T, SchemaError>) -> Result<T, SchemaError> {
    match super::reads::recorded_refusal() {
        Some((slot, refusal)) => Err(SchemaError::SlotReadRefused { slot: Box::new(slot), refusal }),
        None => result,
    }
}

/// The checks on a bare expansion: no definition it reaches holds an active
/// slot, and every pattern it introduces passes the slot-pattern rules.
fn check_introduced(slot: &Slot, property: &str, def: &PropertyDef, reached: &DefinitionTable) -> Result<(), SchemaError> {
    if let Some(found) = super::reachable_slots(def, reached).into_iter().next() {
        let related = vec![SlotRelated {
            message: format!(
                "`{{{{ {} }}}}` is reachable from the returned definition{}",
                found.expression,
                found.origin.as_ref().map(|file| format!(" through `{}`", file.display())).unwrap_or_default()
            ),
            file: found.origin.clone(),
            span: found.origin.is_some().then(|| found.span.clone()),
        }];
        return Err(located(slot, property, SlotResultKind::ReachableSlot, related));
    }
    let mut patterns = Vec::new();
    def_patterns(def, &mut patterns);
    let mut pending: Vec<DefKey> = Vec::new();
    crate::simplified::query::collect_def_refs(def, &mut pending);
    let mut seen = std::collections::HashSet::new();
    while let Some(key) = pending.pop() {
        if let Some(definition) = reached.get(&key)
            && seen.insert(key)
        {
            def_patterns(&definition.body, &mut patterns);
            crate::simplified::query::collect_def_refs(&definition.body, &mut pending);
        }
    }
    for pattern in patterns {
        check_slot_pattern(&pattern).map_err(|reason| located(slot, property, SlotResultKind::PatternRejected(reason), Vec::new()))?;
    }
    Ok(())
}

/// Whether `def` lowers to JSON Schema against `definitions`, as the
/// property it becomes will be.
fn lowers(def: &PropertyDef, definitions: &DefinitionTable, property: &str) -> Result<(), SchemaError> {
    let mut shape = SchemaShape::default();
    shape.properties.insert(property.to_string(), def.clone());
    crate::simplified::to_json_schema_with_definitions(&SchemaRoot::Single(shape), definitions).map(drop)
}

/// A failure of the expansion with the written suffix. The bare expansion
/// succeeded, so a merge or invariant failure is the suffix's: a conflicting
/// default, or a constraint the returned type cannot take, is
/// [`SlotResultKind::SuffixConflict`], any other
/// definition error [`SlotResultKind::InvalidExpansion`]. Refusals and
/// import cycles keep their own variants.
fn suffix_error(slot: &Slot, property: &str, error: SchemaError) -> SchemaError {
    let suffix = || SlotRelated {
        message: "the suffix written after the slot".to_string(),
        file: slot.origin.clone(),
        span: None,
    };
    match error {
        error @ (SchemaError::SlotReadRefused { .. } | SchemaError::ImportCycle { .. } | SchemaError::SlotResult { .. }) => error,
        error @ (SchemaError::ConstraintConflict { .. } | SchemaError::Convert { .. }) => {
            located(slot, property, SlotResultKind::SuffixConflict(returned_type_wording(&error.to_string())), vec![suffix()])
        }
        error => located(
            slot,
            property,
            SlotResultKind::InvalidExpansion(returned_type_wording(&grammar_message(error))),
            vec![suffix()],
        ),
    }
}

/// `message` with the name the expansion gave the returned definition
/// spelled as what it is; that name is never written by anyone.
fn returned_type_wording(message: &str) -> String {
    let name = crate::resolve::TEMPLATE_TYPE_NAME;
    message
        .replace(&format!("{name}: "), "")
        .replace(&format!("named type `{name}`"), "the returned type")
        .replace(&format!("`{name}`"), "the returned type")
}

/// A grammar error's message alone; another error's text.
fn grammar_message(error: SchemaError) -> String {
    match error {
        SchemaError::Grammar { message, .. } => message,
        other => other.to_string(),
    }
}

fn located(slot: &Slot, property: &str, kind: SlotResultKind, related: Vec<SlotRelated>) -> SchemaError {
    match slot_error(slot.clone(), kind) {
        SchemaError::SlotResult { slot, kind, .. } => {
            SchemaError::SlotResult { slot, kind, property: Some(property.to_string()), related }
        }
        other => other,
    }
}

/// The first local type (`Name` or `Name@this`) `def` names, at any depth.
fn local_reference(def: &PropertyDef) -> Option<String> {
    let mut found = None;
    visit_def(def, &mut |atom| {
        if found.is_none()
            && let TypeExpr::Imported { name, .. } = &atom.ty
            && atom.ty.is_local_reference()
        {
            found = Some(name.clone());
        }
    });
    found
}

/// Every pattern `def` holds at any depth: `pattern(...)` constraints and
/// `<pattern::…>` keys.
fn def_patterns(def: &PropertyDef, out: &mut Vec<String>) {
    visit_def(def, &mut |atom| {
        for constraint in atom.constraints.iter().chain(&atom.array_constraints) {
            if let Constraint::Pattern(pattern) = constraint {
                out.push(pattern.clone());
            }
        }
        if let TypeExpr::InlineObject(shape) = &atom.ty {
            for constraint in &shape.constraints {
                if let Constraint::Pattern(pattern) = constraint {
                    out.push(pattern.clone());
                }
            }
            for key in &shape.pattern_keys {
                if let PatternKey::Pattern(pattern) = &key.key {
                    out.push(pattern.clone());
                }
            }
        }
    });
}

/// Calls `visit` on every atom of `def`, nested ones included.
fn visit_def(def: &PropertyDef, visit: &mut dyn FnMut(&PropertyAtom)) {
    match def {
        PropertyDef::Single(atom) => visit_atom(atom, visit),
        PropertyDef::Union(arms) => arms.iter().for_each(|atom| visit_atom(atom, visit)),
    }
}

fn visit_atom(atom: &PropertyAtom, visit: &mut dyn FnMut(&PropertyAtom)) {
    visit(atom);
    for constraint in atom.constraints.iter().chain(&atom.array_constraints) {
        if let Constraint::Property { declaration, .. } = constraint {
            visit_def(declaration, visit);
        }
    }
    match &atom.ty {
        TypeExpr::InlineObject(shape) => {
            for def in shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)) {
                visit_def(def, visit);
            }
            for constraint in &shape.constraints {
                if let Constraint::Property { declaration, .. } = constraint {
                    visit_def(declaration, visit);
                }
            }
        }
        TypeExpr::Union(arms) => arms.iter().for_each(|arm| visit_atom(arm, visit)),
        TypeExpr::Tuple(tuple) => {
            tuple.elements.iter().for_each(|element| visit_def(&element.def, visit));
            if let Some(spread) = &tuple.spread {
                visit_atom(spread, visit);
            }
        }
        TypeExpr::Function(function) => {
            if let FunctionParameters::List(parameters) = &function.parameters {
                parameters.iter().for_each(|parameter| visit_def(&parameter.def, visit));
            }
            visit_def(&function.returns, visit);
        }
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) | TypeExpr::Slot(_) => {}
    }
}
