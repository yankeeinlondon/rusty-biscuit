//! The entry-point parity matrix: one fixture, two tables, one comparison.
//!
//! Every entry point that resolves file references must give the same answer
//! for the same reference. The fixture is a small monorepo (a package area
//! holding one package) beside a fixture `HOME` and an `outside.md` that lies
//! above `HOME` and outside every repository:
//!
//! ```text
//! {root}/
//!   outside.md
//!   home/                      the fixture HOME
//!     magic-doc.md             the `@` target (the chain's home tier)
//!     .claudine/prompts/       the configured extra `@` root
//!       configured-doc.md
//!     notes/beside.md
//!   repo/                      git repository; Cargo workspace
//!     root-only.md
//!     area/area-doc.md         package area
//!     area/pkg/                package (depth 1)
//!       pkg-only.md
//!       docs/                  depth 2
//!         guide/               depth 3
//! ```
//!
//! Each depth directory holds its own `sibling.md` and `beside.md`. Every
//! target is a Markdown file whose only content is `## TARGET <id>`, where
//! `<id>` is its `/`-spelled path below the fixture root, so `::file`,
//! `::code`, and `::toc-linking` output all name the file they resolved.
//!
//! **Table 1** ([`DocumentCell`]) puts one reference in one document per cell,
//! so a failing cell cannot mask another. **Table 2** ([`ValueCell`]) supplies
//! the reference as a caller value from a launch directory. A cell expects one
//! file, compared by [`canonicalize_simplified`] then [`PathIdentity`], or one
//! [`ResolutionFailure`] class; messages and wrapper types are never compared.
//!
//! Rows come from an exhaustive `match` over [`EntryPoint`] (no `_` arm), so a
//! new entry point without rows does not compile, and each runner matches the
//! same enum, so it does not compile in any runner either.
//!
//! `home/.claudine/prompts/` is Claudine's user prompt root, the one extra `@`
//! root a shipped binary configures. The darkmatter and dmls runners register
//! the same directory on their request snapshot
//! ([`ParityFixture::configured_magic_root`]), so `@configured-doc.md` is
//! reachable only through a configured root at every entry point that has one.
//!
//! Shared by `#[path]` with the darkmatter-cli, dmls, and claudine-cli
//! runners; each declares this file in `[package.metadata.ci.tests]
//! source-inputs`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use biscuit_file::{PathIdentity, ResolutionFailure, canonicalize_simplified, to_portable_string};

/// The package whose runner executes an entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Darkmatter,
    DarkmatterCli,
    Dmls,
    ClaudineCli,
}

/// Every entry point of change 1 that resolves a file reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryPoint {
    /// `Markdown::compose_with`.
    ComposePipeline,
    /// `Markdown::compose_preflight`.
    Preflight,
    /// `DarkmatterSchemas::validate` plus eager normalization.
    SchemaValidation,
    /// `md compose <document>`.
    MdCompose,
    /// `md schema validate <document>`.
    MdSchemaValidate,
    /// `md compose <value>`: the document path argument (Table 2).
    MdArgument,
    /// DMLS published diagnostics for schema `file` values.
    DmlsDiagnostics,
    /// `textDocument/documentLink`.
    DmlsDocumentLinks,
    /// The workspace link graph: `references`, `transcludes`, and
    /// `uses_file` edges.
    DmlsLinkGraph,
    /// `textDocument/definition`.
    DmlsDefinition,
    /// `textDocument/codeAction` (create-file quick fixes, offered only for a
    /// broken Markdown link; every other consumer must be offered none).
    DmlsCodeActions,
    /// Claudine composition of a prompt.
    ClaudineComposition,
    /// Claudine completion of a schema `file` value (Table 2).
    ClaudineCompletion,
}

