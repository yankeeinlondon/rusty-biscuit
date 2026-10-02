//! Composition argument partition (pre-clap).
//!
//! Composition subcommands (`compose`, `inline-compose`, `sequence`) forward
//! provider switches to the underlying agent. This module runs **after**
//! [`super::normalize`] (Rules 1/2/4) and settles everything that needs no
//! composition file:
//!
//! - the **Claudine argv** handed to clap: the file, setters before it, and
//!   Claudine-owned options with their values, wherever they appear before an
//!   authored `--`;
//! - the [`ArgumentsAfterFile`]: every other token after the file, in order,
//!   plus the opaque suffix after the first `--`.
//!
//! Which of those tokens are setters, positionals (`argv`), or provider
//! switches and their values depends on the file's authored `$schema` and
//! `agent`, so type-aware ownership
//! ([`claudine::composition::own_arguments`]) decides it inside the command,
//! after the file is resolved. Help and version never get that far, so they
//! never open a file.
//!
//! ## Ownership model
//!
//! Tokens after the composition subcommand are classified left to right:
//!
//! 1. A token matching Claudine's clap surface (option name, alias, or short,
//!    in space or `=`/attached form) always belongs to Claudine, with the
//!    value slot of a value-bearing option in space form. After the file, the
//!    removed option leaves a [`CallerArgument::ClaudineOption`] marker so the
//!    tokens on either side never join one provider value run.
//! 2. The first bare non-setter token is the composition file. Setters before
//!    it stay with Claudine.
//! 3. A literal `--` after the file starts an **explicit** opaque tail: the
//!    delimiter is consumed and everything after it is forwarded verbatim
//!    with no further classification, later `--` tokens included.
//!
//! An unowned switch, or a `--`, appearing *before* the composition file is a
//! [`PartitionError`], because the file must be resolvable independently of
//! provider argv. So is an argument after the file that is not valid UTF-8:
//! the child argv is `String`-based, and a lossy conversion would send
//! different bytes than the caller wrote.
//!
//! The owned-flag surface is derived from the clap command definitions (see
//! [`OwnedFlags::for_composition`]); it is never a second hand-maintained list.

use std::collections::HashSet;
use std::ffi::OsString;

use clap::{ArgAction, Args, CommandFactory};
use claudine::composition::{ArgumentsAfterFile, CallerArgument};

use crate::args::Cli;
use crate::commands::compose::ComposeArgs;
use crate::commands::sequence::SequenceArgs;

use super::{COMPOSITION_SUBCOMMANDS, as_utf8, find_subcommand, looks_like_flag, looks_like_setter};

/// Error surfaced when composition argv cannot be partitioned deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PartitionError {
    /// A non-Claudine switch appeared before the composition file.
    SwitchBeforeFile { subcommand: String, switch: String },
    /// A literal `--` appeared before the composition file.
    SeparatorBeforeFile { subcommand: String },
    /// An argument after the composition file is not valid UTF-8.
    /// `position` counts arguments after the file from 1; the bytes
    /// themselves are never echoed.
    NonUtf8Argument { position: usize },
}

impl std::fmt::Display for PartitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SwitchBeforeFile { subcommand, switch } => write!(
                f,
                "provider switch '{switch}' appears before the composition file.\n\
                 The composition file must come first so it can be resolved independently \
                 of provider arguments.\n\
                 Supported order: claudine {subcommand} <file> [key=value ...] \
                 [CLAUDINE_OPTIONS] [AGENT_ARGS ...]"
            ),
            Self::SeparatorBeforeFile { subcommand } => write!(
                f,
                "'--' appears before the composition file.\n\
                 Everything after '--' is forwarded opaquely to the agent and cannot contain \
                 the composition file.\n\
                 Supported order: claudine {subcommand} <file> [key=value ...] \
                 [CLAUDINE_OPTIONS] -- [AGENT_ARGS ...]"
            ),
            Self::NonUtf8Argument { position } => write!(
                f,
                "argument {position} after the composition file is not valid UTF-8.\n\
                 Claudine forwards provider arguments unchanged and will not rewrite one \
                 into different text. Pass the argument as valid UTF-8."
            ),
        }
    }
}

