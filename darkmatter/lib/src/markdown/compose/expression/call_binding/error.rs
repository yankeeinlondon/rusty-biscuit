//! Call-binding errors: type, arity, ambiguous overload, and
//! implementation-contract errors, with the context the spec's "Type Errors"
//! section requires.

use std::fmt;

use serde_json::Value;

use crate::markdown::coercion::{ConversionError, ValueFailure, ValueKind, compact, render_path};

/// The four kinds of call-binding error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallErrorKind {
    /// An argument cannot bind to its parameter (in every candidate).
    Type,
    /// No signature takes this many arguments.
    Arity,
    /// Several overloads bind at the same tier.
    AmbiguousOverload,
    /// A handler returned a value its declared return type does not accept.
    ImplementationContract,
}

/// Why one argument failed to bind.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentProblem {
    /// The registration rejects this source type before any conversion
    /// (`length(true)`).
    SourceExcluded,
    /// `null` given to a parameter whose type has no explicit `| null`.
    NullNotAdmitted,
    /// The coercion engine's type error, one failure per failing path.
    Conversion(ConversionError),
}

/// One argument that failed to bind.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgumentError {
    pub parameter: String,
    /// Zero-based argument index; messages show it one-based.
    pub index: usize,
    /// The parameter's declared type, constraints included.
    pub expected: String,
    pub received: Value,
    pub problem: ArgumentProblem,
}

impl ArgumentError {
    pub fn received_kind(&self) -> ValueKind {
        ValueKind::of(&self.received)
    }

    /// The failing paths, rendered from the parameter name (`values[2]`).
    pub fn paths(&self) -> Vec<String> {
        match &self.problem {
            ArgumentProblem::Conversion(error) => error
                .failures
                .iter()
                .map(|failure| render_path(&self.parameter, &failure.path))
                .collect(),
            _ => vec![self.parameter.clone()],
        }
    }

    fn write_lines(&self, f: &mut fmt::Formatter<'_>, indent: &str) -> fmt::Result {
        let argument = self.index + 1;
        match &self.problem {
            ArgumentProblem::SourceExcluded => write!(
                f,
                "\n{indent}{} (argument {argument}): expected {}; received {}; this function does not accept a {} here",
                self.parameter,
                self.expected,
                describe_received(&self.received),
                self.received_kind(),
            ),
            ArgumentProblem::NullNotAdmitted => write!(
                f,
                "\n{indent}{} (argument {argument}): expected {}; received null; the parameter does not declare `| null`",
                self.parameter, self.expected,
            ),
            ArgumentProblem::Conversion(error) => {
                for failure in &error.failures {
                    let label = render_path(&self.parameter, &failure.path);
                    let relative = ValueFailure {
                        path: Vec::new(),
                        ..failure.clone()
                    };
                    let line = relative.to_string().replace('\n', &format!("\n{indent}  "));
                    write!(f, "\n{indent}{label} (argument {argument}): {line}")?;
                }
                Ok(())
            }
        }
    }
}

fn describe_received(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        other => format!("{} {}", ValueKind::of(other), compact(other)),
    }
}

/// Why one candidate signature did not bind.
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureProblem {
    Arity {
        min: usize,
        /// `None` when a rest parameter takes any number.
        max: Option<usize>,
        supplied: usize,
    },
    /// Every argument that failed, in argument order.
    Arguments(Vec<ArgumentError>),
}

/// One candidate signature and why it failed.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateFailure {
    /// The signature as written (`pr_list(count: number(integer; min(1))): string[]`).
    pub signature: String,
    pub problem: SignatureProblem,
}

/// A call that cannot bind, or a handler result that breaks its contract.
#[derive(Debug, Clone, PartialEq)]
pub enum CallBindingError {
    /// Some argument cannot bind: one candidate for a single signature,
    /// every candidate (arity failures included) for overloads.
    Type {
        function: String,
        candidates: Vec<CandidateFailure>,
    },
    /// No signature takes this many arguments.
    Arity {
        function: String,
        supplied: usize,
        candidates: Vec<CandidateFailure>,
    },
    /// Several overloads bind at the best tier; `exact` says whether they
    /// all accepted the arguments unchanged.
    AmbiguousOverload {
        function: String,
        exact: bool,
        signatures: Vec<String>,
    },
    /// A handler's result fails its declared `returns(...)`: a defect in the
    /// implementation, never the caller's error.
    ImplementationContract(Box<ContractViolation>),
}

