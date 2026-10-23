//! Function registrations: compiled overloads plus the Rust-side facts a
//! signature cannot express (evaluation mode, source exclusions, numeric
//! recovery), validated once when the registration is built.

use super::signature::CompiledSignature;
use super::translate::{NoNamedTypes, TranslateError, TypeResolver};
use crate::markdown::coercion::{Shape, ValueKind};
use crate::markdown::schemas::simplified::{
    FunctionType, PropertyAtom, PropertyDef, TypeExpr, grammar::parse_type_definition, serialize_property_def,
};

/// When a function's arguments are evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvaluationMode {
    /// Every argument is evaluated, then the call binds once.
    #[default]
    Eager,
    /// Arguments are evaluated and bound one at a time as the function
    /// needs them (`and`, `or`); see [`super::LazyCall`].
    Lazy,
}

/// `numeric_recovery`: when `subject` fails to bind for any reason and a
/// supplied `fallback` binds, the call binds with the fallback selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericRecovery {
    pub subject: String,
    pub fallback: String,
}

/// Why a registration cannot be served.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RegistrationError {
    #[error("{function}: the signature does not parse: {message}")]
    Grammar { function: String, message: String },
    #[error("{function}: a registration describes `function(...)` types only; found `{found}`")]
    NotAFunction { function: String, found: String },
    /// An executable function whose parameter or result would be a function
    /// value; function values do not exist at runtime.
    #[error("{function}: overload {} takes or returns a function value at `{path}`, which no call can supply", overload + 1)]
    FunctionValued { function: String, overload: usize, path: String },
    #[error("{function}: overload {}: {source}", overload + 1)]
    Translate {
        function: String,
        overload: usize,
        source: TranslateError,
    },
    #[error("{function}: `{rule}` names parameter `{parameter}`, which no overload declares")]
    UnknownParameter {
        function: String,
        rule: &'static str,
        parameter: String,
    },
    #[error("{function}: parameter `{parameter}` has conflicting registration rules")]
    ConflictingRules { function: String, parameter: String },
    #[error("{function}: invalid numeric recovery: {message}")]
    InvalidRecovery { function: String, message: String },
    #[error("{function}: a lazy function has exactly one signature and no numeric recovery")]
    InvalidLazy { function: String },
    #[error("{function}: overloads {} and {} have the same binding contract", first + 1, second + 1)]
    DuplicateOverload { function: String, first: usize, second: usize },
}

/// A callable function: its overloads compiled once, with its registered
/// behavior facts. Binding reads only this descriptor.
#[derive(Debug, Clone)]
pub struct FunctionRegistration {
    pub(super) name: String,
    pub(super) overloads: Vec<CompiledSignature>,
    pub(super) evaluation: EvaluationMode,
    pub(super) recovery: Option<NumericRecovery>,
}

impl FunctionRegistration {
    /// Starts a registration whose signatures are written in the grammar: one
    /// `function(...)`, or overloads joined with `|`.
    pub fn builder(name: impl Into<String>, signatures: impl Into<String>) -> RegistrationBuilder {
        RegistrationBuilder::new(name.into(), Source::Grammar(signatures.into()))
    }

    /// Starts a registration from already-parsed `function(...)` types.
    pub fn builder_from_types(name: impl Into<String>, overloads: Vec<FunctionType>) -> RegistrationBuilder {
        RegistrationBuilder::new(name.into(), Source::Types(overloads))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn overloads(&self) -> &[CompiledSignature] {
        &self.overloads
    }

    pub fn evaluation(&self) -> EvaluationMode {
        self.evaluation
    }

    pub fn numeric_recovery(&self) -> Option<&NumericRecovery> {
        self.recovery.as_ref()
    }
}

enum Source {
    Grammar(String),
    Types(Vec<FunctionType>),
}

/// Collects a registration's facts; [`RegistrationBuilder::build`] compiles
/// and validates them.
pub struct RegistrationBuilder {
    name: String,
    source: Source,
    evaluation: EvaluationMode,
    exclusions: Vec<(String, Vec<ValueKind>)>,
    recovery: Option<NumericRecovery>,
}

impl RegistrationBuilder {
    fn new(name: String, source: Source) -> Self {
        Self {
            name,
            source,
            evaluation: EvaluationMode::Eager,
            exclusions: Vec::new(),
            recovery: None,
        }
    }

