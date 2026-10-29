//! Function registrations: compiled overloads plus the Rust-side facts a
//! signature cannot express (evaluation mode, source exclusions, numeric
//! recovery), and the narrowing fact the signatures' `narrows(...)` clauses
//! declare, validated once when the registration is built.

use std::borrow::Cow;
use std::ops::Range;

use super::signature::{CompiledParameter, CompiledSignature};
use super::translate::{NoNamedTypes, TranslateError, TypeResolver};
use crate::markdown::coercion::{Shape, ValueKind};
use crate::markdown::schemas::simplified::{
    FunctionType, NarrowsKind, PropertyAtom, PropertyDef, TypeExpr, grammar::parse_type_definition,
    serialize_property_def,
};
use crate::markdown::schemas::simplified::narrowing;
use crate::markdown::schemas::{GrammarRelated, SchemaError};

/// When a function's arguments are evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvaluationMode {
    /// Every argument is evaluated, then the call binds once.
    #[default]
    Eager,
    /// Arguments are evaluated and bound one at a time as the function
    /// needs them (`and`, `or`); see [`super::LazyCall`].
    Lazy,
    /// Inline callbacks stay unevaluated until the evaluator binds each item.
    Callback,
}

/// `numeric_recovery`: when `subject` fails to bind for any reason and a
/// supplied `fallback` binds, the call binds with the fallback selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericRecovery {
    pub subject: String,
    pub fallback: String,
}

/// When a predicate's result proves its [`NarrowingFact`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactKind {
    /// A `true` result proves the argument suitable for the fact's type; a
    /// `false` one proves nothing.
    TrueImplies,
    /// The result is `true` exactly when the argument is of the fact's type:
    /// the true side narrows to it, and the false side removes it.
    TrueIff,
    /// A `false` result proves the argument suitable for the fact's type; a
    /// `true` one proves nothing (`is_empty` is false only for a present
    /// value).
    FalseImplies,
}

/// What a [`NarrowingFact`] proves about its argument.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactType {
    /// The value is not `null`. Presence only: an internal fact with no
    /// schema type, never compatibility with a concrete type.
    NonNull,
    /// A type in the signature grammar (`string`, `file`, `null`,
    /// `unknown[]`). For [`FactKind::TrueIff`] it is `null` or the bare type
    /// of one runtime family, since only a family has a decidable
    /// complement.
    Type(Cow<'static, str>),
}

/// A narrowing fact, declared by a function's `narrows(...)` clauses: what a call's boolean result proves about
/// the original value of the argument bound to `parameter`.
///
/// Analysis reads it from the registration; no predicate runs to narrow a type.
#[derive(Debug, Clone, PartialEq)]
pub struct NarrowingFact {
    parameter: String,
    index: usize,
    kind: FactKind,
    ty: FactType,
    target: Option<PropertyAtom>,
}

#[allow(missing_docs)]
impl NarrowingFact {
    pub fn parameter(&self) -> &str {
        &self.parameter
    }

    /// The parameter's position, the same in every overload.
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn kind(&self) -> FactKind {
        self.kind
    }

    pub fn fact_type(&self) -> &FactType {
        &self.ty
    }

    /// The compiled [`FactType::Type`]; `None` for [`FactType::NonNull`].
    pub fn target(&self) -> Option<&PropertyAtom> {
        self.target.as_ref()
    }
}

/// Why a registration cannot be served.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RegistrationError {
    /// `span` and `related` are byte ranges in the signature text, as
    /// [`SchemaError::Grammar`](crate::markdown::schemas::SchemaError::Grammar)
    /// reports them.
    #[error("{function}: the signature does not parse: {message}")]
    Grammar {
        function: String,
        message: String,
        span: Range<usize>,
        related: Vec<GrammarRelated>,
    },
    #[error("{function}: a registration describes `function(...)` types only; found `{found}`")]
    NotAFunction { function: String, found: String },
    /// A function-valued result or a parameter outside callback evaluation mode;
    /// stored function values do not exist at runtime.
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
    /// A programmatically built declaration has no source text, so the
    /// failure names its overloads (zero-based): the one whose clause is
    /// invalid, or the two that disagree, earlier first.
    #[error("{function}: invalid narrowing fact: {message}")]
    InvalidFact { function: String, message: String, overloads: Vec<usize> },
    #[error("{function}: invalid malformed-reference rule: {message}")]
    InvalidReferenceRule { function: String, message: String },
}

