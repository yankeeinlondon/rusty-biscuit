//! Value hovers: a `$schema` string explains the file it selects, and a
//! `file`-typed value says where it resolves; keys keep their schema hover.
//!
//! Each case is driven in-process through the provider registry, as an
//! editor's hover request is, over a fresh overlay cache. The fixture is one
//! canonical temporary root:
//!
//! | path | contents |
//! |---|---|
//! | `repo/` | Git repository; its `schemas/` folder is the tree root |
//! | `bare/` | a second repository with no `schemas/` folder anywhere |
//! | `loose/` | a folder in no repository |
//! | `home/` | an empty home, so `~/schemas` is absent until a test makes it |
//!
//! Snapshots are built here; the process environment, home, and current
//! directory are never read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::canonicalize_simplified;
use darkmatter::markdown::compose::RequestSnapshot;
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::{ContextFailure, DocumentResolution, RepositoryContexts};
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use dmls::work_counters::work_counts;
use lsp_types::{Hover, HoverContents, InitializeParams, Position, Range, Uri};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).expect("objects");
    std::fs::create_dir_all(root.join(".git/refs/heads")).expect("refs");
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

/// `path` as the `file` URI an editor sends.
fn uri_of(path: &Path) -> Uri {
    let url = url::Url::from_file_path(path).expect("file URL");
    url.as_str().replace('[', "%5B").replace(']', "%5D").parse().expect("URI")
}

/// The link destination the hover writes for `path`: its file URI with `(`
/// and `)` encoded.
fn link_uri(path: &Path) -> String {
    dmls::workspace::file_path_to_uri(path).expect("file URI").as_str().replace('(', "%28").replace(')', "%29")
}

fn markdown_of(hover: &Hover) -> &str {
    match &hover.contents {
        HoverContents::Markup(markup) => &markup.value,
        other => panic!("a Markdown hover expected: {other:?}"),
    }
}

/// The byte offset `nudge` bytes into the first `needle` of `text`.
fn at(text: &str, needle: &str, nudge: usize) -> usize {
    text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in the document")) + nudge
}

struct Hovers {
    _dir: tempfile::TempDir,
    root: PathBuf,
    contexts: RepositoryContexts,
    config: DmlsConfig,
    registry: ProviderRegistry,
    graph: WorkspaceGraph,
}

impl Hovers {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        init_repository(&root.join("repo"));
        std::fs::create_dir_all(root.join("repo/schemas")).expect("tree root");
        init_repository(&root.join("bare"));
        std::fs::create_dir_all(root.join("loose")).expect("loose folder");
        std::fs::create_dir_all(root.join("home")).expect("home");
        let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home")));
        Self {
            _dir: dir,
            contexts: RepositoryContexts::new(snapshot),
            config: DmlsConfig::default(),
            registry: ProviderRegistry::with_substrate(),
            graph: WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts),
            root,
        }
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn schemas(&self) -> PathBuf {
        self.repo().join("schemas")
    }

    /// `repo/docs/{name}`, the usual document.
    fn doc(&self, name: &str) -> PathBuf {
        self.repo().join("docs").join(name)
    }

    /// Writes `text` to `document` and runs `f` over a fresh overlay of it,
    /// resolved through the document's own context unless `resolution`
    /// replaces it.
    fn with_document<T>(
        &self,
        document: &Path,
        text: &str,
        encoding: PositionEncoding,
        resolution: Option<DocumentResolution>,
        f: impl FnOnce(&DocumentContext, &ProviderRegistry) -> T,
    ) -> T {
        write(document, text);
        let resolution = resolution.unwrap_or_else(|| self.contexts.for_document(document));
        let uri = uri_of(document);
        let workspace = [self.repo()];
        let overlays = OverlayState::default();
        let overlay = overlays.for_document(&uri, text, document, &self.config, &workspace, &resolution);
        assert!(overlay.is_some(), "the document has a frontmatter overlay");
        let source_map = SourceMap::new(uri.clone(), 1, encoding, Arc::from(text));
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), encoding);
        let ctx = DocumentContext {
            uri: &uri,
            path: document,
            text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: None,
            config: &self.config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        f(&ctx, &self.registry)
    }

    fn hover(&self, document: &Path, text: &str, offset: usize) -> Option<Hover> {
        self.with_document(document, text, PositionEncoding::Utf16, None, |ctx, registry| registry.hover(ctx, offset))
    }

    /// The Markdown of the hover `nudge` bytes into the first `needle`.
    fn markdown(&self, document: &Path, text: &str, needle: &str, nudge: usize) -> String {
        let hover = self.hover(document, text, at(text, needle, nudge)).unwrap_or_else(|| panic!("a hover at `{needle}`"));
        markdown_of(&hover).to_string()
    }

    /// The hover of a document whose `$schema` names `reference`, on the
    /// value.
    fn schema_value(&self, document: &Path, reference: &str) -> String {
        let text = format!("---\n$schema: {reference}\ntitle: Review\n---\n\nBody\n");
        self.markdown(document, &text, reference, 1)
    }
}

