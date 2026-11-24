# The `darkmatter` Rust library

## Depend on it

Inside the rusty-biscuit monorepo, add a path dependency:

```toml
[dependencies]
darkmatter = { path = "../darkmatter/lib" }   # adjust the relative path
serde_json = "1"
```

Most types live under `darkmatter::markdown`. The ones you will use:

| Path | What |
|---|---|
| `darkmatter::markdown::Markdown` | a document: frontmatter + body |
| `darkmatter::markdown::{Frontmatter, MergeStrategy, MarkdownError, MarkdownResult}` | frontmatter and error types |
| `darkmatter::markdown::output::{TerminalOptions, HtmlOptions, MermaidMode, …}` | rendering options |
| `darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot, ComposeOperation, ComposeReport, ComposeWarning, ComposeFailure, build_resolution_context}` | composition |
| `darkmatter::markdown::compose::policy::BlockingPolicy` | the library form of `--deny` |
| `darkmatter::markdown::compose::conditions::evaluate_condition_against` | evaluate one expression |
| `darkmatter::prelude::*` | renderable components: `DarkmatterPage`, `CodeBlock`, `TocTree`, `FileTree`, `DeltaReport`, … |

`darkmatter::Markdown` is **not** re-exported at the crate root. Some older
README snippets use it, and `write_terminal`, which does not exist either.
Use the paths above. The examples on this page were compiled and run
against the darkmatter source of 2026-11-08.

## `Markdown`: load, read, change, render

```rust
use std::path::Path;
use darkmatter::markdown::{Markdown, MergeStrategy};
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions};
use serde_json::json;

let mut md: Markdown = "---\ntitle: Hi\n---\n# Body".into();   // From<&str>/From<String> parse frontmatter
let md2 = Markdown::try_from(Path::new("README.md"))?;         // from disk
let md3 = Markdown::new("# no frontmatter");
// Markdown::from_url(&url).await? fetches over HTTP (async)

let title: Option<String> = md.fm_get("title")?;               // typed read; None if absent
md.fm_insert("draft", false)?;                                 // set/overwrite one key
md.fm_set_defaults(json!({"lang": "en"}))?;                    // fill missing keys only
md.fm_merge_with(json!({"author": "Ken"}), MergeStrategy::PreferDocument)?; // or PreferExternal / ErrorOnConflict
md.frontmatter_mut().as_map_mut().shift_remove("draft");       // raw IndexMap access (order preserved)

let body: &str = md.content();
let whole: String = md.as_string();                            // frontmatter + body
let ansi = md.as_terminal(TerminalOptions::default())?;        // styled terminal text
let html = md.as_html(HtmlOptions::default())?;                // HTML
let ast = md.as_ast()?;                                        // MDAST node (serde-serializable)
```

Other useful methods:

- `toc()` returns a `MarkdownToc`, with headings, slugs, code blocks, and
  internal links.
- `delta(&other)` returns a `MarkdownDelta`, with a classification
  (`NoChange` … `Rewritten`), section changes, and frontmatter changes.
- `links()`, `image_references()`, `inline_html_links()`
- `validate_references(ReferenceValidationOptions)` returns a report with
  `is_valid()` and `issues`.
- `cleanup()` and `cleanup_with(&MarkdownRuleOverrides)` format the body. To
  get `md clean` behavior exactly (frontmatter repair and diagnostics), use
  `markdown::cleanup::clean_document`.
- `normalize(Some(level))` and `relevel(level)` fix or shift heading levels.
  `validate_structure()` reports problems in the heading hierarchy.
- `remove_section("## Title")` removes one section; it also accepts a
  `"## Prefix*"` pattern and `"!prelude"`.

Duplicate keys anywhere in YAML frontmatter are a parse error. Key order is
preserved at every depth.

## Composition: the request is the authority

Every call that resolves a file reference runs inside a `ComposeRequest`.
A request is built from two inputs:

- `ComposeOptions`: **what** to do. This covers the operations to run,
  overrides, shell, remote, and schema policy.
- `RequestSnapshot`: **where** the request runs. This covers the request
  directory, home, environment, and extra `@` roots.

The request fixes one `FileResolutionContext` up front. Validation,
pre-flight, composition, and every transcluded child all resolve against
that same context.

```rust
use std::collections::HashMap;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

// Binary entry point: read the process ONCE (cwd, HOME/USERPROFILE, env).
let snapshot = RequestSnapshot::from_process()?;

// Library code: never read the process; state every input.
let snapshot = RequestSnapshot::new("/work/project")
    .with_home(Some("/home/me".into()))
    .with_env(HashMap::from([("NOTES".into(), "/home/me/notes".into())]));
// .at_request_dir(dir) re-anchors and keeps home/env/roots; .with_magic_root(path, PathPosition) adds @ roots

let options = ComposeOptions::new()
    .with_source_file("docs/guide.md")              // enables ./ and bare references relative to the doc
    .with_external_state(serde_json::json!({"name": "Alice"}))   // = --state (defaults)
    .with_set_overrides(serde_json::json!({"draft": true}));     // = --set (overrides)
let request = ComposeRequest::prepare(options, &snapshot)?;
// ComposeRequest::with_context(options, ctx) when you already built a context
```

`RequestSnapshot::new` starts with **no home and an empty environment**. So
`~/x` fails and `env.X` is `null` unless you supply them. This is deliberate:
a library call states everything it depends on.

### Run it

