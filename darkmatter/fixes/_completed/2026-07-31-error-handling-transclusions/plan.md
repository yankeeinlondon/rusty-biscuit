---
total_phases: 8
created: 2026-09-18
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
---

# Plan: Transclusion Failures Are Errors by Default

Implements `darkmatter/fixes/2026-07-31-error-handling-transclusions/spec.md`
(reviewed, ready for implementation). All file references are relative to the
repository root unless noted.

## Work Summary

Today a transclusion that fails during resolution or composition can degrade
output while `compose` still returns `Ok`: `ComposeOptions::fail_fast`, the
`ignore_invalid` chain (`ComposeOptions::ignore_invalid_references`,
frontmatter `ignore_invalid`, env `IGNORE_INVALID`), and a forced-fatal
structural set in the transclusion result loop (`phases.rs:385-397`) overlap
without forming one policy. This plan inverts the default: **every failure
attributable to an enabled transclusion operation returns `Err`** unless the
request explicitly enables tolerance through one request-scoped policy —
`md compose --allow-transclusion-failures`,
`DARKMATTER_ALLOW_TRANSCLUSION_FAILURES`, or
`ComposeOptions::with_allow_transclusion_failures(bool)` — resolved once at
the root request boundary and inherited by every recursive child.

The work touches five surfaces:

1. **Library policy core** — new `ComposeOptions` field (`Option<bool>`
   representation, ergonomic `bool` builder), root-boundary resolution with
   boolish environment parsing (invalid values are typed errors naming the
   variable), and participation in the exhaustive field-classification
   authority (`ReferenceGraphOptionsIdentity` + compose-cache fingerprint in
   `lib/src/markdown/compose/context/options.rs`).
2. **Report model** — `ComposeReport::transclusion_failures_tolerated`
   counter, stable warning code `dm.transclusion.tolerated_failure`
   (stage `transclusion`, source `darkmatter.compose`), and a stable
   deduplication family key (source-document identity + directive kind +
   authored body span or frontmatter section slot) so generic report merging
   cannot collapse distinct failures nor inflate the counter.
3. **Transclusion engine** — the result-application loop
   (`lib/src/markdown/compose/pipeline/phases.rs:377-436`) becomes strict by
   default; tolerated failures replace the directive with a
   `fit_notice_to_span`-shaped notice (extending the retained
   `PreparedTransclusion::failure_anchor` mechanism), omit failed
   prologue/epilogue sections, and record the counter + coded warning. The
   engine stops reading `fail_fast` (including the empty `::file-links`
   branch at `engine.rs:1279`) and stops reading `ignore_invalid`.
4. **Preflight** — the condition-blind collection walk
   (`lib/src/markdown/compose/preflight/collect.rs`) omits an unavailable
   child under tolerance (no graph edge, no commands, no duplicate warning or
   count) while remaining fail-closed for dynamic command shapes, missing
   runtime context, and internal invariants.
5. **CLI** — `--allow-missing-transclusions` is replaced by
   `--allow-transclusion-failures`; one resolved value drives reference
   validation deferral, approval preflight, and terminal composition;
   `--allow-any-missing-reference` implies it; strict errors render an
   escape-hatch hint; degraded runs print one count summary to stderr.

The atomic breaking change also **removes** the `ignore_invalid` chain
entirely (options field + builder, frontmatter key, baseline-schema entries,
unprefixed env var) and fixes repository-owned broken transclusions rather
than adding tolerance.

**Successful completion looks like:**

- A compose with a failing `::file`/`::code`/`::url`/`::toc-linking`/
  `::file-links` directive or `prologue`/`epilogue` reference returns `Err`
  by default, with no partial `Markdown` or `ComposeReport` on the strict
  path; the CLI exits nonzero and prints no composed document on stdout.
- With tolerance enabled, the same request returns `Ok` with a visible
  per-kind notice in place of each failed body directive, omitted failed
  frontmatter sections, `transclusion_failures_tolerated` incremented once
  per distinct failure, and one coded warning per failure — while
  `when=false` skips, empty `::file-links` results, and nullable/empty
  whole-value targets keep their existing successful non-failure behavior.
