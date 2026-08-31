---
hash: ef46db3751d8e999-1f17b774a5a1ed90
last_updated: 2026-08-01
---
# Compose Pipeline

## Contents

- Pipeline Overview
- API
- Demand-Driven Runtime Context Evidence
- Text Replacement
- Interpolation
- ComposeReport
- Pre-Flight Shell Approval
- Shell Command Caching
- Error Handling
- Transclusion
- Module Structure

Use heading search to jump to the listed subsystem.


The darkmatter compose pipeline provides document preparation through four phases:
Inline Pre, Transclusion, Inline Post, and Finalization.

## Pipeline Overview

**Inline Pre** (serial):

1. **Frontmatter Interpolation (pass 1)** - `{{ variable }}` in frontmatter resolves before effective state is built; keys referencing a whole-value `$(...)` are deferred to pass 2
2. **Schema Validation** - Validate frontmatter against `$schema` or `ComposeOptions::baseline_schema`. Runs after `--set` / `--state` overrides and frontmatter interpolation, but before shell expansion. **Coerces** schema-recognized top-level scalars to their declared types (default-on, e.g. the string `"true"` → real boolean) and writes the coerced values back into frontmatter, skipping `$(...)`-pending values. Problems on fields still holding `$(...)` are deferred to downstream re-validation only when frontmatter shell expansion is enabled; when it is disabled they fail fast
3. **Frontmatter Shell Expansion** - top-level `$(cmd)` frontmatter values execute after interpolation and write trimmed `stdout` back into frontmatter. Tokens in executed position follow the `$()` token-resolution ladder (literal → `name(...)` safe function → executable → frontmatter property → null); an all-expression `$()` is rejected with a `{{ }}` suggestion
4. **Frontmatter Interpolation (pass 2)** - resolves the keys deferred in pass 1 against the now-concrete shell-expanded values
5. **Text Replacement** - `replace:` frontmatter replaces literal strings
6. **Page Blocks** - `::block`/`::end-block` conditional regions
7. **Interpolation** - `{{ variable }}` expressions expand to values
8. **Shell Expansion** - Execute `::shell` directives execute approved commands and inject combined `stdout` + `stderr`
9. **Link Resolve** - Resolve all local link targets (Markdown hyperlinks/images and supported HTML embeds) to absolute paths

Schema-selected `file` caller overrides are materialized during this stage.
Lazy local values become the first unprobed absolute candidate; eager values
become the first existing regular file. `ComposeOptions` retains the raw
override and accepts per-property file-reference origins so independently
authored layers—CLI setters, proxy inputs, and sequence task parameters—can be
folded without re-anchoring. `materialize_caller_overrides` exposes that same
schema stage for a provenance-preserving handoff; it does not run the rest of
the compose pipeline.

**Transclusion** (prepared serially, resolved concurrently via Rayon):


- `::file ./doc.md` - Include markdown with recursive processing
- `::code ./main.rs` - Include as fenced code block
- `::toc-linking` - Generate heading link lists from external documents' raw source headings
- `::file-links` - Discover document files and render as a linked file tree
- `prologue` / `epilogue` - Frontmatter-driven file includes
- `when="..."` conditions, cycle detection, depth limits
- Heading re-leveling for included markdown (H6 overflow handled gracefully)

**Inline Post** (serial):

- **Cleanup** - Normalizes markdown formatting. It strips incidental single
  newlines from top-level and list-item prose by default, removing source-only
  list continuation indentation before applying list/indent cleanup. It can
  reflow complete logical prose blocks with
  `ComposeOptions::with_fixed_width(...)`; newly wrapped list continuations
  retain their complete list and blockquote container prefix. Use
  `ComposeOptions::with_incidental_newline_mode(IncidentalNewlineMode::Preserve)`
  to keep source single newlines.
- **Normalization** - Adjusts heading levels

**Finalization** (root-only serial):

- **Link Normalization** - Converts absolute path links back into portable forms:
    - **Same-repo**: Paths inside the same git repository are made relative to the document
    - **Home-dir**: Paths under the user's home directory use the `~/` prefix
    - **ENV-var**: Paths under whitelisted environment variables (e.g. `PROJECT_ROOT`) use `${VAR}/` prefix

## API

