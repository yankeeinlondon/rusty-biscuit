//! Projects Darkmatter's typed catalogs into the generated lifecycle verb
//! types of `darkmatter/schemas/partials/lifecycle.yaml`.
//!
//! Every expression function (canonical name) and every side-effect verb but
//! `set` is a generated lifecycle verb. Each gets a `<verb>-short` type, the
//! values its short form accepts, and, unless an overload is variadic, a
//! `<verb>-long` type, the closed named-parameter maps its long form accepts.
//! Both are built from the parsed catalog signatures (each descriptor's
//! `function_type`), never from display strings or a local type table, and
//! keep one arm per overload so a call is never accepted by a merge of
//! overloads.
//!
//! The action value's shape decides how the parser reads it, and the
//! projection follows it:
//!
//! - `null` is a call with no arguments, as is `[]`;
//! - a list is the positional arguments, one per item;
//! - a mapping is the long form;
//! - any other scalar is a call with that one argument.
//!
//! So a scalar form takes only the scalar types of the first parameter: a
//! literal list or mapping there is never one argument, and an `unknown`
//! first parameter's scalar form is a string, number, or boolean. A
//! parameter that takes a list or mapping still gets a scalar form for a
//! template, which can evaluate to one ([`TEMPLATE_OPERAND`]).
//!
//! Every operand type is the catalog type with `deferred` added to each atom
//! that is not `unknown` or `null`, so a `{{ … }}` template is accepted there
//! and checked once the event renders it. The schema system makes an
//! optional object property accept `null` and never requires a property
//! whose type admits `null`; a long form therefore spells presence through
//! its arms instead of through optionality. Each subset of the optional
//! parameters is its own closed arm with every present parameter marked
//! `required`, and an arm holding a parameter whose type admits `null` counts
//! its keys with `min-keys`. A tuple's optional position already rejects
//! `null` unless its type admits it.
//!
//! The projected types alone cannot carry a runtime registration's own rules
//! (`number`'s and `round`'s numeric recovery, `length`'s refusal of a
//! boolean, the binder's overload preference). Every arm but a bare `null`
//! or a named reference therefore carries `operands(<verb>-short)` or
//! `operands(<verb>-long)`, and schema validation binds a value so typed
//! through the verb's runtime registration: the binder's verdict, not the
//! arms', decides its literal operands.
//!
//! The generator entry point is the `lifecycle_schema_generator` example
//! (`just regen-lifecycle-schema`); builds embed the checked-in YAML.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::effects::{EFFECT_DESCRIPTORS, effect_function_catalog, effect_function_descriptors};
use crate::markdown::compose::expression::{
    ExpressionFunctionDescriptor, FunctionCatalog, expression_function_catalog, expression_function_descriptors,
};
use crate::markdown::schemas::simplified::grammar::parse_type_expr;
use crate::markdown::schemas::simplified::{
    Constraint, FunctionParameter, FunctionParameters, FunctionType, LOCAL_REFERENCE, PropertyAtom, PropertyDef,
    SchemaShape, SimplifiedType, TupleElement, TupleType, TypeExpr, serialize_property_atom,
};

use super::LOOP_ITERATION_KEYS;
use super::actions::{CommunicationChannel, is_lifecycle_control_verb};

/// The keys a lifecycle action map reserves beside its verbs.
const RESERVED_KEYS: &[&str] = &["when", "then", "else", "no_error"];

/// The long-form key every generated action accepts beside its parameters.
const NO_ERROR: &str = "no_error";

/// A generated region of `partials/lifecycle.yaml`, bounded by
/// `# BEGIN GENERATED <name>` and `# END GENERATED <name>` lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedRegion {
    /// The helper types and the per-verb `-short`/`-long` types, in `types:`.
    Types,
    /// The generated keys of the `expression` category.
    Expression,
    /// The generated keys of `bundle`.
    Bundle,
    /// The generated keys of `initialize-bundle`.
    InitializeBundle,
}

impl GeneratedRegion {
    /// Every region, in file order.
    pub const ALL: [Self; 4] = [Self::Types, Self::Expression, Self::Bundle, Self::InitializeBundle];

    /// The name its marker lines carry.
    pub fn name(self) -> &'static str {
        match self {
            Self::Types => "types",
            Self::Expression => "expression",
            Self::Bundle => "bundle",
            Self::InitializeBundle => "initialize-bundle",
        }
    }

    #[allow(missing_docs)]
    pub fn begin_marker(self) -> String {
        format!("# BEGIN GENERATED {}", self.name())
    }

    #[allow(missing_docs)]
    pub fn end_marker(self) -> String {
        format!("# END GENERATED {}", self.name())
    }
}

/// Which catalog a generated verb comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerbOrigin {
    /// `darkmatter/schemas/partials/mutations.yaml`.
    SideEffect,
    /// `darkmatter/schemas/partials/functions.yaml`.
    ExpressionFunction,
}