impl EntryPoint {
    pub const ALL: [EntryPoint; 13] = [
        Self::ComposePipeline,
        Self::Preflight,
        Self::SchemaValidation,
        Self::MdCompose,
        Self::MdSchemaValidate,
        Self::MdArgument,
        Self::DmlsDiagnostics,
        Self::DmlsDocumentLinks,
        Self::DmlsLinkGraph,
        Self::DmlsDefinition,
        Self::DmlsCodeActions,
        Self::ClaudineComposition,
        Self::ClaudineCompletion,
    ];

    pub fn owner(self) -> Owner {
        match self {
            Self::ComposePipeline | Self::Preflight | Self::SchemaValidation => Owner::Darkmatter,
            Self::MdCompose | Self::MdSchemaValidate | Self::MdArgument => Owner::DarkmatterCli,
            Self::DmlsDiagnostics
            | Self::DmlsDocumentLinks
            | Self::DmlsLinkGraph
            | Self::DmlsDefinition
            | Self::DmlsCodeActions => Owner::Dmls,
            Self::ClaudineComposition | Self::ClaudineCompletion => Owner::ClaudineCli,
        }
    }

    /// This entry point's cells in both tables.
    pub fn rows(self) -> Vec<Row> {
        use Consumer::*;
        const EDITOR: &[Consumer] = &[File, Code, TocLinking, SchemaFile, MarkdownLink];
        match self {
            // Row (a) reaches the library only as `::file ~/…`.
            Self::ComposePipeline => document_rows(self, &[File, Code, TocLinking, SchemaFile], &Form::ALL, true),
            Self::Preflight => document_rows(self, &[File, Code, TocLinking, SchemaFile], &Form::ALL, true),
            Self::SchemaValidation => document_rows(self, &[SchemaFile], &Form::ALL, false),
            // `md` configures no extra `@` root: its snapshot is the process's
            // directory, home, and environment, so it has no
            // `Form::ConfiguredMagic` cell.
            Self::MdCompose => document_rows(self, &[File, Code, TocLinking, SchemaFile], &UNCONFIGURED, false),
            Self::MdSchemaValidate => document_rows(self, &[SchemaFile], &UNCONFIGURED, false),
            // A quoted `'~/…'` argument is the only `~` spelling `md` sees.
            Self::MdArgument => value_rows(self, &UNCONFIGURED, true),
            Self::DmlsDiagnostics => document_rows(self, &[SchemaFile], &Form::ALL, false),
            Self::DmlsDocumentLinks => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsLinkGraph => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsDefinition => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsCodeActions => document_rows(self, EDITOR, &Form::ALL, false),
            Self::ClaudineComposition => document_rows(self, &[File, Code, TocLinking, SchemaFile], &Form::ALL, false),
            Self::ClaudineCompletion => value_rows(self, &Form::ALL, false),
        }
    }
}

/// Every row of both tables for the entry points `owner` runs.
pub fn rows_for(owner: Owner) -> Vec<Row> {
    EntryPoint::ALL
        .into_iter()
        .filter(|entry| entry.owner() == owner)
        .flat_map(EntryPoint::rows)
        .collect()
}

/// The construct in a document that carries the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Consumer {
    File,
    Code,
    TocLinking,
    /// A frontmatter value typed `file(eager)`.
    SchemaFile,
    /// A Markdown link `[target](…)`.
    MarkdownLink,
}

impl Consumer {
    fn slug(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Code => "code",
            Self::TocLinking => "toc",
            Self::SchemaFile => "schema",
            Self::MarkdownLink => "link",
        }
    }

    /// A whole document whose one reference is `reference`.
    pub fn document(self, reference: &str) -> String {
        match self {
            Self::File => format!("# Cell\n\n::file {reference}\n"),
            Self::Code => format!("# Cell\n\n::code {reference}\n"),
            Self::TocLinking => format!("# Cell\n\n::toc-linking {reference}\n"),
            Self::SchemaFile => format!(
                "---\n$schema:\n  target: file(eager)\ntarget: \"{reference}\"\n---\n\n# Cell\n\n{STORED_VALUE_PREFIX}{{{{ target }}}}\n"
            ),
            Self::MarkdownLink => format!("# Cell\n\n[target]({reference})\n"),
        }
    }
}

