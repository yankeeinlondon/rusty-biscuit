---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T07:18:31-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
next: 2026-07-13-file-resolution/review-4.md
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-3.md
previous: 2026-07-13-file-resolution/review-2.md
---

# Review 3: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Review 2's six findings remain
present in the current implementation. No implementation commit after Review 2
changed the resolver, completion, harness-diagnostic, request-context, or
file-reference documentation locations named by that review; inspection of the
current tree confirms the same behavior rather than relying on history alone.

The shared `biscuit-file` foundation remains materially sound for
non-recursive resolution, but Claudine has not completed the integration the
specification requires. The real completion producer still has a private
grammar and a root order that differs from execution; recursive interpolation
still bypasses effective anchoring and sigil rejection; I/O failures still lose
their candidate plan at the harness boundary; package-area and immutable
request context are still absent from most document-backed surfaces; and the
public documentation remains incomplete and partially incorrect.

## Findings

### 1. High — Claudine completion still does not use the shared parser, context, or candidate builder

The production composition completer still classifies tokens with its own
`PartialKind::classify`, including a manual `strip_prefix('@')`, then dispatches
through its private `ScopeSet` walker
(`claudine/cli/src/completion/composition/mod.rs:79-106,140-156`). No Claudine
production call reaches `FileReference::complete_partial_in_context`; its only
callers remain inside `biscuit-file`.

The private root orders are observably different. Completion searches repo
prompts before package-area and discrete-package prompts
(`claudine/cli/src/completion/scopes.rs:169-176,280-325`). Execution registers
package-area roots before repo prompt roots and omits the discrete package's
`prompts/` directory
(`claudine/lib/src/composition/resolve.rs:262-306`). It also resolves via the
ambient `FileReference::resolve()` path
(`claudine/lib/src/composition/resolve.rs:39-54`). Consequently, a collision can
complete to one file and execute another, while a package-only completion can
emit an `@name` that execution cannot resolve.

The existing completion tests validate the private completer's own behavior,
and `biscuit-file/lib/tests/completion_round_trip.rs` validates a synthetic
consumer. There is still no Level 1 subprocess test that takes a value from the
real `claudine __complete` producer and executes that exact value through the
real composition resolver. This is a D1/D9 and Acceptance Criteria 1/7 gap.

**Required change:** construct one `FileResolutionContext` from the completion
`ScopeContext`, use `complete_partial_in_context`, and make runtime composition
resolution consume the same ordered magic roots. Add a real
`claudine __complete` → composition-resolution subprocess round trip with both
a repo/package-area collision and a discrete-package-only prompt.

### 2. High — Recursive interpolation still bypasses effective anchoring and injected-sigil rejection

`compute_effective_anchoring` still returns `None` immediately when
`parsed.recursive` is true
(`biscuit-file/lib/src/file_reference/resolve.rs:168-188`). The effective-kind
reclassification and `injected_sigil` rejection therefore apply only to
non-recursive references.

This makes `%{{ROOT}}/plan.md` behave differently from the recursive form of the
path produced by `ROOT` when that value is absolute. It also allows an
interpolated leading `@`, `!`, `%`, `vault:`, or HTTP(S) scheme to evade the
author-controlled-sigil rule whenever `%` was authored. The test suite has
non-recursive interpolation cases and recursive grammar/root-plan cases, but no
test combining recursive resolution with OQ1 reclassification or injected
sigils. This violates D1/D3, OQ1 option 2, and Acceptance Criteria 7/9.

**Required change:** compute effective local anchoring after interpolation for
recursive and direct references alike, reject injected grammar sigils before
either resolution path builds roots, and add Level 1 cases for recursive
absolute, explicit-relative, and implicit-relative expansion plus every
prohibited sigil.

### 3. High — I/O failures still discard the detailed candidate plan before diagnostics

`resolve_harness_path` creates `ResolutionDetail` before projecting the shared
outcome, but preserves it only in the `Ok(None)` arm. The `Err(error)` arm calls
`unresolvable` without the detail
(`claudine/lib/src/harness/resolve.rs:74-96`).
`HarnessError::FileReferenceUnresolvable` still stores only the authored
reference, source path, and boxed error
(`claudine/lib/src/harness/error.rs:199-220`), and its diagnostic projection
leaves `kind`, `repository_root`, and `candidates` null
(`claudine/lib/src/harness/error.rs:366-380`).

