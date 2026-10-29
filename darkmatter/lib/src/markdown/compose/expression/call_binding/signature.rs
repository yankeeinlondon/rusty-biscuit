//! Compiled call signatures: one `function(...)` description translated once
//! into shapes, with its arity precomputed.

use std::fmt;

use serde_json::Value;

use super::error::{CallBindingError, ContractViolation};
use super::translate::{TranslateError, TypeResolver, shape_from_property_def};
use crate::markdown::coercion::{Shape, ValueKind, check_exact};
use crate::markdown::schemas::simplified::{FunctionType, PropertyAtom, PropertyDef, serialize_property_def};

/// One compiled parameter.
#[derive(Debug, Clone)]
pub struct CompiledParameter {
    name: String,
    index: usize,
    shape: Shape,
    optional: bool,
    rest: bool,
    admits_null: bool,
    type_text: String,
    declared: PropertyDef,
    /// Original source kinds the registration rejects before conversion.
    pub(super) excluded: Vec<ValueKind>,
}

#[allow(missing_docs)]
impl CompiledParameter {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    pub fn is_optional(&self) -> bool {
        self.optional
    }

    pub fn is_rest(&self) -> bool {
        self.rest
    }

    /// The declared type as written, without descriptions.
    pub fn type_text(&self) -> &str {
        &self.type_text
    }

    /// The declared grammar type, before translation; design-time checks
    /// compare it with [`is_subtype`](crate::markdown::schemas::is_subtype).
    pub fn declared(&self) -> &PropertyDef {
        &self.declared
    }

    pub fn excluded_sources(&self) -> &[ValueKind] {
        &self.excluded
    }

    /// Whether an explicit `null` may be passed: only when the type
    /// declares a `null` option, even for `any`/`unknown`.
    pub fn admits_null(&self) -> bool {
        self.admits_null
    }
}

/// One `function(...)` overload compiled for binding: parameters with their
/// shapes, the return shape, arity, and fallibility.
#[derive(Debug, Clone)]
pub struct CompiledSignature {
    function: String,
    pub(super) parameters: Vec<CompiledParameter>,
    returns: Shape,
    returns_text: String,
    declared_returns: Box<PropertyDef>,
    fallible: bool,
    min_arguments: usize,
    max_arguments: Option<usize>,
    /// `(subject, fallback)` parameter indexes of a numeric recovery.
    pub(super) recovery: Option<(usize, usize)>,
}

