---
status: draft — reviewed, ready for implementation
created: 2026-07-31
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-18
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
amends:
  - ../../../biscuit-file/features/2026-07-31-portable-strings/spec.md
---

# Transclusion failures should be errors by default

## Summary

A transclusion that fails during resolution or composition can currently become
a warning while `compose` still returns `Ok`. The caller receives a successful
exit code and a document with missing content. This inverts that default:
**transclusion failures are errors unless the request explicitly allows
degraded output**.

Tolerance is one request-scoped policy exposed as:

- `md compose --allow-transclusion-failures`;
- `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES`; and
- `ComposeOptions::with_allow_transclusion_failures(bool)`.

When tolerance is enabled, a body directive is replaced by a visible notice,
frontmatter `prologue`/`epilogue` failures omit the failed section, and every
tolerated failure produces structured report data and a warning. Composition
then returns `Ok`.

This is a breaking cleanup, implemented atomically. The existing
`ignore_invalid`/`IGNORE_INVALID` policy is removed rather than broadened into a
second spelling, and `fail_fast` retains its established meaning for other
compose stages.

> **Reader's note.** The initial draft proposed a new frontmatter key, legacy
> aliases, and possible removal of `fail_fast`. Review found that those choices
> would let a transcluded child weaken request policy, retain three overlapping
> controls, and break interpolation's existing `fail_fast` contract. The
> reviewed design instead makes transclusion tolerance part of the root
> request authority and removes the obsolete transclusion-specific controls.

## Current behavior

### Three mechanisms overlap without forming one policy

1. `ComposeOptions::fail_fast`, defaulting to `false`, controls leniency across
   several compose stages. In transclusion result application, it also decides
   whether most child failures become warnings. It is not an `md compose`
   option; `md validate --fail-fast` belongs to reference validation and has a
   different contract.
2. `ComposeOptions::ignore_invalid_references`, the document's
   `ignore_invalid` frontmatter key, and `IGNORE_INVALID` form a separate
   precedence chain. They cover target-resolution and some `::file-links`
   failures, but not failures from composing a resolved child.
3. Cycle, depth, remote-fetch, and missing-runtime-context failures are forced
   fatal in the transclusion result loop regardless of those controls.

The result depends on which layer detects a failure. Some tolerated failures
remove a directive, some insert a notice, and others remain fatal.

### Reference validation is a separate gate

`md compose` validates references before composition. The existing
`--allow-missing-transclusions` flag only defers matching validation issues; it
does not configure `ComposeOptions`. Consequently, a missing target can pass
the first gate and then fail during composition. Any replacement policy must
connect these two stages or its CLI escape hatch will not work for unresolved
targets.

### Approval preflight is a third gate

When shell-capable operations are enabled, `md compose` runs the
condition-blind approval preflight after reference validation and before the
terminal compose pass. That walk recursively resolves `::file`, `::url`,
`prologue`, and `epilogue` children so commands cannot evade approval by
appearing in a branch that is false in the current state. Resolution, loading,
or child-composition failures currently abort this walk independently of the
reference-validation allow flags. Deferring an unresolved target at validation
therefore does not by itself guarantee that tolerant composition is reached.

The replacement policy must connect validation, approval preflight, and
terminal composition without making preflight condition-aware or allowing it
to overlook commands in a child that can execute.

### `transclusions_skipped` is not a failure counter

`ComposeReport::transclusions_skipped` includes legitimate control-flow
outcomes such as a false `when=` condition and an empty `::file-links` match.
It cannot tell an embedding caller whether output was degraded by a failure.

Whole-value `::file`, `::code`, and `::url` targets that successfully evaluate
to `null` or an empty string are another non-failure outcome. The interpolation
stage removes those directives and records its existing nullable-target warning
before the transclusion stage sees them. They currently increment neither
`transclusions_skipped` nor a failure counter.

### The 2026-07-31 fix remains valid

The portable-strings work fixed a separate artifact-corruption bug. A failed
child compose previously left the authored `::file child.md` line in output
because no replacement was recorded. `PreparedTransclusion::failure_anchor`
now retains the replacement target and `fit_notice_to_span` preserves its
indentation and trailing newline. This specification keeps that mechanism and
extends visible replacement to every tolerated body-directive failure.

## Goals

- A complete compose and a degraded compose must have different default
  outcomes.
- One request-scoped policy must govern failures from all transclusion kinds
  and all recursion depths.