/// A YAML schema whose `$schema` mapping has `count` properties.
fn properties(count: usize) -> String {
    let mut text = String::from("$schema:\n");
    for index in 0..count {
        text.push_str(&format!("  p{index}: string\n"));
    }
    text
}

fn property_lines(count: usize) -> String {
    (0..count).map(|index| format!("p{index}: string")).collect::<Vec<_>>().join("\n")
}

// ── `$schema` values ────────────────────────────────────────────────────────

/// The `review-3.md` acceptance case: a bare name selecting the tree root's
/// `feature-review.yaml`, the repository's own shipped schema, whose header
/// comment is not a description and whose definition is longer than ten
/// lines.
#[test]
fn a_review_document_previews_the_first_ten_lines_of_the_shipped_feature_review_schema() {
    let hovers = Hovers::new();
    let shipped = include_str!("../../../../schemas/feature-review.yaml").replace("\r\n", "\n");
    let target = hovers.schemas().join("feature-review.yaml");
    write(&target, &shipped);
    let markdown = hovers.schema_value(&hovers.doc("review-3.md"), "feature-review.yaml");

    let definition: Vec<&str> = shipped
        .split_once("$schema:\n")
        .expect("the shipped schema exports `$schema`")
        .1
        .trim_end_matches('\n')
        .lines()
        .collect();
    let preview: Vec<&str> = definition.iter().take(10).map(|line| line.strip_prefix("    ").unwrap_or(line)).collect();
    assert!(definition.len() > 10, "the shipped definition is long enough to truncate");
    assert!(preview[0].starts_with("- description: string"), "{preview:?}");
    assert_eq!(
        markdown,
        format!(
            "Schema Definition:\n\n```yaml\n{}\n```\n\n*Showing **10** of **{}** lines*\n\nResolves to [schemas/feature-review.yaml]({})",
            preview.join("\n"),
            definition.len(),
            link_uri(&target)
        )
    );
}

#[test]
fn the_schema_key_hover_is_unchanged() {
    let hovers = Hovers::new();
    write(&hovers.schemas().join("shape.yaml"), &properties(2));
    let text = "---\n$schema: shape.yaml\ntitle: Review\n---\n";
    let key = hovers.markdown(&hovers.doc("key.md"), text, "$schema", 1);
    let missing = hovers.markdown(&hovers.doc("key-miss.md"), &text.replace("shape.yaml", "gone.yaml"), "$schema", 1);
    assert_eq!(key, missing, "the key hover does not depend on the target");
    assert!(!key.contains("Schema Definition") && !key.contains("Resolves to"), "{key}");
    let range = hovers.hover(&hovers.doc("key.md"), text, 5).unwrap().range.unwrap();
    assert_eq!(range, Range::new(Position::new(1, 0), Position::new(1, 7)), "ranged on the key");
}