```rust
use darkmatter::markdown::{Markdown, compose::{ComposeOptions, ComposeOperation}};

// Compose with all operations enabled (default)
let (composed, report) = md.compose()?;

// Only run specific operations
let options = ComposeOptions::new()
    .only(&[ComposeOperation::Interpolation])
    .with_external_state(json!({"key": "value"}))
    .with_fail_fast(true);
let (composed, report) = md.compose_with(options)?;

// Disable specific operations
let options = ComposeOptions::new()
    .disable(ComposeOperation::Cleanup)
    .disable(ComposeOperation::Normalization);

// With a baseline schema (library-only; no CLI flag)
let baseline: darkmatter::markdown::schemas::SimplifiedSchema = /* ... */;
let options = ComposeOptions::new()
    .with_baseline_schema(baseline);

// Cleanup options
let options = ComposeOptions::new()
    .with_incidental_newline_mode(
        darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
    )
    .with_fixed_width(80);

// In-place mutation (no clone)
let report = md.compose_mut()?;

// Full pipeline with transclusion (requires source file path)
let md = Markdown::try_from(std::path::Path::new("docs/root.md"))?;
let options = ComposeOptions::new()
    .with_source_file("docs/root.md");
let (composed, report) = md.compose_with(options)?;
println!("{}", report.summary());
```

## Demand-Driven Runtime Context Evidence

Darkmatter exposes the capture requirements separately from population:

```rust
use darkmatter::markdown::compose::{
    ComposeContext, ContextCaptureEvidence, ContextGroup, ContextRequirements,
};

let requirements = ContextRequirements::for_document(&markdown);
if requirements.contains(ContextGroup::Git) {
    // Ask the invocation owner for its retained Git evidence.
}
let evidence = ContextCaptureEvidence::new(invocation_environment)
    .with_git(git_info)
    .with_repository(repository_root, repo_info)
    .with_file_changes(file_changes);
let context = ComposeContext::capture_with_evidence(
    source_base_dir,
    &requirements,
    &evidence,
);
```

`ContextRequirements::for_content` scans active `ctx.*` references in one
content fragment. `for_document` scans both authored frontmatter values and the
body, preserving interpolation-literal masking and date/time aliases. `all`,
`contains`, and `iter` support explicit orchestration without exposing the
population modules. Date/time is always present in a requirements set.

`ContextCaptureEvidence` carries the invocation environment plus optional
Sniff-owned `GitInfo`, file changes, `RepoInfo`, `LanguageBreakdown`, Markdown
metadata, `OsInfo`, `HardwareInfo`, and GPU observations. Its builders
distinguish **not supplied** from an explicitly observed absence (`None` or an
empty vector). `with_documents_for_source` derives the canonical best matching
skill from a supplied repository root/topology and source directory without
performing Git or topology discovery.

Supplied capture is fail-closed. If a requested fact was not supplied,
Darkmatter emits the existing `PartialRuntimeCapture` diagnostic and populates
the group's empty/null projection. It never fills the gap by reading ambient
CWD, HOME, environment, Git, repository topology, OS, hardware, or GPU state.
The injected environment is used for both `env.*` and the `ctx.agent` /
`ctx.model` projection, so later process-environment mutation cannot change the
capture.

The supplied entry points are:

- `ComposeContext::capture_with_evidence`
- `ComposeContext::capture_for_content_with_evidence`
- `ComposeContext::capture_for_document_with_evidence`

Existing `ComposeContext::capture_for_content`,
`ComposeContext::capture_for_document`, and `ComposeOptions::new()` remain
ambient compatibility APIs and use the same `populate_*` code. Ambient capture
snapshots the environment once and reuses its original `GitRepo` handle for
file changes rather than discovering the repository a second time.

## Text Replacement

The `replace:` frontmatter key enables literal string replacement.

```yaml
---
replace:
  PLACEHOLDER: actual value
  VERSION: "2.0"
---
This PLACEHOLDER will become "actual value".
Version: VERSION
```

### Replacement Rules

- Keys must be literal strings (case-sensitive)
- Overlap: longest key wins, then lexicographic order
- Values: scalars only (strings, numbers, booleans, null)
- `null` → empty string
- Non-map `replace` silently skipped
- Single-pass (replacements not re-scanned)

## Interpolation

Expressions between `{{ }}` are evaluated and replaced with values. To render `{{ ... }}` literally instead of evaluating it, use the interpolation-literal syntax `{{{ ... }}}`; the content is never evaluated and composes down to `{{ ... }}`. See `darkmatter/docs/inline/interpolation.md`.

