//! Placeholder types: the approval type a template persists and matches, kept
//! separate from the validator that checks each value at run time.
//!
//! Only a bare top-level property (`dir`, `fm.dir`) reads its declared type,
//! from the owning document's `EffectiveSchema.simplified` through the shared
//! declared layer. Every other
//! expression, an undeclared name, a reserved root, and a document without a
//! typed projection fall back to `string`, which accepts any present scalar.
//!
//! A hole is typed at its site's [`TypeScope`]: when an enclosing guard
//! narrows the subject it reads, the narrowed type becomes the approval type
//! and a [`HoleRefinement`] the value must also pass, beside the original
//! validator.

use crate::markdown::compose::expression::scope::{Declaration, VariableClass};
use crate::markdown::compose::expression::{Expr, GuardOrigin, TypeScope, parse};
use crate::markdown::schemas::completion::alternative_accepts_in;
use crate::markdown::schemas::simplified::query::{reachable_definitions, resolved_alternatives};
use crate::markdown::schemas::simplified::serialize_property_atom;
use crate::markdown::schemas::{
    Constraint, DefinitionTable, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr, is_string_like,
};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A placeholder's type: what an approval matches and what a value must pass.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleType {
    pub approval: ApprovalType,
    pub validator: HoleValidator,
    pub provenance: TypeProvenance,
    /// The guard-narrowed type at the hole's site, when a guard narrowed it.
    /// The approval type follows it, so a value is checked against it as
    /// well as against [`validator`](Self::validator) before it fills.
    pub refinement: Option<Box<HoleRefinement>>,
}

/// A hole's type as the guards enclosing its site narrowed it.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleRefinement {
    /// The non-null alternatives that remain.
    pub definition: PropertyDef,
    /// The guards that narrowed the declaration.
    pub guards: Vec<GuardOrigin>,
    /// The recursive definitions `definition` reaches, so a remaining
    /// recursive alternative is judged by its body after the scope is gone.
    pub definitions: DefinitionTable,
}

impl HoleRefinement {
    /// Whether `value` is one of the remaining alternatives, constraints
    /// included. Never coerces.
    pub fn accepts(&self, value: &Value) -> bool {
        definition_accepts(&self.definition, &self.definitions, value)
    }

    /// The narrowed type as written in a schema.
    pub fn text(&self) -> String {
        declaration_text(&self.definition)
    }
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
    /// The property's resolved declaration, constraints included, with the
    /// recursive definitions it reaches: a validator is checked at fill time,
    /// after the scope that typed it is gone, so it owns them.
    Declared {
        definition: Box<PropertyDef>,
        definitions: Arc<DefinitionTable>,
    },
}

/// Why a hole has the type it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeProvenance {
    /// A bare top-level property with a declaration.
    BareProperty { name: String },
    /// A bare property whose projected type is `unknown`: the document has
    /// no schema, or a root-union arm or raw JSON Schema layer leaves it
    /// untyped.
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
            refinement: None,
        }
    }

    /// The type as the approval prompt shows it after the placeholder's name:
    /// the approval type, plus `(validated as …)` when the value is checked
    /// against a type the approval type does not name: the guard refinement
    /// when there is one, otherwise the declaration.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::expression::{DeclaredTypes, TypeScope};
    /// use darkmatter::markdown::compose::shell_expansion::template::project_hole;
    /// use darkmatter::markdown::schemas::{PropertyDef, SchemaShape, SimplifiedSchema};
    /// use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
    ///
    /// let mut shape = SchemaShape::new();
    /// let count = PropertyDef::Single(parse_type_expr("count", "number(min(1))").unwrap());
    /// shape.properties.insert("count".into(), count);
    /// let schema = SimplifiedSchema::Single(shape);
    /// let declared = DeclaredTypes::new(Some(&schema));
    /// let hole = project_hole("count", &TypeScope::unguarded(&declared));
    /// assert_eq!(hole.display_type(), "string (validated as number(min(1)))");
    /// ```
    pub fn display_type(&self) -> String {
        let approval = self.approval.text().to_string();
        if self.approval != ApprovalType::BroadString {
            return approval;
        }
        let validated = match (&self.refinement, &self.validator) {
            (Some(refinement), _) => &refinement.definition,
            (None, HoleValidator::Declared { definition, .. }) => definition.as_ref(),
            (None, HoleValidator::FallbackScalar) => return approval,
        };
        if is_plain_string(validated) {
            approval
        } else {
            format!("{approval} (validated as {})", declaration_text(validated))
        }
    }
}