- Missing-runtime-context and invariant failures remain errors in both modes;
  a denied remote host is checked before any transport-cache read even when
  failures are tolerated.
- The CLI flag alone is sufficient for an unresolved target: validation
  defers it, preflight omits the unavailable child without recording a graph
  edge or duplicate diagnostic, and composition produces the degraded
  artifact.
- Strict and tolerant requests never share graph identity or compose-cache
  results.
- `rg` finds no remaining `ignore_invalid`/`IGNORE_INVALID`/
  `--allow-missing-transclusions` usage; `just test` and `just lint` pass in
  `darkmatter`; downstream consumers (claudine) still build and pass.

## Phase 1 — Rulings, Spikes, and Repository Audit

Foundation phase: confirm the implementation-level rulings the reviewed spec
implies but does not spell out, de-risk the two hardest mechanics (preflight
omission and concurrent ordering/dedup), and inventory every repository
surface the breaking change touches. Nothing in Phase 2+ starts until the
rulings are recorded.

### Necessary Rules

- **R1 — Fatal-set boundary.** Under tolerance, `CycleDetected`,
  `MaxDepthExceeded`, and `RemoteFetchFailed` become *tolerated* failures
  when the prepared anchor provides a trustworthy replacement span (spec §1
  explicitly lists cycle and maximum-depth failures among the tolerable
  set). The forced-fatal `is_structural` classification in
  `pipeline/phases.rs:385-397` shrinks to: missing/partial runtime-context
  failures, internal-invariant failures with no trustworthy anchor, and
  process-level failures. Authorization denials never gain authority — the
  host stays denied and no cached bytes are read — but a denied-host failure
  with a trustworthy directive anchor may be replaced by a notice in
  tolerant mode.
- **R2 — Resolution point and representation.** The policy is stored as
  `Option<bool>` on `ComposeOptions` (explicit value wins). A resolution
  helper (explicit option > `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES` from the
  captured `ComposeContext` environment > `false`) is invoked exactly once at
  the root pipeline boundary and the resolved `bool` is stamped onto the
  options that every stage and recursive child (including `as_markdown`)
  inherits via the existing clone path. No stage, engine, or child re-reads
  the environment. The same helper is exposed for CLI reuse so validation,
  preflight, and composition share one resolved value.
- **R3 — Family-key representation.** `ComposeWarning` gains a dedicated
  opaque dedup/family key (source-document identity + directive kind +
  authored body span, or frontmatter section slot including slot index).
  Generic `ComposeReport::merge` deduplicates only on that key for the coded
  `dm.transclusion.tolerated_failure` family; distinct directives and
  frontmatter entries never collapse, repeated projection of the same
  failure does. The `transclusion_failures_tolerated` counter is kept
  consistent with the deduplicated warning set at merge time so the summary
  count can never exceed the individually reported failures.
- **R4 — Environment error surface.** Boolish parsing uses the repository's
  existing spellings (`1/true/yes/on`, `0/false/no/off`). An invalid value
  produces a typed error naming `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES` —
  surfaced by the CLI *before* validation, preflight, or composition begins,
  and returned by the library when root-boundary resolution runs.
- **R5 — Prologue/epilogue identity.** A tolerated frontmatter failure
  contributes no section (content stays `None`) but still increments the
  counter and records the coded warning; its family key uses the
  source-document identity, section kind (`prologue`/`epilogue`), and slot
  index.
- **R6 — Empty `::file-links` result.** Always a successful skip in both
  modes: remove the directive, increment `transclusions_skipped`, no
  tolerated-failure warning. The `fail_fast`-dependent
  `_No matching files_` notice branch (`engine.rs:1279-1281`) and its string
  are deleted.
- **R7 — Counter visibility.** `transclusion_failures_tolerated` appears in
  `ComposeReport::summary()` alongside the other counters (human-facing,
  explicitly unstable prose), and the CLI renders its own single
  `N transclusion failures tolerated` stderr summary after the individual
  warnings. Notice wording and warning messages are not machine APIs.

### Spikes