### Variable Resolution

| Pattern | Description |
|---------|-------------|
| `{{ foo }}` | Frontmatter value |
| `{{ user.name }}` | Nested object path |
| `{{ doc }}` | The whole frontmatter object |
| `{{ doc.build }}` | A frontmatter property by the `doc.*` namespace (a property literally named `doc` is `doc.doc`); intercepted before any `ctx.*` fallback |
| `{{ ctx.today }}` | Runtime context |
| `{{ env.HOME }}` | Environment variable |
| `{{ file_exists(path) }}` | Read-side function (also `frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema`, `absolute`, `relative`); resolves on every surface, both interpolation passes included |

Read-side functions and `doc.*` resolve identically on every surface
(frontmatter both passes, body, `when=`, `$()` ternary condition/branches,
claudine loop/hook). Every frontmatter surface (both interpolation passes and
the `$()` ternary condition/branch) is local-only, so a remote URL argument
fails loudly there; only body interpolation carries a remote runtime. See
`darkmatter/docs/topics/darkmatter-expressions.md`.

### Context Values (`ctx.*`)

Context is captured once per compose run and reused across the full document
graph. Capture is **demand-driven**: only groups whose variables are actually
referenced in the document are captured, and within a captured group all
properties are computed.

| Key | Description |
|-----|-------------|
| `ctx.now` | ISO 8601 local datetime |
| `ctx.now_utc` | ISO 8601 UTC datetime |
| `ctx.utc` | Alias for `now_utc` (backward compat) |
| `ctx.today` | Local date (YYYY-MM-DD) |
| `ctx.yesterday` | Yesterday's date |
| `ctx.tomorrow` | Tomorrow's date |
| `ctx.day` | Day of week (Monday, etc.) |
| `ctx.dow` | Alias for `day` (backward compat) |
| `ctx.day_abbr` | Abbreviated day (Mon, etc.) |
| `ctx.year` | Current year |
| `ctx.month` | Month number (01-12) |
| `ctx.month_name` | Month name (January, etc.) |
| `ctx.month_name_abbr` | Abbreviated (Jan, etc.) |
| `ctx.day_of_month` | Numeric day of month |
| `ctx.day_of_month_suffixed` | Day with ordinal suffix (1st, 2nd, etc.) |
| `ctx.time` | Time in hh:mm AM/PM format |
| `ctx.time_military` | 24-hour time |
| `ctx.timezone` | Timezone abbreviation (e.g., PDT) |
| `ctx.timezone_offset` | UTC offset (e.g., -0700) |
| `ctx.timezone_iana` | IANA timezone (e.g., America/Los_Angeles) |
| `ctx.season` | Meteorological season (Spring, Summer, Fall, Winter) |
| `ctx.timestamp` | EPOCH timestamp in seconds |
| `ctx.timestamp_ms` | EPOCH timestamp in milliseconds |
| `ctx.repo` | Repository name; null if not in a git repo |
| `ctx.repo_root` | Absolute path to repo root; null if not in a git repo |
| `ctx.is_monorepo` | Whether the repo is a monorepo |
| `ctx.package_root` | Absolute path to current package root; null if not in a package |
| `ctx.current_package` | Current package name; null if not in a monorepo package |
| `ctx.current_package_area` | Current package area; null if not in a monorepo area |
| `ctx.area` | Scope name (package or area); empty string at root |
| `ctx.area_description` | Human-readable scope description |
| `ctx.current_packages` | `string[]` of packages under CWD (`name (relative)`) |
| `ctx.depends_on` | `object[]` of internal dependencies (`{ package, dependencies }`) |
| `ctx.used_by` | `object[]` of internal dependents (`{ package, users }`) |
| `ctx.packages` | `string[]` of package names |
| `ctx.package_areas` | `string[]` of package area names |
| `ctx.dirty_files` | `string[]` of dirty file paths |
| `ctx.staged_files` | `string[]` of staged file paths |
| `ctx.untracked_files` | `string[]` of untracked file paths |
| `ctx.dirty_packages` | `string[]` of dirty package names |
| `ctx.staged_packages` | `string[]` of staged package names |
| `ctx.current_package_has_dirty_files` | Whether current package has dirty files |
| `ctx.current_package_has_staged_files` | Whether current package has staged files |
| `ctx.programming_languages_in_repo` | `string[]` of unique languages; null if not in a repo |
| `ctx.programming_language` | Context-sensitive primary language |
| `ctx.package_manager` | Context-sensitive package manager |
| `ctx.docs_readme` | `string[]` of README paths, scope-filtered |
| `ctx.docs_blast_radius` | `string[]` of docs with blast_radius frontmatter |
| `ctx.docs_drift` | `string[]` of docs at risk of drift |
| `ctx.docs_skill` | Repo-relative path to best matching SKILL.md; null if none |
| `ctx.os` | "Windows", "macOS", or "Linux"; null for other |
| `ctx.os_distro` | Linux distribution name; empty on macOS/Windows |
| `ctx.os_package_manager` | Primary system package manager |
| `ctx.os_version` | Operating system version |
| `ctx.memory_total` | Total system memory in bytes |
| `ctx.memory_used` | Percentage of memory currently used |
| `ctx.memory_avail` | Available memory in bytes |
| `ctx.cpu_cores` | Number of logical CPU cores |
| `ctx.cpu_arch` | CPU architecture (e.g., aarch64, x86_64) |
| `ctx.gpu` | GPU device name(s), comma-separated; null if none |
| `ctx.agent` | Executing agentic CLI name (from `AGENT` env var); defaults to `"unknown"` |
| `ctx.model` | Active model identifier (from `MODEL` env var); defaults to `"default"` |

