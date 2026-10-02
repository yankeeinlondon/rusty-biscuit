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
//! `home/.claudine/prompts/` is the configured extra `@` root
//! ([`ParityFixture::configured_magic_root`]). Claudine registers it as its
//! user prompt root, `md` receives it as `--magic-root`, and the darkmatter and
//! dmls runners register it on their request snapshot, so
//! `@configured-doc.md` is reachable only through a configured root, at every
//! entry point.
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
    /// `md <route> <value>`: one `md` route's source-file argument (Table 2).
    /// Every route opens its argument through the same reader, and each runs
    /// as its own entry point so a route that bypasses it fails on its own.
    MdArgument(MdRoute),
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
    /// Claudine completion (`claudine __complete … compose <value>
    /// <property>=`), Table 2. Completion resolves a caller's reference in one
    /// place, the committed prompt argument whose `$schema` supplies the
    /// setter suggestions, so the value is that argument and the suggestions
    /// name the file it resolved to. A `file` property's own value
    /// suggestions are a directory walk, not a resolution. Completion reports
    /// no failure class: no suggestions is [`Observed::Unresolved`].
    ClaudineCompletion,
    /// `claudine compose --dry-run <value>`: the prompt argument completion
    /// resolves, composed (Table 2), so a completion cell and its execution
    /// are compared against the same expectation.
    ClaudinePromptArgument,
    /// `claudine compose --dry-run <document> target=<value>`: a
    /// caller-supplied schema `file` property value (Table 2).
    ClaudineSuppliedValue,
}

impl EntryPoint {
    pub const ALL: [EntryPoint; 32] = [
        Self::ComposePipeline,
        Self::Preflight,
        Self::SchemaValidation,
        Self::MdCompose,
        Self::MdSchemaValidate,
        Self::MdArgument(MdRoute::Render),
        Self::MdArgument(MdRoute::Compose),
        Self::MdArgument(MdRoute::Clean),
        Self::MdArgument(MdRoute::Toc),
        Self::MdArgument(MdRoute::FrontmatterGet),
        Self::MdArgument(MdRoute::FrontmatterSet),
        Self::MdArgument(MdRoute::FrontmatterRm),
        Self::MdArgument(MdRoute::Hash),
        Self::MdArgument(MdRoute::DeltaBase),
        Self::MdArgument(MdRoute::DeltaUpdated),
        Self::MdArgument(MdRoute::Graph),
        Self::MdArgument(MdRoute::Edit),
        Self::MdArgument(MdRoute::ValidateRefs),
        Self::MdArgument(MdRoute::SchemaValidate),
        Self::MdArgument(MdRoute::SchemaDetect),
        Self::MdArgument(MdRoute::SchemaTriggers),
        Self::MdArgument(MdRoute::CodeBlockFile),
        Self::MdArgument(MdRoute::CodeBlockDefault),
        Self::DmlsDiagnostics,
        Self::DmlsDocumentLinks,
        Self::DmlsLinkGraph,
        Self::DmlsDefinition,
        Self::DmlsCodeActions,
        Self::ClaudineComposition,
        Self::ClaudineCompletion,
        Self::ClaudinePromptArgument,
        Self::ClaudineSuppliedValue,
    ];

