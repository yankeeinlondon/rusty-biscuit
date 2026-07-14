//! Composition provider-argument ownership partition (pre-clap).
//!
//! Composition subcommands (`compose`, `inline-compose`, `sequence`) forward
//! any CLI switch Claudine does not own to the underlying agent, mirroring the
//! direct-wrapper launch contract. This module runs **after** [`super::normalize`]
//! (Rules 1/2/4) and **replaces** the retired Rule 3 synthetic-separator
//! coupling: instead of protecting trailing setters behind an inserted `--`,
//! it partitions the normalized argv into two explicit vectors —
//!
//! - the **Claudine argv** handed to clap (file, `key=value` setters, and
//!   Claudine-owned options with their values), and
//! - the **provider tail** ([`ProviderArgs`]) threaded into execution as
//!   first-class launch state.
//!
//! ## Ownership model
//!
//! Tokens after the composition subcommand are classified left to right:
//!
//! 1. A token matching Claudine's clap surface (option name, alias, or short,
//!    in space or `=`/attached form) always belongs to Claudine — even after
//!    an implicit agent tail has started — preserving the wrapper's flag
//!    precedence (`-m`, `-o`, `-y`, `--model`, `--silent`, …). Its value slot
//!    (for value-bearing options in space form) is kept with it.
//! 2. The first **non-Claudine switch** after the composition file has been
//!    identified starts an implicit agent tail. Every non-Claudine token from
//!    that point (including setter-shaped values and bare operands) is
//!    forwarded in original order.
//! 3. A literal `--` after the file starts an **explicit** opaque tail: the
//!    delimiter is consumed by Claudine and everything after it is forwarded
//!    verbatim with no further classification.
//!
//! An unowned switch — or a `--` — appearing *before* the composition file is
//! a [`PartitionError`], because the file must be resolvable independently of
//! provider argv.
//!
//! The owned-flag surface is derived from the clap command definitions (see
//! [`OwnedFlags::for_composition`]); it is never a second hand-maintained list.

use std::collections::HashSet;
use std::ffi::OsString;

use clap::{ArgAction, Args, CommandFactory};

use crate::args::Cli;
use crate::commands::compose::ComposeArgs;
use crate::commands::sequence::SequenceArgs;

use super::{COMPOSITION_SUBCOMMANDS, as_utf8, find_subcommand, looks_like_flag, looks_like_setter};

/// The provider-argument tail forwarded to the underlying agent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProviderArgs {
    /// Forwarded tokens, in original order.
    pub args: Vec<String>,
    /// `true` when the tail was introduced by an explicit `--` boundary
    /// (opaque, unclassified) rather than by an implicit non-Claudine switch.
    pub explicit: bool,
}