- The CLI flag must cover reference validation, condition-blind approval
  preflight, and compose execution.
- Library callers must be able to detect degraded output without parsing human
  messages.
- Parallel transclusion resolution must not make the selected strict-mode
  error or report ordering nondeterministic.
- Document content must not be able to grant itself permission to suppress a
  request-level failure.

## Non-goals

- Changing missing hyperlink or image-reference policy.
- Treating an empty `::file-links` match as a failure.
- Making internal invariant or missing-runtime-context failures suppressible.
- Changing the meaning of `fail_fast` outside transclusion.
- Stabilizing human-readable notice or error prose as a machine API.

## Proposed behavior

### 1. Strict is the default

Every failure attributable to an enabled transclusion operation returns `Err`
unless tolerance is enabled. This includes failures while parsing, preparing,
resolving, fetching, rendering, or recursively composing:

- body `::file`, `::code`, `::url`, `::toc-linking`, and `::file-links`
  directives;
- frontmatter `prologue` and `epilogue` references;
- local and permitted remote targets;
- child schema, expression, shell, render, cycle, and maximum-depth failures.

No partially composed `Markdown` or `ComposeReport` is returned on the strict
path. With concurrent resolution, the returned error is the first failure in
stable prepared/source order, never whichever worker finishes first.

Three categories are not transclusion failures:

- a directive excluded by `when=false`;
- `::file-links` successfully discovering zero matches; and
- a whole-value `::file`, `::code`, or `::url` target successfully evaluating
  to `null` or an empty string.

The first two return `Ok`, increment `transclusions_skipped`, and produce no
failure warning. An empty `::file-links` result always removes the directive;
it no longer uses `fail_fast` to choose between an empty replacement and
`_No matching files_`. A nullable target retains the behavior established by
the nullable-directive-targets fix: the interpolation stage removes it, emits
the typed nullable-target warning, and increments neither
`transclusions_skipped` nor `transclusion_failures_tolerated`. Tolerance does
not control or reclassify that successful-absence path.

### 2. Tolerance is request authority

Add one resolved policy to `ComposeOptions`. The private representation may be
`Option<bool>` so an explicit `false` can override the environment, while the
public builder remains ergonomic:

```rust
ComposeOptions::new().with_allow_transclusion_failures(true)
```

Resolution precedence, highest first, is:

1. an explicit library/CLI option;
2. `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES` from the captured
   `ComposeContext` environment;
3. `false`.

The value is resolved once at the root request boundary and inherited by every
recursive child, including `as_markdown(...)`. A child's frontmatter and a
later ambient environment change cannot alter it. Boolean environment parsing
uses the repository's existing boolish spellings; an invalid value is an
error naming the variable rather than silently selecting strict or tolerant
behavior.

There is deliberately no `allow_transclusion_failures` frontmatter property.
Composition policy belongs to the caller because transcluded content is not
authorized to decide whether its own failure may be hidden.

### 3. Tolerated output is explicit and structured

For each tolerated failure:

- a body directive is replaced by its per-kind failure notice, shaped by
  `fit_notice_to_span` so list and blockquote structure remains intact;
- a frontmatter `prologue` or `epilogue` contributes no section because it has
  no authored body span in which to place a notice;
- the original directive syntax never survives in output;
- `ComposeReport::transclusion_failures_tolerated` increments by one; and
- a `ComposeWarning` is recorded with stage `transclusion`, source
  `darkmatter.compose`, stable code `dm.transclusion.tolerated_failure`, and
  available line/path provenance.

`transclusions_skipped` retains its non-failure meaning and does not increment
for tolerated failures. Nested reports merge both the new counter and warnings
into the root report in stable prepared/source order.

The coded-warning family declares a stable deduplication key so generic
`ComposeReport::merge` deduplication cannot collapse distinct failures that
share the code. Its family key is the source-document identity, directive kind,
and authored body span or frontmatter section slot (including the slot index).
Repeated projection of the same failure deduplicates; different directives or
frontmatter entries remain independently countable and visible. The
`transclusion_failures_tolerated` counter uses those same distinct identities,
so report merging cannot leave the summary count larger than the number of
individually reported tolerated failures.

Notice wording and warning messages are human-facing and explicitly unstable.
Callers use the counter and warning code, not string matching. Notices must be
plain Markdown safe for subsequent terminal and browser rendering; diagnostics
continue through the existing `BlockError`/`TerminalRenderable` path rather
than ad hoc ANSI or `eprintln!` formatting.

