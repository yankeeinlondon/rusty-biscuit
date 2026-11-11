//! `RUSTY_BISCUIT_PRE_PUSH` and `RUSTY_BISCUIT_PRE_PUSH_AREAS`, validated
//! before any discovery or child process.

use std::ffi::OsStr;
use std::fmt;

/// How much of the push the hook gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushMode {
    /// `strict` when a reviewed update starts a CI run, else `scope-only`.
    Auto,
    Strict,
    /// Runs the gates but lets an ordinary gate failure through. A review
    /// failure or an interruption still blocks.
    Warn,
    ScopeOnly,
}

/// A mode setting and whether it used the deprecated `off` spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeSetting {
    pub mode: PushMode,
    /// `off` was given; it now means `scope-only`, and the hook says so.
    pub deprecated_off: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeError {
    /// The value as given, lossily decoded for display.
    pub value: String,
}

impl fmt::Display for ModeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unknown RUSTY_BISCUIT_PRE_PUSH value {:?}; expected auto, scope-only, warn, or strict (off is a deprecated alias of scope-only)",
            self.value
        )
    }
}

impl std::error::Error for ModeError {}

impl PushMode {
    /// Reads `RUSTY_BISCUIT_PRE_PUSH`. Absent and empty both mean `auto`, as
    /// the shell's `${RUSTY_BISCUIT_PRE_PUSH:-auto}` did; any other spelling,
    /// including surrounding whitespace, is an error.
    pub fn parse(value: Option<&OsStr>) -> Result<ModeSetting, ModeError> {
        let setting = |mode, deprecated_off| ModeSetting { mode, deprecated_off };
        match value.map(|value| value.to_str()) {
            None | Some(Some("")) | Some(Some("auto")) => Ok(setting(PushMode::Auto, false)),
            Some(Some("strict")) => Ok(setting(PushMode::Strict, false)),
            Some(Some("warn")) => Ok(setting(PushMode::Warn, false)),
            Some(Some("scope-only")) => Ok(setting(PushMode::ScopeOnly, false)),
            Some(Some("off")) => Ok(setting(PushMode::ScopeOnly, true)),
            _ => Err(ModeError {
                value: value.unwrap_or_default().to_string_lossy().into_owned(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PushMode::Auto => "auto",
            PushMode::Strict => "strict",
            PushMode::Warn => "warn",
            PushMode::ScopeOnly => "scope-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionError;

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RUSTY_BISCUIT_PRE_PUSH_AREAS is not valid UTF-8; it names packages or areas")
    }
}

impl std::error::Error for SelectionError {}

/// Reads `RUSTY_BISCUIT_PRE_PUSH_AREAS`: package or area names separated by
/// ASCII whitespace, taken literally (no glob or shell expansion).
///
/// `None` means no override. An absent, empty, or whitespace-only value is
/// no override.
pub fn parse_selection(value: Option<&OsStr>) -> Result<Option<Vec<String>>, SelectionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let text = value.to_str().ok_or(SelectionError)?;
    let names: Vec<String> = text.split_ascii_whitespace().map(str::to_owned).collect();
    Ok((!names.is_empty()).then_some(names))
}
