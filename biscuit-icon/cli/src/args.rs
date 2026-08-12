use std::collections::BTreeSet;

use clap::{Args, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

/// Curated domain icons + on-demand Iconify lookup.
#[derive(Parser, Debug)]
#[command(name = "icon", version, about, long_about = None)]
pub struct Cli {
    /// Increase user-facing diagnostic verbosity (-v, -vv, -vvv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Enable developer tracing on stderr (-d, -dd, -dd) or RUST_LOG.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Prefer Nerd Font glyphs when the icon defines one.
    #[arg(long, global = true, env = "ICON_NERD_FONT")]
    pub nerd: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Default `show` filter when no subcommand is given (e.g. `icon mdi:home`).
    #[arg(value_name = "FILTER", add = ArgValueCompleter::new(icon_name_completer))]
    pub filter: Option<String>,

    /// `show` flags, flattened so they work at the top level too (e.g.
    /// `icon mdi:home --svg`). The same flags are also attached to the
    /// `show` subcommand via flatten, and to the `domain` subcommand for
    /// parity on single-icon rendering.
    #[command(flatten)]
    pub show: ShowFlags,
}

/// Show-command format and behavior flags. Used at the top level (for the
/// `icon <filter> --{flag}` shorthand), on the `show` subcommand, and on the
/// `domain` subcommand for the single-icon form.
#[derive(Args, Debug, Clone)]
pub struct ShowFlags {
    /// Limit to these sets (comma-separated prefixes), e.g. `fa,mdi`.
    #[arg(long, value_name = "CSV", add = ArgValueCompleter::new(set_name_completer))]
    pub from: Option<String>,

    /// Emit raw SVG text.
    #[arg(long)]
    pub svg: bool,

    /// Emit the SVG wrapped in a Markdown code block with syntax highlighting.
    #[arg(long)]
    pub code_block: bool,

    /// Emit the icon as a CSS `url('data:image/svg+xml,…')` string.
    #[arg(long)]
    pub css: bool,

    /// Show metadata table (Set, Icon, Categories, Tags, Author, License).
    #[arg(long)]
    pub meta: bool,

    /// List all matches one per line (non-TTY default; forces list in TTY).
    #[arg(long)]
    pub list: bool,

    /// Launch the interactive picker (TTY only; errors in non-TTY).
    #[arg(long)]
    pub pick: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show icons by id, filter, or picker (default command).
    Show {
        /// One or more `prefix:name` ids or filter substrings.
        #[arg(value_name = "ID", add = ArgValueCompleter::new(icon_name_completer))]
        ids: Vec<String>,

        /// `show` flags, flattened so they work both before and after `show`.
        #[command(flatten)]
        show: ShowFlags,
    },
    /// List Iconify set names, optionally filtered.
    Sets {
        /// Substring to match against set prefixes/titles.
        #[arg(value_name = "FILTER", add = ArgValueCompleter::new(set_name_completer))]
        filter: Option<String>,
    },
    /// Cache maintenance.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Curated domain icons (offline enum sets).
    Domain {
        /// Enum name, `enum:variant`, or substring filter.
        #[arg(value_name = "ARG", add = ArgValueCompleter::new(domain_arg_completer))]
        arg: Option<String>,

        /// `show`-style format flags, applied to the single-icon form
        /// (`icon domain <enum>:<variant> --svg`, etc.) and to each
        /// `Icon` cell of the variants table.
        #[command(flatten)]
        show: ShowFlags,
    },
    /// Generate dynamic shell completions.
    Completions {
        /// Target shell.
        #[arg(value_name = "SHELL")]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// List cached icons with metadata.
    List,
    /// Delete cached icons, optionally filtered.
    Clear {
        /// Only clear icons whose `prefix:name` contains this substring.
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
}

/// Offers `prefix:name` ids matching the current token, merging the built-in
/// domain catalog with cached names. Completion never fails the shell.
fn icon_name_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let needle = current.to_string_lossy().to_lowercase();
    let mut ids = BTreeSet::new();

    for id in biscuit_icon::domain::all_iconify_ids() {
        if id.to_lowercase().contains(&needle) {
            ids.insert(id.to_string());
        }
    }

    if let Ok(cache) = biscuit_icon::cache::IconCache::open_default()
        && let Ok(hits) = cache.search_names(&needle)
    {
        ids.extend(hits);
    }

    ids.into_iter().take(100).map(CompletionCandidate::new).collect()
}

/// Offers icon-set prefixes matching the current token, merging built-in
/// domain prefixes with cached set metadata. Completion never fails the shell.
///
/// When the current value contains commas (e.g. `--from mdi,ic`) only the
/// active segment after the last comma is completed; the already-entered
/// prefix is preserved in each candidate.
fn set_name_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let (prefix, needle) = match current_str.rfind(',') {
        Some(idx) => (&current_str[..=idx], &current_str[idx + 1..]),
        None => ("", current_str.as_ref()),
    };
    let needle = needle.to_lowercase();
    let mut prefixes = BTreeSet::new();

    for id in biscuit_icon::domain::all_iconify_ids() {
        if let Some((p, _)) = id.split_once(':')
            && p.to_lowercase().contains(&needle)
        {
            prefixes.insert(p.to_string());
        }
    }

    if let Ok(cache) = biscuit_icon::cache::IconCache::open_default() {
        if let Ok(sets) = cache.all_sets() {
            for set in sets {
                if set.prefix.to_lowercase().contains(&needle) {
                    prefixes.insert(set.prefix);
                }
            }
        }
        // Also include prefixes from cached icons even if no set metadata exists.
        if let Ok(cached) = cache.cached_prefixes() {
            for p in cached {
                if p.to_lowercase().contains(&needle) {
                    prefixes.insert(p);
                }
            }
        }
    }

    prefixes
        .into_iter()
        .take(50)
        .map(|p| CompletionCandidate::new(format!("{prefix}{p}")))
        .collect()
}

/// Offers the 16 curated domain set names (`os`, `emoji`, `brand`, ...) when
/// the shell asks for completions of the `icon domain <arg>` positional.
/// Substring match is case-insensitive; completion never fails the shell.
fn domain_arg_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let needle = current.to_string_lossy().to_lowercase();
    biscuit_icon::domain::domain_sets()
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| needle.is_empty() || name.contains(needle.as_str()))
        .map(|name| CompletionCandidate::new(name.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_arg_completer_returns_all_sets_for_empty_token() {
        let candidates = domain_arg_completer(std::ffi::OsStr::new(""));
        let names: Vec<String> =
            candidates.iter().map(|c| c.get_value().to_string_lossy().into_owned()).collect();
        assert_eq!(names.len(), 16, "expected all 16 domain sets; got: {names:?}");
        assert!(names.contains(&"os".to_string()));
        assert!(names.contains(&"emoji".to_string()));
        assert!(names.contains(&"brand".to_string()));
    }

    #[test]
    fn domain_arg_completer_substring_is_case_insensitive() {
        let candidates = domain_arg_completer(std::ffi::OsStr::new("OS"));
        let names: Vec<String> =
            candidates.iter().map(|c| c.get_value().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["os".to_string()], "expected only `os` for `OS`; got: {names:?}");
    }

    #[test]
    fn domain_arg_completer_unknown_token_returns_empty() {
        let candidates = domain_arg_completer(std::ffi::OsStr::new("zzz"));
        assert!(candidates.is_empty(), "expected no completions; got: {candidates:?}");
    }
}