/// Why the projection cannot be built. Each one stops generation rather than
/// dropping or loosening a verb.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProjectionError {
    /// A side-effect signature has no typed catalog overload with the same
    /// verb and parameter names.
    #[error("the side effect `{signature}` has no typed entry in mutations.yaml")]
    MissingEffectType { signature: String },
    /// A typed side-effect overload names no side-effect signature.
    #[error("the typed side effect `{signature}` matches no side-effect descriptor")]
    UndescribedEffect { signature: String },
    /// A generated verb's name is already taken.
    #[error("the generated verb `{verb}` collides with {owner}")]
    Collision { verb: String, owner: String },
    /// A long-form parameter is named like the action-level `no_error` flag.
    #[error("`{verb}` has a parameter named `no_error`, which its long form reserves")]
    NoErrorParameter { verb: String },
    /// A parameter references a type the catalogs do not define, or one in
    /// another file.
    #[error("`{verb}` references the type `{name}`, which no catalog defines as a local helper type")]
    UnknownHelperType { verb: String, name: String },
    /// A parameter type the projection has no rule for.
    #[error("`{verb}` parameter `{parameter}` has the type `{type_text}`, which has no lifecycle projection")]
    UnsupportedParameterType {
        verb: String,
        parameter: String,
        type_text: String,
    },
    /// Two generated types share a name.
    #[error("the generated type name `{name}` is used twice")]
    DuplicateTypeName { name: String },
    /// A generated region's markers are missing, repeated, or out of order.
    #[error("`{marker}` must appear exactly once, before its END marker")]
    Marker { marker: String },
}

/// A name a catalog defines that is not a generated verb because a
/// hand-written lifecycle definition owns it there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCollision {
    #[allow(missing_docs)]
    pub verb: &'static str,
    /// Where the catalog entry would have been read.
    pub catalog: VerbOrigin,
    /// The definition that wins, and where.
    pub winner: &'static str,
}

/// The typed catalogs a projection reads.
#[derive(Debug, Clone)]
pub struct ProjectionCatalogs<'a> {
    /// Every expression-function overload.
    pub expression_functions: &'a [ExpressionFunctionDescriptor],
    /// Every typed side-effect overload.
    pub side_effects: &'a [ExpressionFunctionDescriptor],
    /// The side-effect inventory's signatures (`ensure_file(file, content)`):
    /// the verbs and parameter names the typed side effects must cover.
    pub side_effect_signatures: Vec<&'a str>,
    /// The catalogs whose named helper types parameters may reference.
    pub helper_catalogs: Vec<&'a FunctionCatalog>,
}

impl ProjectionCatalogs<'static> {
    /// Darkmatter's embedded catalogs and side-effect inventory.
    pub fn shipped() -> Self {
        Self {
            expression_functions: expression_function_descriptors(),
            side_effects: effect_function_descriptors(),
            side_effect_signatures: EFFECT_DESCRIPTORS.iter().map(|descriptor| descriptor.signature).collect(),
            helper_catalogs: vec![expression_function_catalog(), effect_function_catalog()],
        }
    }
}

impl ProjectionCatalogs<'_> {
    fn helper_type(&self, name: &str) -> Option<&PropertyDef> {
        self.helper_catalogs.iter().find_map(|catalog| catalog.named_type(name))
    }
}

/// What one overload contributes to its verb's types.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedOverload {
    /// The overload's index among its function's overloads, in catalog
    /// order (the binder's overload index).
    pub index: usize,
    /// The arms of a call with no arguments (`null`, `[]`); empty when the
    /// overload takes at least one argument.
    pub zero_arguments: Vec<PropertyAtom>,
    /// The values a one-argument scalar call may hold.
    pub scalar: Vec<PropertyAtom>,
    /// The positional list (a tuple, or a list for a rest-only overload);
    /// `None` when the overload has no parameters.
    pub positional: Option<PropertyAtom>,
    /// The long-form arms; empty for a variadic verb.
    pub long: Vec<PropertyAtom>,
}

impl ProjectedOverload {
    /// The short-form arms this overload alone contributes, in the order the
    /// verb's short type lists them.
    pub fn short_arms(&self) -> Vec<PropertyAtom> {
        let mut arms = self.zero_arguments.clone();
        arms.extend(self.scalar.iter().cloned());
        arms.extend(self.positional.iter().cloned());
        arms
    }
}

/// One generated verb's types.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedVerb {
    #[allow(missing_docs)]
    pub name: String,
    #[allow(missing_docs)]
    pub overloads: Vec<ProjectedOverload>,
    /// The `<verb>-short` arms, deduplicated: zero-argument forms, then each
    /// overload's scalar arms, then each overload's positional arm.
    pub short: Vec<PropertyAtom>,
    /// The `<verb>-long` arms, deduplicated; `None` for a variadic verb.
    pub long: Option<Vec<PropertyAtom>>,
}

