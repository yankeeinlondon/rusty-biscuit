//! The host-neutral hook a template is resolved through.
//!
//! The library never evaluates an expression: a host admits each slot
//! passively, evaluates it, and hands back a JSON value. Nothing
//! here names a host type; a host's typed failure travels inside
//! [`SlotFailure`] and is recovered with [`SlotFailure::downcast_ref`].

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::Value;

use crate::errors::SchemaError;
use crate::simplified::Slot;
use crate::span::SourceSpan;

/// What resolves a template's slots.
pub trait SlotHost {
    /// Whether `slot` may be evaluated at all. Passive: no I/O and no
    /// evaluation, so a forbidden read is refused whether or not its branch
    /// would run. Every slot is admitted before any is evaluated.
    ///
    /// ## Errors
    ///
    /// Why the slot is refused.
    fn admit(&self, slot: &Slot) -> Result<(), SlotAdmissionError>;

    /// Evaluates one admitted slot: the only effectful step.
    ///
    /// ## Errors
    ///
    /// The host's typed failure.
    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure>;

    /// Checked before each slot is evaluated and before a result is
    /// published; `true` stops the resolution with
    /// [`SlotResolutionError::Canceled`] and publishes nothing.
    fn is_canceled(&self) -> bool {
        false
    }
}

/// One slot to evaluate.
#[derive(Debug, Clone, Copy)]
pub struct SlotRequest<'a> {
    /// The slot.
    pub slot: &'a Slot,
    /// The schema file the slot was written in ([`Slot::origin`]); `None`
    /// for a slot of an inline document `$schema` or of in-memory content,
    /// which evaluates in `context`'s own source directory.
    pub defining_file: Option<&'a Path>,
    /// The request context of the resolution. A host derives the slot's own
    /// context from it for `defining_file`, never by recapturing process
    /// state.
    pub context: &'a FileResolutionContext,
}

/// One resolution of a pending schema.
#[derive(Debug, Clone)]
pub struct SlotRunRequest {
    /// The request context the slots evaluate in.
    pub context: FileResolutionContext,
}

/// Why a host refuses to evaluate a slot ([`SlotHost::admit`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAdmissionError {
    /// The host's diagnostic code for the refusal, when it has one.
    pub code: Option<&'static str>,
    /// What is refused, in words.
    pub message: String,
    /// The refused part of the expression, in the slot's own source
    /// coordinates (inside [`Slot::expression_span`]); `None` for the whole
    /// slot.
    pub span: Option<SourceSpan>,
}

impl fmt::Display for SlotAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for SlotAdmissionError {}

/// A host's failure to evaluate a slot, carried unchanged so the host can
/// read its own typed error (and that error's code and data location) back.
#[derive(Debug, Clone)]
pub struct SlotFailure {
    source: Arc<dyn Error + Send + Sync + 'static>,
    /// The host's diagnostic code for the failure, when it has one.
    pub code: Option<&'static str>,
}

impl SlotFailure {
    /// Wraps the host's error `source`.
    pub fn new(source: impl Error + Send + Sync + 'static, code: Option<&'static str>) -> Self {
        Self { source: Arc::new(source), code }
    }

    /// Wraps an error the host already shares.
    pub fn from_shared(source: Arc<dyn Error + Send + Sync + 'static>, code: Option<&'static str>) -> Self {
        Self { source, code }
    }

    /// The host's error.
    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.source.as_ref()
    }

    /// The host's error as `T`, when it is one.
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.source.downcast_ref::<T>()
    }
}

impl fmt::Display for SlotFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}

impl Error for SlotFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Why resolving a template produced no schema. Nothing is published in
/// any case.
#[derive(Debug)]
#[non_exhaustive]
pub enum SlotResolutionError {
    /// The host refused these slots; none was evaluated.
    Refused(Vec<(Slot, SlotAdmissionError)>),
    /// These slots failed to evaluate. Every slot of the round was
    /// attempted, so each failure is listed.
    Failed(Vec<(Slot, SlotFailure)>),
    /// The values were produced but the schema they make is not valid
    /// ([`SchemaError::SlotResult`] at the offending slot), or resolving it
    /// failed otherwise.
    Schema(Box<SchemaError>),
    /// [`SlotHost::is_canceled`] stopped the resolution.
    Canceled,
}

impl fmt::Display for SlotResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list = |f: &mut fmt::Formatter<'_>, verb: &str, items: Vec<(&Slot, String)>| {
            let count = items.len();
            write!(f, "{count} slot{} {verb}", if count == 1 { "" } else { "s" })?;
            for (slot, reason) in items {
                write!(f, "; `{{{{ {} }}}}`: {reason}", slot.expression)?;
            }
            Ok(())
        };
        match self {
            Self::Refused(refused) => {
                list(f, "refused", refused.iter().map(|(slot, error)| (slot, error.to_string())).collect())
            }
            Self::Failed(failed) => {
                list(f, "failed", failed.iter().map(|(slot, error)| (slot, error.to_string())).collect())
            }
            Self::Schema(error) => fmt::Display::fmt(error, f),
            Self::Canceled => f.write_str("the slot resolution was canceled"),
        }
    }
}

impl Error for SlotResolutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}
