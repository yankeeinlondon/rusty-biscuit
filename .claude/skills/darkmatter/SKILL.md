---
name: darkmatter
description: Expert guidance for the Darkmatter Rust library, `md` CLI, and DMLS language server. Use when composing or rendering Markdown, working with frontmatter, expressions, schemas, file references, remote content, hashing, browser or terminal output, or extending Darkmatter's CLI and editor integrations.
---

# Darkmatter

Darkmatter is the monorepo's Markdown composition, schema, expression, and
rendering system. The package area contains the `darkmatter` library,
`darkmatter-cli` binary (`md`), `dmls` language server, and `zed-dmls`
extension.

## Non-negotiable boundaries

- Capture one request-scoped composition context. Pass the same
  `FileResolutionContext`, repository observation, remote policy, cache state,
  identity, and meta-schema controls through `ComposeOptions`; never recapture
  CWD or repository state in a downstream resolver.
- Resolve file-like values through `biscuit_file::FileReference`. Preserve its
  source context, explicit-vs-implicit syntax, and typed errors.
- Read the home only through `biscuit_file::home_dir()` (`RequestSnapshot::from_process`
  is the one production site; tests build expectations with it too). A canonical
  path that leaves a private comparison (link targets, cache and preflight keys,
  `referenced_files`, diagnostics) comes from `biscuit_file::canonicalize_simplified`;
  `path_lookup_guard.rs` in lib and cli rejects any unlisted raw `canonicalize`.
- Treat composition as effectful and validation as passive. Schema parsing,
  trigger matching, completion, hover, and validation must not perform I/O,
  execute expressions, fetch remotes, or mutate documents.
- Keep network access deny-all by default. Route every allowed remote read or
  write through `biscuit_file::FetchPolicy` and exact-host consent.
- Render terminal output with `TerminalRenderable` components. The browser tier
  is headless and must use browser-protocol input, never host focus or OS input.
- Preserve source spans and typed diagnostic provenance through parsing,
  composition, schema validation, and DMLS projection.

## Choose the owning surface

Documentation under `docs/` becomes the authority when implementation lands;
it never links to a specification or feature/fix design. Preserve documented
file locations; an unimplemented grammar is not a reason to move schema
definitions away from their references or replace them with placeholders.

| Work | Start with |
|---|---|
| Compose APIs, stages, expressions, file resolution, cache | [compose.md](compose.md) |
| SimplifiedSchema, triggers, validation, meta-types | [schema.md](schema.md) |
| Composition requests, contexts, glob consumers | [requests.md](requests.md) |
| Remote reads and the transport cache | [remote-cache.md](remote-cache.md) |
| `md` file arguments and CLI conventions | [cli.md](cli.md) |
| Expression grammar, inserted text, warnings | [expressions.md](expressions.md) |
| Lifecycle grammar, executor, run accounting, request/reply `Run` | [lifecycle.md](lifecycle.md) |
| Test fixtures, source guards, parity matrix | [testing.md](testing.md) |
| Public modules and extracted library surfaces | [library-surfaces.md](library-surfaces.md) |
| DMLS architecture, protocol behavior, and rollout history | [dmls.md](dmls.md) |
| Render tree, style lowering, disclosure blocks, code blocks | [rendering.md](rendering.md) |
| Terminal rendering options | [terminal.md](terminal.md) |
| Frontmatter model, shared views, literal tokens, in-place leaf edits | [frontmatter.md](frontmatter.md) |
| Error/status block conventions | [errors.md](errors.md) |
| `md clean`/DMLS formatting rules and frontmatter repair | [cleanup.md](cleanup.md) |
| Document comparison | [comparison.md](comparison.md) |
| Module layout | [structure.md](structure.md) |
| Parser details | [pulldown-cmark.md](pulldown-cmark.md) |
| Frontmatter ecosystem comparison | [frontmatter-crates.md](frontmatter-crates.md) |

Load only the topic needed for the task. For the SimplifiedSchema language
itself (grammar, named types, conversion, `SimplifiedSchema`/`Prepared`
validation, layering), load the `simplified-schema` skill
([SKILL.md](../simplified-schema/SKILL.md)). For render-tree implementation work,
also load the `renderable` skill; for terminal components, load
`biscuit-terminal`; for file-reference or JSON/YAML/TOML conversion work, load
`biscuit-file`.

## Composition authority

