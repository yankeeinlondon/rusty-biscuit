//! Placeholder types: the approval type a template persists and matches, kept
//! separate from the validator that checks each value at run time.
//!
//! Only a bare top-level property (`dir`, `fm.dir`) reads its declared type,
//! from the owning document's `EffectiveSchema.simplified`. Every other
//! expression, an undeclared name, a reserved root, and a document without a
//! typed projection fall back to `string`, which accepts any present scalar.

use crate::markdown::compose::expression::{Expr, FmPath, fm_namespace_path, parse, reserved_root_descriptors};
use crate::markdown::schemas::completion::alternative_accepts;
use crate::markdown::schemas::simplified::serialize_property_atom;
use crate::markdown::schemas::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SimplifiedSchema, SimplifiedType, TypeExpr,
    flattened_alternatives, is_string_like,
};
use serde_json::Value;
use std::hash::{Hash, Hasher};

/// A placeholder's type: what an approval matches and what a value must pass.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleType {
    pub approval: ApprovalType,
    pub validator: HoleValidator,
    pub provenance: TypeProvenance,
}

/// The type a template approval records for a hole.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalType {
    /// `{{ string }}`: any present scalar after its own validation. A shell
    /// template matching exception, not a member of the subtype relation.
    BroadString,
    /// A supported string-like type narrower than `string`.
    StringLike(StringLikeType),
}

/// A resolved, single, string-like atom reduced to its type identity:
/// canonical `alphanumeric` options, sorted `enum` members, or a `literal`
/// value, with every ordinary constraint and description removed.
///
/// Equality and hashing use the canonical text, so two spellings of one type
/// identity are the same approval type.
#[derive(Debug, Clone)]
pub struct StringLikeType {
    atom: Box<PropertyAtom>,
    text: String,
}

/// How a value is validated before conversion. Kept apart from
/// [`ApprovalType`]: a broad `string` approval can still validate a value as
/// the number its property declares.
#[derive(Debug, Clone, PartialEq)]
pub enum HoleValidator {
    /// Any present scalar: string, number, or boolean.
    FallbackScalar,
    /// The property's resolved declaration, constraints included.
    Declared(Box<PropertyDef>),
}

/// Why a hole has the type it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeProvenance {
    /// A bare top-level property with a declaration.
    BareProperty { name: String },
    /// A bare property, but the document has no typed projection (a raw JSON
    /// Schema, no schema, or a root union whose arms disagree on it).
    NoProjection,
    /// A bare property the schema does not declare.
    Undeclared,
    /// Any expression other than a bare top-level property.
    NotBareProperty,
    /// A read of a reserved root or a host global (`ctx.*`, `env.*`, `loop.*`).
    ReservedRoot,
}

impl StringLikeType {
    /// The identity of `atom`, or `None` unless it is a supported string-like
    /// type narrower than `string`.
    pub fn from_atom(atom: &PropertyAtom) -> Option<Self> {
        if !is_string_like(atom) {
            return None;
        }
        let TypeExpr::Primitive(ty) = atom.ty else {
            return None;
        };
        if ty == SimplifiedType::String {
            return None;
        }
        let mut identity = PropertyAtom::bare(ty);
        if let Some(options) = atom.alphanumeric_options() {
            let options: Vec<_> = options.options().collect();
            if !options.is_empty() {
                identity.constraints.push(Constraint::Options(options));
            }
        }
        for constraint in &atom.constraints {
            match constraint {
                Constraint::Members(members) => {
                    let mut members = members.clone();
                    members.sort();
                    members.dedup();
                    identity.constraints.push(Constraint::Members(members));
                }
                Constraint::LiteralValue(value) => identity.constraints.push(Constraint::LiteralValue(value.clone())),
                _ => {}
            }
        }
        let text = serialize_property_atom(&identity);
        Some(Self { atom: Box::new(identity), text })
    }

    /// The resolved atom, for the subtype relation.
    pub fn atom(&self) -> &PropertyAtom {
        &self.atom
    }

    /// The canonical type text, as written inside a rule's `{{ }}`.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl PartialEq for StringLikeType {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for StringLikeType {}

impl Hash for StringLikeType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl ApprovalType {
    /// The approval type for a declaration: its string-like identity when it
    /// has one narrower than `string`, otherwise the broad `string`.
    pub fn for_declaration(def: &PropertyDef) -> Self {
        match def {
            PropertyDef::Single(atom) => StringLikeType::from_atom(atom).map_or(Self::BroadString, Self::StringLike),
            PropertyDef::Union(_) => Self::BroadString,
        }
    }

    /// The canonical type text inside a rule's `{{ }}`.
    pub fn text(&self) -> &str {
        match self {
            Self::BroadString => "string",
            Self::StringLike(ty) => ty.text(),
        }
    }
}

impl HoleType {
    /// The broad `string` fallback with scalar validation.
    pub fn fallback(provenance: TypeProvenance) -> Self {
        Self {
            approval: ApprovalType::BroadString,
            validator: HoleValidator::FallbackScalar,
            provenance,
        }
    }