```rust
// Optional: which shell commands would run? Nothing executes.
let preflight = md.compose_preflight(&request)?;
let commands: Vec<String> = preflight.approval_set();

// Compose. On success: a new Markdown plus a report.
let (composed, report) = md.compose_with(&request)?;
println!("{}", composed.content());          // body only (frontmatter is in composed.frontmatter())
for w in &report.warnings {                  // Vec<ComposeWarning>: stage, message, code, path, resolution_failure, …
    eprintln!("{:?} {}", w.code, w.message);
}

// Prefer this when you need the warnings that came before a failure:
match md.compose_with_warnings(&request) {
    Ok((composed, report)) => { /* … */ }
    Err(failure) => {                        // Box<ComposeFailure> { error, warnings, read_set }
        for w in &failure.warnings { eprintln!("warning: {}", w.message); }
        // failure.error.resolution_failure() -> Option<biscuit_file::ResolutionFailure>
        //   (InvalidReference | MissingContext | NoMatch | Io | UnsupportedRemote)
        return Err(failure.error.into());
    }
}
```

Compare failures by their class, using `MarkdownError::resolution_failure()`,
never by message text. The class is found through nested transclusions and
schema `file` values too.

### Common `ComposeOptions` knobs

| CLI | Library |
|---|---|
| `--state JSON` / `--set JSON` | `with_external_state(v)` / `with_set_overrides(v)` |
| `path=value` setters | `with_setters(SetterPlan)` (`compose::setters`) |
| `--deny all`, `--deny CODE` | `with_blocking_policy(BlockingPolicy::default().deny_all())`, `.deny_code("dm.…")`, `.exempt_code(…)` |
| `--allow missing-transclusions` | `with_allow_missing_transclusions(true)` (and other `with_allow_*` methods) |
| `--allow host=HOST` | `with_allowed_host("example.com")` or `with_remote_read_config(…)` |
| `--cache-root DIR` | `with_cache_root(dir)` |
| `--timeout S` | `with_shell_timeout(Duration)` |
| `--width N` | `with_output_width(renderable::output::Width)` |
| `--disable baseline-schema` / explicit baseline | build options without `with_darkmatter_baseline_schema()`, or pass `with_baseline_schema(SchemaRoot)` |
| skip a stage | `.disable(ComposeOperation::Normalization)` or `.only(&[…])` |
| `::map`/`::filter` limits | `with_max_iteration_visits(NonZeroU64)`, `with_max_expansion_bytes(NonZeroU64)` (defaults 100,000 and 16 MiB) |

The `ComposeOperation` stages are `FrontmatterInterpolation`,
`FrontmatterShellExpansion`, `TextReplacement`, `PageBlocks`,
`CollectionBlocks`, `Interpolation`, `ShellExpansion`, `ShellBlocks`,
`LinkResolve`, `BlockTransclusion`, `FrontmatterTransclusion`,
`CodeTransclusion`, `Cleanup`, `Normalization`, and `LinkNormalization`.

With a `BlockingPolicy` installed, a blocked warning fails composition with
`MarkdownError::BlockedDiagnostics`. The library leaves warnings from the
last stages in `report.warnings`. Check them yourself before you publish
the result, as `md` does before it writes.

### Shell commands from a library

Shell commands always go through approval. A library caller has these
options:

- `with_shell_approval_handler(Arc<dyn ShellApprovalHandler>)` decides each
  unknown command. `approve(&self, ShellApprovalRequest)` returns
  `ShellApprovalDecision::{AllowOnce, AllowExactPersist, AllowCommandPersist,
  AllowTemplatePersist, Deny, BlacklistPersist}`. The traits are in
  `markdown::compose::shell_expansion`.
- `with_pre_approved_commands(…)` approves commands you already know.
- `with_shell_policy_root(dir)` chooses where the whitelist and blacklist
  files live. The default is the context's repository root, else home, else
  the request directory.
- `.disable(ComposeOperation::ShellExpansion)` and the other shell stages
  turn shell execution off.

With no handler, a command that is not whitelisted fails composition with
`ShellExpansionError::ApprovalRequired`. The built-in blacklist cannot be
overridden. Call `compose_preflight` first if you want to show the user the
list of commands before anything runs.

## Evaluate one expression

```rust
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::compose::conditions::evaluate_condition_against;

let ctx = build_resolution_context(&RequestSnapshot::new("/work/project"))?;
let ok: bool = evaluate_condition_against(
    "draft && length(tags) > 1",
    &serde_json::json!({"draft": true, "tags": ["a", "b"]}),
    &ctx,
)?;
```

Lookup order is `fm.*`, then keys in the data, then `env.*` (from the
context, never the process), then `ctx.*` (captured lazily). A host can add
its own functions with `ComposeOptions::with_host_globals(HostGlobals)`. The
host-functions topic page in the canonical docs covers this.

## Lifecycle events

The library parses the lifecycle stacks (`initialize`, `composed`, `failed`,
`cancelled`, `finalize`) but **runs nothing** unless a host drives a run
through `darkmatter::lifecycle`. `md compose` is one such host. The
lifecycle topic pages in the canonical docs cover the details.

## Rendering components

`darkmatter::prelude` re-exports components that implement both
`TerminalRenderable` and `BrowserRenderable`. Use them to build terminal or
browser output directly: `DarkmatterPage` (page frame: margins, max width,
background), `CodeBlock`, `TocTree`, `FileTree`, `DeltaReport`, and
`ValidationReportView`.

`CodeBlock::render_terminal_excerpt` takes source regions as
`(first_line, text)` pairs in source order and renders them in one panel
with source line numbers and gap markers.