/// Error surfaced when composition argv cannot be partitioned deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PartitionError {
    /// A non-Claudine switch appeared before the composition file.
    SwitchBeforeFile { subcommand: String, switch: String },
    /// A literal `--` appeared before the composition file.
    SeparatorBeforeFile { subcommand: String },
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
/// argv (for clap) and the provider tail (for execution).
///
/// Returns `(claudine_argv, None)` unchanged for any non-composition argv.
///
/// ## Errors
///
/// [`PartitionError`] when a non-Claudine switch or a `--` appears before the
/// composition file.
pub(crate) fn partition_composition_tail(
    argv: Vec<OsString>,
) -> Result<(Vec<OsString>, ProviderArgs), PartitionError> {
    let Some((sub_idx, sub_name)) = find_subcommand(&argv, COMPOSITION_SUBCOMMANDS) else {
        return Ok((argv, ProviderArgs::default()));
    };

    let owned = OwnedFlags::for_composition();

    // Everything up to and including the subcommand stays in the Claudine argv.
    let mut claudine: Vec<OsString> = argv[..=sub_idx].to_vec();
    let mut tail: Vec<String> = Vec::new();
    let mut tail_started = false;
    let mut explicit = false;
    let mut file_seen = false;

    let mut i = sub_idx + 1;
    while i < argv.len() {
        let token = &argv[i];
        let text = as_utf8(token);

        // Explicit `--` boundary: opaque, unclassified tail.
        if text == Some("--") {
            if !file_seen {
                return Err(PartitionError::SeparatorBeforeFile {
                    subcommand: sub_name.to_string(),
                });
            }
            for opaque in &argv[i + 1..] {
                tail.push(opaque.to_string_lossy().into_owned());
            }
            explicit = true;
            break;
        }

        // Claudine-owned flags win everywhere before an explicit boundary,
        // including after an implicit tail has started.
        if let Some(text) = text
            && looks_like_flag(text)
        {
            match owned.classify(text) {
                Ownership::ConsumesNext => {
                    claudine.push(token.clone());
                    if let Some(value) = argv.get(i + 1) {
                        claudine.push(value.clone());
                        i += 2;
                        continue;
                    }
                    i += 1;
                    continue;
                }
                Ownership::SelfContained => {
                    claudine.push(token.clone());
                    i += 1;
                    continue;
                }
                Ownership::Unowned => {
                    // A non-Claudine switch starts (or extends) the implicit tail.
                    if !tail_started && !file_seen {
                        return Err(PartitionError::SwitchBeforeFile {
                            subcommand: sub_name.to_string(),
                            switch: text.to_string(),
                        });
                    }
                    tail.push(text.to_string());
                    tail_started = true;
                    i += 1;
                    continue;
                }
            }
        }

        // Positional token (file, setter, operand) or a non-UTF-8 token.
        if tail_started {
            tail.push(token.to_string_lossy().into_owned());
        } else if !file_seen {
            // Before the file: a setter-shaped token is a Claudine setter and
            // leaves the file unclaimed; any other bare token is the file.
            if !text.map(looks_like_setter).unwrap_or(false) {
                file_seen = true;
            }
            claudine.push(token.clone());
        } else {
            // File already seen, no tail yet: a second bare positional falls
            // through to clap's existing multiple-file diagnostic; a setter is
            // applied as a Claudine override.
            claudine.push(token.clone());
        }
        i += 1;
    }

    Ok((claudine, ProviderArgs { args: tail, explicit }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    fn strs(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    fn partition(tokens: &[&str]) -> (Vec<String>, ProviderArgs) {
        let (claudine, tail) = partition_composition_tail(argv(tokens)).expect("no partition error");
        let claudine: Vec<String> = claudine
            .into_iter()
            .map(|t| t.to_string_lossy().into_owned())
            .collect();
        (claudine, tail)
    }

    #[test]
    fn non_composition_argv_passes_through() {
        let (claudine, tail) = partition(&["claudine", "codex", "-c", "x"]);
        assert_eq!(claudine, strs(&["claudine", "codex", "-c", "x"]));
        assert!(tail.args.is_empty());
    }

    #[test]
    fn reported_command_forwards_config_switch() {
        // The headline bug: `-c model_reasoning_effort=low` must reach codex,
        // and the setter-shaped value must NOT become a frontmatter override.
        let (claudine, tail) = partition(&[
            "claudine",
            "sequence",
            "fleet.md",
            "-y",
            "--provider",
            "codex",
            "-c",
            "model_reasoning_effort=low",
        ]);
        assert_eq!(
            claudine,
            strs(&["claudine", "sequence", "fleet.md", "-y", "--provider", "codex"])
        );
        assert_eq!(tail.args, strs(&["-c", "model_reasoning_effort=low"]));
        assert!(!tail.explicit);
    }

    #[test]
    fn setter_before_tail_stays_with_claudine() {
        let (claudine, tail) = partition(&["claudine", "compose", "file.md", "name=Ken", "--codexflag"]);
        assert_eq!(
            claudine,
            strs(&["claudine", "compose", "file.md", "name=Ken"])
        );
        assert_eq!(tail.args, strs(&["--codexflag"]));
    }

    #[test]
    fn owned_flag_after_tail_is_reclaimed() {
        // `-m gpt5` is Claudine's model flag; it wins even after the tail
        // started. `--config` and its bare operand stay in the tail.
        let (claudine, tail) = partition(&[
            "claudine", "compose", "file.md", "--config", "foo", "-m", "gpt5",
        ]);
        assert_eq!(
            claudine,
            strs(&["claudine", "compose", "file.md", "-m", "gpt5"])
        );
        assert_eq!(tail.args, strs(&["--config", "foo"]));
    }

    #[test]
    fn explicit_separator_forwards_opaque_tail() {
        let (claudine, tail) = partition(&[
            "claudine", "compose", "file.md", "--", "-c", "--silent", "value",
        ]);
        assert_eq!(claudine, strs(&["claudine", "compose", "file.md"]));
        // Even `--silent` (a Claudine flag) is opaque after `--`.
        assert_eq!(tail.args, strs(&["-c", "--silent", "value"]));
        assert!(tail.explicit);
    }

    #[test]
    fn switch_before_file_errors() {
        let err = partition_composition_tail(argv(&["claudine", "compose", "--unknown", "file.md"]))
            .expect_err("switch before file must error");
        assert!(matches!(err, PartitionError::SwitchBeforeFile { .. }));
    }

    #[test]
    fn separator_before_file_errors() {
        let err = partition_composition_tail(argv(&["claudine", "compose", "--", "file.md"]))
            .expect_err("separator before file must error");
        assert!(matches!(err, PartitionError::SeparatorBeforeFile { .. }));
    }

    #[test]
    fn value_flag_value_is_not_mistaken_for_file() {
        // `-m gpt5` consumes its value; `file.md` is still the file, and the
        // trailing `--x` starts the tail.
        let (claudine, tail) = partition(&["claudine", "compose", "-m", "gpt5", "file.md", "--x"]);
        assert_eq!(
            claudine,
            strs(&["claudine", "compose", "-m", "gpt5", "file.md"])
        );
        assert_eq!(tail.args, strs(&["--x"]));
    }

    #[test]
    fn second_bare_positional_stays_for_clap_diagnostic() {
        // Two ordinary non-setter positionals must keep flowing to clap's
        // existing multiple-file error rather than starting a tail.
        let (claudine, tail) = partition(&["claudine", "compose", "a.md", "b.md"]);
        assert_eq!(claudine, strs(&["claudine", "compose", "a.md", "b.md"]));
        assert!(tail.args.is_empty());
    }

    #[test]
    fn bundled_bool_shorts_are_owned() {
        let (claudine, tail) = partition(&["claudine", "compose", "file.md", "-yq", "--x"]);
        assert_eq!(
            claudine,
            strs(&["claudine", "compose", "file.md", "-yq"])
        );
        assert_eq!(tail.args, strs(&["--x"]));
    }

    // ── Drift detection: the owned surface is derived from clap definitions ──

    #[test]
    fn owned_surface_is_derived_from_clap_and_non_empty() {
        let owned = OwnedFlags::for_composition();
        assert!(
            !owned.value_flags.is_empty() && !owned.bool_flags.is_empty(),
            "owned surface must be populated from the clap command definitions"
        );
        // Representative value-bearing options (space form consumes next token).
        for flag in ["--provider", "--model", "-m", "--set", "--output", "-o"] {
            assert!(
                owned.is_value_flag(flag),
                "{flag} must be a value-bearing owned flag; if the clap surface \
                 changed, this drift test is the intended failure point"
            );
        }
        // Representative boolean options.
        for flag in ["--yolo", "-y", "--silent", "--dry-run", "--sandbox"] {
            assert!(owned.is_bool_flag(flag), "{flag} must be a boolean owned flag");
        }
        // The `sequence`-only flag is in the union.
        assert!(owned.is_value_flag("--fail-fast"));
    }
}
