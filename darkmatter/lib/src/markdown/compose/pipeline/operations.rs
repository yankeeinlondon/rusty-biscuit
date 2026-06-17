//! Compose operation registry: the `ComposeOperation` enum, its phase
//! grouping, the authoritative descriptor table, and the fixed-size
//! `ComposeOperationSet`.
//!
//! This is the registry/ordering half of the pipeline driver — the single
//! source of truth for which operations exist, what phase each runs in, and
//! their default execution order.

/// Every discrete operation in the compose pipeline.
///
/// Operations are grouped into four phases for execution:
/// - **Inline Pre**: serial, runs before transclusion
/// - **Transclusion**: concurrent, recursive document inclusion
/// - **Inline Post**: serial, runs after transclusion
/// - **Finalization**: root-only serial, runs after Inline Post on the outermost document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeOperation {
    /// Resolves `{{ variable }}` expressions inside frontmatter values
    /// using non-templated frontmatter values, `ctx`, and `env` as inputs.
    /// Runs before the final effective state is built.
    FrontmatterInterpolation,

    /// Executes shell commands embedded in frontmatter values.
    /// Runs before the final effective state is built.
    FrontmatterShellExpansion,

    /// Applies the frontmatter `replace` map to substitute text
    /// patterns throughout the document body.
    TextReplacement,

    /// Evaluates `::block when="..."` / `::end-block` conditional
    /// regions and removes blocks whose conditions are false.
    PageBlocks,

    /// Expands `{{variable}}` handlebars expressions using frontmatter,
    /// environment variables, and context variables.
    Interpolation,

    /// Executes approved `::shell` directives and replaces them
    /// with the command's stdout output.
    ShellExpansion,

    /// Executes approved `::shell-block` directives and replaces them
    /// with the combined output of the block's commands.
    ShellBlocks,

    /// Resolves `::file` and `::url` directives by including the
    /// referenced markdown document (recursively composed).
    BlockTransclusion,

    /// Resolves `prologue` and `epilogue` frontmatter references
    /// by prepending/appending the referenced documents.
    FrontmatterTransclusion,

    /// Resolves `::code` directives by including file content
    /// as a fenced code block.
    CodeTransclusion,

    /// Expands `::toc-linking` directives by generating a linked
    /// table of contents from an external document's headings.
    TocLinking,

    /// Expands `::file-links` directives by discovering a bounded set of
    /// document files and rendering them as a linked
    /// [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
    /// tree.
    FileLinks,

    /// Normalizes markdown formatting: injects blank lines between
    /// block elements and aligns table columns.
    Cleanup,

    /// Adjusts heading levels to ensure a valid hierarchy
    /// (e.g., no H3 before an H2).
    Normalization,

    /// Resolves all local link targets (Markdown hyperlinks/images and
    /// supported HTML embeds) to absolute paths during the Inline-Pre stage.
    LinkResolve,

    /// Converts absolute path links back into portable forms during the
    /// Finalization stage (relative paths within the same repo, `~/` for
    /// home-relative paths, `${VAR}` for whitelisted environment-relative
    /// paths).
    LinkNormalization,
}

/// Operation-level performance metric kinds.
///
/// These correspond to the user-toggleable [`ComposeOperation`] variants that
/// have dedicated timing metrics in the runner. Transclusion operations are
/// measured as parse/prepare/resolve/apply sub-stages rather than per
/// operation, so they have no entry here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeOperationPerfMetric {
    /// Frontmatter interpolation.
    FrontmatterInterpolation,
    /// Frontmatter shell expansion.
    FrontmatterShellExpansion,
    /// Text replacement.
    TextReplacement,
    /// Page blocks.
    PageBlocks,
    /// Body interpolation.
    Interpolation,
    /// Shell directive expansion.
    ShellExpansion,
    /// Shell block execution.
    ShellBlocks,
    /// Local link resolution to absolute paths.
    LinkResolve,
    /// Cleanup formatting pass.
    Cleanup,
    /// Heading normalization.
    Normalization,
    /// Link normalization to portable forms.
    LinkNormalization,
}

/// Metadata describing a single compose pipeline operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposeOperationDescriptor {
    /// The operation this descriptor describes.
    pub operation: ComposeOperation,
    /// Stable index used for fixed-size operation sets and reports.
    pub index: usize,
    /// Execution phase this operation belongs to.
    pub phase: ComposePhase,
    /// Whether the operation is enabled by default.
    pub default_enabled: bool,
    /// Human-readable label for reports and diagnostics.
    pub label: &'static str,
    /// Operation-level perf metric, if one exists.
    pub perf_kind: Option<ComposeOperationPerfMetric>,
}