    /// The type as the approval prompt shows it after the placeholder's name:
    /// the approval type, plus `(validated as …)` when the value is checked
    /// against a declaration the approval type does not name.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::shell_expansion::template::project_hole;
    /// use darkmatter::markdown::schemas::{PropertyDef, SchemaShape, SimplifiedSchema};
    /// use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
    ///
    /// let mut shape = SchemaShape::new();
    /// let count = PropertyDef::Single(parse_type_expr("count", "number(min(1))").unwrap());
    /// shape.properties.insert("count".into(), count);
    /// let schema = SimplifiedSchema::Single(shape);
    /// let hole = project_hole("count", Some(&schema), &[]);
    /// assert_eq!(hole.display_type(), "string (validated as number(min(1)))");
    /// ```
    pub fn display_type(&self) -> String {
        let approval = self.approval.text().to_string();
        match (&self.approval, &self.validator) {
            (ApprovalType::BroadString, HoleValidator::Declared(def)) if !is_plain_string(def) => {
                format!("{approval} (validated as {})", declaration_text(def))
            }
            _ => approval,
        }
    }
}

impl HoleValidator {
    /// Whether `value` passes this validator. Never coerces: a number fails an
    /// `alphanumeric` declaration. Ordinary constraints (`min`, `pattern`, …)
    /// are enforced. Null, arrays, and objects never pass the fallback.
    pub fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::FallbackScalar => matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_)),
            Self::Declared(def) => flattened_alternatives(def)
                .into_iter()
                .filter(|alternative| !alternative.is_array)
                .any(|alternative| alternative_accepts(alternative.atom, value)),
        }
    }
}

/// Types the placeholder expression `expression` for a command owned by a
/// document whose schema projection is `schema`.
///
/// `host_globals` names the roots a host injects (Claudine's `loop`, `outputs`,
/// …); a bare read of one is a reserved-root read, not a property.
pub fn project_hole(expression: &str, schema: Option<&SimplifiedSchema>, host_globals: &[&str]) -> HoleType {
    let Some(name) = bare_property(expression, host_globals) else {
        let provenance = if reads_reserved_root(expression, host_globals) {
            TypeProvenance::ReservedRoot
        } else {
            TypeProvenance::NotBareProperty
        };
        return HoleType::fallback(provenance);
    };
    let Some(schema) = schema else {
        return HoleType::fallback(TypeProvenance::NoProjection);
    };
    match declaration(schema, &name) {
        Declaration::Found(def) => HoleType {
            approval: ApprovalType::for_declaration(&def),
            validator: HoleValidator::Declared(Box::new(def)),
            provenance: TypeProvenance::BareProperty { name },
        },
        Declaration::Undeclared => HoleType::fallback(TypeProvenance::Undeclared),
        Declaration::Ambiguous => HoleType::fallback(TypeProvenance::NoProjection),
    }
}

enum Declaration {
    Found(PropertyDef),
    Undeclared,
    Ambiguous,
}

/// A root union contributes a declaration only when every arm that declares
/// the property declares it identically.
fn declaration(schema: &SimplifiedSchema, name: &str) -> Declaration {
    match schema {
        SimplifiedSchema::Single(shape) => shape
            .properties
            .get(name)
            .cloned()
            .map_or(Declaration::Undeclared, Declaration::Found),
        SimplifiedSchema::Union(arms) => {
            let mut found: Option<&PropertyDef> = None;
            for arm in arms {
                let SchemaArm::Inline(shape) = arm else {
                    return Declaration::Ambiguous;
                };
                match (shape.properties.get(name), found) {
                    (Some(def), Some(previous)) if def != previous => return Declaration::Ambiguous,
                    (Some(def), _) => found = Some(def),
                    (None, _) => {}
                }
            }
            found.cloned().map_or(Declaration::Undeclared, Declaration::Found)
        }
    }
}

/// The property a bare top-level read names: `dir` or `fm.dir`.
fn bare_property(expression: &str, host_globals: &[&str]) -> Option<String> {
    let Ok(Expr::Variable(path)) = parse(expression) else {
        return None;
    };
    let name = match fm_namespace_path(&path) {
        Some(FmPath::Member(rest)) => rest,
        Some(FmPath::Whole) => return None,
        None if is_root_name(&path, host_globals) => return None,
        None => path.as_str(),
    };
    (!name.contains('.')).then(|| name.to_string())
}

fn reads_reserved_root(expression: &str, host_globals: &[&str]) -> bool {
    match parse(expression) {
        Ok(Expr::Variable(path)) => {
            let root = path.split('.').next().unwrap_or_default();
            is_root_name(root, host_globals) && fm_namespace_path(&path).is_none()
        }
        _ => false,
    }
}

fn is_root_name(name: &str, host_globals: &[&str]) -> bool {
    host_globals.contains(&name) || reserved_root_descriptors().iter().any(|root| root.name == name)
}

fn is_plain_string(def: &PropertyDef) -> bool {
    matches!(def, PropertyDef::Single(atom) if !atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::String))
}

/// The declaration's type text without its description.
fn declaration_text(def: &PropertyDef) -> String {
    let text = |atom: &PropertyAtom| {
        let mut atom = atom.clone();
        atom.description = None;
        serialize_property_atom(&atom)
    };
    match def {
        PropertyDef::Single(atom) => text(atom),
        PropertyDef::Union(atoms) => format!("[{}]", atoms.iter().map(text).collect::<Vec<_>>().join(", ")),
    }
}