impl ProjectedVerb {
    /// Projects one function's overloads, in catalog order. `helper` resolves
    /// a local named type a parameter references.
    ///
    /// ## Errors
    ///
    /// [`ProjectionError::NoErrorParameter`],
    /// [`ProjectionError::UnsupportedParameterType`], or
    /// [`ProjectionError::UnknownHelperType`].
    pub fn from_function_types<'h>(
        name: &str,
        overloads: &[&FunctionType],
        helper: &dyn Fn(&str) -> Option<&'h PropertyDef>,
    ) -> Result<Self, ProjectionError> {
        let parameters: Vec<&[FunctionParameter]> = overloads.iter().map(|function| parameters_of(function)).collect();
        let variadic = parameters.iter().any(|list| list.iter().any(|parameter| parameter.rest));
        let mut projected = Vec::with_capacity(overloads.len());
        for (index, list) in parameters.iter().enumerate() {
            let required = list.iter().filter(|parameter| !parameter.optional && !parameter.rest).count();
            let takes = |count: usize| {
                count >= required && (list.iter().any(|parameter| parameter.rest) || count <= list.len())
            };
            let short_name = format!("{name}-short");
            let short_arm = |atom: PropertyAtom| bind_operands(atom, &short_name);
            let scalar = match list.first() {
                Some(first) if takes(1) => scalar_arms(name, first, helper)?.into_iter().map(short_arm).collect(),
                _ => Vec::new(),
            };
            let long_name = format!("{name}-long");
            let long = if variadic {
                Vec::new()
            } else {
                long_arms(name, list)?.into_iter().map(|atom| bind_operands(atom, &long_name)).collect()
            };
            projected.push(ProjectedOverload {
                index,
                zero_arguments: if takes(0) {
                    zero_argument_arms().into_iter().map(short_arm).collect()
                } else {
                    Vec::new()
                },
                scalar,
                positional: positional_arm(list).map(short_arm),
                long,
            });
        }
        let mut short = Vec::new();
        for overload in &projected {
            push_unique(&mut short, overload.zero_arguments.iter().cloned());
        }
        for overload in &projected {
            push_unique(&mut short, overload.scalar.iter().cloned());
        }
        for overload in &projected {
            push_unique(&mut short, overload.positional.iter().cloned());
        }
        let long = (!variadic).then(|| {
            let mut long = Vec::new();
            for overload in &projected {
                push_unique(&mut long, overload.long.iter().cloned());
            }
            long
        });
        Ok(Self {
            name: name.to_string(),
            overloads: projected,
            short,
            long,
        })
    }

    #[allow(missing_docs)]
    pub fn short_type_name(&self) -> String {
        format!("{}-short", self.name)
    }

    /// `None` for a variadic verb, which has no long form.
    pub fn long_type_name(&self) -> Option<String> {
        self.long.as_ref().map(|_| format!("{}-long", self.name))
    }
}

/// The generated lifecycle verb types, built from the typed catalogs.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleSchemaProjection {
    /// Side effects, then expression functions, each sorted by name.
    pub verbs: Vec<(VerbOrigin, ProjectedVerb)>,
    /// The catalog helper types parameters reference, copied verbatim from
    /// their catalog, sorted by name.
    pub helper_types: Vec<(String, PropertyDef)>,
    /// Catalog names a hand-written definition owns instead.
    pub collisions: Vec<RecordedCollision>,
    /// Functions with a callback (`function`-typed) parameter, which no YAML
    /// value can carry and so have no generated verb.
    pub callback_exclusions: Vec<String>,
}

impl LifecycleSchemaProjection {
    /// Projects Darkmatter's embedded catalogs.
    ///
    /// ## Errors
    ///
    /// See [`Self::project`].
    pub fn shipped() -> Result<Self, ProjectionError> {
        Self::project(&ProjectionCatalogs::shipped())
    }

    /// Projects `catalogs`.
    ///
    /// ## Errors
    ///
    /// A [`ProjectionError`] for missing side-effect metadata, a new name
    /// collision, a reserved or unsupported parameter, or an unknown helper
    /// type.
    pub fn project(catalogs: &ProjectionCatalogs<'_>) -> Result<Self, ProjectionError> {
        check_effect_coverage(catalogs)?;

        let mut collisions = Vec::new();
        let mut callback_exclusions = Vec::new();
        let mut functions: BTreeMap<(VerbOrigin, &str), Vec<&ExpressionFunctionDescriptor>> = BTreeMap::new();
        for (origin, descriptors) in [
            (VerbOrigin::SideEffect, catalogs.side_effects),
            (VerbOrigin::ExpressionFunction, catalogs.expression_functions),
        ] {
            for descriptor in descriptors {
                functions.entry((origin, &*descriptor.name)).or_default().push(descriptor);
            }
        }

        let mut verbs = Vec::new();
        let mut helper_names: Vec<String> = Vec::new();
        for ((origin, name), mut overloads) in functions {
            if origin == VerbOrigin::SideEffect && name == "set" {
                collisions.push(RecordedCollision {
                    verb: "set",
                    catalog: origin,
                    winner: "lifecycle `set`, a mapping of destinations to values, in every action map",
                });
                continue;
            }
            if let Some(owner) = hand_owner(name) {
                return Err(ProjectionError::Collision {
                    verb: name.to_string(),
                    owner: owner.to_string(),
                });
            }
            if origin == VerbOrigin::ExpressionFunction
                && catalogs.side_effects.iter().any(|descriptor| &*descriptor.name == name)
            {
                return Err(ProjectionError::Collision {
                    verb: name.to_string(),
                    owner: "a side effect of the same name".to_string(),
                });
            }
            if let Some(&key) = LOOP_ITERATION_KEYS.iter().find(|key| **key == name) {
                collisions.push(RecordedCollision {
                    verb: key,
                    catalog: origin,
                    winner: "the loop iteration control at a `loop:` root; the verb everywhere else",
                });
            }
            overloads.sort_by_key(|descriptor| descriptor.overload);
            let types: Vec<&FunctionType> = overloads.iter().map(|descriptor| &descriptor.function_type).collect();
            if types.iter().any(|function| has_callback_parameter(function)) {
                callback_exclusions.push(name.to_string());
                continue;
            }
            for function in &types {
                for parameter in parameters_of(function) {
                    collect_helper_names(name, &parameter.def, catalogs, &mut helper_names)?;
                }
            }
            let helper = |helper_name: &str| catalogs.helper_type(helper_name);
            verbs.push((origin, ProjectedVerb::from_function_types(name, &types, &helper)?));
        }
        helper_names.sort_unstable();
        let helper_types = helper_names
            .into_iter()
            .map(|name| {
                let def = catalogs.helper_type(&name).cloned().expect("collected helper names resolve");
                (name, def)
            })
            .collect();

        let projection = Self {
            verbs,
            helper_types,
            collisions,
            callback_exclusions,
        };
        projection.check_type_names()?;
        Ok(projection)
    }