The shared resolver's Level 1 I/O test proves the failing candidate and typed
source are available before this boundary. The downstream adapter still drops
that record, so `permission_io` cannot expose the attempted candidate or prior
probes through `err.detail.*`. This violates D8 and Acceptance Criteria 8/13.

**Required change:** attach the owned `ResolutionDetail` to the I/O/error
variant as well as no-match, project all available fields identically, and add
a Claudine Level 1 diagnostic test that asserts authored/effective kind,
repository root, ordered candidates, terminal I/O disposition, and failure
slug.

### 4. High — Package-area context and missing-package classification still diverge by surface

`HarnessResolutionContext` still contains only `source_path` and `repo_root`,
and its adapter constructs a fresh `FileResolutionContext` without
`with_package_area`
(`claudine/lib/src/harness/resolve.rs:22-30,105-125`). Darkmatter's shared
`document_resolution_context` likewise accepts no package-area input and never
sets one (`darkmatter/lib/src/markdown/compose/util.rs:82-102`). Only the
external-sequence adapter supplies a package area.

For `!foo`, those surfaces therefore use the repository-root fallback while a
sequence can use the actual package area. The same authored reference can
resolve to different files across lifecycle proxy, sequence, schema,
expression, transclusion, and link surfaces. In addition,
`MissingPackageContext` is still absent from `file_reference_failure_slug`, so
a package reference with neither anchor defaults to `invalid_syntax` instead of
`missing_context` (`claudine/lib/src/harness/error.rs:392-406`). The supposedly
exhaustive mapping test does not include this variant.

This violates D2/D3 and Acceptance Criteria 1/6/12.

**Required change:** capture the package area once and derive it into every
nested document context. Add package-area-versus-repository collision fixtures
for each adapter, map `MissingPackageContext` to `missing_context`, and make the
error-vocabulary test explicitly cover every current variant and expected slug.

### 5. High — Resolution context remains per-call and ambient, not immutable per request

The specification requires Claudine to capture CWD, HOME, environment,
repository, package area, magic roots, and vault roots once, then derive only
the authoring source/base for nested documents. The current paths continue to
rebuild those inputs:

- Top-level composition uses `FileReference::resolve()` and
  `with_prompt_magic_paths`, which reread current CWD and rediscover repository,
  package area, and home (`claudine/lib/src/composition/resolve.rs:39-54,262-279`).
- Each harness reference calls `FileResolutionContext::new`, taking a new
  environment/home snapshot (`claudine/lib/src/harness/resolve.rs:105-125`).
- Darkmatter's document seam constructs another new context and rediscovers a
  repository when the optional root is absent or out of bounds
  (`darkmatter/lib/src/markdown/compose/util.rs:82-102`).
- Expression and frontmatter contexts independently rediscover repository and
  home (`darkmatter/lib/src/markdown/compose/context/options.rs:873-910`).

There is no derive API that preserves a request snapshot while changing only
`source_path` and `base_dir`. Environment, HOME, CWD, or filesystem discovery
changes during one composition can therefore alter later references. This
violates D2/D10 and Acceptance Criteria 12/14.

**Required change:** introduce an immutable request resolution snapshot at the
Claudine preparation boundary, derive child contexts without ambient reads,
and thread it through top-level composition, lifecycle, sequence, schema,
expression, transclusion, and link resolution. Add Level 1 tests that mutate
CWD/environment after capture and prove every surface retains the original
snapshot while nested documents change only their authoring base.

### 6. Medium — Public topic and skill documentation still omit and misstate the shipped API

The authoritative topic's method table still omits `FileResolutionContext`,
`resolve_in_context`, `resolve_detailed`, candidate-plan/probe types, and
`complete_partial_in_context`
(`biscuit-file/docs/topics/file-references.md:355-368`). Its error table omits
`BareRepository`, `UnsupportedUserHome`, `MissingHomeContext`,
`MissingPackageContext`, `RepositoryRootNotContainingSource`, and remote
variants (`:403-416`). The skill reference mentions the context APIs briefly
but repeats the incomplete error vocabulary
(`.claude/skills/biscuit-file/references/file-references.md:100-117`).

The topic also states without qualification that interpolated grammar sigils
are rejected, while Finding 2 shows that recursive references bypass that rule.
Its resolution algorithm says direct candidates are checked with `is_file()`,
which contradicts the fallible metadata probing required by D8 and implemented
in the resolver.

This leaves documentation, skill guidance, implementation, and tests out of
agreement, violating Acceptance Criterion 7.

