---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-17
review_iterations: 3
completed: true
created: 2026-10-17
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
related:
    - 2026-10-11-dmls-request-timeouts
    - 2026-10-17-dmls-index-store
human_review: true
implemented: true
implemented_by: "claude/opus"
---

# DMLS Startup Schema Registry and Logging

This fix does three things:

1. It stops one malformed YAML file from breaking schemas for a whole
   repository, and stops DMLS from logging that failure on every request.
2. It reorders DMLS startup so the server does only the work that matters
   first: classify every schema file, list the workspace's files, and parse
   the documents the editor has open. Everything else is parsed when a feature
   needs it.
3. It makes DMLS's logs say what it found and where its time goes.

The persistent backlink and symbol store that builds on this is a separate
feature, `2026-10-17-dmls-index-store`. Until it lands, the full index this
spec builds on demand lives in memory.

## The Incident

On 2026-10-17 at 17:30:52Z, while `prompts/agent-skills.md` was open in Zed,
DMLS began writing this warning without pause:

```text
WARN dmls::overlay: retaining last-good trigger registry
  path=…/.ai/prompts/agent-skills.md
  error=trigger-schema file `…/schemas/feature-review.yaml` is malformed:
  invalid trigger-schema match expression: trigger-schema envelope is not
  valid YAML: mapping values are not allowed in this context at line 17 column 93
```

The author closed Zed to stop it. Investigation showed:

- `schemas/feature-review.yaml` was mid-edit. Line 17 briefly held an
  unquoted value containing `` `<ID> (<priority>): <title>` ``; the `: ` at
  column 93 is invalid in a plain YAML scalar. The file was saved, quoted and
  valid, at 17:31:34Z. Stripping the quotes reproduces the exact error.
- The file **does not claim** `kind: trigger-schema`. It is an ordinary
  SimplifiedSchema file that happens to live in a schema root.
- Nothing reached disk. DMLS logs to stderr unless given `--log-file`, and Zed
  keeps LSP stderr in memory. The cost was CPU and editor memory.

## Findings

### F1. Any unparseable YAML in a schema root is a trigger load error

`parse_trigger_envelope_from_str`
(`darkmatter/lib/src/markdown/schemas/triggers/envelope.rs`) parses YAML
**before** deciding whether the file claims the envelope, and maps a parse
failure to `SchemaError::TriggerMatch`. Discovery
(`triggers/discovery.rs::scan_with_embedded`) feeds it every `.yaml`/`.yml`
in every searched root, and discovery is transactional, so one syntax error in
an unrelated file rejects the whole scan.

This contradicts the documented contract in three places:

- `darkmatter/docs/topics/schemas/definition.md`: "an invalid opted-in
  envelope rejects the scan, while unrelated YAML files are ignored."
- `triggers/discovery.rs` module docs: "A file that does not claim
  `kind: trigger-schema` is silently ignored."
- `dmls/src/overlay/mod.rs` (authoring path): "ordinary schema-root YAML
  remains silent."

Blast radius, verified in a scratch repository:

- `md compose` of **any** document under that root fails
  (`schema validation failed`, exit 1). Claudine composes through the same
  path, so every Claudine launch in the repository would fail too.
- A freshly started DMLS has no last-good registry, so documents silently lose
  their trigger payloads.
- DMLS's schema-file authoring path publishes a trigger diagnostic on the
  ordinary YAML file.

### F2. A YAML syntax error is reported as a match-grammar error

Even for a file that *does* claim the envelope, a syntax error is wrapped as
"invalid trigger-schema match expression" and carries the hint "Fix the match
grammar or envelope shape… combinators, property conditions, and the `$path`
predicate". Neither is true: the file never got as far as the match grammar.

### F3. DMLS rescans every schema root on every request

`OverlayCache::trigger_registry` (`dmls/src/overlay/mod.rs`) calls
`scan_triggers` uncached. It is reached from `schema_for` on every overlay
build, which happens on hover, completion, semantic tokens, inlay hints,
document highlights, and diagnostics. Each call enumerates all five roots and
reads and parses every YAML file in them, plus payload resolution. That is
wasted I/O while the scan is healthy, and it is the loop that made F4 visible.

### F4. The retained-registry warning repeats on every request

The `tracing::warn!` in `trigger_registry` fires on every failed scan. The
code right below it already deduplicates the *diagnostic* transition (it
compares path and error text with the recorded failure), but the log line
ignores that comparison.