/// Authoritative metadata table for every [`ComposeOperation`] variant.
///
/// Entries are ordered by default execution order (the order returned by
/// [`ComposeOperation::default_order`]) so that the descriptor table is the
/// single source of truth for both operation metadata and run order. The
/// `index` field is a stable identifier that matches the historical enum
/// discriminant values used by [`ComposeOperationSet`].
pub(crate) const COMPOSE_OPERATION_DESCRIPTORS: &[ComposeOperationDescriptor] = &[
    ComposeOperationDescriptor {
        operation: ComposeOperation::FrontmatterInterpolation,
        index: 0,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "frontmatter interpolation",
        perf_kind: Some(ComposeOperationPerfMetric::FrontmatterInterpolation),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::FrontmatterShellExpansion,
        index: 1,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "frontmatter shell expansion",
        perf_kind: Some(ComposeOperationPerfMetric::FrontmatterShellExpansion),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::TextReplacement,
        index: 2,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "text replacement",
        perf_kind: Some(ComposeOperationPerfMetric::TextReplacement),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::PageBlocks,
        index: 3,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "page blocks",
        perf_kind: Some(ComposeOperationPerfMetric::PageBlocks),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::Interpolation,
        index: 4,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "interpolation",
        perf_kind: Some(ComposeOperationPerfMetric::Interpolation),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::ShellExpansion,
        index: 5,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "shell expansion",
        perf_kind: Some(ComposeOperationPerfMetric::ShellExpansion),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::ShellBlocks,
        index: 6,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "shell blocks",
        perf_kind: Some(ComposeOperationPerfMetric::ShellBlocks),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::LinkResolve,
        index: 14,
        phase: ComposePhase::InlinePre,
        default_enabled: true,
        label: "link resolve",
        perf_kind: Some(ComposeOperationPerfMetric::LinkResolve),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::BlockTransclusion,
        index: 7,
        phase: ComposePhase::Transclusion,
        default_enabled: true,
        label: "block transclusion",
        perf_kind: None,
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::FrontmatterTransclusion,
        index: 8,
        phase: ComposePhase::Transclusion,
        default_enabled: true,
        label: "frontmatter transclusion",
        perf_kind: None,
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::CodeTransclusion,
        index: 9,
        phase: ComposePhase::Transclusion,
        default_enabled: true,
        label: "code transclusion",
        perf_kind: None,
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::TocLinking,
        index: 10,
        phase: ComposePhase::Transclusion,
        default_enabled: true,
        label: "TOC linking",
        perf_kind: None,
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::FileLinks,
        index: 11,
        phase: ComposePhase::Transclusion,
        default_enabled: true,
        label: "file links",
        perf_kind: None,
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::Cleanup,
        index: 12,
        phase: ComposePhase::InlinePost,
        default_enabled: true,
        label: "cleanup",
        perf_kind: Some(ComposeOperationPerfMetric::Cleanup),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::Normalization,
        index: 13,
        phase: ComposePhase::InlinePost,
        default_enabled: true,
        label: "normalization",
        perf_kind: Some(ComposeOperationPerfMetric::Normalization),
    },
    ComposeOperationDescriptor {
        operation: ComposeOperation::LinkNormalization,
        index: 15,
        phase: ComposePhase::Finalization,
        default_enabled: true,
        label: "link normalization",
        perf_kind: Some(ComposeOperationPerfMetric::LinkNormalization),
    },
];

/// Fixed-size operation set keyed by [`ComposeOperation`] discriminants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ComposeOperationSet {
    enabled: [bool; ComposeOperation::COUNT],
}

impl ComposeOperationSet {
    /// Creates an empty operation set.
    pub fn empty() -> Self {
        Self {
            enabled: [false; ComposeOperation::COUNT],
        }
    }

    /// Creates a set containing every operation.
    pub fn all() -> Self {
        ComposeOperation::default_order().iter().copied().collect()
    }

    /// Enables an operation.
    pub fn insert(&mut self, op: ComposeOperation) {
        self.enabled[op.index()] = true;
    }

    /// Disables an operation.
    pub fn remove(&mut self, op: ComposeOperation) {
        self.enabled[op.index()] = false;
    }

    /// Returns `true` when the operation is enabled.
    pub fn contains(&self, op: ComposeOperation) -> bool {
        self.enabled[op.index()]
    }

