---
hash: ef46db3751d8e999-0e96352d3a62ac3e
last_updated: 2026-09-18
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

Before the numbered stages, a schema-aware input prelude applies the effective
schema to immutable caller records containing each property's raw value and
captured file-resolution origin. An exactly selected eager or non-recursive
lazy `file` arm installs a native semantic identity; eager requires an existing
file, while lazy binds the first ordered candidate without probing it. Portable
Markdown presentation is retained separately and the raw record remains
available for fresh preparation. Ordinary strings and document-authored file
references keep their existing source-relative and repository-aware rules.

1. **Frontmatter Interpolation (pass 1)** - `{{ variable }}` in frontmatter resolves before effective state is built; materialized caller-file parameters already expose their origin-resolved native values, and keys referencing a whole-value `$(...)` are deferred to pass 2
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

Calls to `package(`/`package_area(` demand `Repo` and `ipv4(`/`ipv6(` demand
`Network` (`FUNCTION_GROUPS` in `capture/groups.rs`). Those functions read
`CapturedObservations` (package roots with names, scoped interface addresses)
retained on the `ComposeContext` beside the JSON values and copied into
`ResolutionContext::observations`; they never rediscover topology or
interfaces. A call whose group was never captured fails with the fatal
`ExpressionError::FunctionContextNotCaptured`. A call the function's spec
makes a compose error (`recent_commits(0)`, a URL passed to `package`) returns
`ExpressionError::ContractViolation`, which `is_authoring_fatal` never demotes
to a warning; use `Other` only for failures a lenient caller (subtree,
preflight discovery) may tolerate.
`recent_commits(count)` demands
no group: each reached call walks history from the file-resolution
`repository_root` and renders through the capture's `render_recent_commits`.
It is the lazy half of the first variable/function pair (R29): `ctx.recent_commits`
is the eager snapshot of the same descriptor, and one descriptor entry projects
both so the variable and the function cannot drift.

`has_agentic_cli(agent)` demands `Agent`; the capture retains a lazily scanned
`InstalledAiClients` `PATH` index that every clone of the request shares, and a
name outside the generated `AGENTIC_CLI_NAMES` table is a `ContractViolation`.
`has_alias`, `has_builtin_function`, `has_user_function`, and the shell half of
`can_execute` demand no group: `ResolutionContext::shell_probe` is a
`ShellProbe` built from the request environment's `SHELL` (Windows falls back
to `pwsh`, then `powershell`, on that environment's `PATH`). One launch answers
all three kinds through `shell_expansion/launcher.rs`, the bounded launcher
alias resolution shares; the name reaches the shell only as an environment
variable and is never executed. `ResolutionContext::default()` disables
probing, and `ComposeOptions::suppress_shell_probes` (set by shell-command
preflight discovery) keeps preflight from launching a profile. `can_execute`
checks `has_binary` first and skips the launch when it succeeds.

`as_markdown(content)` composes `content` through `compose/nested.rs`: the
calling pipeline installs a `NestedComposeSlot::Active` handle on
`ComposeOptions` (and so on the frontmatter and body `ResolutionContext`s) per
document. The child reuses the caller's options with the root source as its
resolution base, the request context epoch, the shell runtime, and the
transclusion ancestry: each call pushes a unique node, so `as_markdown` and
`::file` share `max_transclusion_depth`, and a cycle or depth failure below the
call is restored as the typed `TransclusionError` when the caller finishes.
Other nested failures are `ContractViolation`s naming the call site. Root-only
stages (link normalization, the pre-approved gate) key on
`PipelineRuntime::is_root`, not stack depth. The result is the composed body
plus only the frontmatter keys the content authored; nested warnings carry an
`as_markdown > <stage>` stage. Surfaces outside a compose request
(`local_expression_resolution_context`, passive validation) leave the slot
`Unavailable`, where the call is a `ContractViolation`.

