//! The callable checks a catalog's reader makes after the shared document
//! reader admitted it, and where each failure is located: the byte ranges
//! of the parsed document's source map, so the CLI and the editor point at
//! the same declaration.

use std::ops::Range;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use simplified_schema::errors::CatalogRelated;
use simplified_schema::simplified::{
    Extends, FunctionParameters, FunctionType, LOCAL_REFERENCE, ObjectComposition, PatternKey, PropertyAtom, PropertyDef,
    SchemaShape, SchemaSourceMap, SchemaSourcePath, SchemaSpanKind, TypeExpr,
};

use super::CatalogError;
use crate::markdown::compose::expression::call_binding::RegistrationError;

/// Builds located [`CatalogError::Invalid`] errors for one catalog.
pub(super) struct Locator<'a> {
    path: &'a Path,
    map: Option<&'a SchemaSourceMap>,
}

impl<'a> Locator<'a> {
    pub(super) fn new(path: &'a Path) -> Self {
        Self { path, map: None }
    }

    pub(super) fn with_map(self, map: &'a SchemaSourceMap) -> Self {
        Self { map: Some(map), ..self }
    }

    fn error(&self, message: String, span: Option<Range<usize>>, related: Vec<CatalogRelated>) -> CatalogError {
        CatalogError::Invalid { path: self.path.to_path_buf(), message, span, related }
    }

    fn related(&self, message: String, span: Option<Range<usize>>) -> CatalogRelated {
        CatalogRelated { message, file: self.path.to_path_buf(), span }
    }

    fn span(&self, path: &SchemaSourcePath, kind: SchemaSpanKind) -> Option<Range<usize>> {
        self.map?.spans(path, kind).first().cloned()
    }

    /// A failure only the file locates.
    pub(super) fn file(&self, message: String) -> CatalogError {
        self.error(message, None, Vec::new())
    }

    /// A failure of the whole `types` table.
    pub(super) fn declaration(&self, message: String) -> CatalogError {
        self.error(message, self.span(&SchemaSourcePath::root(), SchemaSpanKind::Declaration), Vec::new())
    }

    fn key_span(&self, name: &str) -> Option<Range<usize>> {
        let path = SchemaSourcePath::root().property(name);
        self.span(&path, SchemaSpanKind::MappingKey).or_else(|| self.span(&path, SchemaSpanKind::Definition))
    }

    /// A failure of the entry `name`, at its key.
    pub(super) fn key(&self, name: &str, message: String) -> CatalogError {
        self.error(message, self.key_span(name), Vec::new())
    }

    /// The authored range of overload `index` of the entry `name`, which has
    /// `count` overloads: the union arm, or the whole definition of a single
    /// signature.
    fn arm_span(&self, name: &str, index: usize, count: usize) -> Option<Range<usize>> {
        let path = SchemaSourcePath::root().property(name);
        let arm = (count > 1).then(|| self.span(&path.union_arm(index), SchemaSpanKind::UnionArm)).flatten();
        arm.or_else(|| self.span(&path, SchemaSpanKind::Definition)).or_else(|| self.key_span(name))
    }

    /// A failure of overload `index` of `name`.
    pub(super) fn arm(&self, name: &str, index: usize, count: usize, message: String) -> CatalogError {
        self.error(message, self.arm_span(name, index, count), Vec::new())
    }

    /// A failure of overload `index` of `name` that conflicts with the
    /// earlier overload `earlier`.
    pub(super) fn arm_related(
        &self,
        name: &str,
        index: usize,
        earlier: usize,
        count: usize,
        message: String,
        related: String,
    ) -> CatalogError {
        let related = self.related(related, self.arm_span(name, earlier, count));
        self.error(message, self.arm_span(name, index, count), vec![related])
    }

    /// A registration failure of the callable `name`: an overload's when the
    /// error names one, with the earlier overload of a pair as related.
    pub(super) fn registration(&self, name: &str, count: usize, error: &RegistrationError) -> CatalogError {
        let message = format!("`types.{name}`: {error}");
        let (primary, earlier) = match error {
            RegistrationError::FunctionValued { overload, .. } | RegistrationError::Translate { overload, .. } => {
                (Some(*overload), None)
            }
            RegistrationError::DuplicateOverload { first, second, .. } => (Some(*second), Some(*first)),
            RegistrationError::InvalidFact { overloads, .. } => match overloads.as_slice() {
                [only] => (Some(*only), None),
                [first, later, ..] => (Some(*later), Some(*first)),
                [] => (None, None),
            },
            _ => (None, None),
        };
        match (primary, earlier) {
            (Some(index), Some(earlier)) => {
                self.arm_related(name, index, earlier, count, message, format!("overload {} is declared here", earlier + 1))
            }
            (Some(index), None) => self.arm(name, index, count, message),
            _ => self.key(name, message),
        }
    }
}