/// The body line a [`Consumer::SchemaFile`] cell renders its normalized
/// value on; see [`ParityFixture::stored_value_targets`].
pub const STORED_VALUE_PREFIX: &str = "stored-value=";

/// A reference form; one row per form in each table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    /// `./sibling.md`
    ExplicitRelative,
    /// `beside.md`
    BareBeside,
    /// `root-only.md`, which exists only at the repository root.
    BareRootOnly,
    /// `&root-only.md`
    RepositoryRoot,
    /// `^area-doc.md` (package, then area, then repository).
    RepositoryScoped,
    /// `@magic-doc.md`, found in the `@` chain's home tier.
    Magic,
    /// `@configured-doc.md`, found only under the configured extra `@` root.
    ConfiguredMagic,
    /// `~/notes/beside.md`
    Home,
    /// `../…/outside.md`, climbing past the repository root.
    TreeEscape,
}

impl Form {
    pub const ALL: [Form; 9] = [
        Self::ExplicitRelative,
        Self::BareBeside,
        Self::BareRootOnly,
        Self::RepositoryRoot,
        Self::RepositoryScoped,
        Self::Magic,
        Self::ConfiguredMagic,
        Self::Home,
        Self::TreeEscape,
    ];

    fn slug(self) -> &'static str {
        match self {
            Self::ExplicitRelative => "explicit",
            Self::BareBeside => "beside",
            Self::BareRootOnly => "root-only",
            Self::RepositoryRoot => "amp",
            Self::RepositoryScoped => "caret",
            Self::Magic => "magic",
            Self::ConfiguredMagic => "configured",
            Self::Home => "home",
            Self::TreeEscape => "escape",
        }
    }

    /// The spelling from a directory `levels_below_root` directories below
    /// the fixture root (only the tree escape depends on it).
    fn spelling(self, levels_below_root: usize) -> String {
        match self {
            Self::ExplicitRelative => "./sibling.md".into(),
            Self::BareBeside => "beside.md".into(),
            Self::BareRootOnly => "root-only.md".into(),
            Self::RepositoryRoot => "&root-only.md".into(),
            Self::RepositoryScoped => "^area-doc.md".into(),
            Self::Magic => "@magic-doc.md".into(),
            Self::ConfiguredMagic => "@configured-doc.md".into(),
            Self::Home => "~/notes/beside.md".into(),
            Self::TreeEscape => format!("{}outside.md", "../".repeat(levels_below_root)),
        }
    }
}

/// Every form except [`Form::ConfiguredMagic`], for an entry point that
/// configures no extra `@` root.
const UNCONFIGURED: [Form; 8] = [
    Form::ExplicitRelative,
    Form::BareBeside,
    Form::BareRootOnly,
    Form::RepositoryRoot,
    Form::RepositoryScoped,
    Form::Magic,
    Form::Home,
    Form::TreeEscape,
];

/// A document's directory in the package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Depth {
    /// The package root, `repo/area/pkg`.
    One,
    /// `repo/area/pkg/docs`
    Two,
    /// `repo/area/pkg/docs/guide`
    Three,
}

impl Depth {
    pub const ALL: [Depth; 3] = [Self::One, Self::Two, Self::Three];

    fn relative(self) -> &'static str {
        match self {
            Self::One => "repo/area/pkg",
            Self::Two => "repo/area/pkg/docs",
            Self::Three => "repo/area/pkg/docs/guide",
        }
    }

    fn levels_below_root(self) -> usize {
        self.relative().split('/').count()
    }
}

/// A reference written in a document under the fixture `HOME`
/// (`notes/…`), for rows (a) and (b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotesReference {
    /// `./beside.md`: resolves beside the document either way.
    Beside,
    /// `../../outside.md`: above `HOME`, so it escapes a document opened
    /// through `~` and resolves in one opened by its absolute path.
    Outside,
}