    /// The verb named `name`.
    pub fn verb(&self, name: &str) -> Option<&ProjectedVerb> {
        self.verbs.iter().map(|(_, verb)| verb).find(|verb| verb.name == name)
    }

    fn check_type_names(&self) -> Result<(), ProjectionError> {
        let mut seen = std::collections::HashSet::new();
        let names = std::iter::once(TEMPLATE_OPERAND.to_string())
            .chain(self.helper_types.iter().map(|(name, _)| name.clone()))
            .chain(
                self.verbs
                    .iter()
                    .flat_map(|(_, verb)| std::iter::once(verb.short_type_name()).chain(verb.long_type_name())),
            );
        for name in names {
            if !seen.insert(name.clone()) {
                return Err(ProjectionError::DuplicateTypeName { name });
            }
        }
        Ok(())
    }

    /// The lines of one generated region, without its marker lines, each
    /// ending in a newline.
    pub fn render_region(&self, region: GeneratedRegion) -> String {
        let mut out = String::new();
        match region {
            GeneratedRegion::Types => {
                let template = template_operand_type();
                let helpers = std::iter::once((TEMPLATE_OPERAND, &template))
                    .chain(self.helper_types.iter().map(|(name, def)| (name.as_str(), def)));
                for (name, def) in helpers {
                    let arms = match def {
                        PropertyDef::Single(atom) => std::slice::from_ref(atom),
                        PropertyDef::Union(arms) => arms.as_slice(),
                    };
                    write_entry(&mut out, "    ", name, arms);
                }
                for (_, verb) in &self.verbs {
                    write_entry(&mut out, "    ", &verb.short_type_name(), &verb.short);
                    if let (Some(name), Some(long)) = (verb.long_type_name(), &verb.long) {
                        write_entry(&mut out, "    ", &name, long);
                    }
                }
            }
            GeneratedRegion::Expression => {
                for (_, verb) in &self.verbs {
                    match verb.long_type_name() {
                        Some(long) => {
                            let _ = writeln!(out, "        {}:", verb.name);
                            let _ = writeln!(out, "            - {}", verb.short_type_name());
                            let _ = writeln!(out, "            - {long}");
                        }
                        None => {
                            let _ = writeln!(out, "        {}: {}", verb.name, verb.short_type_name());
                        }
                    }
                }
            }
            GeneratedRegion::Bundle | GeneratedRegion::InitializeBundle => {
                for (_, verb) in &self.verbs {
                    let _ = writeln!(out, "        {}: {}", verb.name, verb.short_type_name());
                }
            }
        }
        out
    }

    /// `text` with each generated region's lines replaced by
    /// [`Self::render_region`]; every other line is kept byte for byte.
    ///
    /// ## Errors
    ///
    /// [`ProjectionError::Marker`] when a region's markers are missing,
    /// repeated, or out of order.
    pub fn rewrite(&self, text: &str) -> Result<String, ProjectionError> {
        let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
        for region in GeneratedRegion::ALL {
            let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
            let (begin, end) = region_bounds(&borrowed, region)?;
            let replacement: Vec<String> =
                self.render_region(region).split_inclusive('\n').map(str::to_string).collect();
            lines.splice(begin + 1..end, replacement);
        }
        Ok(lines.concat())
    }
}

/// The line indexes of `region`'s BEGIN and END markers in `lines`.
///
/// ## Errors
///
/// [`ProjectionError::Marker`] unless each marker appears exactly once, BEGIN
/// first.
pub fn region_bounds(lines: &[&str], region: GeneratedRegion) -> Result<(usize, usize), ProjectionError> {
    let find = |marker: String| {
        let found: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.trim() == marker)
            .map(|(index, _)| index)
            .collect();
        match found.as_slice() {
            [index] => Ok(*index),
            _ => Err(ProjectionError::Marker { marker }),
        }
    };
    let begin = find(region.begin_marker())?;
    let end = find(region.end_marker())?;
    if end < begin {
        return Err(ProjectionError::Marker {
            marker: region.begin_marker(),
        });
    }
    Ok((begin, end))
}

/// The hand-written definition that owns `name`, if any.
fn hand_owner(name: &str) -> Option<&'static str> {
    if CommunicationChannel::from_verb(name).is_some() {
        Some("a communication channel")
    } else if name == "shell" {
        Some("the `shell` action")
    } else if is_lifecycle_control_verb(name) {
        Some("a lifecycle control verb")
    } else if name == "set" {
        Some("lifecycle `set`")
    } else if RESERVED_KEYS.contains(&name) {
        Some("a reserved action-map key")
    } else {
        None
    }
}