/// A callable function: its overloads compiled once, with its registered
/// behavior facts. Binding reads only this descriptor.
#[derive(Debug, Clone)]
pub struct FunctionRegistration {
    pub(super) name: String,
    pub(super) overloads: Vec<CompiledSignature>,
    pub(super) evaluation: EvaluationMode,
    pub(super) recovery: Option<NumericRecovery>,
    pub(super) fact: Option<NarrowingFact>,
    pub(super) malformed_reference: Option<usize>,
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

    pub fn narrowing_fact(&self) -> Option<&NarrowingFact> {
        self.fact.as_ref()
    }

    /// The position of the parameter registered with
    /// [`RegistrationBuilder::false_for_malformed_reference`]: a text argument
    /// there that is not a well-formed file reference makes the call `false`.
    pub fn malformed_reference_parameter(&self) -> Option<usize> {
        self.malformed_reference
    }

    /// Whether a call with `count` arguments handles a missing value at
    /// argument `index`: every overload taking `count` arguments (and at
    /// least one does) binds it to a parameter that explicitly declares
    /// `null`. The return type plays no part, so `trim(text: string | null):
    /// string | null` handles its argument as `is_null` does. A plain
    /// `unknown` parameter, an optional non-null one, or overloads that
    /// disagree at `index` do not. Whether the call's position lets it handle
    /// anything (`is_empty(trim(x))` does not) is the caller's call-site
    /// rule, not this registration's.
    pub fn handles_absence_at(&self, index: usize, count: usize) -> bool {
        let mut candidates = self.overloads.iter().filter(|signature| signature.accepts_count(count)).peekable();
        candidates.peek().is_some()
            && candidates.all(|signature| signature.parameter_for(index).is_some_and(CompiledParameter::admits_null))
    }