impl std::error::Error for PartitionError {}

/// How a flag token consumes (or does not consume) the following token.
enum Ownership {
    /// Not a Claudine-owned flag.
    Unowned,
    /// Owned; entirely self-contained (`--flag`, `--flag=v`, `-abc`, `-mv`).
    SelfContained,
    /// Owned; its value lives in the next argv token (`--model v`, `-m v`).
    ConsumesNext,
}

/// Claudine-owned option surface for the composition subcommands, derived
/// from the clap command definitions.
pub(crate) struct OwnedFlags {
    value_flags: HashSet<String>,
    bool_flags: HashSet<String>,
}

impl OwnedFlags {
    /// Build the owned surface from the root [`Cli`] globals plus the
    /// `ComposeArgs`/`SequenceArgs` command definitions. The union is a
    /// superset covering all three composition subcommands.
    pub(crate) fn for_composition() -> Self {
        let mut owned = Self {
            value_flags: HashSet::new(),
            bool_flags: HashSet::new(),
        };
        // Root globals (`--plain`, `--verbose`/`-v`, `--debug`, `--help`/`-h`,
        // `--version`) propagate onto every subcommand.
        owned.absorb_command(&Cli::command());
        owned.absorb_args::<ComposeArgs>();
        owned.absorb_args::<SequenceArgs>();
        owned
    }

    fn absorb_args<A: Args>(&mut self) {
        let cmd = A::augment_args(clap::Command::new("__argv_introspect"));
        self.absorb_command(&cmd);
    }

    fn absorb_command(&mut self, cmd: &clap::Command) {
        for arg in cmd.get_arguments() {
            let has_flag_name = arg.get_long().is_some() || arg.get_short().is_some();
            if !has_flag_name {
                continue;
            }
            let takes_value = !matches!(
                arg.get_action(),
                ArgAction::SetTrue
                    | ArgAction::SetFalse
                    | ArgAction::Count
                    | ArgAction::Help
                    | ArgAction::HelpShort
                    | ArgAction::HelpLong
                    | ArgAction::Version
            );
            let bucket = if takes_value {
                &mut self.value_flags
            } else {
                &mut self.bool_flags
            };
            if let Some(long) = arg.get_long() {
                bucket.insert(format!("--{long}"));
            }
            if let Some(aliases) = arg.get_all_aliases() {
                for alias in aliases {
                    bucket.insert(format!("--{alias}"));
                }
            }
            if let Some(short) = arg.get_short() {
                bucket.insert(format!("-{short}"));
            }
            if let Some(shorts) = arg.get_all_short_aliases() {
                for short in shorts {
                    bucket.insert(format!("-{short}"));
                }
            }
        }
    }

    fn is_value_flag(&self, flag: &str) -> bool {
        self.value_flags.contains(flag)
    }

    fn is_bool_flag(&self, flag: &str) -> bool {
        self.bool_flags.contains(flag)
    }

    /// Whether `token` is a Claudine option whose value is the next argv
    /// token (`--model`, `-m`, but not `--model=x` or `-mx`).
    pub(crate) fn consumes_next(&self, token: &str) -> bool {
        looks_like_flag(token) && matches!(self.classify(token), Ownership::ConsumesNext)
    }

    /// Classify a flag-shaped token against the owned surface.
    fn classify(&self, token: &str) -> Ownership {
        if let Some(long_body) = token.strip_prefix("--") {
            let name = long_body.split('=').next().unwrap_or(long_body);
            let full = format!("--{name}");
            if token.contains('=') {
                // `--flag=value` is self-contained regardless of arity.
                return if self.is_value_flag(&full) || self.is_bool_flag(&full) {
                    Ownership::SelfContained
                } else {
                    Ownership::Unowned
                };
            }
            if self.is_value_flag(&full) {
                return Ownership::ConsumesNext;
            }
            if self.is_bool_flag(&full) {
                return Ownership::SelfContained;
            }
            return Ownership::Unowned;
        }

        // Short cluster: `-y`, `-yq`, `-m value`, `-mvalue`.
        let body: Vec<char> = token[1..].chars().collect();
        let mut idx = 0;
        while idx < body.len() {
            let short = format!("-{}", body[idx]);
            if self.is_value_flag(&short) {
                // Remaining chars (if any) are this flag's attached value.
                return if idx + 1 < body.len() {
                    Ownership::SelfContained
                } else {
                    Ownership::ConsumesNext
                };
            }
            if self.is_bool_flag(&short) {
                idx += 1;
                continue;
            }
            return Ownership::Unowned;
        }
        Ownership::SelfContained
    }
}