#[test]
fn short_ten_line_and_eleven_line_definitions() {
    let hovers = Hovers::new();
    for (count, truncated) in [(2, false), (10, false), (11, true)] {
        let target = hovers.schemas().join(format!("p{count}.yaml"));
        write(&target, &properties(count));
        let markdown = hovers.schema_value(&hovers.doc(&format!("p{count}.md")), &format!("p{count}.yaml"));
        let shown = property_lines(count.min(10));
        let truncation = if truncated { format!("\n\n*Showing **10** of **{count}** lines*") } else { String::new() };
        assert_eq!(
            markdown,
            format!(
                "Schema Definition:\n\n```yaml\n{shown}\n```{truncation}\n\nResolves to [schemas/p{count}.yaml]({})",
                link_uri(&target)
            ),
            "{count} lines"
        );
    }
}

#[test]
fn a_nonblank_description_leads_and_comments_never_do() {
    let hovers = Hovers::new();
    let cases = [
        ("tagged", "kind: schema\ndescription: Tagged *reviews*.\n$schema:\n  a: string\n", Some("Tagged *reviews*.")),
        ("informal", "description: |\n  Informal reviews.\n$schema:\n  a: string\n", Some("Informal reviews.")),
        ("blank", "description: \"  \"\n$schema:\n  a: string\n", None),
        ("absent", "$schema:\n  a: string\n", None),
        ("comment", "# Only a comment.\n$schema:\n  a: string\n", None),
    ];
    for (name, schema, description) in cases {
        let target = hovers.schemas().join(format!("{name}.yaml"));
        write(&target, schema);
        let markdown = hovers.schema_value(&hovers.doc(&format!("{name}.md")), &format!("{name}.yaml"));
        let lead = description.map(|description| format!("{description}\n\n")).unwrap_or_default();
        assert_eq!(
            markdown,
            format!("{lead}Schema Definition:\n\n```yaml\na: string\n```\n\nResolves to [schemas/{name}.yaml]({})", link_uri(&target)),
            "{name}"
        );
    }
}

/// An informal schema may declare `categories`, as a tagged one may.
#[test]
fn informal_and_tagged_targets_with_categories_preview_their_definition() {
    let hovers = Hovers::new();
    let informal = "categories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n";
    for (name, schema) in [("informal", informal.to_string()), ("tagged", format!("kind: schema\n{informal}"))] {
        let target = hovers.schemas().join(format!("{name}.yaml"));
        write(&target, &schema);
        let markdown = hovers.schema_value(&hovers.doc(&format!("{name}.md")), &format!("{name}.yaml"));
        assert_eq!(
            markdown,
            format!("Schema Definition:\n\n```yaml\ntitle: string\n```\n\nResolves to [schemas/{name}.yaml]({})", link_uri(&target)),
            "{name}"
        );
    }
}

#[test]
fn a_json_target_previews_its_whole_document_in_a_json_fence() {
    let hovers = Hovers::new();
    let target = hovers.schemas().join("shape.json");
    let json = "{\r\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\r\n  \"description\": \"JSON shape.\",\r\n  \"type\": \"object\"\r\n}\r\n";
    write(&target, json);
    let markdown = hovers.schema_value(&hovers.doc("json.md"), "shape.json");
    assert_eq!(
        markdown,
        format!(
            "JSON shape.\n\nSchema Definition:\n\n```json\n{}\n```\n\nResolves to [schemas/shape.json]({})",
            json.replace("\r\n", "\n").trim_end(),
            link_uri(&target)
        )
    );
}

#[test]
fn a_delegating_target_previews_its_reference_and_links_the_named_file() {
    let hovers = Hovers::new();
    write(&hovers.schemas().join("real.yaml"), &properties(1));
    let target = hovers.schemas().join("alias.yaml");
    write(&target, "$schema: ./real.yaml\n");
    let markdown = hovers.schema_value(&hovers.doc("alias.md"), "alias.yaml");
    assert_eq!(
        markdown,
        format!("Schema Definition:\n\n```yaml\n./real.yaml\n```\n\nResolves to [schemas/alias.yaml]({})", link_uri(&target))
    );
}