`ping(address, timeout?)` and `ping_under(address, timeout, attempts?)` are
network effects under their own grant, separate from HTTP fetch permission even
though both are spelled `--allow-host`. `ComposeOptions::icmp` is an
`IcmpAuthority` (`compose/icmp.rs`) projected into every `ResolutionContext` the
request builds; its grants are derived from `remote_read_config.allowed_hosts`
on each projection, so the flag stays the single entry point. An exact IP
literal grants ICMP *and* keeps its existing exact-HTTP meaning (a spelled IPv6
zone constrains the grant to that zone); a strict CIDR grants ICMP only and is
withheld from the `FetchPolicy` allowlist by `RemoteReadConfig::http_hosts`; a
hostname or wildcard grants HTTP only and never authorizes an address
indirectly. The address argument is a validated `ScopedIpAddr` literal — no DNS
— and the budget goes through `sniff::network::icmp::ProbeBudget::from_millis`,
which range-checks before converting, so a non-finite, non-positive,
fractional, or overflowing number is a `ContractViolation` before any packet.
An ungranted target answers `null`, records one warning on the authority's
shared sink (drained into `ComposeReport::warnings` at the root), and sends
nothing even for a multi-attempt call; a send failure is a `ContractViolation`
that aborts the series after earlier replies. `ping` maps to `true`/`false`,
`ping_under` adds `"unstable"`. Transport is `sniff::network::icmp`; the
authority owns an injectable `EchoProbe` so tests are deterministic, and its
discovery mode is what makes preflight passive.

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

### Lazy reserved roots (`current`, `current_env`)

`context/current.rs` owns the request's refresh authority. Three pieces:

- `CurrentProvider` — the invocation's capability to observe **one** cataloged
  key now. A `DarkmatterOwned` request (`ComposeOptions::for_document`,
  `ComposeOptions::new()`) installs `AnchoredRefresh`, which re-captures a
  mutable key's group at the context's retained anchor and answers
  `Repo`-group keys from the request's one repository observation (D3) — the
  eager `Repo` capture, or one discovery at the anchor. `for_document` is the
  request boundary: it fixes that observation at creation, before validation,
  pre-flight, or compose run. A request built through `new()` or
  `new_with_context(..).with_context_authority(DarkmatterOwned)` is fixed by
  the root pipeline entry instead, unconditionally — not gated on whether the
  root plans a `current.repo*` read, because a transcluded child may be the
  only reader and a child pipeline never establishes request state. The
  provider never discovers. `ComposeOptions::with_current_provider` installs
  an embedder's own. A caller-supplied or caller-extended request with no
  provider observes nothing.
- `CurrentAuthority` — request-owned, carried on `ComposeOptions` beside
  `IcmpAuthority`, projected into every `ResolutionContext` and into
  `EffectiveStateBuilder::with_current_authority`. It owns the shared
  `PartialRuntimeCapture` sink the root pipeline drains into
  `ComposeReport::warnings`, and has a `discovering()` mode preflight installs
  so a passive walk observes nothing.
- `CurrentScope` — the per-lookup memo, cleared at every expression boundary by
  `EvaluationLookup::begin_expression_scope`. The boundary is called by
  `Evaluator::{eval, eval_value, eval_json}`, `conditions::evaluate_condition`,
  and the two `$()` ternary evaluations. That is the Q2 memo scope: repeated
  reads of one key inside one expression agree; the next expression refreshes.

Resolution rules, enforced in `EffectiveState`, `FrontmatterSeedState`, and
`LayeredLookup` **before** frontmatter, external state, and injected globals,
so neither can shadow a reserved root:

- `current.<key>` where `<key>` is cataloged: an `Invocation` or `Document`
  key is request-owned and reads the eager capture; every other group refreshes
  through the provider.
- A key the provider does not hold is `null` plus one `PartialRuntimeCapture`
  diagnostic — never the stale `ctx` value and never ambient discovery.
- `current_env.<KEY>` is `std::env::var` at reference time, with `env`'s
  missing-value semantics.
- Bare `current` enumerates the descriptor names with null values; bare
  `current_env` resolves to nothing, as bare `env` does.
- Any other path (`current.ctx.x`, `current.env.x`, `current_env.ctx.x`,
  `current.<typo>`) raises the authoring-fatal
  `ExpressionError::ReservedRootPathUnknown`.

`DeferredCapabilities` (`capture/capabilities.rs`) is the planning twin of
`ContextRequirements`: it names the `current` keys, `current_env` names, and
cataloged function calls a source can reach, and observes none of them. A
`current.*` reference adds no eager requirement, and an eager requirement never
answers a `current.*` read. `ComposePreflightReport::deferred_context` carries
the union across the walked graph.