impl HoleValidator {
    /// Whether `value` passes this validator. Never coerces: a number fails an
    /// `alphanumeric` declaration. Ordinary constraints (`min`, `pattern`, …)
    /// are enforced. Null, arrays, and objects never pass the fallback.
    /// A declaration passes a value one of its non-array alternatives accepts,
    /// a recursive alternative judged by its definition's body.
    pub fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::FallbackScalar => matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_)),
            Self::Declared { definition, definitions } => definition_accepts(definition, definitions, value),
        }
    }
}

/// Whether a non-array alternative of `def` accepts `value`, a reference
/// replaced by its body's alternatives. A reference `definitions` lacks
/// accepts nothing.
fn definition_accepts(def: &PropertyDef, definitions: &DefinitionTable, value: &Value) -> bool {
    resolved_alternatives(def, definitions)
        .into_iter()
        .filter(|alternative| !alternative.is_array)
        .any(|alternative| alternative_accepts_in(alternative.atom, value, definitions))
}

/// The definitions `def` reaches in `definitions`, transitively, so a
/// validator can own them. An alternative reaching a missing definition
/// contributes nothing and so accepts nothing.
fn reached_definitions(def: &PropertyDef, definitions: &DefinitionTable) -> DefinitionTable {
    let atoms = match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms.as_slice(),
    };
    let mut reached = DefinitionTable::default();
    for table in atoms.iter().filter_map(|atom| reachable_definitions(atom, definitions)) {
        for (key, definition) in table.iter() {
            reached.insert(key.clone(), definition.body.clone(), definition.origin.clone());
        }
    }
    reached
}

/// Types the placeholder expression `expression` at a site whose types are
/// `scope`: the declaration from the owner's shared declared layer, narrowed
/// by every guard `scope` carries.
///
/// The declared layer's host roots (Claudine's `loop`, `outputs`, …) make a
/// bare read of one a reserved-root read, not a property. The broad `string`
/// approval applies after the scope lookup: a guard that narrows a subject
/// to a non-string type still approves as `{{ string }}`, and the narrowed
/// type travels as the hole's [`HoleRefinement`].
pub fn project_hole(expression: &str, scope: &TypeScope<'_>) -> HoleType {
    let Ok(expr) = parse(expression) else {
        return HoleType::fallback(TypeProvenance::NotBareProperty);
    };
    let mut hole = declared_hole(&expr, scope);
    if let Some(narrowed) = scope.type_of_expr(&expr).filter(|ty| ty.is_refined())
        && let Some(definition) = narrowed.narrowed_definition()
    {
        hole.approval = ApprovalType::for_declaration(&definition);
        hole.refinement = Some(Box::new(HoleRefinement {
            definitions: reached_definitions(&definition, scope.definitions()),
            definition,
            guards: narrowed.refinements().to_vec(),
        }));
    }
    hole
}

/// The hole type `expr` has before any guard.
fn declared_hole(expr: &Expr, scope: &TypeScope<'_>) -> HoleType {
    let declared = scope.declared();
    let Expr::Variable(path) = expr else {
        return HoleType::fallback(TypeProvenance::NotBareProperty);
    };
    let name = match declared.classify_variable(path) {
        VariableClass::Document { key, rest: "" } => key,
        VariableClass::Namespace { .. } | VariableClass::Host { .. } => {
            return HoleType::fallback(TypeProvenance::ReservedRoot);
        }
        VariableClass::Local { .. } => return HoleType::fallback(TypeProvenance::NotBareProperty),
        VariableClass::Document { .. } if scope.resolves_to_local(path) => {
            return HoleType::fallback(TypeProvenance::NotBareProperty);
        }
        VariableClass::Document { .. } | VariableClass::WholeFrontmatter => {
            return HoleType::fallback(TypeProvenance::NotBareProperty);
        }
    };
    match declared.declaration(name) {
        Declaration::Found(def) => HoleType {
            approval: ApprovalType::for_declaration(&def),
            validator: HoleValidator::Declared {
                definitions: Arc::new(reached_definitions(&def, declared.definitions())),
                definition: Box::new(def.into_owned()),
            },
            provenance: TypeProvenance::BareProperty { name: name.to_string() },
            refinement: None,
        },
        Declaration::Undeclared => HoleType::fallback(TypeProvenance::Undeclared),
        Declaration::NoProjection => HoleType::fallback(TypeProvenance::NoProjection),
    }
}

fn is_plain_string(def: &PropertyDef) -> bool {
    matches!(def, PropertyDef::Single(atom) if !atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::String))
}

/// The declaration's type text without its description.
pub(super) fn declaration_text(def: &PropertyDef) -> String {
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