/// Checks every named-type reference and template slot of the entry `name`
/// (a helper or a callable): a catalog resolves only its own helper types and
/// evaluates nothing, so a reference to another file, to a callable, or to an
/// undeclared name, and every `{{ … }}` slot, is refused. Runs for every
/// entry in authored order, used or not.
pub(super) fn local_references(
    at: &Locator<'_>,
    name: &str,
    def: &PropertyDef,
    helpers: &IndexMap<String, PropertyDef>,
    callables: &[&str],
) -> Result<(), CatalogError> {
    if let Some(slot) = simplified_schema::slots::definition_slots(def).first() {
        return Err(at.key(
            name,
            format!(
                "`types.{name}` holds the template slot `{{{{ {} }}}}`; a callable catalog is read passively and \
                 resolves no slot",
                slot.expression
            ),
        ));
    }
    let mut references = Vec::new();
    let site = ReferenceSite {
        at,
        path: Some(SchemaSourcePath::root().property(name)),
        enclosing: at.key_span(name),
    };
    def_references(def, &site, &mut references);
    for (target, reference, span) in references {
        if reference != LOCAL_REFERENCE {
            return Err(at.error(
                format!(
                    "`types.{name}` references `{target}@{reference}`; a callable catalog resolves only its own \
                     helper types"
                ),
                span,
                Vec::new(),
            ));
        }
        if helpers.contains_key(target) {
            continue;
        }
        let (message, related) = if callables.contains(&target) {
            (
                format!("`types.{name}` uses the callable `{target}` as a type; only helper types can be referenced"),
                vec![at.related(format!("`{target}` is declared here"), at.key_span(target))],
            )
        } else {
            (format!("`types.{name}` references `{target}`, which this catalog does not declare"), Vec::new())
        };
        return Err(at.error(message, span, related));
    }
    Ok(())
}

/// Refuses two callables whose names fold to one ASCII-lowercase spelling,
/// as dispatch looks them up: at the later key, with the earlier as related.
pub(super) fn folded_names(at: &Locator<'_>, callables: &[&str]) -> Result<(), CatalogError> {
    let mut seen: Vec<(String, &str)> = Vec::new();
    for name in callables {
        let folded = name.to_ascii_lowercase();
        if let Some((_, first)) = seen.iter().find(|(spelling, _)| *spelling == folded) {
            return Err(at.error(
                format!(
                    "`types.{name}` and `types.{first}` name the same callable; names are matched without regard \
                     to ASCII case"
                ),
                at.key_span(name),
                vec![at.related(format!("`{first}` is declared here"), at.key_span(first))],
            ));
        }
        seen.push((folded, name));
    }
    Ok(())
}

/// Refuses an authored expression callable whose canonical name a built-in
/// function's canonical name or alias already uses, located at its key with
/// the built-in declaration as related.
pub(super) fn reserved_name(at: &Locator<'_>, name: &str) -> Result<(), CatalogError> {
    let folded = name.to_ascii_lowercase();
    let Some(canonical) = crate::markdown::compose::expression::functions::reserved_builtin_name(&folded) else {
        return Ok(());
    };
    let message = if canonical == folded {
        format!("`types.{name}` reuses the name of the built-in function `{canonical}`")
    } else {
        format!("`types.{name}` reuses `{folded}`, an alias of the built-in function `{canonical}`")
    };
    Err(at.error(message, at.key_span(name), vec![builtin_name_related(canonical)]))
}

/// The declaration claiming a built-in function's canonical name and aliases.
pub(crate) fn builtin_name_related(canonical: &str) -> CatalogRelated {
    CatalogRelated {
        message: format!("the built-in function `{canonical}` is declared here"),
        file: PathBuf::from(super::CATALOG_PATH),
        span: builtin_key_span(canonical),
    }
}

/// The byte range of the built-in catalog's `types` key `canonical`, read
/// from its embedded source on this failure path only.
fn builtin_key_span(canonical: &str) -> Option<Range<usize>> {
    let document =
        simplified_schema::simplified::parse_standalone_schema_document(super::CATALOG_SOURCE, Path::new(super::CATALOG_PATH))
            .ok()??;
    let path = SchemaSourcePath::root().property(canonical);
    document.named_type_source_map().spans(&path, SchemaSpanKind::MappingKey).first().cloned()
}

/// Whether a signature, or any function type nested in its parameters or
/// return, declares `narrows(...)`.
pub(super) fn declares_narrowing(function: &FunctionType) -> bool {
    function.narrows.is_some() || function_defs(function).any(def_declares_narrowing)
}

fn function_defs(function: &FunctionType) -> impl Iterator<Item = &PropertyDef> {
    let parameters = match &function.parameters {
        FunctionParameters::Void => &[][..],
        FunctionParameters::List(parameters) => parameters.as_slice(),
    };
    parameters.iter().map(|parameter| &parameter.def).chain(std::iter::once(&function.returns))
}

fn def_declares_narrowing(def: &PropertyDef) -> bool {
    def_atoms(def).any(atom_declares_narrowing)
}