impl NotesReference {
    pub const ALL: [NotesReference; 2] = [Self::Beside, Self::Outside];

    fn spelling(self) -> &'static str {
        match self {
            Self::Beside => "./beside.md",
            Self::Outside => "../../outside.md",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Beside => "notes-beside",
            Self::Outside => "notes-outside",
        }
    }
}

/// How the document holding a Table 1 reference is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// A document in the package at `Depth`, opened by its path.
    Repository(Form, Depth),
    /// Row (a): a document under `HOME` reached through `~`
    /// (`::file ~/notes/…` from the repository root).
    OpenedThroughHome(NotesReference),
    /// Row (b): the same document opened by its absolute path.
    OpenedByAbsolutePath(NotesReference),
}

/// One Table 1 cell: entry point × consumer × form × depth (or opening).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentCell {
    pub entry: EntryPoint,
    pub consumer: Consumer,
    pub placement: Placement,
}

/// Where a Table 2 value is supplied from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Launch {
    RepositoryRoot,
    /// The package directory, `repo/area/pkg`.
    Package,
}

impl Launch {
    pub const ALL: [Launch; 2] = [Self::RepositoryRoot, Self::Package];

    fn relative(self) -> &'static str {
        match self {
            Self::RepositoryRoot => "repo",
            Self::Package => "repo/area/pkg",
        }
    }
}

/// The value a Table 2 cell supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Supplied {
    Form(Form),
    /// Row (a): `~/notes/<document>`, a document whose one `::file`
    /// reference is the `NotesReference`.
    ThroughHome(NotesReference),
    /// Row (b): the same document by its absolute path.
    AbsolutePath(NotesReference),
}

/// One Table 2 cell: entry point × value × launch directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueCell {
    pub entry: EntryPoint,
    pub value: Supplied,
    pub launch: Launch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Row {
    Document(DocumentCell),
    Value(ValueCell),
}

impl Row {
    pub fn entry(&self) -> EntryPoint {
        match self {
            Self::Document(cell) => cell.entry,
            Self::Value(cell) => cell.entry,
        }
    }
}

fn document_rows(entry: EntryPoint, consumers: &[Consumer], forms: &[Form], through_home: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    for &consumer in consumers {
        for &form in forms {
            for depth in Depth::ALL {
                rows.push(Row::Document(DocumentCell {
                    entry,
                    consumer,
                    placement: Placement::Repository(form, depth),
                }));
            }
        }
        for reference in NotesReference::ALL {
            rows.push(Row::Document(DocumentCell {
                entry,
                consumer,
                placement: Placement::OpenedByAbsolutePath(reference),
            }));
            if through_home {
                rows.push(Row::Document(DocumentCell {
                    entry,
                    consumer,
                    placement: Placement::OpenedThroughHome(reference),
                }));
            }
        }
    }
    rows
}

fn value_rows(entry: EntryPoint, forms: &[Form], documents: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    for launch in Launch::ALL {
        for &form in forms {
            rows.push(Row::Value(ValueCell { entry, value: Supplied::Form(form), launch }));
        }
        if documents {
            for reference in NotesReference::ALL {
                rows.push(Row::Value(ValueCell { entry, value: Supplied::ThroughHome(reference), launch }));
                rows.push(Row::Value(ValueCell { entry, value: Supplied::AbsolutePath(reference), launch }));
            }
        }
    }
    rows
}

/// A cell's expected result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expected {
    File(PathBuf),
    Failure(ResolutionFailure),
}

/// What an entry point produced for one cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// The one file the reference resolved to.
    File(PathBuf),
    /// The failure class on the reference's error.
    Failure(ResolutionFailure),
    /// The entry point succeeded without naming a file. Only an entry point
    /// whose success carries no path (pre-flight's schema validation)
    /// reports this; it satisfies an `Expected::File` cell.
    Accepted,
    /// Anything else, described for the report (several files, a failure
    /// with no class, an unexpected success).
    Unexpected(String),
}