- **Spike 1 — Preflight omission map.** Enumerate every `Err` path in
  `preflight/collect.rs`'s recursive walk (target resolution near
  `collect.rs:428`, frontmatter references near `collect.rs:507-520`, remote
  child fetch, parse failures) and classify each as suppressible vs fatal
  per §4 and R1. Design the shared suppressibility classifier signature the
  engine result loop and the preflight walk will both call, and confirm a
  suppressed child can be omitted without recording a
  `PreflightGraphNode` edge while successfully inspected children still
  contribute all commands regardless of `when=`. Deliverable: a short design
  note in this fix's directory plus the agreed classifier signature.
- **Spike 2 — Ordering and dedup under concurrency.** Verify that the rayon
  `into_par_iter().collect::<Vec<_>>()` result vector preserves
  prepared/source order (so the sequential apply loop's first `Err` is the
  stable strict error), and design the family-key dedup so nested report
  merges deduplicate repeated projection without collapsing distinct
  failures or inflating the counter (R3). Deliverable: a design note and
  micro-test sketch covering repeated runs.

### Work-Group A — Spikes (concurrent with Work-Group B)

- [ ] **Run Spike 1**
  - Walk `preflight/collect.rs` end to end; produce the suppressibility
    classification table and the shared classifier design.
  - Confirm the `UrlExecutionDisabled` `continue` precedent
    (`collect.rs:439`, `collect.rs:516`) as the shape for tolerant omission.
- [ ] **Run Spike 2**
  - Pin down the strict first-error selection and tolerant report ordering
    guarantees; produce the merge/dedup design for R3.

### Work-Group B — Repository audit (concurrent with Work-Group A)

- [ ] **Audit compose invocations**
  - Inventory every `md compose` invocation in repository justfiles,
    GitHub workflows, `scripts/`, fixtures, and documentation; flag any
    using `--allow-missing-transclusions` or relying on today's
    warning-not-error transclusion behavior.
  - Inventory every `ignore_invalid` frontmatter key, `IGNORE_INVALID` env
    reference, and `with_ignore_invalid_references`/`ignore_invalid_references`
    API use (known: `lib/src/markdown/compose/context/options.rs`,
    `engine.rs`, `type_tests.rs`, `compose/tests/transclusion.rs`,
    `schemas/simplified/mod.rs:826,875`, `docs/schemas/*`,
    `docs/transclusion/*`).
  - Check downstream consumers (claudine uses `ComposeOptions` but none of
    the removed APIs — verify) and list claudine fixtures whose documents
    contain broken transclusions that the strict default would now fail.
- [ ] **Fix-or-file inventory**
  - For each broken repository-owned transclusion found, record whether it
    will be fixed (preferred) in Phase 5 or genuinely needs tolerance in CI.

**Phase 1 validation:** both spike notes and the audit inventory exist in
this fix's directory; every ruling above is either confirmed or escalated
before Phase 2 begins.

## Phase 2 — Policy and Report Model (Library Core)

Build the request-scoped policy and the structured reporting it produces.
No engine behavior changes yet. Depends on Phase 1 rulings (R2, R3, R4).

- [ ] **Add policy field**
  - Add `allow_transclusion_failures: Option<bool>` (private representation)
    to `ComposeOptions` in `lib/src/markdown/compose/context/options.rs`
    with `with_allow_transclusion_failures(bool)` builder, `Debug`
    coverage, and default `None`.
  - Document the precedence chain and the "no frontmatter property, ever"
    authority rule on the field/builder docs.
