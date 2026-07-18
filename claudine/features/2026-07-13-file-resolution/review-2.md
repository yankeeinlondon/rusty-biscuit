---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T18:55:35-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: false
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-2.md
previous: 2026-07-13-file-resolution/review-1.md
---

# Review 2: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Repository-first implicit
resolution, explicit-relative pinning, fallible direct probes, native home
discovery, schema-root isolation, no-match diagnostic detail, and the required
real-terminal candidate rendering are implemented. Review 1's findings in
those areas are closed in the current tree.

The remaining gaps are contract-level rather than polish: Claudine's real
completion pipeline does not use the shared completion/candidate builder and
can emit an `@name` that executes as a different file or does not resolve;
recursive interpolation bypasses the ratified effective-anchoring and
sigil-injection rules; detailed I/O failures discard their candidate record at
the harness boundary; package-area context is not carried by most nested
surfaces; and the purported request-scoped context still captures ambient state
per resolution. These violate D1/D2/D8/D9 and Acceptance Criteria 1, 6, 7, 8,
12, and 13.

## Findings

### 1. High — Production completion and execution still use different grammars and root orders

The new `FileReference::complete_partial_in_context` API is exercised only by
`biscuit-file` tests; no Claudine production caller uses it. Claudine still
classifies `@` itself in `PartialKind::classify`, walks a private `ScopeSet`,
and constructs `@<basename>` directly
(`claudine/cli/src/completion/composition/mod.rs:81-106` and
`magic_at.rs:34-83`). Its magic order is repository prompts, package-area
prompts, package prompts, repository `.claudine`, extras, then user prompts
(`completion/scopes.rs:161-177`).

Execution builds a different root list: package-area root, package-area
`prompts`, repository `prompts`, repository `.claudine/prompts`, and user
prompts; it does not include the discrete package prompt root
(`claudine/lib/src/composition/resolve.rs:125-169`). This produces two concrete
failure modes:

- With the same basename in repository and package-area prompt directories,
  completion displays the repository candidate first but execution resolves
  the package-area candidate.
- A file found only in a discrete package's `prompts/` directory can be emitted
  as `@name`, while execution has no corresponding root and fails to resolve it.

The new round-trip test (`biscuit-file/lib/tests/completion_round_trip.rs`) tests
a synthetic consumer of the new API, not `claudine __complete` followed by the
real composition resolver. D9 and Acceptance Criterion 7 require the actual
producer and consumer to share the context and candidate builder.

**Required change:** build the Claudine completion context once from
`ScopeContext`, call `complete_partial_in_context` for supported file-reference
forms, and make runtime prompt resolution consume the same ordered roots. Add an
L1 subprocess round trip that takes an actual `claudine __complete` value and
executes it unchanged, including repository/package-area collisions and a
package-only magic candidate.

### 2. High — Detailed permission/I/O failures lose the candidate plan before diagnostics

`resolve_harness_path` correctly creates `ResolutionDetail` from the shared
`DetailedResolution`, but attaches it only to the `Ok(None)` no-match arm. The
`Err(error)` arm calls `unresolvable` and discards the detail
(`claudine/lib/src/harness/resolve.rs:74-95`).
`HarnessError::FileReferenceUnresolvable` has no resolution-detail field
(`claudine/lib/src/harness/error.rs:199-220`), and its diagnostic projection
therefore populates only reference, failure, and source path
(`harness/error.rs:366-380`).

The shared resolver already retains the failed candidate and its `Io`
disposition—`detailed_resolution::io_probe_failure_stops_with_typed_error_identifying_candidate`
proves that—but Claudine drops it. For a permission or invalid-path failure,
`err.detail.kind`, `repository_root`, and `candidates` are consequently null,
and the terminal cannot identify the attempted path/provenance from structured
data. This fails D8 and Acceptance Criteria 8 and 13.

**Required change:** carry an owned detailed-resolution projection on the
resolution-error arm as well as no-match. Preserve authored/effective kind,
repository root, every prior probe, and the terminal I/O candidate. Add L1
assertions on the full `err.detail.*` payload and a platform-appropriate L1
probe fixture that produces a non-`NotFound` I/O error.

### 3. High — Recursive interpolation bypasses effective anchoring and sigil-injection rejection