/// The fixture on disk. `root` must exist and should be empty; `md`
/// runners pass their `CliProcessFixture` workspace, whose `home/` is the
/// child's `HOME`.
pub struct ParityFixture {
    root: PathBuf,
}

const TARGET_MARKER: &str = "TARGET ";

impl ParityFixture {
    pub fn create(root: &Path) -> Self {
        let fixture = Self { root: root.to_path_buf() };
        let repo = fixture.repo();
        std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
        std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            &repo.join("area/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&repo.join("area/pkg/src/lib.rs"), "");

        let mut targets = vec![
            "outside.md".to_string(),
            "home/magic-doc.md".into(),
            "home/.claudine/prompts/configured-doc.md".into(),
            "home/notes/beside.md".into(),
            "repo/root-only.md".into(),
            "repo/area/area-doc.md".into(),
            "repo/area/pkg/pkg-only.md".into(),
        ];
        for depth in Depth::ALL {
            targets.push(format!("{}/sibling.md", depth.relative()));
            targets.push(format!("{}/beside.md", depth.relative()));
        }
        for id in targets {
            write(&root.join(&id), &format!("## {TARGET_MARKER}{id}\n"));
        }
        fixture
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    pub fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The extra `@` root a runner registers on its request snapshot:
    /// Claudine's user prompt root under the fixture `HOME`.
    pub fn configured_magic_root(&self) -> PathBuf {
        self.home().join(".claudine/prompts")
    }

    pub fn package(&self) -> PathBuf {
        self.root.join("repo/area/pkg")
    }

    pub fn depth_dir(&self, depth: Depth) -> PathBuf {
        self.root.join(depth.relative())
    }

    pub fn launch_dir(&self, launch: Launch) -> PathBuf {
        self.root.join(launch.relative())
    }

    /// The document the entry point opens for `cell`, written to disk.
    ///
    /// Row (a)'s document is the repository-root document that transcludes
    /// the `HOME` document through `~`; its consumer is the `HOME`
    /// document's.
    pub fn write_document(&self, cell: &DocumentCell) -> PathBuf {
        let consumer = cell.consumer;
        match cell.placement {
            Placement::Repository(form, depth) => {
                let path = self
                    .depth_dir(depth)
                    .join(format!("cell-{}-{}.md", consumer.slug(), form.slug()));
                write(&path, &consumer.document(&form.spelling(depth.levels_below_root())));
                path
            }
            Placement::OpenedByAbsolutePath(reference) => self.write_notes_document(consumer, reference),
            Placement::OpenedThroughHome(reference) => {
                let notes = self.write_notes_document(consumer, reference);
                let name = notes.file_name().unwrap().to_string_lossy().into_owned();
                let path = self.repo().join(format!("through-home-{name}"));
                write(&path, &Consumer::File.document(&format!("~/notes/{name}")));
                path
            }
        }
    }

    fn write_notes_document(&self, consumer: Consumer, reference: NotesReference) -> PathBuf {
        let path = self
            .home()
            .join(format!("notes/cell-{}-{}.md", consumer.slug(), reference.slug()));
        write(&path, &consumer.document(reference.spelling()));
        path
    }

    /// The value a Table 2 cell supplies, writing any document it names.
    pub fn value(&self, cell: &ValueCell) -> String {
        let launch_levels = cell.launch.relative().split('/').count();
        match cell.value {
            Supplied::Form(form) => form.spelling(launch_levels),
            Supplied::ThroughHome(reference) => {
                let notes = self.write_notes_document(Consumer::File, reference);
                format!("~/notes/{}", notes.file_name().unwrap().to_string_lossy())
            }
            Supplied::AbsolutePath(reference) => {
                self.write_notes_document(Consumer::File, reference).to_string_lossy().into_owned()
            }
        }
    }

    pub fn expected_document(&self, cell: &DocumentCell) -> Expected {
        match cell.placement {
            Placement::Repository(form, depth) => self.expected_form(form, &self.depth_dir(depth), true),
            Placement::OpenedThroughHome(reference) => self.expected_notes(reference, true),
            Placement::OpenedByAbsolutePath(reference) => self.expected_notes(reference, false),
        }
    }

    pub fn expected_value(&self, cell: &ValueCell) -> Expected {
        match cell.value {
            Supplied::Form(form) => {
                let dir = self.launch_dir(cell.launch);
                let package = cell.launch == Launch::Package;
                self.expected_form(form, &dir, package)
            }
            Supplied::ThroughHome(reference) => self.expected_notes(reference, true),
            Supplied::AbsolutePath(reference) => self.expected_notes(reference, false),
        }
    }

    /// `dir` is where `./` and bare references start; `in_package` says
    /// whether `^` sees the package and its area from there (from the
    /// repository root it sees only the repository root).
    fn expected_form(&self, form: Form, dir: &Path, in_package: bool) -> Expected {
        let file_or_miss = |path: PathBuf| {
            if path.is_file() {
                Expected::File(path)
            } else {
                Expected::Failure(ResolutionFailure::NoMatch)
            }
        };
        match form {
            Form::ExplicitRelative => file_or_miss(dir.join("sibling.md")),
            Form::BareBeside => file_or_miss(dir.join("beside.md")),
            Form::BareRootOnly | Form::RepositoryRoot => Expected::File(self.repo().join("root-only.md")),
            Form::RepositoryScoped if in_package => Expected::File(self.repo().join("area/area-doc.md")),
            Form::RepositoryScoped => Expected::Failure(ResolutionFailure::NoMatch),
            Form::Magic => Expected::File(self.home().join("magic-doc.md")),
            Form::ConfiguredMagic => Expected::File(self.configured_magic_root().join("configured-doc.md")),
            Form::Home => Expected::File(self.home().join("notes/beside.md")),
            // The repository root is a tree boundary (`RelativeTreeEscape`).
            Form::TreeEscape => Expected::Failure(ResolutionFailure::InvalidReference),
        }
    }

    /// Through `~`, `HOME` is the document's tree root and a boundary; by
    /// absolute path the tree root's origin is Fallback, not a boundary.
    fn expected_notes(&self, reference: NotesReference, through_home: bool) -> Expected {
        match (reference, through_home) {
            (NotesReference::Beside, _) => Expected::File(self.home().join("notes/beside.md")),
            (NotesReference::Outside, true) => Expected::Failure(ResolutionFailure::InvalidReference),
            (NotesReference::Outside, false) => Expected::File(self.root.join("outside.md")),
        }
    }

    /// Every fixture target named in rendered output, in order of first
    /// appearance.
    pub fn marked_targets(&self, output: &str) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::new();
        for (offset, _) in output.match_indices(TARGET_MARKER) {
            let rest = &output[offset + TARGET_MARKER.len()..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
                .collect();
            let path = self.root.join(id.trim_end_matches('.'));
            if !found.contains(&path) {
                found.push(path);
            }
        }
        found
    }

    /// The file a [`Consumer::SchemaFile`] cell's rendered `stored-value=`
    /// line names; see [`Self::stored_value_path`].
    pub fn stored_value_targets(&self, output: &str, tree_root: &Path) -> Vec<PathBuf> {
        output
            .lines()
            .filter_map(|line| line.trim().strip_prefix(STORED_VALUE_PREFIX))
            .map(|value| self.stored_value_path(value.trim(), tree_root))
            .collect()
    }

    /// The file an eager `file` value names once normalized: `~/`-relative
    /// under `HOME`, absolute, or relative to [`Self::tree_root`].
    pub fn stored_value_path(&self, value: &str, tree_root: &Path) -> PathBuf {
        if let Some(rest) = value.strip_prefix("~/") {
            self.home().join(rest)
        } else if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            tree_root.join(value)
        }
    }

    /// What a relative stored value of `cell`'s document is relative to: the
    /// repository root for a document in the repository, else the
    /// document's own folder.
    pub fn tree_root(&self, cell: &DocumentCell) -> PathBuf {
        match cell.placement {
            Placement::Repository(..) => self.repo(),
            Placement::OpenedThroughHome(_) | Placement::OpenedByAbsolutePath(_) => self.home().join("notes"),
        }
    }

    /// `observed` against `expected`, or a one-line description of the
    /// mismatch for the runner's report.
    pub fn compare(&self, expected: &Expected, observed: &Observed) -> Result<(), String> {
        match (expected, observed) {
            (Expected::File(want), Observed::File(got)) => {
                if identity(want) == identity(got) {
                    Ok(())
                } else {
                    Err(format!("expected {}, got {}", self.show(want), self.show(got)))
                }
            }
            (Expected::File(_), Observed::Accepted) => Ok(()),
            (Expected::Failure(want), Observed::Failure(got)) if want == got => Ok(()),
            (Expected::File(want), other) => Err(format!("expected {}, got {other:?}", self.show(want))),
            (Expected::Failure(want), Observed::File(got)) => {
                Err(format!("expected {want:?}, got {}", self.show(got)))
            }
            (Expected::Failure(want), other) => Err(format!("expected {want:?}, got {other:?}")),
        }
    }

    fn show(&self, path: &Path) -> String {
        let canonical = canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
        let root = canonicalize_simplified(&self.root).unwrap_or_else(|_| self.root.clone());
        match canonical.strip_prefix(&root) {
            Ok(relative) => format!("{{root}}/{}", to_portable_string(relative)),
            Err(_) => to_portable_string(&canonical),
        }
    }
}

/// One spelling per file: `/var` and `/private/var` (macOS) or a verbatim
/// prefix (Windows) name the same file.
pub fn identity(path: &Path) -> PathIdentity {
    PathIdentity::new(&canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// Folds a runner's per-cell results into one report naming every
/// mismatched cell, and checks the runner covered every entry point it owns.
pub struct ParityReport {
    owner: Owner,
    executed: Vec<EntryPoint>,
    cells: usize,
    mismatches: Vec<String>,
}

impl ParityReport {
    pub fn new(owner: Owner) -> Self {
        Self { owner, executed: Vec::new(), cells: 0, mismatches: Vec::new() }
    }

    pub fn record(&mut self, fixture: &ParityFixture, row: &Row, expected: &Expected, observed: &Observed) {
        self.cells += 1;
        if !self.executed.contains(&row.entry()) {
            self.executed.push(row.entry());
        }
        if let Err(mismatch) = fixture.compare(expected, observed) {
            self.mismatches.push(format!("{row:?}: {mismatch}"));
        }
    }

    /// Panics naming every mismatched cell, or an owned entry point that ran
    /// no cell.
    pub fn assert_parity(&self) {
        let missing: Vec<EntryPoint> = EntryPoint::ALL
            .into_iter()
            .filter(|entry| entry.owner() == self.owner && !self.executed.contains(entry))
            .collect();
        assert!(missing.is_empty(), "{:?} runner executed no cell for {missing:?}", self.owner);
        assert!(
            self.mismatches.is_empty(),
            "{} of {} cells disagree:\n{}",
            self.mismatches.len(),
            self.cells,
            self.mismatches.join("\n")
        );
    }
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A `::toc-linking` fallback chain in a document at [`Depth::One`]. Every
/// entry point that reads a directive target must apply the chain's grammar
/// and selection rule: the first existing alternative wins, a trailing
/// `false` intentionally renders nothing, and otherwise the first
/// alternative's class is the failure. `&missing.md` and `./missing-too.md`
/// exist nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainCase {
    /// `"&missing.md | &root-only.md"`: the fallback is selected.
    FallbackSucceeds,
    /// `"&root-only.md | ./sibling.md"`: both exist; the first wins.
    FirstWins,
    /// `"&missing.md | false"`: suppressed, never a broken target.
    Suppressed,
    /// `"&missing.md | ./missing-too.md"`: nothing exists.
    Unresolved,
}

impl ChainCase {
    pub const ALL: [ChainCase; 4] = [Self::FallbackSucceeds, Self::FirstWins, Self::Suppressed, Self::Unresolved];

    fn slug(self) -> &'static str {
        match self {
            Self::FallbackSucceeds => "fallback",
            Self::FirstWins => "first",
            Self::Suppressed => "suppressed",
            Self::Unresolved => "unresolved",
        }
    }

    /// The quoted directive target.
    pub fn reference(self) -> &'static str {
        match self {
            Self::FallbackSucceeds => "\"&missing.md | &root-only.md\"",
            Self::FirstWins => "\"&root-only.md | ./sibling.md\"",
            Self::Suppressed => "\"&missing.md | false\"",
            Self::Unresolved => "\"&missing.md | ./missing-too.md\"",
        }
    }
}

/// What a chain resolved to, expected or observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainOutcome {
    File(PathBuf),
    /// No file and no failure: the chain's `false` suppressed it.
    Suppressed,
    Failure(ResolutionFailure),
    /// Anything else, described for the report.
    Unexpected(String),
}

