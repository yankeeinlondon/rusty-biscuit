use std::collections::BTreeSet;

use clap::{Parser, Subcommand};
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

    /// Default `icons` filter when no subcommand is given (e.g. `icon mdi:home`).
    #[arg(value_name = "FILTER", add = ArgValueCompleter::new(icon_name_completer))]
    pub filter: Option<String>,

    /// Limit to these sets when using the default `icons` command.
    #[arg(long, value_name = "CSV")]
    pub from: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List icons whose name matches FILTER (rendered visually).
    Icons {
        /// Substring or `prefix:name` to match.
        #[arg(value_name = "FILTER", add = ArgValueCompleter::new(icon_name_completer))]
        filter: Option<String>,
        /// Limit to these sets (comma-separated prefixes), e.g. `fa,mdi`.
        #[arg(long, value_name = "CSV")]
        from: Option<String>,
    },
    /// List Iconify set names, optionally filtered.
    Sets {
        /// Substring to match against set prefixes/titles.
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
    /// Cache maintenance.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
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
    /// Delete all cached icons.
    Clear,
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