    /// The fact a call with `count` arguments proves: present when an
    /// overload taking `count` arguments declares the fact's parameter.
    pub fn narrowing_fact_for(&self, count: usize) -> Option<&NarrowingFact> {
        let fact = self.fact.as_ref()?;
        self.overloads
            .iter()
            .any(|signature| {
                signature.accepts_count(count)
                    && signature.parameter(&fact.parameter).is_some_and(|parameter| parameter.index() < count)
            })
            .then_some(fact)
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
    malformed_reference: Option<String>,
}

impl RegistrationBuilder {
    fn new(name: String, source: Source) -> Self {
        Self {
            name,
            source,
            evaluation: EvaluationMode::Eager,
            exclusions: Vec::new(),
            recovery: None,
            malformed_reference: None,
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

    /// Declares that text bound to `parameter` which is not a well-formed
    /// file reference makes the call return `false` without any access, so
    /// analysis can report an authored literal of that kind as always false.
    /// The handler must honor it.
    pub fn false_for_malformed_reference(mut self, parameter: impl Into<String>) -> Self {
        self.malformed_reference = Some(parameter.into());
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
    /// function-valued results or stored parameters, callback parameters outside
    /// callback evaluation mode, untranslatable types, rules
    /// naming unknown parameters, conflicting rules, an invalid recovery or
    /// lazy registration, two overloads with one binding contract, or an
    /// invalid narrowing fact (on a function that does not return
    /// `boolean`, on a parameter at different positions, or of a type its
    /// kind cannot prove).
    pub fn build_with(self, resolver: &dyn TypeResolver) -> Result<FunctionRegistration, RegistrationError> {
        let name = self.name;
        let types = match self.source {
            Source::Types(types) => types,
            Source::Grammar(text) => function_types(&name, &text)?,
        };
        let mut overloads = Vec::with_capacity(types.len());
        for (overload, ty) in types.iter().enumerate() {
            let signature = CompiledSignature::compile_with_callbacks(&name, ty, resolver, self.evaluation == EvaluationMode::Callback).map_err(|source| match source {
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

        let fact = compile_declared_fact(&name, &types)?;

        let malformed_reference = match self.malformed_reference {
            Some(parameter) => Some(reference_rule_index(&name, &overloads, parameter)?),
            None => None,
        };

        Ok(FunctionRegistration {
            name,
            overloads,
            evaluation: self.evaluation,
            recovery: self.recovery,
            fact,
            malformed_reference,
        })
    }
}

/// The malformed-reference rule's parameter position: a boolean function
/// whose every overload declares `parameter` as one argument at one position.
fn reference_rule_index(
    function: &str,
    overloads: &[CompiledSignature],
    parameter: String,
) -> Result<usize, RegistrationError> {
    let fail = |message: String| RegistrationError::InvalidReferenceRule {
        function: function.to_string(),
        message,
    };
    if overloads.iter().any(|signature| !matches!(signature.returns(), Shape::Boolean { .. })) {
        return Err(fail("only a function returning `boolean` can be false for a malformed reference".to_string()));
    }
    let positions: Vec<_> = overloads.iter().map(|signature| signature.parameter(&parameter)).collect();
    let Some(Some(first)) = positions.first() else {
        return Err(RegistrationError::UnknownParameter {
            function: function.to_string(),
            rule: "false_for_malformed_reference",
            parameter,
        });
    };
    let index = first.index();
    if positions.iter().any(|position| position.is_none_or(|other| other.index() != index || other.is_rest())) {
        return Err(fail(format!("parameter `{parameter}` must be a single argument at one position in every overload")));
    }
    Ok(index)
}

/// Compiles the narrowing fact a function's `narrows(...)` clauses declare.
///
/// Validity is decided by the schema-owned
/// [`declaration_problem`](crate::markdown::schemas::simplified::narrowing::declaration_problem),
/// the same rules the grammar applies; registration re-checks because a
/// programmatically built [`FunctionType`] never passed through the grammar.
/// This only lowers a valid declaration. `Ok(None)` means no overload
/// declares a clause.
///
/// ## Errors
///
/// [`RegistrationError::InvalidFact`] for any declaration problem.
pub(crate) fn compile_declared_fact(
    function: &str,
    overloads: &[FunctionType],
) -> Result<Option<NarrowingFact>, RegistrationError> {
    if let Some(problem) = narrowing::declaration_problem(overloads) {
        let overloads = match problem.scope {
            narrowing::ProblemScope::Overload(index) => vec![index],
            narrowing::ProblemScope::Overloads { first, later } => vec![first, later],
        };
        return Err(RegistrationError::InvalidFact {
            function: function.to_string(),
            message: problem.overload_message(),
            overloads,
        });
    }
    let Some((overload, clause)) =
        overloads.iter().find_map(|overload| overload.narrows.as_ref().map(|clause| (overload, clause)))
    else {
        return Ok(None);
    };
    let index = overload
        .parameters
        .as_slice()
        .iter()
        .position(|parameter| parameter.name == clause.parameter)
        .expect("a valid declaration names a declared parameter");
    let (ty, target) = match &clause.target {
        None => (FactType::NonNull, None),
        Some(atom) => (FactType::Type(Cow::Owned(narrowing::canonical_target(atom))), Some(atom.clone())),
    };
    let kind = match clause.kind {
        NarrowsKind::TrueImplies => FactKind::TrueImplies,
        NarrowsKind::TrueIff => FactKind::TrueIff,
        NarrowsKind::FalseImplies => FactKind::FalseImplies,
    };
    Ok(Some(NarrowingFact {
        parameter: clause.parameter.clone(),
        index,
        kind,
        ty,
        target,
    }))
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
    let def = parse_type_definition(function, text).map_err(|error| {
        let (span, related) = match &error {
            SchemaError::Grammar { span, related, .. } => (span.clone(), related.clone()),
            _ => (0..0, Vec::new()),
        };
        RegistrationError::Grammar {
            function: function.to_string(),
            message: error.to_string(),
            span,
            related,
        }
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