/// A handler result that its declared return type does not accept exactly.
/// The result is reported as returned, never converted.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractViolation {
    pub function: String,
    pub signature: String,
    /// The declared return type as written.
    pub expected: String,
    pub returned: Value,
    pub error: ConversionError,
}

impl CallBindingError {
    pub fn kind(&self) -> CallErrorKind {
        match self {
            Self::Type { .. } => CallErrorKind::Type,
            Self::Arity { .. } => CallErrorKind::Arity,
            Self::AmbiguousOverload { .. } => CallErrorKind::AmbiguousOverload,
            Self::ImplementationContract(_) => CallErrorKind::ImplementationContract,
        }
    }

    pub fn function(&self) -> &str {
        match self {
            Self::Type { function, .. }
            | Self::Arity { function, .. }
            | Self::AmbiguousOverload { function, .. } => function,
            Self::ImplementationContract(violation) => &violation.function,
        }
    }

    /// Every argument error across all candidates.
    pub fn argument_errors(&self) -> impl Iterator<Item = &ArgumentError> {
        let candidates: &[CandidateFailure] = match self {
            Self::Type { candidates, .. } | Self::Arity { candidates, .. } => candidates,
            _ => &[],
        };
        candidates.iter().flat_map(|candidate| match &candidate.problem {
            SignatureProblem::Arguments(errors) => errors.as_slice(),
            SignatureProblem::Arity { .. } => &[],
        })
    }
}

impl fmt::Display for CallBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Type { function, candidates } => match candidates.as_slice() {
                [only] => {
                    write!(f, "{function}: invalid arguments for {}", only.signature)?;
                    write_problem(f, &only.problem, "  ")
                }
                several => {
                    write!(f, "{function}: no signature accepts these arguments")?;
                    for candidate in several {
                        write!(f, "\n  {}", candidate.signature)?;
                        write_problem(f, &candidate.problem, "    ")?;
                    }
                    Ok(())
                }
            },
            Self::Arity {
                function,
                supplied,
                candidates,
            } => {
                write!(f, "{function}: no signature takes {supplied} argument{}", plural(*supplied))?;
                for candidate in candidates {
                    write!(f, "\n  {}", candidate.signature)?;
                    write_problem(f, &candidate.problem, "    ")?;
                }
                Ok(())
            }
            Self::AmbiguousOverload {
                function,
                exact,
                signatures,
            } => {
                let how = if *exact { "unchanged" } else { "after conversion" };
                write!(
                    f,
                    "{function}: ambiguous call: {} signatures accept these arguments {how}",
                    signatures.len()
                )?;
                for signature in signatures {
                    write!(f, "\n  {signature}")?;
                }
                Ok(())
            }
            Self::ImplementationContract(violation) => {
                let ContractViolation {
                    function,
                    signature,
                    expected,
                    returned,
                    error,
                } = violation.as_ref();
                write!(
                    f,
                    "{function}: implementation contract violated: {signature} returned {}, which is not `{expected}`",
                    describe_received(returned)
                )?;
                for failure in &error.failures {
                    let line = failure.to_string().replace('\n', "\n    ");
                    write!(f, "\n  {line}")?;
                }
                Ok(())
            }
        }
    }
}

fn write_problem(f: &mut fmt::Formatter<'_>, problem: &SignatureProblem, indent: &str) -> fmt::Result {
    match problem {
        SignatureProblem::Arity { min, max, supplied } => {
            let expected = match max {
                Some(max) if max == min => format!("{min}"),
                Some(max) => format!("{min} to {max}"),
                None => format!("at least {min}"),
            };
            write!(
                f,
                "\n{indent}expected {expected} argument{}, received {supplied}",
                plural(max.unwrap_or(*min))
            )
        }
        SignatureProblem::Arguments(errors) => {
            for error in errors {
                error.write_lines(f, indent)?;
            }
            Ok(())
        }
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

impl std::error::Error for CallBindingError {}