fn atom_declares_narrowing(atom: &PropertyAtom) -> bool {
    match &atom.ty {
        TypeExpr::Function(function) => declares_narrowing(function),
        TypeExpr::Union(arms) => arms.iter().any(atom_declares_narrowing),
        TypeExpr::InlineObject(shape) => shape_defs(shape).any(def_declares_narrowing),
        TypeExpr::Tuple(tuple) => tuple.any_atom(atom_declares_narrowing),
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) | TypeExpr::Slot(_) => false,
    }
}

fn def_atoms(def: &PropertyDef) -> Box<dyn Iterator<Item = &PropertyAtom> + '_> {
    match def {
        PropertyDef::Single(atom) => Box::new(std::iter::once(atom)),
        PropertyDef::Union(arms) => Box::new(arms.iter()),
    }
}

fn shape_defs(shape: &SchemaShape) -> impl Iterator<Item = &PropertyDef> {
    shape.properties.values().chain(shape.pattern_keys.iter().map(|key| &key.def))
}

/// An address into the catalog source map and the nearest enclosing range.
#[derive(Clone)]
struct ReferenceSite<'a> {
    at: &'a Locator<'a>,
    path: Option<SchemaSourcePath>,
    enclosing: Option<Range<usize>>,
}

impl ReferenceSite<'_> {
    fn child(&self, descend: impl FnOnce(&SchemaSourcePath) -> SchemaSourcePath) -> Self {
        Self { path: self.path.as_ref().map(descend), ..self.clone() }
    }

    fn atom(&self) -> Self {
        let enclosing = self.path.as_ref().and_then(|path| {
            self.at.span(path, SchemaSpanKind::Atom).or_else(|| self.at.span(path, SchemaSpanKind::UnionArm))
        });
        Self { enclosing: enclosing.or_else(|| self.enclosing.clone()), ..self.clone() }
    }

    /// Function bodies, tuple elements, and grouped array unions have no
    /// separate source-map entries. Keep the owning expression's range.
    fn unindexed(self) -> Self {
        Self { path: None, ..self }
    }

    fn span(&self, reference: &str) -> Option<Range<usize>> {
        let kind = if reference == LOCAL_REFERENCE { SchemaSpanKind::ImportName } else { SchemaSpanKind::ImportReference };
        self.path.as_ref()
            .and_then(|path| self.at.span(path, kind).or_else(|| self.at.span(path, SchemaSpanKind::CompositionOperand)))
            .or_else(|| self.enclosing.clone())
    }
}

type LocatedReference<'a> = (&'a str, &'a str, Option<Range<usize>>);

fn def_references<'a>(def: &'a PropertyDef, site: &ReferenceSite<'_>, out: &mut Vec<LocatedReference<'a>>) {
    match def {
        PropertyDef::Single(atom) => atom_references(atom, site, out),
        PropertyDef::Union(arms) => {
            let site = site.atom();
            for (index, arm) in arms.iter().enumerate() {
                atom_references(arm, &site.child(|path| path.union_arm(index)), out);
            }
        }
    }
}

fn atom_references<'a>(atom: &'a PropertyAtom, site: &ReferenceSite<'_>, out: &mut Vec<LocatedReference<'a>>) {
    let site = site.atom();
    match &atom.ty {
        TypeExpr::Imported { name, reference } => out.push((name, reference, site.span(reference))),
        TypeExpr::InlineObject(shape) => {
            if let Some(composition) = &shape.composition {
                for (index, operand) in composition.operands().into_iter().enumerate() {
                    let member = site.child(|path| match composition.as_ref() {
                        ObjectComposition::Inline(_) => path.spread(index),
                        ObjectComposition::Extends(Extends::Scalar(_)) => path.property("$extends"),
                        ObjectComposition::Extends(Extends::Sequence(_)) => path.property("$extends").union_arm(index),
                    });
                    let reference = operand.resolved_reference();
                    out.push((&operand.name, reference, member.span(reference)));
                }
            }
            for (name, def) in &shape.properties {
                def_references(def, &site.child(|path| path.property(name)), out);
            }
            for pattern in &shape.pattern_keys {
                let member = site.child(|path| path.property(match &pattern.key {
                    PatternKey::CatchAll => "<string>".to_string(),
                    PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
                    PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
                    PatternKey::Pattern(regex) => format!("<pattern::{regex}>"),
                }));
                def_references(&pattern.def, &member, out);
            }
        }
        TypeExpr::Union(arms) => {
            let site = site.unindexed();
            arms.iter().for_each(|arm| atom_references(arm, &site, out));
        }
        TypeExpr::Tuple(tuple) => {
            let site = site.unindexed();
            for element in &tuple.elements {
                def_references(&element.def, &site, out);
            }
            if let Some(spread) = &tuple.spread {
                atom_references(spread, &site, out);
            }
        }
        TypeExpr::Function(function) => {
            let site = site.unindexed();
            for def in function_defs(function) {
                def_references(def, &site, out);
            }
        }
        TypeExpr::Primitive(_) | TypeExpr::Ref(_) | TypeExpr::Slot(_) => {}
    }
}