All date/time variables have `_utc` variants (e.g., `today_utc`, `day_utc`,
`year_utc`). Week boundary variables are also available:
`start_of_week_sun`, `end_of_week_sun`, `start_of_week_mon`, `end_of_week_mon`
(plus UTC variants).

List-valued variables (`string[]` / `object[]`) are real arrays. A bare
`{{ ctx.foo }}` renders an array **line-separated** (one element per line). For
other shapes use the list-formatting functions: `as_csv`, `as_tsv`,
`as_space_separated`, `as_line_separated`, `as_unordered_list`, and
`as_ordered_list` (the Markdown-list renderers auto-nest nested arrays and the
`depends_on` / `used_by` object shape). The former `_list` twin variables (e.g.
`ctx.dirty_files_list`) are removed — use `{{ as_unordered_list(ctx.dirty_files) }}`.

Full specification lives in `darkmatter/docs/topics/context-variables.md`.

### Fallback Expressions

```handlebars
{{ color || "unknown" }}
{{ primary || secondary || "default" }}
```

Uses first truthy value, or the fallback.

### Ternary Expressions

```handlebars
{{ active ? "enabled" : "disabled" }}
{{ count > 0 ? "has items" : "empty" }}
```

### Comparison Operators

- `==` - equality
- `!=` - inequality
- `>` - greater than
- `>=` - greater than or equal
- `<` - less than

Numeric strings auto-convert for comparisons.

### Helper Functions

```handlebars
{{ length(name) }}           // String length
{{ length(items) }}          // Array length
{{ length(data) }}           // Object key count

{{ number("42") }}           // Parse string to number
{{ number(x, -1) }}          // With default on failure

{{ round(3.7) }}             // Round to integer (4)
{{ round(value, 0) }}        // With default

{{ link(doc.path) }}         // Markdown link using relative text and absolute destination
{{ link("https://example.com", "Example") }}  // Link with explicit description
{{ has_skill("rust") }}      // true when a skill directory exists in user or local roots
{{ has_local_skill("rust") }} // true when a skill directory exists in local roots only
```

### Code Region Protection

Inline code spans (single backticks) ARE interpolated — the common
templating pattern `` `var_{{ phase }}` `` works without any opt-in.

Fenced and indented code blocks are skipped by default to preserve
literal code samples. Set `interpolate_code_blocks: true` (frontmatter)
or call `ComposeOptions::with_interpolate_code_blocks(true)` to opt
fenced blocks back into the scan.

```markdown
Inline: `{{ evaluated }}`             # always interpolated

```
{{ not_evaluated_by_default }}        # skipped unless opted in
```
```

## ComposeReport