`ComposeRequest` is the request authority: `ComposeOptions` (remote
configuration, cache root and policy, compose identity, baseline/meta-schema
controls, rendering options) plus the one required `FileResolutionContext`
built from a `RequestSnapshot`. A source-derived file reference must resolve
against a context derived from that one. User-facing docs:
`darkmatter/docs/topics/compose-requests.md` (API, builder, Mermaid),
`topics/file-referencing.md` (forms and tree rule), and
`errors/file-reference-failures.md` (the `failure:` row and data field), and
`errors/composition-problems.md` (stopping rules, authored positions).

The root compose pipeline is ordered: the passive call check (a proven-bad
call in any branch rejects the document before anything runs); frontmatter
interpolation pass 1;
schema validation and coercion; frontmatter shell expansion; frontmatter
interpolation pass 2; literal replacement; conditional page blocks and
`::map`/`::filter` regions (one structural traversal, callbacks finish before
body stages); body interpolation; shell directives and blocks; link resolution; concurrent
transclusion; inline cleanup and optional reflow; root-only link
normalization. Keep this order stable. Whole-value `{{ ... }}` and `$(...)`
values are executable state: they must resolve or fail, never leak as literal
syntax. Demand-driven context capture must observe only referenced `ctx.*`
groups.

Every entry point that resolves file references takes a `ComposeRequest`
(`ComposeRequest::prepare(options, &RequestSnapshot)` or
`ComposeRequest::with_context`). The context is required, never `Option`;
only binaries call `RequestSnapshot::from_process()`, and magic roots enter
only through `RequestSnapshot::with_magic_root*`. `match()`, `find_files()`,
and `::file-links <glob>` are `GlobReference`s judged or listed in the
document's (or the caller's origin) context, never the process directory.
Read [requests.md](requests.md) before touching a request, a context view,
a validator's context, or a glob consumer.

`::map` and `::filter` share one executor (`compose/collection_blocks/`); read
the "Map and filter" section of [compose.md](compose.md) before touching it.

Read [compose.md](compose.md) before changing a stage, expression function,
context property, cache key, transclusion directive, or file-resolution path.

## Schema authority