### 4. Some failures remain fatal

Tolerance must not convert a failure into output when Darkmatter cannot uphold
request invariants. At minimum, these remain fatal in both modes:

- missing or partial runtime context for a required captured group;
- internal invariant failures for which no trustworthy replacement span or
  section slot exists; and
- panics, cancellation, or process-level failures outside the typed compose
  error model.

These are not counted as tolerated. Authorization failures such as a denied
remote host do not gain authority from this flag: the read remains denied. If
the denial has a trustworthy directive anchor, tolerant mode may replace that
directive with a notice; it must never fetch or use cached bytes from the
denied host.

### 5. `fail_fast` is decoupled, not removed

`ComposeOptions::fail_fast` remains the established policy for interpolation,
expression, schema-adjacent, and other non-transclusion leniency. The
transclusion engine no longer reads it. This includes removing its inverted use
for empty `::file-links` results.

The public docs for `fail_fast` must stop implying that every pipeline failure
is controlled by that field. They should enumerate its actual remaining
surfaces or describe it as non-transclusion compose leniency.

### 6. Remove `ignore_invalid`

Remove all of the following in the same breaking change:

- `ComposeOptions::ignore_invalid_references` and
  `with_ignore_invalid_references`;
- the `ignore_invalid` frontmatter key and baseline-schema entry; and
- the unprefixed `IGNORE_INVALID` environment variable.

Do not retain them as aliases. Broadening an authored child key from
target-resolution tolerance to all child-compose failures is an authority bug,
while retaining its narrow meaning would preserve overlapping policies and
technical debt. The repository has no established users, so an atomic cleanup
is preferable to a deprecation window for a policy that is already internally
inconsistent.

### 7. CLI validation, preflight, and execution share the flag

Replace `md compose --allow-missing-transclusions` with
`--allow-transclusion-failures`. Before reference validation, the CLI resolves
the effective policy from the flag and the captured environment. A true value
from either surface must:

1. defer transclusion-kind reference-validation errors so composition can
   produce the configured degraded artifact; and
2. configure condition-blind approval preflight to omit a child only when the
   same failure would be suppressible during terminal composition; and
3. set the root `ComposeOptions` tolerance policy.

Preflight remains condition-blind and fail-closed for dynamic command shapes,
missing or partial runtime context, internal invariants, and every other
failure that Section 4 keeps fatal. For a suppressible resolution, load, fetch,
or child-compose failure, tolerant preflight contributes no child graph edge
and discovers no commands from that unavailable child. It does not emit the
user-facing tolerated-failure warning or increment the report counter: the
terminal compose pass owns that diagnostic and count, preventing duplicate
reporting. A child that was successfully inspected must still contribute all
of its commands regardless of current `when=` conditions.

The same resolved value must drive all three stages; the environment-variable
path must not bypass validation deferral or be read again from ambient
`std::env`. An invalid environment value fails before validation, preflight, or
composition begins.

The flag does not defer missing hyperlink or image-reference errors. Existing
`--allow-missing-hyperlinks` and `--allow-missing-image-refs` behavior is
unchanged.

`--allow-any-missing-reference` continues to include transclusions and
therefore implies `--allow-transclusion-failures`; its help text must state the
broader consequence that any later transclusion failure, not only a missing
target, is tolerated for that invocation. This side effect is necessary to
keep the shorthand from passing validation only to fail immediately in the
compose stage.

On a strict transclusion error, the CLI adds a rendered hint naming
`--allow-transclusion-failures` and
`DARKMATTER_ALLOW_TRANSCLUSION_FAILURES`. The library's typed error remains
CLI-agnostic. On a successful degraded run, the CLI renders one summary after
the individual warnings, for example `2 transclusion failures tolerated`.

## Migration and repository impact

The strict default, removed APIs, and CLI rename ship atomically. A staged
release would temporarily require contradictory defaults or aliases that the
final design intentionally rejects.

Before implementation is considered complete:

- audit every `md compose` invocation in repository `justfile`s, workflows,
  scripts, fixtures, and documentation;
- fix repository-owned broken transclusions rather than adding tolerance;
- update any library caller using `with_ignore_invalid_references` to either
  fix its input or opt into the new request-wide policy;