/// Fails unless every side-effect signature has a typed overload with the
/// same verb and parameter names, and every typed overload has a signature.
fn check_effect_coverage(catalogs: &ProjectionCatalogs<'_>) -> Result<(), ProjectionError> {
    let described: Vec<(String, Vec<String>)> =
        catalogs.side_effect_signatures.iter().map(|signature| signature_key(signature)).collect();
    let typed: Vec<(String, Vec<String>)> = catalogs
        .side_effects
        .iter()
        .map(|descriptor| {
            let names = descriptor.parameters.iter().map(|parameter| parameter.name.to_string()).collect();
            (descriptor.name.to_string(), names)
        })
        .collect();
    let render = |(verb, parameters): &(String, Vec<String>)| format!("{verb}({})", parameters.join(", "));
    if let Some(missing) = described.iter().find(|key| !typed.contains(key)) {
        return Err(ProjectionError::MissingEffectType {
            signature: render(missing),
        });
    }
    if let Some(extra) = typed.iter().find(|key| !described.contains(key)) {
        return Err(ProjectionError::UndescribedEffect { signature: render(extra) });
    }
    Ok(())
}

/// The verb and parameter names of an inventory signature such as
/// `ensure_file(file, content)`.
fn signature_key(signature: &str) -> (String, Vec<String>) {
    let (verb, rest) = signature.split_once('(').unwrap_or((signature, ")"));
    let parameters = rest
        .trim_end()
        .trim_end_matches(')')
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    (verb.trim().to_string(), parameters)
}

fn parameters_of(function: &FunctionType) -> &[FunctionParameter] {
    match &function.parameters {
        FunctionParameters::Void => &[],
        FunctionParameters::List(parameters) => parameters,
    }
}

fn has_callback_parameter(function: &FunctionType) -> bool {
    parameters_of(function).iter().any(|parameter| def_any(&parameter.def, &|atom| matches!(atom.ty, TypeExpr::Function(_))))
}

/// Whether `predicate` holds for some atom of `def`, at any depth.
fn def_any(def: &PropertyDef, predicate: &dyn Fn(&PropertyAtom) -> bool) -> bool {
    arms_of(def).iter().any(|atom| atom_any(atom, predicate))
}

fn atom_any(atom: &PropertyAtom, predicate: &dyn Fn(&PropertyAtom) -> bool) -> bool {
    predicate(atom)
        || match &atom.ty {
            TypeExpr::InlineObject(shape) => shape.properties.values().any(|def| def_any(def, predicate)),
            TypeExpr::Tuple(tuple) => tuple.any_atom(|inner| atom_any(inner, predicate)),
            TypeExpr::Union(arms) => arms.iter().any(|arm| atom_any(arm, predicate)),
            _ => false,
        }
}

fn arms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(arms) => arms,
    }
}

/// Adds every local helper type `def` references, transitively, to `names`.
fn collect_helper_names(
    verb: &str,
    def: &PropertyDef,
    catalogs: &ProjectionCatalogs<'_>,
    names: &mut Vec<String>,
) -> Result<(), ProjectionError> {
    let mut referenced = Vec::new();
    collect_references(def, &mut referenced);
    for (name, reference) in referenced {
        let unknown = || ProjectionError::UnknownHelperType {
            verb: verb.to_string(),
            name: name.clone(),
        };
        if reference != LOCAL_REFERENCE {
            return Err(unknown());
        }
        let body = catalogs.helper_type(&name).ok_or_else(unknown)?;
        if !names.contains(&name) {
            names.push(name.clone());
            collect_helper_names(verb, body, catalogs, names)?;
        }
    }
    Ok(())
}

fn collect_references(def: &PropertyDef, out: &mut Vec<(String, String)>) {
    for atom in arms_of(def) {
        collect_atom_references(atom, out);
    }
}

fn collect_atom_references(atom: &PropertyAtom, out: &mut Vec<(String, String)>) {
    match &atom.ty {
        TypeExpr::Imported { name, reference } => out.push((name.clone(), reference.clone())),
        TypeExpr::InlineObject(shape) => shape.properties.values().for_each(|def| collect_references(def, out)),
        TypeExpr::Tuple(tuple) => {
            for element in &tuple.elements {
                collect_references(&element.def, out);
            }
            if let Some(spread) = &tuple.spread {
                collect_atom_references(spread, out);
            }
        }
        TypeExpr::Union(arms) => arms.iter().for_each(|arm| collect_atom_references(arm, out)),
        _ => {}
    }
}

/// `def` with `deferred` added to every atom that is not `unknown` or `null`,
/// at every depth: on a list, both the list and (unless universal) its items.
pub fn defer_operand(def: &PropertyDef) -> PropertyDef {
    match def {
        PropertyDef::Single(atom) => PropertyDef::Single(defer_atom(atom)),
        PropertyDef::Union(arms) => PropertyDef::Union(arms.iter().map(defer_atom).collect()),
    }
}