    pub fn owner(self) -> Owner {
        match self {
            Self::ComposePipeline | Self::Preflight | Self::SchemaValidation => Owner::Darkmatter,
            Self::MdCompose | Self::MdSchemaValidate | Self::MdArgument(_) => Owner::DarkmatterCli,
            Self::DmlsDiagnostics
            | Self::DmlsDocumentLinks
            | Self::DmlsLinkGraph
            | Self::DmlsDefinition
            | Self::DmlsCodeActions => Owner::Dmls,
            Self::ClaudineComposition
            | Self::ClaudineCompletion
            | Self::ClaudinePromptArgument
            | Self::ClaudineSuppliedValue => Owner::ClaudineCli,
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
            Self::MdCompose => document_rows(self, &[File, Code, TocLinking, SchemaFile], &Form::ALL, false),
            Self::MdSchemaValidate => document_rows(self, &[SchemaFile], &Form::ALL, false),
            // A quoted `'~/…'` argument is the only `~` spelling `md` sees.
            // Rows (a) and (b) test the opened document's own references, so
            // only composition, which reads them, runs them.
            Self::MdArgument(route) => {
                let mut rows = value_rows(self, &Form::ALL, route == MdRoute::Compose);
                for launch in Launch::ALL {
                    for value in Supplied::ARGUMENT_ONLY {
                        rows.push(Row::Value(ValueCell { entry: self, value, launch }));
                    }
                }
                rows
            }
            Self::DmlsDiagnostics => document_rows(self, &[SchemaFile], &Form::ALL, false),
            Self::DmlsDocumentLinks => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsLinkGraph => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsDefinition => document_rows(self, EDITOR, &Form::ALL, false),
            Self::DmlsCodeActions => document_rows(self, EDITOR, &Form::ALL, false),
            Self::ClaudineComposition => document_rows(self, &[File, Code, TocLinking, SchemaFile], &Form::ALL, false),
            // The value is the prompt itself, so the notes documents' own
            // references (rows (a) and (b)) are composition's, not completion's.
            Self::ClaudineCompletion => value_rows(self, &Form::ALL, false),
            Self::ClaudinePromptArgument => value_rows(self, &Form::ALL, true),
            Self::ClaudineSuppliedValue => value_rows(self, &Form::ALL, false),
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

/// An `md` route that opens a caller-named source file. Output
/// destinations, cache directories, and `--magic-root` directories are
/// settings, not source files, and have no route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MdRoute {
    /// `md render <value>`
    Render,
    /// `md compose <value>`
    Compose,
    /// `md clean <value>`
    Clean,
    /// `md toc --json <value>`
    Toc,
    /// `md get <value> title`
    FrontmatterGet,
    /// `md set <value> …` (printed, not saved)
    FrontmatterSet,
    /// `md rm <value> title --json`, which saves in place
    FrontmatterRm,
    /// `md hash <value>`
    Hash,
    /// `md delta <value> <control>`
    DeltaBase,
    /// `md delta <control> <value>`
    DeltaUpdated,
    /// `md graph --json <value>`
    Graph,
    /// `md edit <value>` with a no-op editor; a lookup that matches nothing
    /// names a new file at its first candidate.
    Edit,
    /// `md validate refs --graph mermaid <value>`
    ValidateRefs,
    /// `md schema validate <value>`
    SchemaValidate,
    /// `md schema detect <value>`
    SchemaDetect,
    /// `md schema triggers <value>`, which needs the document's repository.
    SchemaTriggers,
    /// `md code-block --file <value>`
    CodeBlockFile,
    /// `md code-block <value>`: a file only when the value is a reference to
    /// an existing file, otherwise literal code.
    CodeBlockDefault,
}

impl MdRoute {
    /// Whether the route writes to the fixture, so the runner restores the
    /// fixture before each of its cells and runs them one at a time.
    pub fn mutates(self) -> bool {
        matches!(self, Self::FrontmatterRm | Self::Edit)
    }
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
    /// The absolute path of `repo/root-only.md`.
    Absolute,
    /// A malformed reserved introducer whose literal file exists in the
    /// launch directory ([`ParityFixture::write_route_files`]); it must be
    /// refused, never opened.
    Malformed(Malformed),
    /// `./@`: the spelling that names the literal `@` file.
    LiteralReserved,
}

impl Supplied {
    /// The `md` argument rows beyond the shared forms.
    pub const ARGUMENT_ONLY: [Supplied; 6] = [
        Self::Absolute,
        Self::Malformed(Malformed::Magic),
        Self::Malformed(Malformed::RepositoryRoot),
        Self::Malformed(Malformed::RepositoryScoped),
        Self::Malformed(Malformed::Legacy),
        Self::LiteralReserved,
    ];
}

/// Text that is not valid reference syntax although a file of that name
/// exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Malformed {
    /// `@`
    Magic,
    /// `&`
    RepositoryRoot,
    /// `^`
    RepositoryScoped,
    /// `!legacy.md`, the removed `!` sigil.
    Legacy,
}

impl Malformed {
    pub const ALL: [Malformed; 4] = [Self::Magic, Self::RepositoryRoot, Self::RepositoryScoped, Self::Legacy];