Darkmatter uses the SimplifiedSchema language (the `simplified-schema` crate),
compiled to Draft 2020-12 JSON Schema.
`DarkmatterSchemas` owns baseline merging and validator caching. The standalone
classifier is `parse_standalone_schema_document`; it recognizes an informal
`$schema` document (optional `types`, `description`, `categories`), a `kind: schema`
document with `$schema` and/or `types`, or a `kind: yaml-schema` (a YAML
document kind, never a `$schema` target or import source: `SelectedByKind`),
and admits each against the library's compiled envelope definitions. A `$schema` target must export
`$schema` (YAML) or be a JSON object declaring Draft 2020-12; anything else is
`NotASchemaDocument`/`UnsupportedJsonDialect`/`InvalidJsonSchema`, while a
`types`-only file still serves `Name@file` imports. A loader refusal of a
trigger file is reported as `TriggerSchemaReferenced`/`TriggerPayloadCycle`
(`triggers::assemble::refused_trigger_file`).
A whole `.yaml`/`.yml` document is validated by
`DarkmatterSchemas::validate_yaml_document` (claimed `kind` > root `$schema` >
none, no baseline or triggers) over a per-context `KindRegistry`; read
[schema.md](schema.md#whole-yaml-documents-schemasyaml_documents) first.
Ownership: the `simplified-schema` crate owns the `schema` and `yaml-schema`
definitions (`simplified-schema/schemas/`) and envelope admission;
Darkmatter owns `schema-trigger.yaml`, the `KindRegistry`, selection, and the
kind checks. A registry is built per call (or once per folder/worker and
passed with `with_kind_registry`); it never fails as a whole, and a
duplicate, reserved, or broken claim is a `KindSelection` error only for a
document whose `kind` it could affect. A YAML file is never Markdown: a
fenced `---`…`---` file is two documents and fails to load.

Important contracts:

- Optional schema properties accept missing or `null` values. A property whose
  whole value admits null (`null`, `unknown`/`any`, or a union with such an
  arm) is never required, whatever its `required`, `eager`, or `generated`
  markers; one shared presence calculation decides this for every consumer.
- `alphanumeric` options are type identity (canonical, never merged or
  widened); `shell` is a passive `darkmatter-shell` format, never parsed or
  run. See [schema.md](schema.md#core-types-unknown-null-alphanumeric-shell).
- Validation keeps source positions, origin information, typed problem codes,
  pending values, and file-reference diagnostics.
  `FileReferenceDiagnostic::resolution_failure()` gives the biscuit-file
  class; the diagnostic is re-derived in the validator's own request context.
- Eager `file(eager)` values may normalize only on successful composition;
  validation-only APIs remain read-only.
- `literal(value)` preserves YAML scalar typing and lowers to JSON Schema
  `const`; quoted and native YAML forms are observably different.
- `expression`, `yaml`, and `json` are passive content formats. They parse or
  coerce but never execute.
- One coercion engine (`simplified_schema::coercion`) converts frontmatter and binds function arguments; numbers enter only through `simplified_schema::numeric` as exact
  `f64`s. See [schema.md](schema.md#conversion-engine) and
  [expressions.md](expressions.md).
- The SimplifiedSchema language lives in the `simplified-schema` crate (no
  Markdown dependency): the AST and grammar (`simplified_schema::simplified`,
  `SchemaRoot` is the parsed `$schema` enum), resolution, conversion,
  validators, formats, `SchemaError`, `ValidationProblem`, the numeric model,
  `coercion`, `MappingOrders`, and the expression syntax. Import those from
  `simplified_schema` (every consumer crate depends on it directly);
  `darkmatter::markdown::schemas` no longer re-exports them. Darkmatter keeps
  only `markdown::{span, literal_token}` and the expression items its
  composition API names (`compose::expression::{Expr, Lexer, parse, …}`).
  Edit the language in `simplified-schema/lib/src/`. What stays in Darkmatter:
  `DarkmatterSchemas`/`EffectiveSchema`, the baseline, schema roots
  (`SchemaSearchRoot`), triggers (`TriggerOrigin` maps a `SchemaOrigin` to its
  `TriggerSource`), the shipped catalog (`embedded::SHIPPED_SOURCES`, which
  `schemas::resolve` wrappers pass to the library resolver so
  `Name@darkmatter:<stem>` resolves), pending classification
  (`coerce::nested_templates`), `ProblemPositions`, and completion over an
  `EffectiveSchema`. Work counters for moved code are recorded by the library
  and folded into `darkmatter::work_counters::snapshot()`.
- The library's standalone object is `simplified_schema::SimplifiedSchema`
  (content and file factories, strict/soft validation, atomic layering, type
  queries, export; `simplified-schema/docs/topics/schema-object.md`).
  Darkmatter's own validation entry points do not route through it: the
  effective schema merges the already lowered baseline, trigger, and
  document layers with the same `merge_baseline` the object's `layer_under`
  uses (memoized by `Arc` identity), and
  `darkmatter/lib/tests/l1/effective_schema_layering.rs` pins the two
  equal.
  `compose::repository_scope_catalog` re-exports the library's projection,
  the same one its raw-path factories capture with: edit it there.
- `type-definition` and `schema` meta-types delegate to the same passive
  parsers used by authoring and DMLS.
- Trigger matching is schema-based and passive. A schema-trigger document is a
  kinded document and must retain its root `kind` declaration.
- Triggers and bare-name `$schema` lookup use the five schema roots of the
  **document's** context (`schemas::roots::SchemaRoots::for_document`):
  `SCHEMAS_DIR` (the folder itself, from `ctx.env()`), package, package area,
  `base_dir()`, `~/schemas`. Never walk ancestors or read the process
  environment for them. Every bare-name consumer (`$schema`, `Name@file`,
  `example(file)`, trigger payloads) selects through
  `simplified_schema::resolve::select_schema_file`; never add another copy. `$path` patterns are `GlobReference`s; see
  [schema.md](schema.md#schema-roots-and-path-triggers).
- The runtime baseline is compiled in with its imports (Content Policy's
  `EDITOR_SCHEMA` types `content_policy`) and is the default for compose,
  `md schema validate`, and DMLS; see [schema.md](schema.md#runtime-baseline).

Read [schema.md](schema.md) for imports, recursive named types (`$defs`,
`TypeExpr::Ref`), unions, pattern dictionaries, suggestions, triggers, and
DMLS schema behavior.

The global catalog `darkmatter/schemas/darkmatter.yaml` declares every
global's shape; the built-in baseline is its resolved `fm`. A catalog acts
as globals only when registered explicitly; a document `$schema` naming one
just gets its entries as optional properties; see
[schema.md](schema.md#authored-global-catalog).

## Remote and cache safety

HTTP(S) composition covers `::file`, `::code`, and read-side expression
arguments where a remote runtime is present; rendered links are never fetched,
and frontmatter interpolation and `$()` fail loudly on a URL. Reads are
deny-all until `--allow host=HOST`; only transport artifacts (raw HTTP bodies) may
persist under a cache root; `Cache-Control` outranks every freshness flag;
host policy is checked before the cache is read. Read
[remote-cache.md](remote-cache.md) before changing any of this.

## Rendering authority

All public Markdown rendering folds through the render tree. Darkmatter builds
one context-aware `renderable::Document`, then performs one target fold.
Component policy is lowered during construction; do not add a post-fold HTML or
terminal decoration pass.

`DarkmatterPage` is only a viewport/page-frame assembler and never inspects or
mutates component content. Browser tests are headless (no window activation or
host input) and assert DOM, computed style, accessibility state, or geometry;
browser HTML and CSS come from the same lowered values, never string repair.

Read [rendering.md](rendering.md) before changing style claims, code-block
themes, disclosure blocks, browser safety, or the render-tree fold.

## CLI orientation

The binary is `md`: `render`, `compose`, `clean`, `toc`, `delta`, `schema
validate|detect|about|triggers`, `hash`, `get|set|rm` (frontmatter), `edit`,
`code-block`, `graph`, and `validate refs`. Users' contract:
`docs/cli/output-and-width.md` and `docs/cli/data-contracts.md`.

Checks are relaxed or tightened only through three verbs: `--allow <id|all>`
(a recovery, or `host=HOST`), `--deny <id|all>` (make diagnostics blocking),
and `--disable <id>` (stop a feature; no `all`). `cli/src/args/policy.rs` is
the one registry of identifiers, commands, and help; on `md compose`,
`--deny` lowers to the library's `compose::policy::BlockingPolicy`. The legacy per-flag spellings are
usage errors (`main.rs::REMOVED_FLAGS`); see `docs/cli/policy.md`. The
pipeline's call precheck consults the policy per document, before its
expressions run (`MarkdownError::BlockedDiagnostics`; a child's is fatal,
never a tolerated transclusion failure); the pre-flight walk's warnings
(a skipped child's invalid trigger) are judged before any command is
approved: by `compose_preflight_approvals*` (`pipeline::stop_if_warnings_blocked`,
before policy files or the handler) and by `validate_pre_approved` before
membership, at the root and `as_markdown` gates; a runtime warning is checked by
`pipeline::stop_if_blocked` before each body stage, after transclusion
`when=` preparation, between `::shell` commands, and before frontmatter
shell expansion, and within a stage before each further expression
(`pipeline::blocking_guard` through the rewriters' `BeforeEval`, for body
and frontmatter interpolation and directive targets) or condition
(`pipeline::ReportWatch`, for `::block` and `when=`), judging only what is
new since the last check (undeclared reads only once state and schema are
final, through `unknown_identifiers::pending_warnings`; never in discovery
passes); a nested `as_markdown` child's stop is restored on the caller
(`NestedComposeSlot::take_blocked`), even when `||` absorbed the call;
the last stages' warnings are checked by the CLI before writing. A new coded warning adds its
code to `COMPOSE_DIAGNOSTIC_CODES`; a new recovery adds a registry entry
naming the codes it governs.

Every other command leaf goes through `cli/src/output/command.rs::CommandRun`:
`CommandRun::begin` (target, `--width`, default width, `DataShape`), build a
`CommandValue` (`markdown::output::command`: the data struct, a `Document` for
the generic folds, optional terminal, Markdown, and HTML overrides), then
`run.finish(result)`; a leaf working on one document calls
`run.set_document` so a failure with no file of its own still names it. A handler returns `Completed::success` or
`Completed::failure` (a report that is the deliverable), or `Err` (no result),
which `finish` turns into a payload for a data output and leaves to `main` for a
formatted one. `DataShape` adds the run status: `Typed` always, `Object` only
when a run needs it, `Bare` as `{ outcome, diagnostics, value }` for a value whose
fields are the user's (`md get`, `md schema detect`). `md clean`'s own
`diagnostics` keep their meaning, so its run-level ones are `run_diagnostics`. A
document command calls `.exact_markdown()` and `commands/render.rs::document_value`,
keeping its text's bytes and page-rendering everything else. An override reads
only the `OutputOptions` it is given (`Terminal::from_output_options`), never the
host: it also serves `plain`. A component that paints regardless of its terminal
(`DeltaReport`, a status block) needs `strip_escape_codes` when
`terminal_profile.degrades_to_plain()`, and `output::stderr_text` on stderr.
A new leaf adds a row to `args/output.rs::COMMAND_LEAVES`, a schema file under
`cli/schemas/output/`, and conformance tests in `cli/tests/l1/output_commands.rs`
that spell the schema path inside each `#[test]`;
`every_leaf_has_a_schema_file_and_a_conformance_test` fails otherwise. A schema's
named types are closed, unlike its root object, so a type must list every field.
One Rust `Diagnostic` serializes into all 17 output schemas, so their
`diagnostic` and `data-location` types are identical
(`output_commands::every_schema_file_shares_one_diagnostic_type`); a new
diagnostic field goes into every file. `DocumentEnvelope.read_set` is always an
array; `ReportSink::reads_from(&DataRuntime)` reads the command's runtime when
the envelope is written, so every exit carries it without per-path pushes.
A schema root is an object: a scalar result is judged under `value`.

Link titles are JSON5 objects or plain text (`docs/topics/hyperlinking.md`); a
leftover `key=value` title is plain text with a warning until `md clean`
migrates it (`cleanup::link_titles`, see [cleanup.md](cleanup.md)). Image titles
keep their own syntax (`render/image_ref.rs`). Titles
(`dm.link.malformed_structured_title`, `dm.link.legacy_structured_title`) are
interpreted only by
`renderable::output::link_metadata::parse_link_title`, which reports a neutral
notice; `compose::policy::link_title_code` gives its code. `Link` stores the notice
(`title_notice`) and never prints. Composition reports them from
`compose::link_title_warnings`, keyed by the link's authored start: once before
anything runs (so a failed run keeps them and `--deny` stops first) and once
after the inline stages (a link a stage wrote). `md render`, which does not
compose, reads `Markdown::link_title_warnings` before cleanup and names the
same codes through `RENDER_DIAGNOSTIC_CODES`.

The reference report is `reference/validate.rs::ValidationReportView`, built in
`reference/report.rs`, implementing `CommandOutput`. `ReferenceValidationReport`
keeps `documents` (one entry per inclusion, depth-first, empty documents
included) so grouping by `(source, included_via)` and the document count never
infer scope from issues; neither `documents` nor `references` is serialized, and
issue JSON reports authored `line`/`column`, `section`, `inserted`, and
`included_via`, null where unknown (`ReferenceOrigin::line`/`span` stay prepared
coordinates, for graph walkers only; `origin.authored` is the projection).
`as_prose()` is the Markdown/HTML document;
`render_terminal` rebuilds the same structure with bare `file://` URIs
and `heading` as a link parameter. Compose and `md graph --validate` show errors
(compose prints nothing when there are none), `md validate refs` shows all
severities. Reasons and constructs live only in
`ReferenceIssueCode::reason_phrase` and `ReferenceSyntax::construct_phrase`.
HTML discovery (`reference/html.rs`) reads one Markdown HTML node as any number of
elements with `scan_elements`, and `LineMap` carries each back to the source past
line breaks and container prefixes; never classify an HTML node line by line.
`visit_html_elements` walks a document's HTML nodes in order, so a raw-text element
split across nodes is one element, and every reader drops what starts in its
`inert_ranges`; never decide inertness in one consumer. See
`docs/cli/validate.md#html-and-css-references`.

`md compose` accepts its Markdown default and the data family; `md render` both
families, defaulting to `terminal`, which is byte-identical to `plain` off a
color TTY. A data output of either prints one envelope (`file`, `outcome`,
`document`, `diagnostics`, `summary`, optional `perf`, `lifecycle_output`;
`markdown::output::command::DocumentEnvelope`; schemas in
`cli/schemas/output/`) for every parsed invocation, and stderr stays empty: no
`-v`, tracing, prompt, or lifecycle line reaches it. Lifecycle output there is
collected per step; in Markdown mode all of it, `stdout:` included, goes to
stderr, because stdout holds only the deliverable. Exit 0 succeeded, 1 ran and
failed (including a refused reference), 2 usage only; handlers return
`ReportedExit`, and only `main` exits. `cli/src/output/mod.rs` resolves target
and width (`OutputEnvironment`; a test passes one to spoof a stream) and owns
stderr (`stderr_terminal`, `stderr_report`: its own TTY and columns, unlimited
off a terminal, whatever `--width` says). `cli/src/artifact.rs::ProcessOpener`
is the `--show` seam: `MD_DRY_RUN` never launches, and `MD_DRY_RUN=fail` is a
host with no default app. The CLI only collects
(`cli/src/output/report.rs::ReportSink`); every code and record comes from
`compose::report_diagnostics` (`warning_diagnostic`, `error_diagnostics`,
`lifecycle_failure_diagnostics`), and a CLI-worded shell failure keeps its
typed diagnostics through `ClassifiedFailure`. `Markdown::compose_with_warnings`
returns a failure with the warnings reported before it (`PipelineRuntime`'s
failed-document stash); a failed pre-flight walk leaves its failed document's
warnings for `ComposeOptions::take_preflight_failure_warnings` (with the
triggers of the siblings walked before it, each trigger once), as does any
policy error `compose_preflight_approvals*` raises after a successful walk;
a successful one returns them in `ComposePreflightApprovals::warnings`; the
in-pipeline pre-approval gate (`validate_pre_approved`) adds its walk's
triggers to the composition report; and a
lifecycle call rejection carries its earlier warnings (`earlier_warnings`);
a failing document keeps the triggers its `as_markdown` children and gate
found (`NestedCompose::finish` → `PipelineRuntime::record_failed_triggers`).
`md compose`'s reference check is passive: its graph preparation runs no
`::shell` (`prepare_content` never re-enables an operation the request
disabled), and `ComposeOptions::for_reference_check` defers what only
composition can judge (values waiting on a frontmatter command, an expression
it cannot finish). A check that still fails (`check_references` returns
`Result`) is a composition failure with status 1, reported through
`error_diagnostics`; its graph build leaves its warnings for
`take_preflight_failure_warnings`. A `Diagnostic`'s `source.line`/`column` are
`Option` (null when no authored position is proven, never a placeholder), a
record that knows only its file keeps the file, a typed error with no file takes
the composed document (`report_diagnostics::with_document_fallback`, applied by
`ReportSink`), a warning's file is stamped from `ComposeReport::document`, and
only `ComposeWarning::location` (never `line_number`, a current-body count) is
published; `source.inserted` marks a construct that wrote the text; only a reference
record has `reference_details`, flattened into `reason`, `construct`,
`reference`, `failure`, and `included_via`. Skipped-trigger
warnings found by a pass that runs ahead of the delivering report (reference
validation via `ReferenceValidationReport::trigger_warnings`, which prepares
each transcluded child in its own context; the lifecycle call check via
`effective_schema_for`; the lifecycle bootstrap; the `--shell` walk via
`ComposePreflightReport::warnings`; the shell-approval walk via
`ComposePreflightApprovals::warnings`) are held in
`compose.rs::PendingTriggerWarnings`: merged once into the first report or
failure, or delivered by any exit that never composes.
The call precheck orders findings by authored position and keeps the prefix
before a rejection. Never print to stdout or stderr on a data-output path.

Every source-file argument opens through `cli/src/io::open_argument`:
`FileReference` grammar first, resolved in the launch context, with the
document context derived from the opening reference. Read [cli.md](cli.md)
before adding a route that reads a file argument or changing CLI style
claims.

## Testing and verification

Use the package-area recipes in `darkmatter/`: `just build`, `just test`,
`just lint`.

Use `just test-l2` only for real-terminal behavior and `just test-browser` for
headless browser behavior. Parser, schema, prompt, template, or configuration
changes require both a passive shipped-artifact corpus test and an end-to-end
test through the normal invocation path. Persisted values require a repeated
read/write/read round trip.

Expression grammar, inserted-text, failure-policy, undeclared-property, host-binding, and
warning-identity rules are in [expressions.md](expressions.md); read it
before changing the lexer, an evaluation surface, or a warning family.
Fixture rules (`CliProcessFixture`, `git init` for repositories), the
source-scan guards (context construction, glob implementation, spawn sites,
semantic results), and the entry-point parity matrix (including the glob
rows) are in [testing.md](testing.md); read it before writing an `md`
integration test, adding a file-resolving entry point, or touching a guard's
allowlist.

Do not run workspace-wide Cargo gates for a Darkmatter-only change; when a
public type or behavior changes, find consumers (Claudine) with Sniff/GitNexus.
