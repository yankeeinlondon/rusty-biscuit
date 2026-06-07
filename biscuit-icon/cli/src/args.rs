use clap::{Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

/// Curated domain icons + on-demand Iconify lookup.
#[derive(Parser, Debug)]
#[command(name = "icon", version, about, long_about = None)]
pub struct Cli {
    /// Increase diagnostic verbosity on stderr (-v, -vv, -vvv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Default `icons` filter when no subcommand is given (e.g. `icon mdi:home`).
    #[arg(value_name = "FILTER", add = ArgValueCompleter::new(icon_name_completer))]
    pub filter: Option<String>,
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

/// Offers cached `prefix:name` ids matching the current token. Best-effort:
/// completion never fails the shell, so cache errors yield no candidates.
fn icon_name_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let needle = current.to_string_lossy();
    let mut out = Vec::new();
    if let Ok(cache) = biscuit_icon::cache::IconCache::open_default()
        && let Ok(hits) = cache.search_names(&needle)
    {
        out.extend(hits.into_iter().map(CompletionCandidate::new));
    }
    out
}