```rust
pub struct ComposeReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub toc_links_generated: usize,
    pub shell_expansions_applied: usize,
    pub shell_approvals_used: usize,
    pub page_blocks_rendered: usize,
    pub page_blocks_skipped: usize,
    pub transclusions_applied: usize,
    pub transclusions_skipped: usize,
    pub link_resolves_applied: usize,
    pub link_normalizations_applied: usize,
    pub max_transclusion_depth: usize,
    pub cleanup_changed: bool,
    pub normalization_report: Option<NormalizationReport>,
    pub warnings: Vec<ComposeWarning>,
}

// Check for changes
if report.has_changes() {
    println!("{}", report.summary());
}
```

## Pre-Flight Shell Approval

Shell approval and shell execution are separate concerns:

- **Approval is condition-blind.** `Markdown::compose_preflight(&options)`
  returns a `ComposePreflightReport`; its `approval_set()` is every command that
  *could* run under any state — both `$(...)` ternary branches, `when=`-false
  `::block` regions, and false-condition transclusions all contribute. Collection
  never evaluates conditions, never runs transclusion's merge, and never executes
  anything.
- **Execution is condition-aware.** The inline shell stages run only the
  commands whose branch is reached, gated by
  `ComposeOptions::with_pre_approved_commands(set)`. The invariant
  `execution_set ⊆ approval_set` makes the gate a pure membership check.
- A body/`::shell-block` command embedding a frontmatter value still pending
  frontmatter-shell expansion is rejected up front as
  `ShellExpansionError::DynamicCommandShape` (never a late `NotPreApproved`).

### Interactive approval: the stage policy snapshot

When no pre-approved set is supplied and an `approval_handler` drives approval
interactively, policy is snapshotted **once per stage**, not per directive. Each
of the three shell stages (frontmatter `$()`, body `::shell`, `::shell-block`)
takes one `ShellRuntimeSnapshot` of the whitelist/blacklist/allow-once state at
stage open, and every directive that stage admits is judged against it. This
keeps the policy mutex out of parsing, approval, and execution.

Two consequences:

- A rule **persisted** by an `AllowExactPersist` / `AllowCommandPersist`
  decision is written to the runtime immediately but becomes *policy input*
  only for a **subsequent** stage (or run). A later directive in the same stage
  matching that fresh rule therefore prompts again. This is conservative by
  construction — it can over-prompt, never under-authorize.
- **Allow-once is exempt.** It is arbitrated live against shared runtime state,
  so one approval covers repeats of that exact command for the rest of the
  stage and across concurrently composed sibling transclusions.

Orchestrators (Claudine) call `compose_preflight`, merge in their own harness
commands, authorize the union once, and pass the merged set back via
`with_pre_approved_commands`. `md compose --shell` reports the condition-blind
candidates. The lower-level `collect_shell_commands(&md, &options)` returns the
raw `ShellCommandEntry` list. See
`docs/inline/preflight-checks.md`.

## Shell Command Caching

Identical commands (same normalized command string) execute **once per compose
run** by default; the memoized `stdout`/`stderr` is reused at every other call
site, including across recursive transclusion (the cache lives in the shared
`ShellExpansionRuntime`, not in `cache::RunLocalCache`). Opt out per directive to
get a full cache bypass (fresh execution at each occurrence) using each family's
own spelling:

- Body `::shell --no-cache <cmd>`
- Frontmatter `$(<cmd>)::no-cache` (combines with `::timeout:N` in either order)
- `::shell-block no_cache=true` (the flag form `--no-cache` stays a parse error)

A repeated command whose executable is on the built-in volatile allowlist
(`uuidgen`, `date`, `openssl`) emits a one-time discoverability warning
suggesting `--no-cache`. See
`docs/inline/shell-expansion.md`.

## Error Handling

With `fail_fast: false` (default):
- Parse errors leave original `{{ expression }}` in place
- Evaluation errors leave original in place
- TOC-linking and non-structural transclusion failures are downgraded to warnings
- Structural transclusion errors (cycles, max depth) still return immediately
- Warnings recorded in report

With `fail_fast: true`:
- Interpolation parse/evaluation errors return immediately
- TOC-linking and other non-structural transclusion failures return immediately
- Structural transclusion errors still return immediately

## Transclusion

The transclusion phase runs after Inline Pre when a source file path is provided. It resolves file-based includes.

### Block Directives

```markdown
<!-- Include another markdown file (recursive) -->
::file ./chapter.md

<!-- Include as fenced code block -->
::code ./main.rs

<!-- Discover document files and render as linked tree -->
::file-links "docs/**/*.md"
::file-links --dir reports --depth 1

<!-- Conditional include -->
::file ./appendix.md when="include_appendix"
```