/// Partition an already-[normalized](super::normalize) argv into the Claudine
/// argv (for clap) and the [`ArgumentsAfterFile`] that type-aware ownership
/// classifies once the file's frontmatter is read.
///
/// Returns the argv unchanged, with no arguments after the file, for any
/// non-composition argv.
///
/// ## Errors
///
/// [`PartitionError`] when a non-Claudine switch or a `--` appears before the
/// composition file, or when an argument after the file is not valid UTF-8.
pub(crate) fn partition_composition_tail(
    argv: Vec<OsString>,
) -> Result<(Vec<OsString>, ArgumentsAfterFile), PartitionError> {
    let Some((sub_idx, sub_name)) = find_subcommand(&argv, COMPOSITION_SUBCOMMANDS) else {
        return Ok((argv, ArgumentsAfterFile::default()));
    };

    let owned = OwnedFlags::for_composition();

    // Everything up to and including the subcommand stays in the Claudine argv.
    let mut claudine: Vec<OsString> = argv[..=sub_idx].to_vec();
    let mut after_file: Vec<CallerArgument> = Vec::new();
    let mut opaque: Option<Vec<String>> = None;
    let mut file_index: Option<usize> = None;

    let mut i = sub_idx + 1;
    while i < argv.len() {
        let token = &argv[i];
        let text = as_utf8(token);

        // Explicit `--` boundary: opaque, unclassified tail.
        if text == Some("--") {
            let Some(file_index) = file_index else {
                return Err(PartitionError::SeparatorBeforeFile {
                    subcommand: sub_name.to_string(),
                });
            };
            let mut suffix = Vec::with_capacity(argv.len() - i - 1);
            for (offset, token) in argv[i + 1..].iter().enumerate() {
                suffix.push(argument_text(token, i + 1 + offset - file_index)?);
            }
            opaque = Some(suffix);
            break;
        }

        // Claudine-owned flags win everywhere before an explicit boundary.
        // After the file, the removed option leaves a marker so ownership
        // never joins the tokens on either side into one value run.
        if let Some(text) = text
            && looks_like_flag(text)
        {
            let consumed = match owned.classify(text) {
                Ownership::ConsumesNext => 1 + usize::from(i + 1 < argv.len()),
                Ownership::SelfContained => 1,
                Ownership::Unowned => {
                    if file_index.is_none() {
                        return Err(PartitionError::SwitchBeforeFile {
                            subcommand: sub_name.to_string(),
                            switch: text.to_string(),
                        });
                    }
                    after_file.push(CallerArgument::Token(text.to_string()));
                    i += 1;
                    continue;
                }
            };
            claudine.extend(argv[i..i + consumed].iter().cloned());
            if file_index.is_some() {
                after_file.push(CallerArgument::ClaudineOption);
            }
            i += consumed;
            continue;
        }

        match file_index {
            // After the file: a setter, positional, or provider value, for
            // ownership to decide.
            Some(file_index) => {
                after_file.push(CallerArgument::Token(argument_text(token, i - file_index)?));
            }
            // Before the file: a setter-shaped token is a Claudine setter and
            // leaves the file unclaimed; any other bare token is the file.
            None => {
                if !text.map(looks_like_setter).unwrap_or(false) {
                    file_index = Some(i);
                }
                claudine.push(token.clone());
            }
        }
        i += 1;
    }

    Ok((claudine, ArgumentsAfterFile::new(after_file, opaque)))
}

/// The UTF-8 text of the argument `position` places after the file.
fn argument_text(token: &OsString, position: usize) -> Result<String, PartitionError> {
    token
        .to_str()
        .map(str::to_owned)
        .ok_or(PartitionError::NonUtf8Argument { position })
}

#[cfg(test)]
mod tests;