- [ ] **Root-boundary resolution**
  - Implement the resolution helper (explicit > captured
    `ComposeContext` env > `false`) with boolish parsing and a typed error
    naming `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES` for invalid values
    (R2, R4); wire it into the root pipeline entry so the resolved `bool`
    is stamped once and inherited by child options (the
    `render_markdown_transclusion` clone path at `engine.rs:1452-1470` and
    the `as_markdown` path already thread options — verify, don't recapture).
  - Ensure no stage reads ambient `std::env`.
- [ ] **Extend report model**
  - Add `ComposeReport::transclusion_failures_tolerated` with merge
    accumulation; add the family/dedup key to `ComposeWarning` plus a
    constructor for the tolerated-failure warning (stage `transclusion`,
    source `darkmatter.compose`, code `dm.transclusion.tolerated_failure`,
    line/path provenance); teach `ComposeReport::merge` to deduplicate on
    the family key while keeping counter and warning set consistent (R3).
  - Surface the counter in `summary()` (R7).
- [ ] **Classification authority**
  - Encode the new field in both the graph value fingerprint and the
    compose-cache fingerprint arms of the exhaustive
    `ComposeOptions` classification (`options.rs` encoders near lines
    2236/2387-2388/2673-2674) so tolerance changes graph-options identity
    and cache identity.
  - Update the identity type tests (`options.rs:3146+`) and the builder
    type tests (`compose/type_tests.rs`).

**Phase 2 validation:** `just test` (darkmatter) green including updated
type tests; a strict-vs-tolerant pair of options yields different graph
identity and cache fingerprint (unit test).

## Phase 3 — Strict Transclusion Engine

Rewire the transclusion stage to the new default. Depends on Phase 2;
consumes R1, R5, R6 and Spike 2's design.

- [ ] **Rework result loop**
  - In `pipeline/phases.rs:377-436`: strict mode returns the first failure
    in stable prepared/source order (no partial `Markdown`/`ComposeReport`);
    tolerant mode converts each suppressible failure into a
    `ResolvedTransclusion` carrying the `failure_anchor` notice fitted by
    `fit_notice_to_span`, incrementing
    `transclusion_failures_tolerated` (not `transclusions_skipped`) and
    recording the coded warning with provenance.
  - Shrink the `is_structural` forced-fatal set to the R1 boundary
    (missing runtime context via `error.missing_runtime_context()` and
    anchor-less invariants stay fatal in both modes).
- [ ] **Replace engine policy reads**
  - Replace every `resolve_ignore_invalid` use (`engine.rs:1641-1656`,
    including the `::file-links` discover-error path at `engine.rs:1296`)
    with the resolved tolerance policy; delete the `IGNORE_INVALID`
    environment read and `parse_bool` helper (the frontmatter read dies with
    it).
- [ ] **Empty file-links cleanup**
  - Delete the `fail_fast` branch (`engine.rs:1275-1293`): an empty
    discovery always removes the directive, increments
    `transclusions_skipped`, and emits no tolerated-failure warning (R6).
- [ ] **Frontmatter section omission**
  - A tolerated `prologue`/`epilogue` failure leaves the section slot
    `None` (existing `flatten` join already omits it) while recording the
    counter and slot-identity warning (R5).
- [ ] **Child inheritance proof**
  - Add tests proving the root policy propagates through recursive
    `::file`, `::url`, and `as_markdown` composition and that a child's
    frontmatter cannot weaken it (there is no property to read).
- [ ] **Decouple `fail_fast`**
  - Remove the transclusion engine's remaining `fail_fast` reads (engine +
    `toc_linking`); update the `fail_fast` field docs (`options.rs:96`) to
    describe it as non-transclusion compose leniency, enumerating its
    actual surfaces.

**Phase 3 validation:** targeted tests in
`lib/src/markdown/compose/tests/transclusion.rs`: strict `Err` for each
failure category, tolerant `Ok` + notice + counter + coded warning,
fatal-in-both categories, and skip categories unaffected.

## Phase 4 — Tolerant Preflight Walk

Make the condition-blind approval walk honor the same policy. Depends on
Phase 3 and Spike 1; **runs concurrently with Phase 5** as a work-group
(different files: `preflight/collect.rs` vs options/schema surfaces —
coordinate the shared classifier landing first).

- [ ] **Shared classifier**
  - Land the suppressibility classifier from Spike 1 as the single
    authority used by both the engine result loop and the preflight walk.
- [ ] **Tolerant child omission**
  - In `collect_recursive`, a suppressible resolution/load/fetch/
    child-compose failure under tolerance omits the child — no graph edge,
    no discovered commands, no user-facing warning, no counter increment —
    modeled on the `UrlExecutionDisabled` `continue` precedent.
  - Strict mode is unchanged; dynamic command shapes, missing runtime
    context, malformed directives, and other §4 failures remain fatal in
    both modes, even inside a currently false branch.