### Frontmatter Directives

```yaml
---
prologue: ./header.md
epilogue: ./footer.md
---
```

- `prologue` content is prepended before the document body
- `epilogue` content is appended after the document body

### Safety Features

- **Cycle detection**: Prevents infinite recursion from ancestry repetition while allowing shared DAG dependencies
- **Max depth limits**: Configurable depth for nested transclusion
- **Heading re-leveling**: Included markdown headings are adjusted to fit the nesting context (H6 overflow handled gracefully)
- **TOC linking source model**: `::toc-linking` reads headings from the referenced file's raw source, not its recursively composed output

## Module Structure

```
darkmatter/lib/src/
├── effects/              # Side-effect engine (EffectEngine, verbs)
│   ├── mod.rs
│   ├── error.rs
│   └── verbs.rs
└── markdown/compose/
    ├── mod.rs           # Public API facade (compose/compose_with/compose_mut) + re-exports
    ├── util.rs          # Shared non-stage helpers (git-root, path abbrev, target range, fm prep)
    ├── pipeline/        # Driver spine + operation registry
    │   ├── mod.rs       # run_compose_pipeline* driver
    │   ├── phases.rs    # Inline-Pre/Transclusion/Inline-Post/Finalization dispatch
    │   └── operations.rs # ComposeOperation, ComposePhase, descriptor table, default_order
    ├── schema_validation.rs # Always-on schema validation stage
    ├── preflight/       # Shell approval-set lifecycle (condition-blind)
    │   ├── mod.rs       # ComposePreflightReport + Markdown::compose_preflight
    │   ├── collect.rs   # Condition-blind graph walk → approval candidates
    │   └── approval.rs  # Deduped normalized approval-set boundary export
    ├── inline/          # Inline stage runners (free fns over &mut Markdown)
    │   ├── replacement.rs    # run_stage → replacement engine
    │   ├── interpolation.rs  # run_stage → {{ }} body interpolation
    │   ├── page_blocks.rs    # run_stage → conditional ::block regions
    │   ├── shell_expansion.rs # run_stage → condition-aware ::shell execution
    │   └── normalize.rs      # run_stage → heading/structure normalization
    ├── replacement.rs   # Text replacement engine
    ├── link_resolve.rs  # Link resolution (absolute paths)
    ├── link_normalization.rs # Link normalization (portable paths)
    ├── remote.rs        # Remote URL discovery, catalog, RemoteReadConfig
    ├── conditions.rs    # Condition evaluation API
    ├── cache/           # Compose result caching
    │   └── hashing.rs   # Context-aware cache hashing
    ├── context/         # Shared pipeline state + runtime context capture
    │   ├── mod.rs
    │   ├── options.rs   # ComposeOptions, ComposeSource, TransclusionOptions
    │   ├── runtime.rs   # ComposeContext (the ctx namespace)
    │   ├── report.rs    # ComposeReport, ComposeWarning, SourceRange
    │   ├── effective_state.rs # EffectiveState, builder, merge logic
    │   ├── capture/     # Requirements, ambient/supplied snapshots, and group populators
    │   ├── format.rs    # CSV, markdown list, byte, ordinal formatters
    │   ├── merge.rs     # User ctx + runtime ctx merge policy
    │   └── diagnostics.rs # ContextMergeDiagnostic types
    ├── expression/       # Expression language (shared by interpolation & conditions)
    │   ├── mod.rs       # EvaluationLookup trait, evaluate(), fs gate
    │   ├── lexer.rs     # Tokenizer
    │   ├── ast.rs       # AST types
    │   ├── parser.rs    # Expression parser
    │   ├── functions/   # domain-owned registrations and dispatch
    │   ├── catalog.rs   # Descriptor catalog (parity-tested against functions)
    │   ├── ctx.rs       # CtxLookup (ctx.* runtime context)
    │   ├── doc_namespace.rs # Reserved doc / doc.* namespace resolution
    │   └── resolve_ctx.rs   # ResolutionContext (base_dir, magic paths, remote)
    └── interpolation/
        ├── mod.rs       # Module exports
        ├── lexer.rs     # Expression finder
        ├── ast.rs       # AST types
        ├── parser.rs    # Expression parser
        └── evaluator.rs # AST evaluation
```