**Required change:** document the explicit-context lifecycle, ambient versus
authoritative entry points, detailed outcomes and probe dispositions,
completion API, full error vocabulary, and fallible probing. Correct the
recursive interpolation claim when the implementation is fixed.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Parser classification, non-recursive interpolation, explicit/implicit precedence, deduplication, direct probing, and home behavior | Level 1 `biscuit-file` unit/integration tests | Appropriate and green. Recursive interpolation remains untested and broken (Finding 2). |
| Motivating bare proxy resolution and paired `./` source pinning | Level 2 tmux tests in `level2_file_resolution_capture.rs` | Appropriate test level. The correctly invoked L2 tier did not reach all tests because an unrelated context-rendering test failed first. |
| Ordered no-match candidates render in a real terminal | Level 2 tmux test `level2_implicit_no_match_lists_two_ordered_candidates_in_tmux` | Appropriate test level and source assertions. Current full-tier evidence is incomplete due to the earlier unrelated gate failure. |
| Proxy routes share diagnostic identity and typed no-match detail | Level 1 harness tests plus Level 2 tmux route-parity test | Appropriate for no-match. Permission/I/O detail is still dropped and lacks downstream Level 1 assertions (Finding 3). |
| Schema, expression, sequence, transclusion, and link resolution share deterministic semantics | Level 1 adapter/process tests | Level 1 is appropriate, but package-area and immutable-request parity are absent (Findings 4-5). |
| A completion value executes unchanged through the same real producer/consumer candidate builder | Synthetic Level 1 `biscuit-file` round trip only | **Gap:** the real Claudine producer uses another grammar and no real subprocess round trip exists (Finding 1). |
| Native macOS/Linux/Windows reference classification and home behavior | Host-independent Level 1 parser tests, POSIX runtime test, target-gated Windows test | Appropriate for path semantics. Native Windows runtime evidence was not produced on this macOS host. |
| Package gates required by Acceptance Criterion 10 | `biscuit-file just test` green; targeted Darkmatter/Claudine L1 green; full Darkmatter gate incomplete; Claudine L2 gate red | Not satisfied. The L2 failure is outside this feature but the required package gate is not green. |

Level 3 is not applicable. This feature makes no claim about OS keyboard or
mouse events, terminal input encoding, paste, IME, or hotkey activation.

## Verification Performed

- Read the full specification, Review 2, implementation plan/inventory,
  current shared resolver/context/completion implementation, Claudine
  completion and harness adapters, Darkmatter document-resolution seams,
  public/skill documentation, and feature-specific tests.
- Confirmed with GitNexus that `document_resolution_context` feeds expression,
  link, transclusion, and schema resolution, while the shared completion API has
  no Claudine production caller.
- `biscuit-file/just test` passed: 367 library/integration tests and 61 CLI
  tests.
- A feature-focused Darkmatter Level 1 filter passed 28/28 tests.
- A feature-focused Claudine/CLI filter passed all 92 selected tests. Five were
  L2-named tests selected by the raw filter; they are not counted as compliant
  L2 evidence because L2 must run through the area recipe. The remaining 87
  selected Level 1 tests passed.
- `darkmatter/just test` compiled and reached 1,775 passing tests with no
  failures before the non-interactive time ceiling; 3,855 tests were not run,
  so this is incomplete evidence rather than a passing or failing gate.
- `claudine/just test-l2` used the canonical parallel self-spawn recipe. It
  stopped after 40 passes when
  `level2_context_default_at_140_fills_cap_in_tmux` failed on a 140-cell row;
  105 tests were not run. The failure is unrelated to file resolution but
  means the L2 gate is not green.
- Lint gates and native Windows runtime tests were not rerun because the static
  high-severity blockers already preclude production readiness.
- Biscuit-file parsing confirmed the requested review frontmatter values.
  `md schema validate` could not validate the document because the repository's
  `schemas/feature-review.yaml` is itself rejected as a standalone
  SimplifiedSchema (`kind: schema` currently has unsupported `description` and
  `$schema` siblings); this is schema-tooling drift, not a frontmatter parse
  failure in this review.
- Preserved the caller's unrelated modifications to `.claudine/memory/commits.md`
  and `CLAUDE.md`.

## Production Readiness Closure

Production readiness requires all five high findings to close, the
documentation finding to be updated, and complete green `just test`,
`just lint`, and relevant `just test-l2` evidence in `biscuit-file`,
`darkmatter`, and `claudine`. In particular, closure must include the real
`claudine __complete` round trip, recursive OQ1 tests, downstream I/O-detail
assertions, package-area collision fixtures, and immutable request-snapshot
tests.