- update the Darkmatter baseline schema and shipped schema documentation to
  remove `ignore_invalid`; and
- update the exhaustive `ComposeOptions` field-classification authority so the
  policy participates in both graph-options identity and the compose-cache
  fingerprint; tolerance changes graph traversal and semantic output.

## Testing

### Library behavior

- Every failure category listed above returns `Err` by default.
- Each suppressible category returns `Ok` with the correct body notice or
  omitted frontmatter section when tolerance is enabled.
- Missing-runtime-context and invariant failures remain errors in both modes.
- Each tolerated failure increments `transclusion_failures_tolerated`, does not
  increment `transclusions_skipped`, and records the stable warning metadata.
- False conditions and empty `::file-links` results remain successful skips
  with no tolerated-failure warning.
- Nullable or empty whole-value directive targets retain their successful
  interpolation-stage omission and nullable-target warning, incrementing
  neither transclusion counter regardless of tolerance.
- Distinct tolerated failures with the same warning code survive report merges,
  while repeated projection of the same source location deduplicates without
  inflating `transclusion_failures_tolerated`.
- Multiple concurrent failures select and report in stable prepared/source
  order across repeated runs.
- Root policy propagates through recursive `::file`, `::url`, and
  `as_markdown(...)` composition; child frontmatter cannot weaken it.
- An explicit library `false` overrides a true captured environment value.
- The new policy participates in graph-options identity and the compose-cache
  fingerprint, preventing strict and tolerant requests from sharing graph or
  semantic results.

### CLI behavior

- `--allow-transclusion-failures` alone is sufficient for an unresolved target:
  reference validation defers it, approval preflight omits the unavailable
  child, and composition emits degraded output.
- With shell-capable operations enabled, tolerant preflight omits an unavailable
  child without recording a graph edge or duplicate warning, then terminal
  composition records exactly one tolerated failure.
- Strict preflight still rejects malformed directives, dynamic command shapes,
  missing runtime context, and other non-suppressible failures even inside a
  currently false branch.
- Without the flag or environment variable, the same invocation exits nonzero
  and emits no composed document on stdout.
- The strict CLI diagnostic renders the escape-hatch hint through terminal
  components.
- A degraded run prints individual warnings plus exactly one count summary to
  stderr while preserving the selected stdout format.
- `--allow-any-missing-reference` implies the same transclusion policy and its
  help text describes that consequence.
- Invalid environment values fail clearly; true and false boolish values are
  covered through the fixture's captured environment.
- Deterministic CLI tests use `CliProcessFixture` and do not restore protected
  ambient environment after `build()`.

### Regression coverage

- Convert `darkmatter/lib/tests/declined_path_transclusion.rs` to assert the
  strict default, preserving its notice assertions under tolerant mode.
- Update `::file-links` tests to remove the `fail_fast`-dependent empty-result
  behavior.
- Preserve nullable-target tests for `::file`, `::code`, and `::url`, including
  their counter behavior in both strict and tolerant requests.
- Add coverage for body indentation/trailing-newline preservation and for
  frontmatter `prologue`/`epilogue` omission.
- Add a regression proving a denied host is checked before any transport-cache
  read even when failures are tolerated.

Use `just test` and `just lint` in `darkmatter`. These are L1/compile checks;
this policy does not require a focused terminal or browser window.

## Documentation impact

Update all surfaces that describe compose error policy:

- `darkmatter/README.md` and `md compose --help`;
- `darkmatter/docs/cli/compose.md`;
- `darkmatter/docs/inline/preflight-checks.md`;
- `darkmatter/docs/topics/transclusion.md`;
- `darkmatter/docs/transclusion/block-transclusion.md`;
- `darkmatter/docs/transclusion/transclusion-design.md`;
- `darkmatter/docs/transclusion/fm-transclusion.md`;
- `darkmatter/docs/inline/file-links.md`;
- `darkmatter/docs/structs/Markdown.md`;
- `darkmatter/docs/topics/schema-definition.md`;
- the shipped Darkmatter schema and generated schema documentation; and
- `.claude/skills/darkmatter/compose.md`.

Documentation must distinguish successful skips from tolerated failures,
describe the stable report counter/code, state that policy is resolved once per
request, and avoid presenting `fail_fast` as the transclusion control.

## Open questions

None. Naming, authority, migration, reporting, and `fail_fast` scope are design
decisions in this reviewed specification.