    pub fn spelling(self) -> &'static str {
        match self {
            Self::Magic => "@",
            Self::RepositoryRoot => "&",
            Self::RepositoryScoped => "^",
            Self::Legacy => "!legacy.md",
        }
    }
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
    /// The value is rendered as literal text and no file is read
    /// ([`MdRoute::CodeBlockDefault`]).
    Literal,
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
    /// The reference resolved to nothing and the entry point names no class.
    /// Only completion reports this (it offers no suggestions); it satisfies
    /// an `Expected::Failure` cell of any class, and the class is compared at
    /// the same value's execution ([`EntryPoint::ClaudinePromptArgument`]).
    Unresolved,
    /// The value was rendered as literal text, naming no file.
    Literal,
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

        for id in target_ids() {
            write(&root.join(&id), &format!("## {TARGET_MARKER}{id}\n"));
        }
        fixture
    }

    /// Every target file, in a fixed order.
    pub fn targets(&self) -> Vec<PathBuf> {
        target_ids().into_iter().map(|id| self.root.join(id)).collect()
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
            Supplied::Absolute => self.repo().join("root-only.md").to_string_lossy().into_owned(),
            Supplied::Malformed(malformed) => malformed.spelling().into(),
            Supplied::LiteralReserved => "./@".into(),
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
        let expected = match cell.value {
            Supplied::Form(form) => {
                let dir = self.launch_dir(cell.launch);
                let package = cell.launch == Launch::Package;
                self.expected_form(form, &dir, package)
            }
            Supplied::ThroughHome(reference) => self.expected_notes(reference, true),
            Supplied::AbsolutePath(reference) => self.expected_notes(reference, false),
            Supplied::Absolute => Expected::File(self.repo().join("root-only.md")),
            Supplied::Malformed(_) => Expected::Failure(ResolutionFailure::InvalidReference),
            Supplied::LiteralReserved => Expected::File(self.launch_dir(cell.launch).join("@")),
        };
        match cell.entry {
            EntryPoint::MdArgument(route) => self.route_expectation(route, cell, expected),
            _ => expected,
        }
    }

    /// How one `md` route reports the shared expectation.
    fn route_expectation(&self, route: MdRoute, cell: &ValueCell, expected: Expected) -> Expected {
        match (route, expected) {
            // Trigger inspection needs the document's repository; a file
            // outside every repository is `MissingContext` once opened.
            (MdRoute::SchemaTriggers, Expected::File(path)) if !path.starts_with(self.repo()) => {
                Expected::Failure(ResolutionFailure::MissingContext)
            }
            // A miss names a new file at the first candidate. Every miss
            // here is from the repository root, where that is the root
            // joined with the payload (`./sibling.md`, `beside.md`,
            // `^area-doc.md`).
            (MdRoute::Edit, Expected::Failure(ResolutionFailure::NoMatch)) => {
                let spelling = self.value(cell);
                let payload = spelling.trim_start_matches(['^', '&', '@']).trim_start_matches("./");
                Expected::File(self.launch_dir(cell.launch).join(payload))
            }
            // Only an existing file is read; anything else is code.
            (MdRoute::CodeBlockDefault, Expected::Failure(_)) => Expected::Literal,
            (_, expected) => expected,
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

    /// Prepares the fixture for the `md` route rows, and restores it after a
    /// mutating route: every target, plus a literal `@`, `&`, `^`, and
    /// `!legacy.md` file in each launch directory, holds
    /// [`route_document`] for its id, so every route can name the file it
    /// opened: `title` (`get`, `toc`), a `target-<slug>` property (schema
    /// detection), and the `## TARGET <id>` heading.
    pub fn write_route_files(&self) {
        for path in self.route_files() {
            let id = to_portable_string(path.strip_prefix(&self.root).unwrap());
            write(&path, &route_document(&id));
        }
    }

    /// Every file a route row can open: the targets and the literal
    /// reserved-name files of [`Self::write_route_files`].
    pub fn route_files(&self) -> Vec<PathBuf> {
        let mut files = self.targets();
        for launch in Launch::ALL {
            for name in Malformed::ALL.map(Malformed::spelling) {
                files.push(self.launch_dir(launch).join(name));
            }
        }
        files
    }

    /// The route file whose [`route_slug`] property `text` names.
    pub fn route_file_by_slug(&self, text: &str) -> Vec<PathBuf> {
        self.route_files()
            .into_iter()
            .filter(|path| {
                let id = to_portable_string(path.strip_prefix(&self.root).unwrap());
                text.contains(&format!("{}:", route_slug(&id))) || text.contains(&format!("\"{}\"", route_slug(&id)))
            })
            .collect()
    }

    /// Every fixture target named in rendered output, in order of first
    /// appearance.
    pub fn marked_targets(&self, output: &str) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::new();
        for (offset, _) in output.match_indices(TARGET_MARKER) {
            let rest = &output[offset + TARGET_MARKER.len()..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '@' | '&' | '^' | '!'))
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
            (Expected::Literal, Observed::Literal) => Ok(()),
            (Expected::Literal, other) => Err(format!("expected literal text, got {other:?}")),
            (Expected::Failure(want), Observed::Failure(got)) if want == got => Ok(()),
            (Expected::Failure(_), Observed::Unresolved) => Ok(()),
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
/// prefix (Windows) name the same file. A file that no longer exists (one
/// `md edit` created and the runner removed) is spelled through its folder.
pub fn identity(path: &Path) -> PathIdentity {
    let canonical = canonicalize_simplified(path).unwrap_or_else(|_| {
        match (path.parent().map(canonicalize_simplified), path.file_name()) {
            (Some(Ok(parent)), Some(name)) => parent.join(name),
            _ => path.to_path_buf(),
        }
    });
    PathIdentity::new(&canonical)
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

/// A route file's content: a `title` and a [`route_slug`] property naming
/// `id`, then the target heading.
pub fn route_document(id: &str) -> String {
    format!("---\ntitle: {id}\n{}: true\n---\n\n## {TARGET_MARKER}{id}\n", route_slug(id))
}

/// `target-` and `id` with every character but ASCII letters and digits
/// spelled as its code point, so no two ids share a slug and the slug is a
/// plain property name.
pub fn route_slug(id: &str) -> String {
    let mut slug = String::from("target-");
    for c in id.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else {
            slug.push_str(&format!("-{:x}-", c as u32));
        }
    }
    slug
}

/// Every target's id: its `/`-spelled path below the fixture root.
fn target_ids() -> Vec<String> {
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
    targets
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

/// Two repositories under one root, for a source outside the launch
/// repository: every runner launches in `launch/` and opens a document or
/// prompt in `source/`.
///
/// ```text
/// {root}/
///   launch/                   git repository; the launch directory
///     magic.md                ## LAUNCH_MAGIC
///     order.yaml              LAUNCH_SCHEMA, found by `@order.yaml`
///     schemas/order.yaml      LAUNCH_SCHEMA, a wrong `&`/`^`/bare answer
///   source/                   a second git repository
///     magic.md                ## SOURCE_MAGIC, a wrong `@` answer
///     order.yaml              SOURCE_SCHEMA
///     schemas/order.yaml      SOURCE_SCHEMA
///     prompts/                the documents and prompts the runners open
///       order.yaml            SOURCE_SCHEMA, the `./` control
/// ```
///
/// The source's own repository anchors `&`, `^`, and bare root lookups; `@`
/// keeps the launch `@` scope. The anchors are independent, so each wrong
/// answer exists on disk and names the other repository. Both schemas declare
/// `zebra` before `apple`, and their `zebra` enums are disjoint.
pub struct CrossRepositoryFixture {
    root: PathBuf,
}

pub const LAUNCH_MAGIC: &str = "LAUNCH_MAGIC";
pub const SOURCE_MAGIC: &str = "SOURCE_MAGIC";
pub const LAUNCH_SCHEMA: &str = "$schema:\n  zebra: enum(launch, other)\n  apple: enum(one, two)\n";
pub const SOURCE_SCHEMA: &str = "$schema:\n  zebra: enum(red, blue)\n  apple: enum(one, two)\n";

/// The repository whose schema a `$schema` reference must find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaOwner {
    Launch,
    Source,
}

impl SchemaOwner {
    /// The `zebra` enum members the owner's schema declares.
    pub fn zebra_values(self) -> [&'static str; 2] {
        match self {
            Self::Launch => ["launch", "other"],
            Self::Source => ["red", "blue"],
        }
    }
}

impl CrossRepositoryFixture {
    pub fn create(root: &Path) -> Self {
        let fixture = Self { root: root.to_path_buf() };
        for repo in [fixture.launch(), fixture.source()] {
            std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
            std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
            write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
            write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        }
        let (launch, source) = (fixture.launch(), fixture.source());
        write(&launch.join("magic.md"), &format!("## {LAUNCH_MAGIC}\n"));
        write(&source.join("magic.md"), &format!("## {SOURCE_MAGIC}\n"));
        for path in [launch.join("order.yaml"), launch.join("schemas/order.yaml")] {
            write(&path, LAUNCH_SCHEMA);
        }
        for path in [source.join("order.yaml"), source.join("schemas/order.yaml"), source.join("prompts/order.yaml")] {
            write(&path, SOURCE_SCHEMA);
        }
        fixture
    }

    pub fn launch(&self) -> PathBuf {
        self.root.join("launch")
    }

    pub fn source(&self) -> PathBuf {
        self.root.join("source")
    }

    /// Every `$schema` spelling a source prompt can use, labelled, with the
    /// repository whose schema it must find.
    pub fn schema_references(&self) -> Vec<(&'static str, String, SchemaOwner)> {
        vec![
            ("bare", "schemas/order.yaml".into(), SchemaOwner::Source),
            ("repository root", "&schemas/order.yaml".into(), SchemaOwner::Source),
            ("repository scoped", "^schemas/order.yaml".into(), SchemaOwner::Source),
            ("explicit relative", "./order.yaml".into(), SchemaOwner::Source),
            (
                "absolute",
                to_portable_string(&self.source().join("schemas/order.yaml")),
                SchemaOwner::Source,
            ),
            ("magic", "@order.yaml".into(), SchemaOwner::Launch),
        ]
    }

    /// `source/prompts/<name>.md`, whose only frontmatter is
    /// `$schema: '<reference>'` plus `extra` lines.
    pub fn write_schema_document(&self, name: &str, reference: &str, extra: &str) -> PathBuf {
        let path = self.source().join(format!("prompts/{name}.md"));
        write(&path, &format!("---\n$schema: '{reference}'\n{extra}---\n\n# Schema\n"));
        path
    }

    /// `source/prompts/magic-doc.md`, whose one reference is
    /// `::file @magic.md`.
    pub fn write_magic_document(&self) -> PathBuf {
        let path = self.source().join("prompts/magic-doc.md");
        write(&path, &Consumer::File.document("@magic.md"));
        path
    }

    /// `source/prompts/enum-doc.md`: `$schema: '@order.yaml'` with
    /// `zebra: launch`, valid only against the launch schema.
    pub fn write_launch_enum_document(&self) -> PathBuf {
        self.write_schema_document("enum-doc", "@order.yaml", "zebra: launch\n")
    }
}