impl ParityFixture {
    /// Writes `case`'s `::toc-linking` document at [`Depth::One`].
    pub fn write_chain_document(&self, case: ChainCase) -> PathBuf {
        let path = self.depth_dir(Depth::One).join(format!("chain-{}.md", case.slug()));
        write(&path, &Consumer::TocLinking.document(case.reference()));
        path
    }

    pub fn expected_chain(&self, case: ChainCase) -> ChainOutcome {
        match case {
            ChainCase::FallbackSucceeds | ChainCase::FirstWins => ChainOutcome::File(self.repo().join("root-only.md")),
            ChainCase::Suppressed => ChainOutcome::Suppressed,
            ChainCase::Unresolved => ChainOutcome::Failure(ResolutionFailure::NoMatch),
        }
    }

    /// `observed` against `case`'s expectation, or a one-line mismatch.
    pub fn compare_chain(&self, case: ChainCase, observed: &ChainOutcome) -> Result<(), String> {
        compare_outcome(&format!("{case:?} `{}`", case.reference()), &self.expected_chain(case), observed)
    }

    /// Writes a document whose one `consumer` directive targets
    /// [`HASH_REFERENCE`], at [`Depth::One`].
    pub fn write_hash_document(&self, consumer: Consumer) -> PathBuf {
        let path = self.depth_dir(Depth::One).join(format!("hash-{}.md", consumer.slug()));
        write(&path, &consumer.document(HASH_REFERENCE));
        path
    }