### F5. Startup parses every Markdown file, and reports one opaque number

```text
INFO dmls::router: startup index complete documents=5676
```

At startup DMLS walks the workspace and fully parses every Markdown file
(headings, links, wiki links, transclusions, `$schema` and file uses, variable
uses, frontmatter keys). `dmls --bench-index .` on this repository on
2026-10-17:

| Stage | Cost |
| --- | --- |
| walk (`ignore` crate, `.gitignore` honored) | 211 ms |
| read all files | 358 ms |
| xxHash all files | 5 ms |
| parse Markdown | 56 s of CPU, summed across worker threads |
| wall clock, end to end | 4.05 s |
| peak memory | 680 MB |

Zed had seven `dmls` processes running, and each one pays this.

Only the features listed under
[Build the full index only when a feature needs it](#r7-build-the-full-index-only-when-a-feature-needs-it)
need every document's contents; the rest need
the file list, the open documents, and the documents those reference.

The log line is accurate but uninformative. `documents` is the Markdown file
count (it matches `rg --files --hidden -g '*.md' -g '*.markdown'` exactly), but
the line does not say so, YAML files (272 here) are never mentioned, and
nothing reports the schema files that shape every document.

The walk itself has two gaps:

- it uses `hidden(false)`, which with the `ignore` crate **descends into
  `.git/`** in a primary checkout (about 3,800 files in this repository's
  common git dir), none of which can ever match;
- the `ignore` crate applies `.gitignore` only inside a git repository
  (`require_git` defaults to `true`). A workspace that is not a repository but
  has a `.gitignore` excluding `node_modules/` walks `node_modules/` anyway.

### F6. Nothing reports where request time goes

DMLS is often slow to answer editor requests
(`2026-10-11-dmls-request-timeouts` records 120-second timeouts). No log line
names a slow request, its document, or how long it took, so each slowness
report starts from nothing.

### F7. `.dmls.toml` has no reference page

The `[workspace]` section (`root`, `include`, `exclude`) is described only by
its source comments in `dmls/src/config/mod.rs`. The README's Configuration
section points readers at the original feature spec for the key list. No page
under `darkmatter/docs/` documents the file's keys, their defaults, or what
`include` controls (both discovery and the file-watch globs).

## Review Decisions

The startup and caching changes are intentional behavior changes, beyond the
logging fix. The acceptance criteria are: unrelated malformed YAML cannot
disable triggers; repeated requests do not rediscover trigger roots; startup
does not parse unopened Markdown; and references, rename, and workspace
symbols retain complete answers.

**Reader's note — cache scope.** The original draft promised that serving any
request would never read a schema-root file. Existing schema assembly also
reads authored schemas, named types, and dependency fingerprints. Removing
all of those reads requires a broader schema snapshot design. This fix removes
trigger rediscovery and trigger-payload freshness reads from ordinary cached
lookups; it preserves existing freshness checks for other schema consumers.
It does not introduce a new eager named-schema validation policy.

**Reader's note — schema scope.** Schema roots come from each document's
resolution context, not a workspace-wide replacement search path. A workspace
folder can cover part of a repository, multiple repositories, or no repository.
Caching must preserve those distinctions and filename shadowing.

## How Startup Works After This Fix

```mermaid
flowchart TD
    S[server starts] --> R["1. schema registry<br/>classify immediate YAML children of applicable schema roots"]
    S --> O["open documents<br/>parsed as the editor opens them"]
    R --> L["2. file list<br/>Markdown + YAML paths, .gitignore honored, .git skipped"]
    L --> D["diagnostics for open documents<br/>(link checks need the file list)"]
    O --> D
    D --> T["3. referenced targets<br/>parsed on first need, cached by xxHash"]
    Q["references / rename / file rename / workspace symbols"] --> F["4. full index<br/>every Markdown file parsed, then kept current"]
```

| Step | Reads file contents? | When |
| --- | --- | --- |
| 1. Schema registry | immediate schema-root YAML and trigger payload dependencies | after initialization, before schema-dependent answers |
| 2. File list | no | at startup, on a background thread |
| Open documents | the open buffers | as the editor opens them, as today |
| 3. Referenced targets | each target, once | the first time a feature needs its headings or transclusions |
| 4. Full index | every Markdown file | the first references, heading/file rename, or workspace-symbols request |

The protocol handshake completes first. Registry preparation and file discovery
run off the message-dispatch thread, with registry preparation scheduled first.
The server continues accepting open/change, cancellation, and shutdown
messages. Requests wait only for the prerequisite they need; local formatting,
for example, does not wait for the workspace file list. Progress is sent only
when supported by the client, using the existing progress reporter and
properly created or client-supplied tokens.

## Requirements

### R1. Decide the claim before treating a parse failure as fatal (F1)

`parse_trigger_envelope_from_str` returns `Ok(None)` for source that is not
valid YAML **unless** the source claims the envelope. For unparseable source,
a claim is a line that:

- starts at column 0 with `kind`, then optional spaces, then `:`;
- has a value that is `trigger-schema`, written plain, single-quoted, or
  double-quoted;
- is optionally followed by whitespace and a `#` comment.

| Source (unparseable elsewhere) | Result |
| --- | --- |
| `kind: trigger-schema` | claimed → error (R2) |
| `kind: "trigger-schema"  # note` | claimed → error (R2) |
| `  kind: trigger-schema` (indented, nested) | not claimed → `Ok(None)` |
| no `kind:` line | not claimed → `Ok(None)` |
| `kind: schema` | not claimed → `Ok(None)` |

Parseable source keeps today's behavior exactly: the claim is read from the
parsed mapping, including quoted keys and JSON-style mappings.

The fallback above is deliberately limited to a conventional block-mapping
declaration after a full YAML parse fails. Normalize CRLF and an initial UTF-8
BOM before checking. Require an exact scalar value: suffixes such as
`trigger-schema-extra`, commented-out declarations, and nested mappings do
not claim the envelope. A matching line inside a multiline quoted scalar is
not a declaration; do not implement this as an unrestricted multiline regex.
Malformed flow mappings or nonconventional key spellings without a qualifying
declaration remain unclaimed. This limitation affects error recovery only,
not the valid YAML grammar; document it with the claim examples.

In the darkmatter library,
[`parse_trigger_envelope_from_str`](../../../lib/src/markdown/schemas/triggers/envelope.rs)
is the shared envelope classifier used by discovery, DMLS authoring, and
payload-cycle checks. All three callers get the fix, including embedded
trigger bundles.
No caller may add its own pre-classification.

After this change the `overlay/mod.rs` authoring-path comment ("ordinary
schema-root YAML remains silent") is accurate again; leave it in place.

### R2. A claimed envelope with invalid YAML gets its own error (F2)

Add a dedicated `SchemaError` variant for "claims `kind: trigger-schema` but is
not valid YAML". It carries the YAML error's message and, when the parser
reports one, its line and column. Filesystem discovery wraps it in
`TriggerLoad`; embedded bundles retain
`EmbeddedTriggerLoad`. Transactional semantics are unchanged.

- Its message names YAML syntax, not the match grammar. For example:
  ``trigger-schema file `…/x.yaml` is not valid YAML at line 17, column 93: mapping values are not allowed in this context``.
- Its help text points at YAML syntax (for example, quote a value that
  contains `: `), never at combinators or `$path`.
- DMLS publishes the diagnostic on the trigger file at the reported position
  when it has one, else at the start of the file as it does today. Convert
  the parser's one-based location through DMLS's source map into the client's
  negotiated position encoding; do not treat byte columns as UTF-16 columns.

`TriggerMatch` continues to cover match-grammar and envelope-shape errors
(such as a missing `match` key); splitting those existing cases is out of
scope. CLI error codes and help must handle the new YAML error explicitly.

### R3. Build the schema registry once, at startup (F3)

After initialization, DMLS prepares the union of schema roots applicable to
workspace documents. In the darkmatter library,
[`SchemaRoots::for_document`](../../../lib/src/markdown/schemas/roots.rs)
remains the authority for the five ordered roots: package, package area,
`base_dir()/schemas`, the snapshot's `SCHEMAS_DIR` folder, and the snapshot's
home `schemas` folder. Use the existing resolution contexts and their package
catalog; do not infer roots by walking ancestors or recapture the environment.

Enumerate only immediate regular `.yaml` and `.yml` children, matching
existing discovery. Schema discovery is independent of the workspace's
Markdown include/exclude and ignore filters. Roots outside the workspace,
including repository-level roots above a workspace subfolder, still apply.
Retain existing symlink rules, case-fold collision errors, filename-byte
ordering, root deduplication, and root-state reporting.

Each readable file is classified as one of:

- **named schema**: a standalone SimplifiedSchema document
  (`parse_standalone_schema_document`), available by name to `$schema` and
  trigger payloads in its root's scope. Classification does not eagerly
  resolve its imports or publish errors on unrelated consuming documents;
- **trigger schema**: claims `kind: trigger-schema` and loads (R1, R2);
- **unclassified**: neither, including a file that does not parse and does not
  claim the trigger envelope, or fails standalone-schema classification.
  Unclassified files are counted and do not
  fail trigger discovery. This does not suppress ordinary YAML/schema-authoring
  diagnostics, or errors when a document explicitly references that file.

A claimed trigger that fails loading is recorded as a trigger failure, not
`unclassified`. An I/O failure is also never an unclassified document.

A document's registry is the subset of this classification for that
document's five roots, with today's filename shadowing applied. It must equal,
trigger for trigger and in the same order, what the darkmatter library's
[`triggers::scan`](../../../lib/src/markdown/schemas/triggers/discovery.rs)
returns for that document. Preserve claimed filenames from non-trigger files:
they can shadow less-local or embedded triggers too. Resolve payloads and
`$path` predicates in the document's context. Cache parsed root contents
independently from document-bound registries; equivalent file sets alone do
not make two resolution contexts interchangeable. Test results, payloads,
shadowing, and error behavior against direct discovery.

**A ready, unchanged trigger-registry lookup performs no directory scan,
file read, hash, or payload resolution.** Other schema assembly retains its
existing dependency checks, as explained under Review Decisions. Registry
updates are prepared outside the request's provider chain and installed
atomically when ready.

The registry changes when DMLS learns that one of its inputs changed:

- `workspace/didChangeWatchedFiles` or `didSave` for a YAML file;
- a server-rescan pass (`ServerState::rescan_workspace`);
- a configuration/context generation change, or a changed package manifest
  that can add, remove, or change an applicable root.

A newly opened document outside the prepared contexts, including an untitled
buffer, uses the existing context rules. Prepare any additional applicable
roots before answering its schema-dependent requests; never silently substitute
the workspace folder's roots.

For each changed path, DMLS re-reads the file, compares its
`biscuit_hash` xxHash with the recorded one, and reclassifies only on a
difference. The registry also records the payload, import, and example files
its triggers read, wherever they live, with their xxHashes; a change to one of
those reloads the triggers that read it. Track deleted and missing
dependencies, failed-load inputs, and directories
searched for unresolved names too; otherwise creating a missing payload
cannot recover a failed registry. Creation, deletion, and root-membership
changes invalidate lookup results even without a comparable file hash.

Transactional loading is kept per ordered root set and resolution-context
generation. A malformed trigger affects only documents for which it is
unshadowed. Retain the compatible last-good registry on trigger-load errors;
if none exists, publish the failure without claiming that triggers are ready.
Clear the trigger-file diagnostic on recovery and refresh affected open
documents. A root metadata/enumeration I/O failure keeps the existing failed
schema outcome; it must not serve last-good data or fall through to a less
local root. An unrelated healthy scope can still become ready.

Dependency tracking is separate from Markdown-buffer ownership: a saved YAML
buffer must invalidate registry consumers even when it is open. The scan
still reads disk, not open buffers. Unsaved edits to a schema file
affect consuming documents only after save, as today.

### R4. Log trigger-registry state changes once (F4)

Here a scope means an ordered root set and compatible resolution context.
Retained failures do not log again on each lookup.

| Event | Level | Message |
| --- | --- | --- |
| Load fails; no failure recorded, or a different file or error text | `warn` | `retaining last-good trigger registry` with `scope`, `trigger_file`, `error`, `has_last_good` |
| Load fails with the same file and error text as recorded | `debug` | same message |
| Load succeeds after a recorded failure | `info` | `trigger registry recovered` with `scope`, `trigger_file` |

On a first load with no last-good registry, use `trigger registry unavailable`
instead of claiming to retain one. Root I/O failures use the existing failure
path and are deduplicated by the same transition rule.

The comparison reuses the one that already decides diagnostic transitions;
there is one source of truth for "this failure is new". With R3 a repeat is
rare (it needs an input change that still fails identically); R4 bounds the
log even if R3 regresses.

### R5. Hold a flat file list in memory (F5)

One walk per effective workspace root (`workspace.root` applied as today)
produces the in-memory file list:

- **Markdown**: files matching `workspace.include` (default `**/*.md`,
  `**/*.markdown`) and not `workspace.exclude`;
- **YAML**: `*.yaml` and `*.yml`, not `workspace.exclude`. `include` does not
  apply to YAML: its defaults are Markdown-only.

The walk:

- honors `.gitignore`, `.ignore`, `.git/info/exclude`, and the global excludes
  file, as today;
- sets `require_git(false)`, so `.gitignore` applies in a workspace that is not
  a git repository;
- keeps `hidden(false)`, because hidden directories such as `.claude/` hold
  real Markdown, but skips every directory named `.git`
  (`WalkBuilder::filter_entry`);
- does not follow symlinks and reports a second path to the same physical file
  as a duplicate under the current canonical-path identity policy.
  Symlink entries are skipped; do not promise hard-link identity detection;
- reads no file contents.

The list is kept current by watched-file events and server-rescan passes,
using the same include/exclude/ignore decisions as the initial walk. YAML
entries never become Markdown graph nodes. Changes to discovery configuration
or ignore files rebuild the list and update watch registration. A listing
error is logged as incomplete discovery, never silently presented as a
complete file list.

Workspace watch registration adds `**/*.yaml` and `**/*.yml`. Those globs
do not cover files outside workspace folders or dependencies in other formats.
Register watches rooted at the actual schema directories and dependency
locations when the client supports them, including creation/deletion and
missing-target recovery. If registration is unsupported or rejected, use the
existing server-rescan-on-save fallback for those inputs and document that
external edits may need a save or restart. This fix adds no periodic polling.
Watch configuration and ignore inputs as well as existing manifest/context
inputs. Empty Markdown include patterns must not disable schema watches.

In fallback mode, each save re-enumerates relevant schema roots and checks
known registry dependencies. For Markdown, it lists paths and checks already
parsed closed targets (or all entries once the full index exists). It does
not parse every unopened file. Unparsed files are read when first needed.
On-demand targets outside workspace folders receive the same stale checks.
Schema rescan inputs remain independent of workspace exclusions.

Wherever DMLS today asks "is this an **indexed** document?" (the first column
of the "Two ways to resolve" table in `darkmatter/docs/lsp/architecture.md`:
graph edge targets and anchor-completion candidates), it asks "is this path in
the file list?" instead. Whether a file exists is still decided as today.

Diagnostics for an open document that depend on whether a path exists
(broken links, missing `::file`/`::code`/`$schema` targets, wiki-link
resolution and ambiguity) are published only once the file list is ready, so
an early publish cannot report a link broken that is not. Publish independent
syntax/schema findings meanwhile, then refresh affected diagnostics when the
list is ready. A failed or incomplete listing must not produce definitive
missing-target results based solely on absence from that list.

Path completion (`completion.rs` link paths, `frontmatter.rs` `file(...)`
values, `wiki.rs` wiki-link files) reads the file list, not the parsed index.

### R6. Parse referenced targets on first need

A document that is not open is parsed only when a feature needs its contents:

| Feature | Needs from the target |
| --- | --- |
| missing-anchor diagnostic (`foo.md#heading`, `[[foo#heading]]`) | headings |
| heading completion after `#` | headings |
| link hover preview | headings |
| transclusion cycle detection | `::file`/`::code` targets, followed outward from the open document |

A parsed target is cached in memory, keyed by path, with the xxHash of the
bytes it was parsed from. A watched-file event or rescan marks
the entry stale while retaining its prior bytes hash and parse. The next need
re-reads it and reuses the parse when the xxHash is unchanged; deletion removes
it. Never reuse old headings after a failed read.

Open buffers, including unsaved files absent from the disk list, override disk
parses and hashes. On close, restore the disk view or remove an unsaved-only
entry. On-demand targets outside the workspace remain resolvable through
`FileReference` in the source document's context and receive dependency
invalidation; the file list must not restrict existing resolution behavior.

Parse target Markdown only when headings or outgoing transclusions are needed;
do not parse arbitrary `::code` bytes as Markdown. Traverse transclusions with
a visited set and cycle detection. Parsing stays passive: it never composes,
runs shell expressions, or fetches remote content. Concurrent requests for the
same target share one parse, and stale worker results cannot replace a newer
buffer version.

### R7. Build the full index only when a feature needs it

The following features need every Markdown document's contents. They build
the full index on first use:

| Feature | What it needs |
| --- | --- |
| find references (headings, links, `[[wiki]]` links) | backlinks |
| heading rename (`textDocument/rename`) and file rename (`workspace/willRenameFiles`) | backlinks |
| workspace symbols | every heading |

- The build parses every Markdown file in the file list, reusing any document
  already parsed under R6 or open in the editor, and reports
  `window/workDoneProgress` as today's startup index does.
- **Rename waits** for the build to finish before computing any edit; a
  partial index would leave links in unparsed files broken. References and
  workspace symbols also wait, so all three answer from a complete index.
- Once built, the full index is kept current file by file, as today's index
  is. Before it is built, a change updates the file list and marks cached
  targets stale under Parse referenced targets on first need.

The build has explicit not-started, building, ready, and failed states.
Concurrent callers share one background build; keep dispatch responsive to
edits, cancellation, and shutdown. Canceling one waiter cancels that request,
not a build another caller still needs. A failed or canceled build never
becomes ready, and a later request may retry. Rename preparation that only
checks the local rename target need not build backlinks.

Publish a complete snapshot only after reconciling file-list/configuration
changes and open-buffer versions received during the build. Do not hold the
workspace-index lock across I/O or parsing. If a listed Markdown file cannot
be read or parsed, report the build failure and return an error to waiting
requests rather than silently issuing partial rename edits or incomplete
results. Reuse the same passive document parser for lazy and full builds.

The persistent store in `2026-10-17-dmls-index-store` replaces this in-memory
build. Its interface stays behind the same feature entry points, so that feature
changes where the data lives, not who asks for it.

### R8. Startup and index log lines (F5)

| When | Level | Line |
| --- | --- | --- |
| R3 finishes | `info` | `schema registry ready roots=… tree_schemas=… scoped_schemas=… trigger_schemas=… unclassified=… elapsed_ms=…` |
| R5 finishes | `info` | `workspace files listed markdown=5676 yaml=272 duplicates=0 elapsed_ms=…` |
| a rescan finishes | `info` | `workspace rescan complete` with the R5 fields |
| R7 finishes | `info` | `full index built markdown=… headings=… links=… trigger=references elapsed_ms=…` |

Field meanings:

- `roots`: distinct searched schema directories across prepared contexts,
  not a fixed five; duplicate physical roots count once.
- `workspace`: the workspace identifier on each workspace log line, so multiple
  folders can be distinguished.
- `failed_scopes`: count alongside registry totals; when nonzero, use
  `schema registry initialized with failures` rather than `ready`.
- `tree_schemas`: named schemas in roots serving a tree, environment, or
  home role in a prepared context; a tree need not span the whole workspace.
- `scoped_schemas`: named schemas in package and package-area roots.
- `trigger_schemas`: trigger schemas in any root.
- `unclassified`: schema-root YAML files that are neither.
- `trigger`: `references`, `rename`, `file-rename`, or `workspace-symbols`, whichever request
  caused the build.

Schema counts are distinct files across searched directories, before
per-document shadowing. Add `failed_triggers` for claimed files that fail to
load, so every enumerated file is accounted for. A directory shared by scoped
and tree roles counts once, as tree-wide within that workspace; describe that
counting rule in the log reference. Markdown/YAML counts describe accepted,
deduplicated file-list entries, not all files on disk. Registry elapsed time
includes trigger payload loading; full-index elapsed time is wall time
separate from any summed worker-stage times. The old
`startup index complete documents=…` line is removed. Each duplicate file is
still logged on its own line at `warn`.

### R9. Log slow requests (F6)

DMLS measures each inbound request from dequeue at the dispatch boundary to
sending its response, including prerequisite waits. Notifications have no
response: measure from dequeue through handler completion, with a separate
`slow notification` / `notification complete` event. Async diagnostics or
background indexing have their own timings; do not pretend they finished with
the notification handler. Use a monotonic clock.

- At or above **500 ms**: one `warn` line,
  `slow request method=textDocument/hover uri=… elapsed_ms=…`, plus the time
  spent in each stage DMLS measures for that request (at least: overlay build,
  schema assembly, trigger registry, and the provider chain) and whether it
  waited for the file list or full index. Stage times are elapsed times,
  absent if that stage did not run. Nested stages are not additive; document
  their nesting rather than presenting them as a sum of total latency.
- Below the threshold: `request complete` at `debug` with the same fields.
  Requests without a document (for example, workspace symbols) omit `uri`.
  Include request ID and outcome to distinguish overlapping requests.
- A canceled request logs its elapsed time and `cancelled=true`. Every
  terminal path, including provider errors and caught panics, emits exactly
  one completion event at the level selected by the threshold.
- Never log source text, frontmatter values, or full protocol payloads.
  Existing stderr/`--log-file` routing and log filtering remain unchanged;
  logs must never enter the stdout protocol stream.

The threshold is a constant in this fix. A setting can be added later.

## Configuration

This fix adds no setting. A setting to narrow which files are in scope is
expected later; until then `workspace.include` and `workspace.exclude` are the
controls for workspace discovery; schema roots remain context-defined.
The new configuration page (see
[Documentation](#documentation)) explains that distinction.

## Tests

| Requirement | Tier | Test |
| --- | --- | --- |
| R1 | L1 lib | envelope unit tests for each row of the R1 table |
| R1 | L1 lib | discovery: a root with a valid trigger and an unparseable, unclaimed YAML file loads the trigger |
| R1 | L1 lib | the incident fixture (an unquoted `findings:` value with `): ` at column 93, no `kind:`) is not a trigger |
| R1 | L1 CLI | end-to-end: in a `git init` fixture repository with that schema-root file, `md compose doc.md` succeeds and `md schema triggers doc.md` exits 0 |
| R1 | L1 lib | corpus: every shipped `schemas/*.yaml` in the repository classifies without error (extend the existing corpus test if one covers schema roots) |
| R2 | L1 lib | a claimed, unparseable envelope yields the new variant with line and column; its message and help name YAML, not the match grammar |
| R2 | L1 DMLS | the authoring diagnostic lands at the reported line and column |
| R1 | L1 DMLS | an unparseable, unclaimed schema-root YAML file gets no trigger diagnostic |
| R3 | L1 DMLS | for fixture documents in a package, a package area, and the tree root (with `SCHEMAS_DIR` and a fixture home set), the per-document registry equals `triggers::scan` |
| R3 | L1 DMLS | repeated ready trigger-registry lookups do no root enumeration, file reads, hashing, or payload resolution (read-counting seam); other schema assembly retains its existing freshness behavior |
| R3 | L1 DMLS | a watched-file event for an envelope, a payload outside every root, or a new file in a root updates the registry; an event whose file has an unchanged xxHash does not reclassify |
| R3 | L1 DMLS | a malformed trigger keeps compatible last-good data only in scopes where the file is unshadowed; a cold failure reports unavailable |
| R4 | L1 DMLS | with a captured `tracing` subscriber: repeated identical failures produce one `warn`; a different error a second; recovery one `info` |
| R5 | L1 DMLS | a fixture with Markdown, YAML, a gitignored `node_modules/`, a symlink, and a `.git/` directory containing `.md` files yields the expected file list and counts |
| R5 | L1 DMLS | a `.gitignore` in a non-repository root excludes `node_modules/`; a linked-worktree fixture (`.git` as a file) still honors `.gitignore` |
| R5 | L1 DMLS | startup with a large fixture parses only the open documents (a parse-counting seam) |
| R5 | L1 DMLS | a broken-link diagnostic is not published for an open document before the file list is ready |
| R6 | L1 DMLS | a missing-anchor check parses exactly the target; a second check reuses it; a changed target is re-read |
| R7 | L1 DMLS | the first references, rename, and workspace-symbols requests each build the full index once; rename's edits include a referrer that was never open or referenced |
| R8 | L1 DMLS | with a captured subscriber, the R8 lines and fields are emitted for a fixture with known counts |
| R9 | L1 DMLS | an injected clock covers below, exactly at, and above 500 ms; errors and cancellation emit one completion event; notifications have no response timing; URI-less requests omit uri |
| R1 | L1 lib | valid quoted-key and JSON envelopes retain behavior; malformed fallback covers CRLF/BOM, suffix/comment/nested nonclaims, and multiline-scalar false positives; embedded wrappers remain correct |
| R3 | L1 DMLS | root I/O errors fail closed; an unclassified file still shadows a later trigger; scoped contexts retain different payload/pattern resolution |
| R3, R5 | L1 DMLS | external home/environment roots, non-YAML examples, missing payload creation, deletion, and rejected watch registration recover through the documented event/fallback paths |
| R3 | L1 DMLS | saving an open YAML buffer invalidates consumers; unsaved changes do not; recovery clears old diagnostics and refreshes affected documents |
| R5, R6 | L1 DMLS | fallback save and configuration/ignore changes preserve lazy parsing; unchanged target bytes reuse their parse; open buffers override disk and close restores disk |
| R7 | L1 DMLS | simultaneous callers share one build; cancellation, unreadable inputs, edits/deletes during build, and workspace file rename cannot produce a partial or stale ready snapshot |

Use deterministic read/parse counters and captured logs for the acceptance
checks above. Prefer an injected clock for the 500 ms boundary and a
read-counting seam over changing filesystem permissions, which is unreliable
across operating systems. Tests run through the area's `just test` recipes
(nextest); protocol tests use an in-process connection or a headless child.

The existing measurements already establish that Markdown parsing dominates
startup, so no new performance spike is required. Keep
`dmls --bench-index` as the full-index benchmark; it must not silently become
a file-list-only benchmark. If a quick same-host sample is recorded during
implementation, label startup/listing and full-index measurements separately.
Idle-memory improvement is an expectation, not a new numeric acceptance gate.

## Documentation

**New canonical pages** under `darkmatter/docs/lsp/`, written for a developer
new to this repository (an example per rule, a Mermaid diagram where a flow
is clearer):

- `configuration.md`: every `.dmls.toml` key and its default, the precedence
  of `.dmls.toml`, editor settings, and defaults, and a `[workspace]` section
  covering `root`, `include` (Markdown discovery and document watch globs,
  independent of mandatory schema/dependency watches), and
  `exclude` (Markdown and YAML). Absorb the configuration content now in
  `topics/schemas/dmls-schema-support.md` and link to it from there.
- `indexing.md`: the startup order and diagram above, the schema registry and
  its scopes, the file list and its ignore rules, on-demand parsing, the
  features that require the full index, and the registry-transition, startup,
  and slow-request log lines.

**Sweep.** Read relevant pages under `darkmatter/docs/`,
`darkmatter/dmls/docs/`, and `darkmatter/dmls/README.md`, and correct any
statement this fix makes untrue.
Pages under `docs/research/` record research; correct them only where they
state current DMLS behavior. Pages known to need changes:

- `lsp/architecture.md`: the "Two ways to resolve" table and the paragraph
  below it ("indexed document" becomes "in the file list"; anchors on
  unopened targets are now checked by parsing the target), the invalidation
  table, and the watch-glob paragraph (YAML is now watched).
- `topics/dmls.md`: the rescan behavior.
- `topics/schemas/definition.md`: the R1 claim rule with an example, and the
  R2 error.
- `topics/schemas/dmls-schema-support.md` and `topics/schemas/schema-activation.md`:
  registry timing and scope.
- `lsp/how-to-use.md` and `lsp/features.md`: any startup or indexing claim.
- `darkmatter/dmls/README.md`: the Configuration section links to
  `docs/lsp/configuration.md` instead of the feature spec; add one line on the
  log output with a link to `docs/lsp/indexing.md`.

Also update `darkmatter/dmls/src/workspace/discover.rs` module docs (R5) and
`.claude/skills/darkmatter/` (`dmls.md`, `schema.md`) where they describe
startup indexing, trigger discovery, or the startup log.

## Out of Scope

- The persistent backlink and symbol store: `2026-10-17-dmls-index-store`.
- A setting to narrow the files in scope.
- One `dmls` process held 2.2 GB about 3½ minutes after startup on
  2026-10-17, against about 330 MB for its siblings. Record it with
  `2026-10-11-dmls-request-timeouts` if it recurs; R9's logs should help.
- General YAML content indexing outside schema roots; reading and tracking
  referenced schema payloads, imports, and examples remains required.

## Open Questions

None blocking. The existing `darkmatter/dmls/docs/` pages are included in
the behavior-correction sweep. Moving or consolidating that tree is a
separate documentation change; preserve its current links in this fix.