`compute_effective_anchoring` returns `None` immediately for every recursive
reference (`biscuit-file/lib/src/file_reference/resolve.rs:177-183`). That
means the OQ1 reclassification and `injected_sigil` rejection are never applied
when `%` is present. A reference such as `%{{ROOT}}/plan.md` with an absolute
`ROOT` remains authored-implicit and searches the repository/base roots instead
of behaving like the equivalent recursive absolute reference. Likewise an
environment value beginning with `@`, `!`, `%`, `vault:`, or an HTTP(S) scheme
is not rejected on the recursive path.

The specification says recursive is a modifier over a kind and ratifies OQ1
option 2 after interpolation; it does not exempt recursive references. Existing
interpolation tests cover only non-recursive references, while the grammar tests
only prove recursive classification before interpolation.

**Required change:** compute the effective local anchoring for recursive
references too, reject injected grammar sigils consistently, and feed the
effective kind into recursive root construction. Add L1 cases for recursive
absolute interpolation, recursive relative interpolation, and each prohibited
injected sigil.

### 4. High — Package-area context is not shared across document-backed surfaces

The last implementation commit made `FileResolutionContext.package_area`
authoritative, and the external-sequence adapter now captures it
(`claudine/lib/src/composition/sequence.rs:129-153`). Most other in-scope
surfaces cannot supply it:

- `HarnessResolutionContext` contains only source path and repository root, and
  `build_resolution_context` never sets a package area
  (`claudine/lib/src/harness/resolve.rs:22-30,99-125`).
- Darkmatter's `document_resolution_context` has no package-area input and
  never calls `with_package_area`
  (`darkmatter/lib/src/markdown/compose/util.rs:82-102`).

On an explicit context with a repository root but no package area, `!foo`
silently uses the repository root (`biscuit-file/lib/src/file_reference/resolve.rs:671-679`).
The same authored package reference can therefore resolve from the package area
in a sequence but from the repository root in lifecycle proxy, expression,
schema, transclusion, or link resolution. This violates D2/D3, cross-surface
parity in Acceptance Criterion 6, and the package semantics Acceptance
Criterion 1 delegates to `FileReference`.

There is a related diagnostic regression: the new
`FileReferenceError::MissingPackageContext` is absent from
`file_reference_failure_slug`, so a genuinely missing package anchor falls
through to `invalid_syntax` instead of `missing_context`
(`claudine/lib/src/harness/error.rs:392-406`). The existing exhaustive-looking
test samples neither this new variant nor the semantic slug expected for each
variant (`harness/error/tests.rs:325-356`).

**Required change:** capture package area once in the request context and derive
it into every nested document context. Do not silently substitute the repository
root when a known package-area context was lost. Map `MissingPackageContext` to
`missing_context`, and add cross-surface L1 collision tests where distinct
package-area and repository-root files prove which anchor won.

### 5. High — The document context is still captured per resolution rather than per request

The explicit shared resolver no longer performs discovery after receiving a
`FileResolutionContext`, which closes the narrow shared-library portion of
review 1 finding 2. The Claudine/Darkmatter call sites still do not meet D2/D10
or Acceptance Criterion 12:

- `FileResolutionContext::new` snapshots live home and the entire process
  environment (`biscuit-file/lib/src/file_reference/context.rs:142-158`).
- Each harness resolution constructs a fresh context
  (`claudine/lib/src/harness/resolve.rs:99-125`).
- Darkmatter's `document_resolution_context` constructs another fresh context
  and re-runs Git discovery whenever its optional cached root is absent or does
  not contain the nested base (`darkmatter/lib/src/markdown/compose/util.rs:82-102`).
- Expression/frontmatter contexts separately rediscover the repository root and
  home (`darkmatter/lib/src/markdown/compose/context/options.rs:863-901`).

Environment or home changes between two references in one composition can
therefore change `{{VAR}}`, `~`, magic HOME, or vault behavior, and nested
documents outside the original root trigger new discovery rather than deriving
from the captured trust boundary. The implementation is ambient-free only
*after each individual context construction*, not after the request context is
captured as the specification requires.

**Required change:** capture environment, home, repository, package area,
magic, and vault roots once at the Claudine preparation boundary. Add a derive
operation that changes only `source_path`/`base_dir` for nested documents. Add
L1 tests that mutate CWD/environment after request capture and prove every
surface continues using the original snapshot.

### 6. Medium — Public topic and skill documentation omit the new detailed/context API