fn defer_atom(atom: &PropertyAtom) -> PropertyAtom {
    let mut atom = atom.clone();
    let mut items_carry_arms = false;
    atom.ty = match atom.ty {
        TypeExpr::InlineObject(mut shape) => {
            for def in shape.properties.values_mut() {
                *def = defer_operand(def);
            }
            TypeExpr::InlineObject(shape)
        }
        TypeExpr::Tuple(tuple) => TypeExpr::Tuple(
            tuple
                .try_map_defs(|_, def| Ok::<_, std::convert::Infallible>(defer_operand(&def)))
                .unwrap_or_else(|never| match never {}),
        ),
        TypeExpr::Union(arms) => {
            // A grouped union's arms keep their own constraints.
            items_carry_arms = true;
            TypeExpr::Union(arms.iter().map(defer_atom).collect())
        }
        other => other,
    };
    let universal = matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::Unknown | SimplifiedType::Null));
    if !universal && !items_carry_arms && !atom.constraints.contains(&Constraint::Deferred) {
        atom.constraints.push(Constraint::Deferred);
    }
    if atom.is_array && !atom.array_constraints.contains(&Constraint::Deferred) {
        atom.array_constraints.push(Constraint::Deferred);
    }
    atom
}

/// `atom` marked as an arm of the generated verb type `type_name`
/// ([`Constraint::Operands`]), so schema validation binds a value it types
/// through the verb's runtime registration. The bare `null` arm and a named
/// reference carry no constraint of their own; the verb's other arms mark
/// the value.
fn bind_operands(mut atom: PropertyAtom, type_name: &str) -> PropertyAtom {
    let marker = Constraint::Operands(type_name.to_string());
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Null) | TypeExpr::Imported { .. } if !atom.is_array => {}
        _ if atom.is_array => atom.array_constraints.push(marker),
        _ => atom.constraints.push(marker),
    }
    atom
}

/// `null` and `unknown[](max(0))`: a call with no arguments, spelled as the
/// hand-written zero-argument verbs spell it.
fn zero_argument_arms() -> [PropertyAtom; 2] {
    [
        PropertyAtom::bare(SimplifiedType::Null),
        parse_type_expr("zero-arguments", "unknown[](max(0))").expect("the empty-list form parses"),
    ]
}

/// The scalar values a one-argument call may hold: `first`'s scalar types,
/// and [`TEMPLATE_OPERAND`] when `first` also takes a list or mapping, which
/// only a template can supply in scalar position.
fn scalar_arms<'h>(
    verb: &str,
    first: &FunctionParameter,
    helper: &dyn Fn(&str) -> Option<&'h PropertyDef>,
) -> Result<Vec<PropertyAtom>, ProjectionError> {
    let unsupported = |atom: &PropertyAtom| ProjectionError::UnsupportedParameterType {
        verb: verb.to_string(),
        parameter: first.name.clone(),
        type_text: serialize_property_atom(atom),
    };
    let mut arms = Vec::new();
    let mut collection = false;
    for atom in arms_of(&defer_operand(&first.def)) {
        if atom.is_array {
            collection = true;
            continue;
        }
        match &atom.ty {
            TypeExpr::Primitive(SimplifiedType::Null) => {}
            TypeExpr::Primitive(SimplifiedType::Object) | TypeExpr::InlineObject(_) | TypeExpr::Tuple(_) => {
                collection = true;
            }
            TypeExpr::Primitive(SimplifiedType::Unknown) => {
                for scalar in [SimplifiedType::String, SimplifiedType::Number, SimplifiedType::Boolean] {
                    let mut arm = PropertyAtom::bare(scalar);
                    arm.constraints.push(Constraint::Deferred);
                    arms.push(arm);
                }
            }
            TypeExpr::Primitive(
                SimplifiedType::Json | SimplifiedType::Yaml | SimplifiedType::Schema | SimplifiedType::TypeDefinition,
            ) => return Err(unsupported(atom)),
            TypeExpr::Primitive(SimplifiedType::Literal) => {
                let scalar = atom.constraints.iter().any(|constraint| {
                    matches!(constraint, Constraint::LiteralValue(value)
                        if !(value.is_null() || value.is_array() || value.is_object()))
                });
                if scalar {
                    arms.push(atom.clone());
                } else {
                    collection = true;
                }
            }
            TypeExpr::Primitive(_) => arms.push(atom.clone()),
            TypeExpr::Imported { name, .. } => {
                let body = helper(name).ok_or_else(|| ProjectionError::UnknownHelperType {
                    verb: verb.to_string(),
                    name: name.clone(),
                })?;
                let scalar = |arm: &PropertyAtom| {
                    !arm.is_array
                        && matches!(&arm.ty, TypeExpr::Primitive(ty) if !matches!(ty,
                            SimplifiedType::Null | SimplifiedType::Unknown | SimplifiedType::Object
                            | SimplifiedType::Json | SimplifiedType::Yaml | SimplifiedType::Schema
                            | SimplifiedType::TypeDefinition))
                };
                match (arms_of(body).iter().all(scalar), arms_of(body).iter().any(scalar)) {
                    (true, _) => arms.push(atom.clone()),
                    (false, false) => collection = true,
                    (false, true) => return Err(unsupported(atom)),
                }
            }
            TypeExpr::Union(_) | TypeExpr::Function(_) | TypeExpr::Ref(_) => return Err(unsupported(atom)),
        }
    }
    if collection {
        arms.push(template_operand_reference());
    }
    let mut unique = Vec::new();
    push_unique(&mut unique, arms);
    Ok(unique)
}

/// The name of the generated type a scalar form uses for a list or mapping
/// parameter: it accepts a `{{ … }}` template and no literal at all.
pub const TEMPLATE_OPERAND: &str = "template-operand";