impl CompiledSignature {
    /// Compiles one parsed `function(...)` description for `function`.
    ///
    /// ## Errors
    ///
    /// A [`TranslateError`] naming the parameter (or `returns`) whose type
    /// cannot be translated; [`TranslateError::FunctionValue`] when a
    /// parameter or the return type reaches a `function` type.
    pub fn compile(function: &str, ty: &FunctionType, resolver: &dyn TypeResolver) -> Result<Self, TranslateError> {
        let declared = ty.parameters.as_slice();
        let mut parameters = Vec::with_capacity(declared.len());
        for (index, parameter) in declared.iter().enumerate() {
            let shape = shape_from_property_def(&parameter.def, resolver).map_err(|error| prefix(&parameter.name, error))?;
            parameters.push(CompiledParameter {
                name: parameter.name.clone(),
                index,
                admits_null: shape.admits_null_explicitly(),
                shape,
                optional: parameter.optional,
                rest: parameter.rest,
                type_text: type_text(&parameter.def),
                declared: parameter.def.clone(),
                excluded: Vec::new(),
            });
        }
        let returns = shape_from_property_def(&ty.returns, resolver).map_err(|error| prefix("returns", error))?;
        let min_arguments = parameters.iter().filter(|parameter| !parameter.optional && !parameter.rest).count();
        let max_arguments = (!parameters.iter().any(|parameter| parameter.rest)).then_some(parameters.len());
        Ok(Self {
            function: function.to_string(),
            parameters,
            returns,
            returns_text: type_text(&ty.returns),
            declared_returns: Box::new(ty.returns.clone()),
            fallible: ty.fallible,
            min_arguments,
            max_arguments,
            recovery: None,
        })
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn parameters(&self) -> &[CompiledParameter] {
        &self.parameters
    }

    pub fn parameter(&self, name: &str) -> Option<&CompiledParameter> {
        self.parameters.iter().find(|parameter| parameter.name == name)
    }

    /// The parameter that the argument at `index` binds to: the rest
    /// parameter for every argument past the fixed ones.
    pub fn parameter_for(&self, index: usize) -> Option<&CompiledParameter> {
        self.parameters
            .get(index)
            .filter(|parameter| !parameter.rest)
            .or_else(|| self.parameters.last().filter(|parameter| parameter.rest))
    }

    pub fn returns(&self) -> &Shape {
        &self.returns
    }

    /// The `returns(...)` type as declared: the design-time type of a call
    /// that selects this overload.
    pub fn declared_returns(&self) -> &PropertyDef {
        &self.declared_returns
    }

    /// The `(subject, fallback)` parameter indexes of the registration's
    /// numeric recovery, when it has one.
    pub fn recovery(&self) -> Option<(usize, usize)> {
        self.recovery
    }

    pub fn is_fallible(&self) -> bool {
        self.fallible
    }

    /// The fewest and most arguments; the most is `None` with a rest
    /// parameter.
    pub fn arity(&self) -> (usize, Option<usize>) {
        (self.min_arguments, self.max_arguments)
    }

    pub fn accepts_count(&self, count: usize) -> bool {
        count >= self.min_arguments && self.max_arguments.is_none_or(|max| count <= max)
    }

    /// Checks a handler's successful result against `returns(...)` without
    /// converting it.
    ///
    /// ## Errors
    ///
    /// [`CallBindingError::ImplementationContract`] when the value does not
    /// match the return type exactly.
    pub fn check_return(&self, value: &Value) -> Result<(), CallBindingError> {
        check_exact(&self.returns, value).map_err(|error| {
            CallBindingError::ImplementationContract(Box::new(ContractViolation {
                function: self.function.clone(),
                signature: self.to_string(),
                expected: self.returns_text.clone(),
                returned: value.clone(),
                error,
            }))
        })
    }
}

impl fmt::Display for CompiledSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.function)?;
        for (position, parameter) in self.parameters.iter().enumerate() {
            if position > 0 {
                f.write_str(", ")?;
            }
            let marker = if parameter.optional { "?" } else { "" };
            let spread = if parameter.rest { "..." } else { "" };
            write!(f, "{spread}{}{marker}: {}", parameter.name, parameter.type_text)?;
        }
        write!(f, "): {}", self.returns_text)
    }
}

fn prefix(name: &str, error: TranslateError) -> TranslateError {
    let join = |path: String| if path.is_empty() { name.to_string() } else if path.starts_with('[') { format!("{name}{path}") } else { format!("{name}.{path}") };
    match error {
        TranslateError::UnresolvedReference { path, name: ty, reference } => TranslateError::UnresolvedReference {
            path: join(path),
            name: ty,
            reference,
        },
        TranslateError::FunctionValue { path } => TranslateError::FunctionValue { path: join(path) },
        TranslateError::InvalidConstraint { path, ty, constraint } => TranslateError::InvalidConstraint {
            path: join(path),
            ty,
            constraint,
        },
        TranslateError::Invalid { path, message } => TranslateError::Invalid {
            path: join(path),
            message,
        },
        TranslateError::TooDeep { path, limit } => TranslateError::TooDeep { path: join(path), limit },
    }
}

/// The declared type as written, without its descriptions.
fn type_text(def: &PropertyDef) -> String {
    let strip = |atom: &PropertyAtom| PropertyAtom {
        description: None,
        ..atom.clone()
    };
    let bare = match def {
        PropertyDef::Single(atom) => PropertyDef::Single(strip(atom)),
        PropertyDef::Union(arms) => PropertyDef::Union(arms.iter().map(strip).collect()),
    };
    serialize_property_def(&bare)
}
