//! Arguments shared by `repo recent-commits`, `repo source-code-changes`, and
//! `repo documentation-changes`.

use clap_complete::engine::{ArgValueCandidates, CompletionCandidate};
use sniff::filesystem::git::{
    RecentCommitsOptions, RecentCommitsProjection, RecentCommitsVerbosity, Selection,
};
use sniff::filesystem::path_kind::ChangeCategory;

use super::{repo_package_area_candidates, repo_package_candidates};

#[derive(clap::Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentCommitsArgs {
    /// Which commits: a count (10), duration (3d, 1w), date (YYYY-MM-DD), 'today', 'yesterday', or a hash to walk back to [default: the last 10 commits]
    pub period: Option<String>,

    /// Keep conventional commits with this operation (any word); repeat to match any of several
    #[arg(long = "operation", value_name = "OPERATION", add = ArgValueCandidates::new(operation_candidates))]
    pub operations: Vec<String>,

    /// Keep conventional commits with this scope
    #[arg(long, value_name = "SCOPE")]
    pub scope: Option<String>,

    /// Keep commits whose author name or email contains this text
    #[arg(long, value_name = "NAME|EMAIL")]
    pub author: Option<String>,

    /// Walk history from this branch instead of HEAD (local first, then remote-tracking)
    #[arg(long, value_name = "BRANCH")]
    pub branch: Option<String>,

    /// Keep commits touching this monorepo package
    #[arg(long, value_name = "PKG", add = ArgValueCandidates::new(repo_package_candidates))]
    pub package: Option<String>,

    /// Keep commits touching this monorepo package area
    #[arg(long, value_name = "AREA", add = ArgValueCandidates::new(repo_package_area_candidates))]
    pub package_area: Option<String>,

    /// Keep commits that change source code
    #[arg(long)]
    pub source_code: bool,

    /// Keep commits that change web assets (HTML, CSS, fonts)
    #[arg(long)]
    pub web: bool,

    /// Keep commits that change images
    #[arg(long)]
    pub images: bool,

    /// Keep commits that change documentation
    #[arg(long)]
    pub documentation: bool,

    /// Keep commits that change configuration
    #[arg(long)]
    pub configuration: bool,

    /// Keep commits that change CI/CD definitions
    #[arg(long)]
    pub cicd: bool,

    /// Show each commit's author in its header line
    #[arg(long)]
    pub show_author: bool,

    /// Include each commit's description and bullet points
    // Clap merges global arguments across command levels by id, so this must
    // keep the global `verbose` id and its `Count` type; any other shape
    // panics or loses the flag to the global counter (see
    // `recent_commits_flag_shadowing`).
    #[arg(short, long, action = clap::ArgAction::Count, conflicts_with = "compact")]
    pub verbose: u8,

    /// Show only each commit's header line
    #[arg(short, long)]
    pub compact: bool,
}

impl RecentCommitsArgs {
    /// The library options these arguments select for `projection`.
    ///
    /// `--compact` wins over a `-v` given before the subcommand, which clap
    /// cannot reject because that value arrives by global propagation.
    ///
    /// ## Errors
    ///
    /// Returns [`sniff::SniffError::InvalidPeriod`] when the period matches no
    /// selection form.
    pub fn to_options(&self, projection: RecentCommitsProjection) -> sniff::Result<RecentCommitsOptions> {
        let mut options = RecentCommitsOptions::new().projection(projection);
        if let Some(period) = &self.period {
            options = options.selection(Selection::parse(period)?);
        }
        for operation in &self.operations {
            options = options.operation(operation);
        }
        if let Some(scope) = &self.scope {
            options = options.scope(scope);
        }
        if let Some(author) = &self.author {
            options = options.author(author);
        }
        if let Some(branch) = &self.branch {
            options = options.branch(branch);
        }
        if let Some(package) = &self.package {
            options = options.package(package);
        }
        if let Some(package_area) = &self.package_area {
            options = options.package_area(package_area);
        }
        for (selected, category) in [
            (self.source_code, ChangeCategory::SourceCode),
            (self.web, ChangeCategory::WebAssets),
            (self.images, ChangeCategory::Images),
            (self.documentation, ChangeCategory::Documentation),
            (self.configuration, ChangeCategory::Configuration),
            (self.cicd, ChangeCategory::Cicd),
        ] {
            if selected {
                options = options.has_file_type(category);
            }
        }
        let verbosity = if self.compact {
            RecentCommitsVerbosity::Compact
        } else if self.verbose > 0 {
            RecentCommitsVerbosity::Verbose
        } else {
            RecentCommitsVerbosity::Normal
        };
        Ok(options.verbosity(verbosity).show_author(self.show_author))
    }
}

/// Common conventional-commit operations; `--operation` accepts any word.
pub(crate) fn operation_candidates() -> Vec<CompletionCandidate> {
    [
        "feat", "fix", "chore", "refactor", "test", "style", "docs", "perf", "build", "ci",
        "revert",
    ]
    .into_iter()
    .map(CompletionCandidate::new)
    .collect()
}