    /// Iterates enabled operations in canonical enum order.
    pub fn iter(&self) -> impl Iterator<Item = ComposeOperation> + '_ {
        ComposeOperation::default_order()
            .iter()
            .copied()
            .filter(|op| self.contains(*op))
    }
}

impl std::fmt::Debug for ComposeOperationSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Default for ComposeOperationSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl FromIterator<ComposeOperation> for ComposeOperationSet {
    fn from_iter<T: IntoIterator<Item = ComposeOperation>>(iter: T) -> Self {
        let mut set = Self::empty();
        for op in iter {
            set.insert(op);
        }
        set
    }
}

/// The execution phase an operation belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposePhase {
    /// Serial operations before transclusion.
    InlinePre,
    /// Concurrent recursive document inclusion.
    Transclusion,
    /// Serial operations after transclusion.
    InlinePost,
    /// Root-only finalization stage. Runs after Inline-Post completes on the
    /// outermost document and is skipped on transcluded children.
    Finalization,
}

impl ComposeOperation {
    /// Total number of compose operations.
    pub const COUNT: usize = COMPOSE_OPERATION_DESCRIPTORS.len();

    /// Returns the descriptor for this operation.
    ///
    /// Descriptors are ordered by default execution order. The lookup is a
    /// small linear search because the table is fixed at 16 entries.
    pub fn descriptor(self) -> &'static ComposeOperationDescriptor {
        COMPOSE_OPERATION_DESCRIPTORS
            .iter()
            .find(|d| d.operation == self)
            .expect("every ComposeOperation has a descriptor")
    }

    /// Stable discriminant index for fixed-size operation sets.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Returns the default phase this operation belongs to.
    pub fn phase(&self) -> ComposePhase {
        self.descriptor().phase
    }

    /// Returns all operations in their default execution order.
    pub fn default_order() -> &'static [ComposeOperation] {
        &[
            // Inline Pre (serial)
            Self::FrontmatterInterpolation,
            Self::FrontmatterShellExpansion,
            Self::TextReplacement,
            Self::PageBlocks,
            Self::Interpolation,
            Self::ShellExpansion,
            Self::ShellBlocks,
            Self::LinkResolve,
            // Transclusion (concurrent)
            Self::BlockTransclusion,
            Self::FrontmatterTransclusion,
            Self::CodeTransclusion,
            Self::TocLinking,
            Self::FileLinks,
            // Inline Post (serial)
            Self::Cleanup,
            Self::Normalization,
            // Finalization (root-only)
            Self::LinkNormalization,
        ]
    }

    /// Human-readable label for reports and diagnostics.
    pub fn label(self) -> &'static str {
        self.descriptor().label
    }

    /// Operation-level perf metric, if one exists.
    pub fn perf_metric(self) -> Option<ComposeOperationPerfMetric> {
        self.descriptor().perf_kind
    }

    /// Returns the set of all operations.
    pub fn all() -> ComposeOperationSet {
        ComposeOperationSet::all()
    }
}

impl std::fmt::Display for ComposeOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrontmatterInterpolation => write!(f, "FrontmatterInterpolation"),
            Self::FrontmatterShellExpansion => write!(f, "Frontmatter Shell Expansion"),
            Self::TextReplacement => write!(f, "TextReplacement"),
            Self::PageBlocks => write!(f, "PageBlocks"),
            Self::Interpolation => write!(f, "Interpolation"),
            Self::ShellExpansion => write!(f, "ShellExpansion"),
            Self::ShellBlocks => write!(f, "ShellBlocks"),
            Self::BlockTransclusion => write!(f, "BlockTransclusion"),
            Self::FrontmatterTransclusion => write!(f, "FrontmatterTransclusion"),
            Self::CodeTransclusion => write!(f, "CodeTransclusion"),
            Self::TocLinking => write!(f, "TocLinking"),
            Self::FileLinks => write!(f, "FileLinks"),
            Self::Cleanup => write!(f, "Cleanup"),
            Self::Normalization => write!(f, "Normalization"),
            Self::LinkResolve => write!(f, "LinkResolve"),
            Self::LinkNormalization => write!(f, "LinkNormalization"),
        }
    }
}

impl std::fmt::Display for ComposePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InlinePre => write!(f, "InlinePre"),
            Self::Transclusion => write!(f, "Transclusion"),
            Self::InlinePost => write!(f, "InlinePost"),
            Self::Finalization => write!(f, "finalization"),
        }
    }
}