#[test]
fn rejected_formats_say_the_file_is_not_a_schema_definition() {
    let hovers = Hovers::new();
    for (name, content) in [
        ("types-only.yaml", "kind: schema\ntypes:\n  Name: string\n"),
        ("no-schema.yaml", "title: string\n"),
        ("no-dialect.json", "{ \"type\": \"object\" }\n"),
        ("array.json", "[]\n"),
    ] {
        let target = hovers.schemas().join(name);
        write(&target, content);
        let markdown = hovers.schema_value(&hovers.doc(&format!("{name}.md")), name);
        assert_eq!(
            markdown,
            format!("The file [`schemas/{name}`]({}) exists but it is **not** a valid schema definition!", link_uri(&target)),
            "{name}"
        );
    }
}

#[test]
fn another_json_dialect_is_named() {
    let hovers = Hovers::new();
    let target = hovers.schemas().join("draft7.json");
    write(&target, "{ \"$schema\": \"http://json-schema.org/draft-07/schema#\", \"type\": \"object\" }\n");
    let markdown = hovers.schema_value(&hovers.doc("draft7.md"), "draft7.json");
    assert_eq!(
        markdown,
        format!(
            "The file [`schemas/draft7.json`]({}) declares the unsupported JSON Schema dialect `http://json-schema.org/draft-07/schema#`. Only `https://json-schema.org/draft/2020-12/schema` is supported.",
            link_uri(&target)
        )
    );
}