/// The definition of [`TEMPLATE_OPERAND`]: a string no literal matches
/// (`[^\s\S]` matches no character), marked `deferred` so a template is
/// accepted unjudged and checked once the event renders it.
pub fn template_operand_type() -> PropertyDef {
    PropertyDef::Single(
        parse_type_expr(
            TEMPLATE_OPERAND,
            "string(pattern('[^\\\\s\\\\S]'); deferred) -> a whole-value {{ … }} template supplying a list or mapping; no literal matches",
        )
        .expect("the template operand parses"),
    )
}

fn template_operand_reference() -> PropertyAtom {
    PropertyAtom {
        ty: TypeExpr::Imported {
            name: TEMPLATE_OPERAND.to_string(),
            reference: LOCAL_REFERENCE.to_string(),
        },
        is_array: false,
        constraints: Vec::new(),
        array_constraints: Vec::new(),
        description: None,
    }
}

/// The positional arm: a tuple of the parameters (a rest parameter as its
/// spread), or a plain list for a rest-only overload.
fn positional_arm(parameters: &[FunctionParameter]) -> Option<PropertyAtom> {
    if parameters.is_empty() {
        return None;
    }
    let mut tuple = TupleType::default();
    for parameter in parameters {
        let def = defer_operand(&parameter.def);
        if parameter.rest {
            tuple.spread = Some(Box::new(list_of(def)));
        } else {
            tuple.elements.push(TupleElement {
                def,
                optional: parameter.optional,
            });
        }
    }
    if tuple.elements.is_empty() {
        return tuple.spread.map(|spread| *spread);
    }
    Some(PropertyAtom {
        ty: TypeExpr::Tuple(tuple),
        is_array: false,
        constraints: Vec::new(),
        array_constraints: Vec::new(),
        description: None,
    })
}

/// A list whose items are `item`.
fn list_of(item: PropertyDef) -> PropertyAtom {
    match item {
        PropertyDef::Single(mut atom) if !atom.is_array => {
            atom.is_array = true;
            atom
        }
        other => PropertyAtom {
            ty: TypeExpr::Union(arms_of(&other).to_vec()),
            is_array: true,
            constraints: Vec::new(),
            array_constraints: Vec::new(),
            description: None,
        },
    }
}

/// One closed object per subset of the optional parameters (none first, then
/// by bitmask in parameter order), each present parameter marked `required`
/// when its type does not admit null. An arm holding a parameter that admits
/// null cannot require it, so it counts its keys with `min-keys`, and its
/// `no_error` flag is a separate arm that counts one more key.
fn long_arms(verb: &str, parameters: &[FunctionParameter]) -> Result<Vec<PropertyAtom>, ProjectionError> {
    if parameters.iter().any(|parameter| parameter.name == NO_ERROR) {
        return Err(ProjectionError::NoErrorParameter { verb: verb.to_string() });
    }
    let optional: Vec<usize> = (0..parameters.len()).filter(|index| parameters[*index].optional).collect();
    let mut arms = Vec::new();
    for mask in 0..(1usize << optional.len()) {
        let mut shape = SchemaShape::new();
        let mut counts_keys = false;
        for (index, parameter) in parameters.iter().enumerate() {
            let position = optional.iter().position(|optional_index| *optional_index == index);
            if position.is_some_and(|bit| mask & (1 << bit) == 0) {
                continue;
            }
            let mut def = defer_operand(&parameter.def);
            if def.admits_null() {
                counts_keys = true;
            } else {
                mark_required(&mut def);
            }
            shape.properties.insert(parameter.name.clone(), def);
        }
        let mut no_error = PropertyAtom::bare(SimplifiedType::Boolean);
        if counts_keys {
            let keys = shape.properties.len();
            arms.push(object_atom(shape.clone(), Some(keys)));
            no_error.constraints.push(Constraint::Required);
            shape.properties.insert(NO_ERROR.to_string(), PropertyDef::Single(no_error));
            arms.push(object_atom(shape, Some(keys + 1)));
        } else {
            shape.properties.insert(NO_ERROR.to_string(), PropertyDef::Single(no_error));
            arms.push(object_atom(shape, None));
        }
    }
    Ok(arms)
}

fn mark_required(def: &mut PropertyDef) {
    let atom = match def {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(arms) => arms.first_mut().expect("a union has arms"),
    };
    atom.constraints.push(Constraint::Required);
}

fn object_atom(shape: SchemaShape, min_keys: Option<usize>) -> PropertyAtom {
    PropertyAtom {
        ty: TypeExpr::InlineObject(shape),
        is_array: false,
        constraints: min_keys.map(Constraint::MinKeys).into_iter().collect(),
        array_constraints: Vec::new(),
        description: None,
    }
}

fn push_unique(into: &mut Vec<PropertyAtom>, atoms: impl IntoIterator<Item = PropertyAtom>) {
    for atom in atoms {
        if !into.contains(&atom) {
            into.push(atom);
        }
    }
}

/// Writes `name` as one type entry: a single atom inline, several as a list.
fn write_entry(out: &mut String, indent: &str, name: &str, arms: &[PropertyAtom]) {
    match arms {
        [atom] => {
            let _ = writeln!(out, "{indent}{name}: {}", yaml_atom(atom));
        }
        _ => {
            let _ = writeln!(out, "{indent}{name}:");
            for atom in arms {
                let _ = writeln!(out, "{indent}    - {}", yaml_atom(atom));
            }
        }
    }
}

