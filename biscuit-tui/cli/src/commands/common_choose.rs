//! Shared CLI argument parsing and source resolution for the
//! `choose-one` and `choose-many` subcommands.
//!
//! Both subcommands share the same set of frame-chrome, ordering, and
//! source-resolution flags. Collecting them here avoids duplicating
//! clap attributes across two structs and gives later phases a single
//! seam to extend when the remaining flags (`--border`, `--margin`,
//! `--height`, `--sort`) come online.
//!
//! ## Source Precedence
//!
//! When no legacy `--options` / `--options-from-file` /
//! `--options-from-dictionary` flag is set, [`resolve_option_strings`]
//! picks the next non-empty source in the order:
//!
//! 1. trailing positional arguments
//! 2. `stdin`, if it is a pipe (non-TTY)
//!
//! and returns an error if none are available.

use std::io::{self, IsTerminal, Read};

use clap::Args;
use tui_chrome::{ChoiceOption, FrameChromeConfig};

/// Shared clap arguments for the `choose-*` subcommands.
///
/// Phase 3 introduces the source-resolution flag (`--delimiter`);
/// later phases extend this struct with `--sort`, `--border`,
/// `--margin`, and `--height`.
#[derive(Debug, Args, Clone, Default)]
pub struct ChooseChromeArgs {
    /// Split each option string into label and value on the first
    /// occurrence of `<CHAR>`.
    ///
    /// Applied after source resolution, so it works uniformly for
    /// STDIN, positional, and legacy sources.
    #[arg(long, value_name = "CHAR")]
    pub delimiter: Option<char>,
}

/// Resolves the raw option strings from the CLI's source-precedence
/// chain.
///
/// ## Returns
///
/// - `Ok(None)` when the caller is using a legacy source
///   (`--options` / `--options-from-file` /
///   `--options-from-dictionary`) and should build the choice input
///   through the existing builder helpers.
/// - `Ok(Some(strings))` when either positional args or piped STDIN
///   supplied the option list.
///
/// ## Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] when no source is
/// available — no legacy flag, no positionals, and STDIN is a TTY —
/// so the user is told which flag to set.
pub fn resolve_option_strings(
    has_legacy_source: bool,
    positional: Vec<String>,
) -> io::Result<Option<Vec<String>>> {
    if has_legacy_source {
        return Ok(None);
    }
    if !positional.is_empty() {
        return Ok(Some(positional));
    }
    if io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no options provided: pass options as positional args, via stdin, or use one of \
             --options, --options-from-file, --options-from-dictionary",
        ));
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let lines: Vec<String> = buf
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if lines.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no options provided: pass options as positional args, via stdin, or use one of \
             --options, --options-from-file, --options-from-dictionary",
        ));
    }
    Ok(Some(lines))
}

/// Splits an option string into `(label, value)` on the first
/// occurrence of `delimiter`.
///
/// When `delimiter` is `None`, or the string does not contain the
/// delimiter, both label and value are the original string. Trim is
/// applied after the split so `"Apple : 1"` with `':'` yields
/// `("Apple", "1")`.
pub fn parse_label_value(s: &str, delimiter: Option<char>) -> (String, String) {
    match delimiter {
        Some(ch) => match s.split_once(ch) {
            Some((label, value)) => (label.trim().to_string(), value.trim().to_string()),
            None => (s.to_string(), s.to_string()),
        },
        None => (s.to_string(), s.to_string()),
    }
}

/// Builds a list of [`ChoiceOption<String>`] from raw option strings.
///
/// The `id` field of each option is set to the **value** (not the
/// label) so `--selected` matches by value. This is the behaviour
/// change spec callers expect when a `--delimiter` splits a
/// `label⟂value` pair.
pub fn build_options(
    raw_strings: Vec<String>,
    delimiter: Option<char>,
) -> Vec<ChoiceOption<String>> {
    raw_strings
        .into_iter()
        .map(|s| {
            let (label, value) = parse_label_value(&s, delimiter);
            ChoiceOption::new(value.clone(), label, value)
        })
        .collect()
}

/// Builds a [`FrameChromeConfig`] from the shared CLI args.
///
/// Phase 3 returns the default config; Phases 9–11 populate border,
/// margin, and height fields as they come online. Marked
/// `#[allow(dead_code)]` because Phase 3 introduces the helper as
/// scaffolding without yet threading it through the subcommand
/// handlers.
#[allow(dead_code)]
pub fn build_chrome(_args: &ChooseChromeArgs) -> FrameChromeConfig {
    FrameChromeConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_label_value_no_delimiter_returns_identity() {
        let (label, value) = parse_label_value("Apple", None);
        assert_eq!(label, "Apple");
        assert_eq!(value, "Apple");
    }

    #[test]
    fn parse_label_value_splits_on_first_delimiter_only() {
        let (label, value) = parse_label_value("Apple:1:2", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "1:2");
    }

    #[test]
    fn parse_label_value_trims_around_delimiter() {
        let (label, value) = parse_label_value("Apple : 1", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "1");
    }

    #[test]
    fn parse_label_value_falls_back_to_identity_when_delimiter_missing() {
        let (label, value) = parse_label_value("Apple", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "Apple");
    }

    #[test]
    fn build_options_sets_id_to_value() {
        let opts = build_options(vec!["Apple:1".into(), "Berry:2".into()], Some(':'));
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].id, "1");
        assert_eq!(opts[0].label, "Apple");
        assert_eq!(opts[0].value, "1");
        assert_eq!(opts[1].id, "2");
        assert_eq!(opts[1].label, "Berry");
        assert_eq!(opts[1].value, "2");
    }

    #[test]
    fn build_options_without_delimiter_sets_id_equal_to_label_equal_to_value() {
        let opts = build_options(vec!["alpha".into(), "beta".into()], None);
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].id, "alpha");
        assert_eq!(opts[0].label, "alpha");
        assert_eq!(opts[0].value, "alpha");
    }

    #[test]
    fn resolve_option_strings_returns_none_for_legacy_source() {
        let resolved = resolve_option_strings(true, vec![]).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_option_strings_prefers_positional_over_stdin() {
        let resolved = resolve_option_strings(false, vec!["a".into(), "b".into()]).unwrap();
        assert_eq!(resolved, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn build_chrome_returns_default_config() {
        let args = ChooseChromeArgs::default();
        let chrome = build_chrome(&args);
        assert!(chrome.is_empty());
    }
}