- [ ] **Graph integrity**
  - Omitted children contribute no `PreflightGraphNode` edge; successfully
    inspected children contribute all of their commands regardless of
    `when=` conditions.

**Phase 4 validation:** preflight tests covering tolerant omission (no
edge, no duplicate diagnostic), strict rejection inside false branches, and
the terminal compose pass owning the single tolerated-failure record.

## Phase 5 — Remove the `ignore_invalid` Chain

The atomic breaking removal. Depends on Phase 3 (call sites already
migrated); runs concurrently with Phase 4.

- [ ] **Remove options API**
  - Delete `ignore_invalid_references`, `with_ignore_invalid_references`,
    and every classification/encoder/Debug reference
    (`options.rs:170,1155,1233,2236,2387-2388,2673-2674`); update
    `compose/type_tests.rs:213-231`.
- [ ] **Remove schema entries**
  - Drop `ignore_invalid` from the baseline schema
    (`lib/src/markdown/schemas/simplified/mod.rs:826,875`) and update the
    shipped `docs/schemas/darkmatter.yaml`, `docs/schemas/darkmatter-schema.md`,
    and generated schema documentation.
- [ ] **Remove key and env var**
  - Delete the frontmatter-key read and `IGNORE_INVALID` env lookup
    (already gutted in Phase 3 — remove the dead code); `rg` the whole
    repository for both spellings including docs and justfiles.
- [ ] **Fix repository usages**
  - Rewrite `compose/tests/transclusion.rs:225,2255` to the new policy;
    fix (not tolerate) repository-owned broken transclusions found in the
    Phase 1 audit; update any justfile/workflow/script that passed
    `--allow-missing-transclusions` or `IGNORE_INVALID`.
  - Verify claudine still builds and its fixtures pass under the strict
    default (`just test claudine` from the repo root).

**Phase 5 validation:** `rg -n "ignore_invalid|IGNORE_INVALID"` returns
only historical spec/plan documents; `just test` and `just lint` green.

## Phase 6 — CLI Gates and Diagnostics

One resolved value across validation, preflight, and execution. Depends on
Phases 2-5.

- [ ] **Flag rename and help**
  - Replace `allow_missing_transclusions` with
    `allow_transclusion_failures` in `cli/src/args/command.rs:160-166` and
    the `ComposeAllowFlags` wiring in `cli/src/commands/mod.rs:131-135`;
    `--allow-any-missing-reference` implies it and its help text states the
    broader consequence (any later transclusion failure is tolerated).
  - Keep `--allow-missing-hyperlinks`/`--allow-missing-image-refs`
    semantics untouched.
- [ ] **Early policy resolution**
  - In `run_compose` (`cli/src/commands/compose.rs`), resolve the policy
    from the flag plus the *captured* environment before reference
    validation; an invalid value fails before validation, preflight, or
    composition begins; the same resolved value configures all three gates
    (no ambient `std::env` re-read, no env bypass of validation deferral).
- [ ] **Validation deferral**
  - Tolerant requests defer transclusion-kind reference-validation errors
    (`compose.rs:353-400`) so composition can produce the degraded
    artifact; strict requests keep today's early exit.
- [ ] **Preflight wiring**
  - Hand the resolved tolerance to the preflight options so
    `compose_preflight_approvals` (`compose.rs:494-515`) uses the identical
    value.
- [ ] **Strict error hint**
  - On a strict transclusion error, render a terminal-component hint naming
    `--allow-transclusion-failures` and
    `DARKMATTER_ALLOW_TRANSCLUSION_FAILURES` alongside the typed error's
    styled block; the library error stays CLI-agnostic.
- [ ] **Degraded-run summary**
  - After the individual warnings, print exactly one stderr summary (for
    example `2 transclusion failures tolerated`) while preserving the
    selected stdout format.
- [ ] **Help pin updates**
  - Update pinned `md compose --help` tests for the new flag and the
    `--allow-any-missing-reference` wording.

**Phase 6 validation:** `CliProcessFixture`-based CLI tests: flag alone
suffices for an unresolved target (defer → omit → degrade); no flag/env →
nonzero exit, no stdout document; invalid env fails clearly; boolish
true/false covered through the fixture's captured environment; degraded run
prints warnings plus exactly one summary.