/// An atom as a YAML scalar: the bare `null` type as YAML `null`, as the
/// hand-written verbs spell it, and everything else double-quoted.
fn yaml_atom(atom: &PropertyAtom) -> String {
    if *atom == PropertyAtom::bare(SimplifiedType::Null) {
        return "null".to_string();
    }
    let text = serialize_property_atom(atom);
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for character in text.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::grammar::parse_type_definition;

    fn shipped() -> LifecycleSchemaProjection {
        LifecycleSchemaProjection::shipped().expect("the shipped catalogs project")
    }

    fn rendered(atoms: &[PropertyAtom]) -> Vec<String> {
        atoms.iter().map(serialize_property_atom).collect()
    }

    #[test]
    fn every_generated_atom_reparses_to_itself() {
        let projection = shipped();
        for (_, verb) in &projection.verbs {
            for atom in verb.short.iter().chain(verb.long.iter().flatten()) {
                let text = serialize_property_atom(atom);
                let reparsed = parse_type_definition(&verb.name, &text)
                    .unwrap_or_else(|error| panic!("`{text}` does not parse: {error}"));
                assert_eq!(reparsed, PropertyDef::Single(atom.clone()), "{text}");
            }
        }
    }

    #[test]
    fn pr_list_keeps_each_overload_separate() {
        let projection = shipped();
        let verb = projection.verb("pr_list").expect("pr_list");

        assert_eq!(
            rendered(&verb.short),
            [
                "template-operand",
                "number(integer;min(1);max(100);deferred;operands(pr_list-short))",
                "tuple([object(deferred)]; operands(pr_list-short))",
                "tuple([number(integer;min(1);max(100);deferred)]; operands(pr_list-short))",
            ]
        );
        assert_eq!(
            rendered(verb.long.as_ref().expect("long form")),
            [
                "{query: object(deferred;required),no_error: boolean}(operands(pr_list-long))",
                "{count: number(integer;min(1);max(100);deferred;required),no_error: boolean}(operands(pr_list-long))",
            ]
        );
    }

    #[test]
    fn a_variadic_verb_has_no_long_form_and_a_list_tail() {
        let projection = shipped();
        let verb = projection.verb("and").expect("and");

        assert!(verb.long.is_none());
        assert_eq!(
            verb.short.last().map(serialize_property_atom).as_deref(),
            Some("(unknown | null)[](operands(and-short))")
        );
        assert!(verb.short.contains(&PropertyAtom::bare(SimplifiedType::Null)));
    }

    /// Every arm a value can be told apart by carries its verb type's name,
    /// so validation binds the value through the verb's registration; the
    /// bare `null` arm and a named reference carry none.
    #[test]
    fn every_arm_but_null_and_a_reference_names_its_verb_type() {
        let projection = shipped();
        for (_, verb) in &projection.verbs {
            let forms = [(verb.short_type_name(), &verb.short)]
                .into_iter()
                .chain(verb.long_type_name().zip(verb.long.as_ref()));
            for (type_name, arms) in forms {
                for atom in arms {
                    let marked = atom
                        .constraints
                        .iter()
                        .chain(&atom.array_constraints)
                        .filter(|constraint| **constraint == Constraint::Operands(type_name.clone()))
                        .count();
                    let unmarked = *atom == PropertyAtom::bare(SimplifiedType::Null)
                        || matches!(atom.ty, TypeExpr::Imported { .. });
                    assert_eq!(marked, usize::from(!unmarked), "{type_name}: {}", serialize_property_atom(atom));
                }
            }
        }
    }

    #[test]
    fn set_is_recorded_and_not_generated() {
        let projection = shipped();

        assert!(projection.verb("set").is_none());
        assert!(projection.collisions.iter().any(|collision| collision.verb == "set"));
        assert!(projection.collisions.iter().any(|collision| collision.verb == "max"));
        assert_eq!(projection.callback_exclusions.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>(), std::collections::BTreeSet::from(["map", "filter"]));
    }

    #[test]
    fn a_parameter_named_no_error_fails_the_projection() {
        let function = match parse_type_expr("f", "function(parameters(no_error: boolean); returns(boolean))")
            .expect("the fixture parses")
            .ty
        {
            TypeExpr::Function(function) => *function,
            other => panic!("not a function: {other:?}"),
        };

        let error = ProjectedVerb::from_function_types("toggle", &[&function], &|_| None).expect_err("reserved");

        assert_eq!(error, ProjectionError::NoErrorParameter { verb: "toggle".to_string() });
    }

    #[test]
    fn rewrite_replaces_only_the_marked_regions() {
        let projection = shipped();
        let mut text = String::from("kind: schema\ntypes:\n    keep: string\n");
        for region in GeneratedRegion::ALL {
            let _ = write!(text, "    {}\n    stale: x\n    {}\n", region.begin_marker(), region.end_marker());
        }
        text.push_str("    tail: number\n");

        let rewritten = projection.rewrite(&text).expect("markers present");

        assert!(rewritten.starts_with("kind: schema\ntypes:\n    keep: string\n"));
        assert!(rewritten.ends_with("    tail: number\n"));
        assert!(!rewritten.contains("stale"));
        assert_eq!(projection.rewrite(&rewritten).expect("idempotent"), rewritten);
        assert!(matches!(
            projection.rewrite("kind: schema\n"),
            Err(ProjectionError::Marker { .. })
        ));
    }
}