    /// `observed` against [`HASH_REFERENCE`]'s expectation: `NoMatch`.
    pub fn compare_hash(&self, consumer: Consumer, observed: &ChainOutcome) -> Result<(), String> {
        let expected = ChainOutcome::Failure(ResolutionFailure::NoMatch);
        compare_outcome(&format!("{consumer:?} `{HASH_REFERENCE}`"), &expected, observed)
    }
}

/// The directives whose target is a file reference with no `#anchor`
/// syntax.
pub const HASH_CONSUMERS: [Consumer; 3] = [Consumer::File, Consumer::Code, Consumer::TocLinking];

/// A directive target whose `#x` is part of the filename, as composition
/// reads it: `root-only.md` exists at the repository root, but
/// `root-only.md#x` does not, so every entry point reports `NoMatch`.
pub const HASH_REFERENCE: &str = "\"&root-only.md#x\"";

fn compare_outcome(label: &str, expected: &ChainOutcome, observed: &ChainOutcome) -> Result<(), String> {
    let agrees = match (expected, observed) {
        (ChainOutcome::File(want), ChainOutcome::File(got)) => identity(want) == identity(got),
        (want, got) => want == got,
    };
    if agrees { Ok(()) } else { Err(format!("{label}: expected {expected:?}, got {observed:?}")) }
}