/// A malformed file, a file whose import is missing, and (on Unix) an
/// unreadable file each keep their own reason and the path that was
/// selected; none is described as missing.
#[test]
fn files_that_cannot_be_used_keep_their_reason_and_path() {
    let hovers = Hovers::new();
    let lead = |name: &str| {
        format!("The file [`schemas/{name}`]({}) could not be used as a schema: ", link_uri(&hovers.schemas().join(name)))
    };

    write(&hovers.schemas().join("malformed.yaml"), "$schema:\n  a: string\n  a: number\n");
    let malformed = hovers.schema_value(&hovers.doc("malformed.md"), "malformed.yaml");
    assert!(malformed.starts_with(&lead("malformed.yaml")), "{malformed}");
    assert!(malformed.contains("duplicate"), "{malformed}");

    write(&hovers.schemas().join("imports.yaml"), "$schema:\n  a: Gone@./gone.yaml\n");
    let imports = hovers.schema_value(&hovers.doc("imports.md"), "imports.yaml");
    assert!(imports.starts_with(&lead("imports.yaml")), "{imports}");
    assert!(imports.contains("gone.yaml") && !imports.contains("Schema Definition"), "{imports}");
    assert_ne!(malformed[lead("x").len()..], imports[lead("x").len()..], "distinct reasons");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Beside the document, not in a schema root: an unreadable file in a
        // root fails the root scan itself (`unreadable_schema_roots`).
        let locked = hovers.doc("locked.yaml");
        write(&locked, &properties(1));
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        // A process that can read it anyway (root) has nothing to show here.
        if std::fs::read(&locked).is_err() {
            let unreadable = hovers.schema_value(&hovers.doc("locked.md"), "./locked.yaml");
            let lead = format!("The file [`docs/locked.yaml`]({}) could not be used as a schema: ", link_uri(&locked));
            assert!(unreadable.starts_with(&lead), "{unreadable}");
            assert!(!unreadable.contains("does not") && !unreadable.contains("Missing:"), "{unreadable}");
        }
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        let in_root = hovers.schemas().join("locked-root.yaml");
        write(&in_root, &properties(1));
        std::fs::set_permissions(&in_root, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&in_root).is_err() {
            let roots = hovers.schema_value(&hovers.doc("locked-root.md"), "locked-root.yaml");
            assert!(
                roots.starts_with("`locked-root.yaml` could not be resolved because the schema roots could not be read: "),
                "{roots}"
            );
            assert!(!roots.contains("file://"), "no root is guessed: {roots}");
        }
        std::fs::set_permissions(&in_root, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

/// A root union whose second item is missing: the first item's file is
/// still a valid schema, and the second explains its miss.
#[test]
fn root_union_items_hover_independently() {
    let hovers = Hovers::new();
    let shape = hovers.schemas().join("shape.yaml");
    write(&shape, &properties(1));
    let text = "---\n$schema: [shape.yaml, gone.yaml, {status: string}]\ntitle: Review\n---\n";
    let document = hovers.doc("union.md");
    let first = hovers.markdown(&document, text, "shape.yaml", 1);
    assert!(first.starts_with("Schema Definition:") && first.ends_with(&format!("({})", link_uri(&shape))), "{first}");
    let second = hovers.markdown(&document, text, "gone.yaml", 1);
    assert!(second.starts_with("`gone.yaml` does not match a schema file."), "{second}");
    let arm = hovers.markdown(&document, text, "string}", 1);
    assert!(!arm.contains("Schema Definition") && !arm.contains("schema file"), "the inline arm keeps its hover: {arm}");

    let inline = "---\n$schema:\n  title: string\ntitle: Review\n---\n";
    let authored = hovers.markdown(&hovers.doc("inline.md"), inline, "string", 1);
    assert!(!authored.contains("Schema Definition") && !authored.contains("file-resolution"), "{authored}");
}

#[test]
fn a_clean_miss_lists_each_searched_root_and_the_creation_path() {
    let hovers = Hovers::new();
    let one = hovers.schema_value(&hovers.doc("one.md"), "gone.yaml");
    assert_eq!(
        one,
        "`gone.yaml` does not match a schema file.\n\nSearched, in order:\n\n- `schemas/gone.yaml`\n\nMissing: `schemas/gone.yaml` does not exist yet; following the link opens this path."
    );

    std::fs::create_dir_all(hovers.root.join("home/schemas")).unwrap();
    let home = format!("{}/home/schemas/gone.yaml", hovers.root.to_string_lossy().replace('\\', "/"));
    let two = hovers.schema_value(&hovers.doc("two.md"), "gone.yaml");
    assert_eq!(
        two,
        format!(
            "`gone.yaml` does not match a schema file.\n\nSearched, in order:\n\n- `schemas/gone.yaml`\n- `{home}`\n\nMissing: `schemas/gone.yaml` does not exist yet; following the link opens this path."
        )
    );
    assert!(!hovers.schemas().join("gone.yaml").exists(), "hover creates nothing");
}

#[test]
fn a_miss_without_schema_roots_explains_the_ordinary_lookup() {
    let hovers = Hovers::new();
    let document = hovers.root.join("bare/docs/zero.md");
    let markdown = hovers.schema_value(&document, "gone.yaml");
    assert!(
        markdown.starts_with(
            "`gone.yaml` was looked up like any other file reference (no schema root folder exists).\n\nSearched, in order:\n\n- `docs/gone.yaml`"
        ),
        "{markdown}"
    );
    assert!(
        markdown.ends_with("\n\nMissing: `docs/gone.yaml` does not exist yet; following the link opens this path."),
        "{markdown}"
    );
}

#[test]
fn a_sibling_only_match_suggests_the_explicit_form_without_linking_it() {
    let hovers = Hovers::new();
    write(&hovers.doc("beside.yaml"), &properties(1));
    let markdown = hovers.schema_value(&hovers.doc("sibling.md"), "beside.yaml");
    assert_eq!(
        markdown,
        "`beside.yaml` is not in any schema root, but a file with that name is beside this document. Write `./beside.yaml` to reference it.\n\nSearched, in order:\n\n- `schemas/beside.yaml`\n\nMissing: `schemas/beside.yaml` does not exist yet; following the link opens this path."
    );
}

#[test]
fn remote_and_contextless_references_resolve_nothing() {
    let hovers = Hovers::new();
    let remote = hovers.schema_value(&hovers.doc("remote.md"), "https://example.com/s.yaml");
    assert_eq!(remote, "`https://example.com/s.yaml` is a remote reference. Remote schemas are not fetched.");

    let text = "---\n$schema: shape.yaml\nsource: ./x.md\n---\n";
    let no_context = Some(DocumentResolution::from_failure(ContextFailure::NotProvided));
    let hover = hovers.with_document(&hovers.doc("none.md"), text, PositionEncoding::Utf16, no_context, |ctx, registry| {
        registry.hover(ctx, at(text, "shape.yaml", 1))
    });
    assert_eq!(
        markdown_of(&hover.expect("a hover")),
        "This document has no file-resolution context, so `shape.yaml` is not resolved."
    );
}

/// Two hovers over one unchanged snapshot read nothing new: the preview was
/// made with the schema, and no request composes or touches the network.
#[test]
fn hovering_reuses_the_prepared_preview_and_never_composes() {
    let hovers = Hovers::new();
    write(&hovers.schemas().join("shape.yaml"), &properties(3));
    let text = "---\n$schema: shape.yaml\ntitle: Review\n---\n";
    let document = hovers.doc("counted.md");
    let offset = at(text, "shape.yaml", 1);
    #[cfg(feature = "effects-instrumentation")]
    let effects = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
    let before = work_counts();
    hovers.with_document(&document, text, PositionEncoding::Utf16, None, |ctx, registry| {
        let prepared = work_counts().since(before);
        assert_eq!(prepared.schema_target_inspections, 1, "the overlay inspected the one target: {prepared:?}");
        let hovered = work_counts();
        let first = registry.hover(ctx, offset).expect("a hover");
        let second = registry.hover(ctx, offset).expect("a hover");
        assert_eq!(first, second);
        let work = work_counts().since(hovered);
        assert_eq!(work.schema_target_inspections, 0, "{work:?}");
        assert_eq!(work.reads, 0, "a `$schema` value hover reads no file: {work:?}");
    });
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
        effects,
        "nothing composed or fetched"
    );
}

// ── `file` values ───────────────────────────────────────────────────────────

const FILE_SCHEMA: &str = "$schema:\n  eager: file(eager)\n  lazy: file\n  nested: \"{ main: file(eager) }\"\n  items: file(eager)[]\n  either: \"file(eager) | number\"\n  entry: \"{ kind: literal(doc), path: file(eager) } | { kind: literal(text), path: string }\"\n";

fn file_document(values: &str) -> String {
    format!("---\n{FILE_SCHEMA}{values}---\n\nBody\n")
}

const LAZY_MISSING: &str = "This file reference is a valid file reference but the file does not exist yet in the filesystem. Because `{p}` is not defined as being **eager** then the file reference is not required to exist even at run time.";
const EAGER_MISSING: &str = "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `{p}` is defined as being an **eager** file reference so that means the file reference must refer to a real file at runtime.";
const ARM_MISSING: &str = "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `{p}` has an **eager** `file` arm, so when this value is read as that file reference it must refer to a real file at runtime.";

#[test]
fn file_values_report_each_outcome() {
    let hovers = Hovers::new();
    let present = hovers.doc("present.md");
    write(&present, "x");
    let text = file_document("eager: ./present.md\nlazy: ./nope.md\nlazy2: x\n");
    let document = hovers.doc("files.md");
    assert_eq!(
        hovers.markdown(&document, &text, "./present.md", 2),
        format!("Resolves to [`docs/present.md`]({})", link_uri(&present))
    );
    assert_eq!(hovers.markdown(&document, &text, "./nope.md", 2), LAZY_MISSING.replace("{p}", "lazy"));

    let eager = file_document("eager: ./gone.md\n");
    assert_eq!(hovers.markdown(&document, &eager, "./gone.md", 2), EAGER_MISSING.replace("{p}", "eager"));

    let invalid = file_document("lazy: \"@\"\n");
    assert_eq!(
        hovers.markdown(&document, &invalid, "\"@\"", 1),
        // The reason is escaped, so its backticks read literally.
        "`@` is not a valid file reference: file reference syntax is invalid: the \\`@\\` sigil requires a relative payload: \\`@\\`"
    );
}

#[test]
fn nested_item_and_union_values_name_their_property() {
    let hovers = Hovers::new();
    let document = hovers.doc("nested.md");
    let text = file_document(
        "nested:\n  main: ./n.md\nitems:\n  - ./i.md\neither: ./u.md\nentry:\n  kind: doc\n  path: ./doc.md\n",
    );
    assert_eq!(hovers.markdown(&document, &text, "./n.md", 2), EAGER_MISSING.replace("{p}", "nested.main"));
    assert_eq!(hovers.markdown(&document, &text, "./i.md", 2), EAGER_MISSING.replace("{p}", "items"));
    assert_eq!(hovers.markdown(&document, &text, "./u.md", 2), ARM_MISSING.replace("{p}", "either"));
    // The `kind: doc` arm is selected, so `path` is only a file.
    assert_eq!(hovers.markdown(&document, &text, "./doc.md", 2), EAGER_MISSING.replace("{p}", "entry.path"));

    // Without a discriminant both arms remain: the file arm is eager.
    let ambiguous = file_document("entry:\n  path: ./doc.md\n");
    assert_eq!(hovers.markdown(&document, &ambiguous, "./doc.md", 2), ARM_MISSING.replace("{p}", "entry.path"));
    // The `kind: text` arm makes `path` a string: no file hover.
    let text_arm = file_document("entry:\n  kind: text\n  path: ./doc.md\n");
    let hover = hovers.markdown(&document, &text_arm, "./doc.md", 2);
    assert!(!hover.contains("file reference") && !hover.contains("Resolves to"), "{hover}");
}

#[test]
fn computed_values_keep_their_hover_and_environment_references_resolve() {
    let hovers = Hovers::new();
    let document = hovers.doc("computed.md");
    let key = hovers.markdown(&document, &file_document("lazy: ./x.md\n"), "lazy: ./x", 1);
    for value in ["\"{{ title }}.md\"", "\"$(echo x.md)\""] {
        let text = file_document(&format!("lazy: {value}\n"));
        assert_eq!(hovers.markdown(&document, &text, value, 2), key, "{value} keeps the property hover");
    }
    // `{{DIR}}` is biscuit-file syntax: a static reference whose variable
    // this document's context does not define.
    let environment = file_document("lazy: \"{{DIR}}/x.md\"\n");
    assert_eq!(
        hovers.markdown(&document, &environment, "{{DIR}}", 2),
        "This file reference needs context this document does not have (a home folder, repository, or package), so it cannot be resolved."
    );
}

#[test]
fn values_that_are_not_file_strings_get_no_file_hover() {
    let hovers = Hovers::new();
    let document = hovers.doc("shapes.md");
    let text = file_document("lazy: null\neager: 3\nnested: {main: ./m.md}\nitems:\n  - ./i.md\n# a comment ./c.md\n");
    for (needle, nudge) in [("null", 1), ("3\n", 0), ("{main", 0), ("- ./i", 0), ("./c.md", 1)] {
        let hover = hovers.hover(&document, &text, at(&text, needle, nudge));
        let markdown = hover.as_ref().map(markdown_of).unwrap_or_default();
        assert!(
            !markdown.contains("file reference") && !markdown.contains("Resolves to"),
            "`{needle}`: {markdown}"
        );
    }
    // Keys keep their schema hover.
    let key = hovers.markdown(&document, &text, "eager: 3", 1);
    assert!(key.contains("eager") && !key.contains("Resolves to"), "{key}");
}

#[test]
fn files_that_cannot_be_checked_say_why_without_a_creation_link() {
    let hovers = Hovers::new();
    let loose = hovers.root.join("loose/doc.md");
    let text = file_document("lazy: \"&root-only.md\"\nremote: https://example.com/x.md\n");
    assert_eq!(
        hovers.markdown(&loose, &text, "&root-only", 1),
        "This file reference needs context this document does not have (a home folder, repository, or package), so it cannot be resolved."
    );

    let schema = "$schema:\n  lazy: file\n";
    let remote = format!("---\n{schema}lazy: https://example.com/x.md\n---\n");
    assert_eq!(
        hovers.markdown(&hovers.doc("remote.md"), &remote, "example", 1),
        "This is a remote URL. It is a well-formed file reference, but the editor never fetches remote content and has not checked whether it exists."
    );

    #[cfg(unix)]
    {
        write(&hovers.doc("afile.md"), "x");
        let through = format!("---\n{schema}lazy: ./afile.md/inner.md\n---\n");
        assert_eq!(
            hovers.markdown(&hovers.doc("io.md"), &through, "inner", 1),
            "The filesystem could not be checked for this file reference (a permission or I/O failure)."
        );
    }

    let no_context = Some(DocumentResolution::from_failure(ContextFailure::NotProvided));
    let plain = format!("---\n{schema}lazy: ./x.md\n---\n");
    // Without a context no document schema is assembled, so the value is
    // not `file`-typed and gets no file hover (its context failure is the
    // document's one diagnostic).
    let hover = hovers.with_document(&hovers.doc("none.md"), &plain, PositionEncoding::Utf16, no_context, |ctx, registry| {
        registry.hover(ctx, at(&plain, "./x.md", 2))
    });
    let markdown = hover.as_ref().map(markdown_of).unwrap_or_default();
    assert!(!markdown.contains("file reference") && !markdown.contains("Resolves to"), "{markdown}");
}

// ── Ranges and URIs ─────────────────────────────────────────────────────────

/// Value hovers range the authored value in both negotiable encodings (UTF-8
/// and UTF-16; the server offers no UTF-32), with
/// non-ASCII text before it on the line, and link a target in a folder with
/// spaces, brackets, parentheses, and non-ASCII characters.
#[test]
fn value_hover_ranges_and_links_survive_unusual_text_and_paths() {
    let hovers = Hovers::new();
    let folder = hovers.repo().join("[x] (y) café");
    // The selected schema types `source` as a file.
    let schema = folder.join("schemas é.yaml");
    write(&schema, "$schema:\n  title: string\n  source: file\n");
    let note = folder.join("note é.md");
    write(&note, "x");
    let text = "---\ntitle: \"é🎉\"\n$schema: \"./[x] (y) café/schemas é.yaml\"\nsource: \"./[x] (y) café/note é.md\"\n---\n";
    let document = hovers.repo().join("unusual.md");

    let schema_line = "$schema: ";
    let schema_value = "\"./[x] (y) café/schemas é.yaml\"";
    let source_value = "\"./[x] (y) café/note é.md\"";
    for (encoding, unit) in [
        (PositionEncoding::Utf8, str::len as fn(&str) -> usize),
        (PositionEncoding::Utf16, |text: &str| text.encode_utf16().count()),
    ] {
        let (schema_hover, source_hover) = hovers.with_document(&document, text, encoding, None, |ctx, registry| {
            (
                registry.hover(ctx, at(text, schema_value, 2)).expect("a `$schema` hover"),
                registry.hover(ctx, at(text, source_value, 2)).expect("a `file` hover"),
            )
        });
        let start = unit(schema_line) as u32;
        assert_eq!(
            schema_hover.range,
            Some(Range::new(Position::new(2, start), Position::new(2, start + unit(schema_value) as u32))),
            "{encoding:?}"
        );
        let start = unit("source: ") as u32;
        assert_eq!(
            source_hover.range,
            Some(Range::new(Position::new(3, start), Position::new(3, start + unit(source_value) as u32))),
            "{encoding:?}"
        );
        let markdown = markdown_of(&schema_hover);
        assert!(
            markdown.ends_with(&format!("Resolves to [\\[x\\] \\(y\\) café/schemas é.yaml]({})", link_uri(&schema))),
            "{markdown}"
        );
        assert!(!link_uri(&schema).contains(['(', ')', '[', ']', ' ']), "{}", link_uri(&schema));
        assert_eq!(
            markdown_of(&source_hover),
            format!("Resolves to [`[x] (y) café/note é.md`]({})", link_uri(&note))
        );
    }
}