### Checked `ctx.*` lookup

`context/checked.rs` classifies each `ctx.<key>` read into one of four
outcomes: `Present`, `NotCaptured`, `ProjectionMissing`, or `Unknown`.
Classification uses only `ContextGroup::for_key`/`projected_keys` and the
snapshot's `capture_requirements()`. Expression evaluation reads through
`EvaluationLookup::get_checked`, which `EffectiveState`, `ResolvingLookup`,
`FrontmatterSeedState`, `LayeredLookup`, `ShortcutLookup`, and `CtxLookup`
override.

- A captured group must project every key it owns, with `null` for absence.
  `ProjectionMissing` raises `ExpressionError::ContextProjectionInvariant`
  (authoring-fatal).
- `NotCaptured` raises `ExpressionError::ContextNotCaptured`, which is
  authoring-fatal.
- Both errors stay typed through every wrapper:
  - `ConditionError::Eval` and `TransclusionError::ConditionEval` carry a
    `cause`.
  - `$()` ternary evaluation uses `ShellExpansionError::ExpressionEvaluation`.
  - Body interpolation errors carry the on-disk document.
  - `MarkdownError::missing_runtime_context()` finds either error in the chain.
  - Lenient transclusion treats either error as structural, so no notice
    replaces the child.
- Pre-flight discovery does not own the verdict:
  - Each document it walks reads `ComposeOptions::extended_for(document)`,
    and its children inherit that context.
  - Its inline body compose sets `defer_missing_runtime_context`.
  - A `$()` value left raw by a missing capture reports that error, not
    `DynamicCommandShape`.
- A `$()` ternary prepares both branches before evaluating its condition, so
  both branches are checked.
- Growth is decided by `ContextAuthority`, set on `ComposeOptions`:
  - `CallerSupplied` is frozen. It is the default for `new_with_context` and
    `with_context`.
  - `DarkmatterOwned` uses `ComposeContext::extend_ambient`, which discovers
    at the retained anchor and never reads CWD. It is the default for
    `ComposeOptions::new()` and `ComposeOptions::for_document`, which the
    `md` CLI builds its request through.
  - `CallerExtended(Arc<dyn ContextExtension>)` grows the context from the
    caller's retained evidence. Claudine passes
    `InvocationContext`/`DocumentEpoch::compose_context_authority()`.
- The request epoch (`context/authority.rs`, `RequestContextEpoch`):
  - It lives on `PipelineRuntime` and is shared by `clone_for_child`. The root
    seeds it after `extend_context_for`.
  - `render_markdown_transclusion` (and the remote arm) hand each child
    `context_for_source`. That is the parent's snapshot plus the child's own
    `for_document` groups (after the `set` overlay), adopted from the epoch.
  - Every group is captured once per request, under a write lock.
  - A child's group set depends only on its path from the root, never on the
    order siblings resolve in.
- Child compose-cache identity (run-local only; semantic results are never
  persisted, and local transclusion stays run-local even after a
  `ContentPolicy` exists, R18/R36):
  - The key's `context_hash` is the hash of the child's snapshot. The hoisted
    phase hash is reused when the child named no new group.
  - `ComposeResult` records the subtree's context-group closure so the parent
    runtime can `record_context_groups` on a run-local hit.
- User-facing contract: `docs/topics/context-variables.md` (authority, epoch,
  captured absence) and `docs/topics/caching.md` (run-local keys and context hash).
- A captured `null`/`""`/`[]` projection is `Present` and renders normally.
  Only a never-captured group is `NotCaptured`; unknown names keep the
  unknown-variable warning.
- A graph that names no discovery-backed group must capture none. That is
  guarded end to end by
  `request_context_epoch::a_graph_naming_no_discovery_backed_group_captures_none`.
- Authored `ctx:` values never satisfy an uncaptured group.
- A bare-name fallback (`when="repo"`) with an uncaptured group is an undefined
  name, not a missing capture.
- Test fixtures must stay well formed: `fixed_for_testing_with` marks each
  inserted key's group captured. Build a malformed snapshot with
  `with_projection_key_removed`.

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

