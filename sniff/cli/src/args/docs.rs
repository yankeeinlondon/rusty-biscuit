//! `sniff docs` filter type.

/// Filter options for the docs subcommand.
#[derive(Debug, Clone, Default)]
pub struct DocsFilter {
    /// Show only README files (case-insensitive).
    pub readme: bool,
    /// Show only documents with "plan" in the filename or path.
    pub plan: bool,
    /// Show only documents under a `src/` directory.
    pub src: bool,
    /// Show only documents with a prompt in frontmatter.
    pub has_prompt: bool,
    /// Show only documents that have a blast_radius frontmatter key.
    pub blast_radius: bool,
    /// Substring filter on filepath/filename (case-insensitive).
    pub filter: Vec<String>,
}