## Phase 7 — Regression and Determinism Hardening

Full test matrix from the spec's Testing section. Library portions can
start after Phase 3; CLI portions depend on Phase 6.

- [ ] **Library behavior matrix**
  - Every failure category (body directives, `prologue`/`epilogue`, local
    and permitted remote targets, child schema/expression/shell/render,
    cycle, depth) returns `Err` by default and `Ok` with correct
    notice/omission under tolerance; runtime-context and invariant
    failures error in both modes.
  - Counter/warning invariants: tolerated failures increment
    `transclusion_failures_tolerated`, never `transclusions_skipped`, and
    record the stable metadata; skips and nullable-target omissions touch
    neither counter in either mode.
- [ ] **Dedup and merge tests**
  - Distinct tolerated failures sharing the code survive report merges;
    repeated projection of the same source location deduplicates without
    inflating the counter; nested reports merge in stable prepared/source
    order.
- [ ] **Determinism tests**
  - Multiple concurrent failures select and report the stable
    prepared/source first-failure across repeated runs (Spike 2 design).
- [ ] **Propagation and identity tests**
  - Root policy through recursive composition; explicit library `false`
    overrides a true captured environment; graph-options identity and
    compose-cache fingerprint separate strict and tolerant requests.
- [ ] **Regression conversions**
  - Convert `lib/tests/declined_path_transclusion.rs` to assert the strict
    default, preserving its notice assertions under tolerant mode
    (Windows-gated; follow the `os` skill for producing evidence).
  - Update `::file-links` tests to drop `fail_fast`-dependent empty-result
    expectations; preserve nullable-target tests for `::file`, `::code`,
    and `::url` in both modes.
  - Add the denied-host regression: host policy is checked before any
    transport-cache read even when failures are tolerated (companion to
    `denied_host_never_reads_a_fresh_cached_entry`).

**Phase 7 validation:** `just test` and `just lint` in `darkmatter` fully
green; no `fail_fast`-shaped transclusion expectations remain.

## Phase 8 — Documentation, Skill, and Final Verification

Depends on all prior phases. Docs tasks are parallelizable as a work-group.

### Work-Group A — Documentation surfaces (concurrent)

- [ ] **Update compose docs**
  - `darkmatter/README.md`, `md compose --help`, `docs/cli/compose.md`,
    `docs/topics/transclusion.md`, `docs/transclusion/block-transclusion.md`,
    `docs/transclusion/transclusion-design.md`,
    `docs/transclusion/fm-transclusion.md`, `docs/inline/file-links.md`,
    `docs/structs/Markdown.md`.
- [ ] **Update policy docs**
  - `docs/inline/preflight-checks.md` (tolerant omission, fail-closed
    classes), `docs/topics/schema-definition.md` and the shipped/generated
    schema docs (`ignore_invalid` removal).
  - Everywhere: distinguish successful skips from tolerated failures,
    document the counter/code, state single resolution per request, and
    stop presenting `fail_fast` as the transclusion control.
- [ ] **Update agent skill**
  - `.claude/skills/darkmatter/compose.md` (and `errors.md` if the warning
    conventions section needs the new coded family).

### Work-Group B — Final verification (after Work-Group A)

- [ ] **Close-out checks**
  - `just test`, `just lint`, `just build` in `darkmatter`; `just test
    claudine` for the downstream consumer.
  - Repository-wide `rg` for `ignore_invalid`, `IGNORE_INVALID`,
    `allow-missing-transclusions`, and `allow_missing_transclusions`
    returns nothing outside historical specs/plans.
  - Run GitNexus `detect_changes` (scope `all`) and review the affected
    flows before handing off; confirm no workspace-wide Cargo gates were
    run for this Darkmatter-area change.
  - Confirm comment/doc drift pass: every behavior-changing symbol's `///`
    docs were touched in the same change.

**Phase 8 validation:** all documentation surfaces describe the new
default and single policy; full verification checklist above passes; the
fix is implementation-complete and ready for review (the author moves it to
`_completed` — never an agent).