The authoritative topic page still documents only the legacy convenience
methods and an incomplete error table. It does not document
`FileResolutionContext`, `resolve_in_context`, `resolve_detailed`,
`candidate_plan`, `complete_partial_in_context`, or the new missing-context
errors (`biscuit-file/docs/topics/file-references.md:346-390`). The biscuit-file
skill reference briefly names the context/detailed methods but its error list
also omits `MissingHomeContext`, `MissingPackageContext`,
`UnsupportedUserHome`, `RepositoryRootNotContainingSource`, and remote errors
(`.claude/skills/biscuit-file/references/file-references.md:105-118`).

Acceptance Criterion 7 explicitly requires documentation, skill guidance,
implementation, completion, and tests to agree.

**Required change:** document the explicit-context lifecycle, ambient versus
authoritative entry points, detailed outcomes/candidate provenance, completion
API, and complete error vocabulary in both authorities.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Parser classification, explicit/implicit precedence, deduplication, direct probing, home behavior, and non-recursive interpolation | Level 1 unit/integration tests in `biscuit-file` | Appropriate and green in this review. Recursive interpolation remains uncovered and broken (Finding 3). |
| Motivating bare proxy resolves repository-first; paired `./` proxy never falls back | Level 2 tmux in `level2_file_resolution_capture.rs` | Appropriate. The fixtures discriminate the two forms end-to-end. |
| Missing implicit reference renders two candidates in repository-then-source order | Level 2 tmux in `level2_implicit_no_match_lists_two_ordered_candidates_in_tmux` | Appropriate for the terminal-rendering claim. Permission/I/O detail has only shared-library L1 evidence and is dropped downstream (Finding 2). |
| Proxy routes share error identity and typed detail | Level 2 tmux route-parity test plus L1 harness tests | No-match parity is covered. The I/O-failure branch lacks structured parity because the candidate record is discarded. |
| Schema, expression, sequence, transclusion, link, and nested-document resolution share one contract | Level 1 adapter/process tests, with Level 2 only for rendering cases | Level 1 is appropriate for deterministic semantics, but package references and request snapshot behavior are not shared (Findings 4-5). |
| Completion emits a value that executes unchanged through the same candidate builder | Synthetic Level 1 round trip inside `biscuit-file` | **Gap:** the real Claudine producer does not call that API. An actual `claudine __complete` → execution L1 process round trip is absent; this user-observable mismatch is high severity (Finding 1). |
| Native macOS/Linux/Windows path and home behavior | Level 1 host-independent parser tests, POSIX home test, target-gated native-Windows home test, and Windows cross-compile record | Appropriate for path semantics. The native-Windows runtime test was not runnable on this macOS review host. |
| Package gates | `biscuit-file just test` completed; Claudine full gate was started but stopped during cold compile | Incomplete independent gate evidence. Static high-severity blockers prevent readiness regardless. |

Level 3 is not applicable to this feature. It does not make a claim about a
physical key press, terminal input encoding, paste, IME, mouse input, or a
hotkey chord. Shell completion round-trip semantics can and should be verified
at Level 1 by driving the completion subprocess directly; the existing chooser
interaction suites retain their own L2/L3 responsibilities.

## Verification Performed

- Read the specification, plan, decisions, migration inventory, previous
  review, the two commits after review 1, shared resolver/context/completion
  implementation, Claudine harness and completion adapters, Darkmatter
  resolution seams, and feature-specific L1/L2 tests.
- `biscuit-file/just test` passed: 345 library/integration tests and 61 CLI
  tests.
- Started `claudine/just test`. The catalog crate completed 21/21 tests; the
  cold Claudine build exceeded the non-interactive command ceiling and was
  stopped cleanly with exit 130 before its tests ran. This is an incomplete run,
  not a product failure.
- Did not claim fresh Darkmatter, lint, or L2 gate results. Existing L2 test
  sources were inspected for requirement discrimination and level suitability.
- Preserved the pre-existing unrelated `CLAUDE.md` worktree modification.

## Production Readiness Closure

Production readiness requires all high findings to close: wire the real
completion producer to the shared context/candidate builder; preserve detailed
I/O candidate data through diagnostics; apply OQ1 to recursive references;
thread package area and one immutable request snapshot through every surface;
and add the named discriminating L1 tests. Then update the public/skill docs and
rerun complete `just test`, `just lint`, and relevant `just test-l2` gates in
`biscuit-file`, `darkmatter`, and `claudine`.