A whole-value `{{ ... }}` target on `::file`, `::code`, or `::url` preserves
typed evaluation. `null` and `""` skip the directive with a typed compose
warning; authored-empty, quoted-empty, mixed, and malformed targets retain
their ordinary parser/path behavior. Page blocks run first, so the recommended
optional-target idiom is condition-aware and warning-free:

```markdown
::block when="file_exists(log)"
::file {{log}}
::end-block
```

### Variable Resolution

| Pattern | Description |
|---------|-------------|
| `{{ foo }}` | Frontmatter value |
| `{{ user.name }}` | Nested object path |
| `{{ spec-name }}` | A kebab-case key; `-` joins mid-identifier only before an identifier character, so `a - b`, `4-2`, `foo--bar` stay subtraction and `iteration-1` is a key |
| `{{ doc['foo--bar'] }}` | Bracket access for a key the identifier form cannot spell (`.`, `--`, leading digit, trailing `-`) |
| `{{ doc }}` | The whole frontmatter object |
| `{{ doc.build }}` | A frontmatter property by the `doc.*` namespace (a property literally named `doc` is `doc.doc`); intercepted before any `ctx.*` fallback |
| `{{ ctx.today }}` | Runtime context, captured once per request |
| `{{ env.HOME }}` | Environment variable, from the snapshot frozen at capture |
| `{{ current.branch }}` | The same key as `ctx.branch`, observed when this expression evaluates |
| `{{ current_env.HOME }}` | The same key as `env.HOME`, reread from the live process environment |
| `{{ file_exists(path) }}` | Read-side function (also `frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema`, `absolute`, `relative`); resolves on every surface, both interpolation passes included |

Read-side functions and `doc.*` resolve identically on every surface
(frontmatter both passes, body, `when=`, `$()` ternary condition/branches,
claudine loop/hook). Every frontmatter surface (both interpolation passes and
the `$()` ternary condition/branch) is local-only, so a remote URL argument
fails loudly there; only body interpolation carries a remote runtime. See
`darkmatter/docs/topics/darkmatter-expressions.md`.

How text becomes an AST — scanning, lexing (including the `-` identifier
rule), the grammar ladder, the failure table (every full-document surface is
fatal), and what DMLS checks — is in `darkmatter/docs/topics/parsing/`. The
`dm.*` code-sharing rules are in `darkmatter/dmls/docs/diagnostics.md`
(§ "`dm.*` registry rules").

### Unknown identifiers (`dm.expression.unknown_identifier`)

A well-formed root that resolves to nothing still renders empty, but a
full-document compose warns about it once per root per source document
(source `darkmatter.expression`, `path` and `line_number` set, location also
in the Prose-escaped message because `md` and Claudine render only the
message). The moving parts:

- **One classifier.** `expression::absence::AbsenceScope` is threaded through
  `evaluate_expr`, so observation follows evaluation. An unchosen branch or a
  short-circuited operand is never read, so it never warns. Handled: a
  fallback primary (and every operand of a primary `a || b || "d"` chain), a
  ternary condition, the condition's own root in its branches when the
  condition is a bare variable, and a direct argument of an absence predicate.
  "Direct" is strict: `x == 1 ? …`, `!x`, `is_empty(lower(x))` all warn.
  `when=` and `$()` ternary conditions are gates, not absence checks: a bare
  unknown `when="x"` warns.
- **Absence predicates** are the `predicates::ABSENCE_PREDICATES` binding group
  (`is_null`, `is_empty`, aliases), read through
  `functions::is_absence_predicate`. Never spell the names in a walker. The
  rule is interim (spec Resolved Decision 9).
- **Observation is opt-in.** `evaluate`/`evaluate_condition`/`Evaluator::eval`
  are unchanged; surfaces call `evaluate_observed`,
  `conditions::evaluate_condition_observed`, or build the `Evaluator` with
  `.observing_missing_roots()` and drain `take_missing_roots()`. The `()`
  observer compiles the check away.
- **Known roots** come from `EvaluationLookup::is_known_variable_root`:
  `EffectiveState`, `ResolvingLookup`, and `FrontmatterSeedState` answer it
  (state key even if `null`/`""`, `ctx`/`env`/`doc`/`current`/`current_env`,
  bare context names, and `null`, which the grammar has no literal for). The
  trait default `true` keeps other lookups silent.