    pub fn evaluation(mut self, mode: EvaluationMode) -> Self {
        self.evaluation = mode;
        self
    }

    /// `reject_sources`: an argument for `parameter` whose original type is
    /// one of `kinds` is a type error before any conversion.
    pub fn reject_sources(mut self, parameter: impl Into<String>, kinds: &[ValueKind]) -> Self {
        self.exclusions.push((parameter.into(), kinds.to_vec()));
        self
    }

    pub fn numeric_recovery(mut self, subject: impl Into<String>, fallback: impl Into<String>) -> Self {
        self.recovery = Some(NumericRecovery {
            subject: subject.into(),
            fallback: fallback.into(),
        });
        self
    }

    /// Builds with no named types available.
    ///
    /// ## Errors
    ///
    /// As [`RegistrationBuilder::build_with`].
    pub fn build(self) -> Result<FunctionRegistration, RegistrationError> {
        self.build_with(&NoNamedTypes)
    }

    /// Compiles every overload, resolving named types through `resolver`,
    /// and validates the registration facts against them.
    ///
    /// ## Errors
    ///
    /// A [`RegistrationError`]: unparseable or non-function signatures,
    /// function-valued parameters or results, untranslatable types, rules
    /// naming unknown parameters, conflicting rules, an invalid recovery or
    /// lazy registration, or two overloads with one binding contract.
    pub fn build_with(self, resolver: &dyn TypeResolver) -> Result<FunctionRegistration, RegistrationError> {
        let name = self.name;
        let types = match self.source {
            Source::Types(types) => types,
            Source::Grammar(text) => function_types(&name, &text)?,
        };
        let mut overloads = Vec::with_capacity(types.len());
        for (overload, ty) in types.iter().enumerate() {
            let signature = CompiledSignature::compile(&name, ty, resolver).map_err(|source| match source {
                TranslateError::FunctionValue { path } => RegistrationError::FunctionValued {
                    function: name.clone(),
                    overload,
                    path,
                },
                source => RegistrationError::Translate {
                    function: name.clone(),
                    overload,
                    source,
                },
            })?;
            overloads.push(signature);
        }

        let mut seen: Vec<&str> = Vec::new();
        for (parameter, kinds) in &self.exclusions {
            if seen.contains(&parameter.as_str()) {
                return Err(RegistrationError::ConflictingRules {
                    function: name,
                    parameter: parameter.clone(),
                });
            }
            seen.push(parameter);
            let mut declared = false;
            for signature in &mut overloads {
                if let Some(target) = signature.parameters.iter_mut().find(|target| target.name() == parameter) {
                    target.excluded = kinds.clone();
                    declared = true;
                }
            }
            if !declared {
                return Err(RegistrationError::UnknownParameter {
                    function: name,
                    rule: "reject_sources",
                    parameter: parameter.clone(),
                });
            }
        }

        if let Some(recovery) = &self.recovery {
            if self.evaluation == EvaluationMode::Lazy {
                return Err(RegistrationError::InvalidLazy { function: name });
            }
            for parameter in [&recovery.subject, &recovery.fallback] {
                if seen.contains(&parameter.as_str()) || recovery.subject == recovery.fallback {
                    return Err(RegistrationError::ConflictingRules {
                        function: name,
                        parameter: parameter.clone(),
                    });
                }
            }
            for signature in &mut overloads {
                signature.recovery = Some(recovery_indexes(&name, signature, recovery)?);
            }
        }

        if self.evaluation == EvaluationMode::Lazy && overloads.len() != 1 {
            return Err(RegistrationError::InvalidLazy { function: name });
        }

        let contracts: Vec<String> = types.iter().zip(&overloads).map(|(ty, signature)| contract(ty, signature)).collect();
        for (second, key) in contracts.iter().enumerate() {
            if let Some(first) = contracts[..second].iter().position(|earlier| earlier == key) {
                return Err(RegistrationError::DuplicateOverload {
                    function: name,
                    first,
                    second,
                });
            }
        }

        Ok(FunctionRegistration {
            name,
            overloads,
            evaluation: self.evaluation,
            recovery: self.recovery,
        })
    }
}

/// Recovery needs a required subject, an optional numeric fallback, and a
/// numeric result.
fn recovery_indexes(
    function: &str,
    signature: &CompiledSignature,
    recovery: &NumericRecovery,
) -> Result<(usize, usize), RegistrationError> {
    let fail = |message: String| RegistrationError::InvalidRecovery {
        function: function.to_string(),
        message,
    };
    let subject = signature
        .parameter(&recovery.subject)
        .ok_or_else(|| fail(format!("`{signature}` has no parameter `{}`", recovery.subject)))?;
    let fallback = signature
        .parameter(&recovery.fallback)
        .ok_or_else(|| fail(format!("`{signature}` has no parameter `{}`", recovery.fallback)))?;
    if subject.is_optional() || subject.is_rest() {
        return Err(fail(format!("the subject `{}` must be a required parameter", subject.name())));
    }
    if !fallback.is_optional() || fallback.is_rest() || !matches!(fallback.shape(), Shape::Number(_)) {
        return Err(fail(format!("the fallback `{}` must be an optional `number` parameter", fallback.name())));
    }
    if !matches!(signature.returns(), Shape::Number(_)) {
        return Err(fail(format!("`{signature}` must return a number")));
    }
    Ok((subject.index(), fallback.index()))
}

/// The `function(...)` options of a grammar definition.
fn function_types(function: &str, text: &str) -> Result<Vec<FunctionType>, RegistrationError> {
    let def = parse_type_definition(function, text).map_err(|error| RegistrationError::Grammar {
        function: function.to_string(),
        message: error.to_string(),
    })?;
    let atoms: Vec<PropertyAtom> = match def {
        PropertyDef::Single(atom) => match atom.ty {
            TypeExpr::Union(arms) if !atom.is_array => arms,
            _ => vec![atom],
        },
        PropertyDef::Union(arms) => arms,
    };
    atoms
        .into_iter()
        .map(|atom| match atom.ty {
            TypeExpr::Function(function_type) if !atom.is_array => Ok(*function_type),
            ty => Err(RegistrationError::NotAFunction {
                function: function.to_string(),
                found: serialize_property_def(&PropertyDef::Single(PropertyAtom { ty, ..atom })),
            }),
        })
        .collect()
}

/// What decides which calls an overload binds: parameter types with their
/// constraints (option order ignored), optional and rest positions, and
/// source exclusions. Names, documentation, and returns do not count.
fn contract(ty: &FunctionType, signature: &CompiledSignature) -> String {
    let mut parts = Vec::new();
    for (parameter, compiled) in ty.parameters.as_slice().iter().zip(signature.parameters()) {
        let mut options: Vec<String> = match &parameter.def {
            PropertyDef::Single(atom) => vec![bare_atom(atom)],
            PropertyDef::Union(arms) => arms.iter().map(bare_atom).collect(),
        };
        options.sort();
        options.dedup();
        parts.push(format!(
            "{}{}{}{:?}",
            if parameter.rest { "..." } else { "" },
            options.join("|"),
            if parameter.optional { "?" } else { "" },
            compiled.excluded_sources()
        ));
    }
    parts.join(",")
}

fn bare_atom(atom: &PropertyAtom) -> String {
    let atom = PropertyAtom {
        description: None,
        ..atom.clone()
    };
    serialize_property_def(&PropertyDef::Single(atom))
}