- **Candidates, not warnings.** Surfaces push `UnknownRootCandidate`s onto
  `ComposeReport` (first read per root only, through the hash-backed
  `UnknownRootCandidates`, whose root set is private so it cannot drift from
  the list; `report.rs`'s identity-work counter pins O(1) per read).
  `unknown_identifiers::reconcile`
  runs once per document just before `attribute_to_document`. It drops roots
  known to the final state, a caller input record, or the effective schema
  (`EffectiveSchema::declares_top_level_property`: `properties`,
  `patternProperties`, union/`allOf`/`if` arms, local `$ref`), then emits.
  A discovery pass drops its candidates. A required-but-unset root never gets
  here: schema validation fails first.
- **Lines are provable or absent.** Body spans project through the
  `BodyOrigin` edit map (see [Error Handling](#error-handling)), then a
  unique-occurrence match. `::block`/`::file when=` lines are body-relative,
  so the stage converts them with `unknown_identifiers::directive_locus` before
  rewriting the body. Frontmatter reads locate at their top-level key.
- Directive targets are not observed. A null `::file {{ x }}` target already
  warns as a skipped nullable target, and a second warning would duplicate it.

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
`{{ ctx.foo }}` embedded in text renders an array as **compact JSON**
(`["a","b","c"]`, and `[]` when empty); `as_json(ctx.foo)` is the explicit
spelling of that default. For other shapes use the list-formatting functions:
`as_csv`, `as_tsv`, `as_space_separated`, `as_line_separated`,
`as_unordered_list`, `as_ordered_list`, `as_json`, and `as_json5` (the
Markdown-list renderers auto-nest nested arrays and the `depends_on` /
`used_by` object shape). The former `_list` twin variables (e.g.
`ctx.dirty_files_list`) are removed — use `{{ as_unordered_list(ctx.dirty_files) }}`.

Only a value embedded in _surrounding text_ is stringified. A frontmatter value
whose entire trimmed content is one span (`path: "{{ ctx.packages }}"`) still
resolves to a typed array, as do loop mutations over a single typed span and
typed dynamic sequence sources.

**Migration.** A bare `{{ ctx.foo }}` used to render newline-joined. Documents
that relied on that move to `{{ as_line_separated(ctx.foo) }}`, which is
unchanged, or to whichever explicit function matches the intent —
`as_unordered_list` for Markdown bullets, `as_csv` for a prose list, `as_json` /
`as_json5` when the structure is the point.

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
- Nullable whole-value directive targets are evaluated during the recursive
  approval walk. An absent target contributes no child edge, but concrete
  siblings in any branch remain discoverable because the walk does not
  evaluate page-block conditions.
- **Execution is condition-aware.** The inline shell stages run only the
  commands whose branch is reached, gated by
  `ComposeOptions::with_pre_approved_commands(set)`. The invariant
  `execution_set ⊆ approval_set` makes the gate a pure membership check.
- A body/`::shell-block` command embedding a frontmatter value still pending
  frontmatter-shell expansion is rejected up front as
  `ShellExpansionError::DynamicCommandShape` (never a late `NotPreApproved`).
- A transclusion target depending on a pending frontmatter-shell value is
  rejected by the same fail-closed dynamic-shape boundary before any nullable
  rewrite or child discovery.
- Nested `as_markdown` content is walked like a child, never composed:
  discovery sets `NestedComposeSlot::Discover`, which records each evaluated
  argument and returns `""`, and every string-literal argument in the authored
  source (including untaken branches) is walked too, against the root source.
- Discovery answers shell probes `false`, composes no nested content, and
  answers `ping`/`ping_under` `null`, so a shape that depends on them is
  rejected up front as `ShellExpansionError::UnevaluatedDependencyShape`: a
  `::shell` line, shell-block body, transclusion target, or frontmatter
  `$(...)` value calling `has_alias`, `has_builtin_function`,
  `has_user_function`, `can_execute`, `as_markdown`, `ping`, or `ping_under`,
  and a non-literal `as_markdown` argument that reads one of them or a pending
  frontmatter-shell value.
- ICMP is the second approvable effect. `ComposePreflightReport::icmp_probes`
  carries typed `PlannedIcmpProbe` records (function, target, timeout,
  attempts, and whether a grant already permits it) rather than command
  strings; discovery installs `IcmpAuthority::discovering()`, which records the
  plan and sends nothing. Every all-literal call in the authored source is
  recorded too, so untaken branches contribute. `md compose --shell` prints
  them.
- Frontmatter interpolation runs before the root's pre-approved gate, so the
  first nested call there runs the gate against the root document first.

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

This shell `no-cache` is unrelated to the HTTP `Cache-Control: no-cache`
directive the remote transport cache honors (always revalidate before reuse).
Keep the two visibly distinct in docs and diagnostics.

## Error Handling

An expression that cannot be parsed or evaluated fails composition whatever
`fail_fast` says: body, frontmatter whole-value and mixed text, directive
targets, `when=`, and `$()` ternaries. The error is a typed
`MarkdownError::Interpolation` anchored to the file, and nothing is emitted.
When the failing `{{ … }}` is provably authored, its source is
`SourceRef::OnDiskSpan` carrying an `AuthoredSpan` (byte range of the whole
construct in the loaded text, plus one-based line and character column);
otherwise it is file-only `SourceRef::OnDisk`, never a guess.

- **Body.** The stages that rewrite the body before interpolation — text
  replacement, page blocks, and directive targets — report the `TextEdit`s
  they applied, and `body_origin::BodyOrigin` composes them into a map from
  the current body back to the loaded text (CRLF-aware). A span projects only
  when every byte was copied from the file. The map is keyed to the exact
  text it describes, so a body rewritten by an untracked path stops
  projecting instead of projecting wrongly. A new stage that rewrites the body
  before interpolation must report its edits through `body_origin::advance`.
- **Frontmatter.** `interpolate_frontmatter_located` records the failing
  string's key/index path and the construct's span in it; the pipeline walks
  the crate-private `locate_frontmatter_value` (the schema locator with block
  and multi-line scalars enabled; schema documents keep the closed v1 grammar
  that rejects them) to the authoring scalar and projects through its
  quoting, escapes, line folding, and indentation with
  `yaml_scalar::decode_scalar_node`, which follows libyaml's rules for plain,
  quoted, literal `|`, and folded `>` scalars on one line or several
  (chomping, indentation indicators, CRLF). A tag (`!!str`) or anchor
  (`&name`) before the scalar is skipped and stays outside the span; the
  locator looks past tags only for frontmatter, never for schema documents.
  An alias (`*name`) projects into the scalar defining its anchor, the one
  place the expression is authored, but only when `&name` occurs exactly once
  before it: for a redefined anchor, or the name repeated in a comment or
  string, the span is the alias token itself (`yaml_scalar::alias_token`),
  where the value is certainly referenced, rather than a guessed definition.
  Only an alias gets that fallback. The parser
  reads the YAML lines joined without the last terminator, so the projection
  decodes the same extent. The scalar must decode to exactly the scanned
  string, which is also what rejects a tag that changes the value (`!!int`);
  a value changed after loading (coercion, shell expansion) stays file-only.
  DMLS projects Expression-typed values through the same decoder, but reads
  an alias's definition from its own YAML tree
  (`yaml_scalar::decode_alias_definition`) instead of searching: the search
  is affordable here only because a frontmatter failure ends the run.
- **Rescans** of replacement output have no authored span.

The helper takes an explicit
`ExpressionFailurePolicy`; only `compose_subtree(..., Lenient)`, preflight's
best-effort frontmatter pass, and the discovery compose pass
(`ComposeOptions::defer_expression_failures`, crate-private) use `Lenient`.
Discovery must stay lenient because it runs without page blocks and would
otherwise reject a failure inside a region a false `::block` removes.

With `fail_fast: false` (default):
- TOC-linking and non-structural transclusion failures are downgraded to warnings
- Structural transclusion errors (cycles, max depth) still return immediately
- Warnings recorded in report

With `fail_fast: true`:
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
    ├── preflight/       # Approval-set lifecycle (condition-blind)
    │   ├── mod.rs       # ComposePreflightReport + Markdown::compose_preflight
    │   ├── collect.rs   # Condition-blind graph walk → shell + ICMP effects
    │   └── approval.rs  # Deduped normalized approval-set boundary export
    ├── icmp.rs          # IcmpAuthority: ICMP grants, transport, planned probes
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
