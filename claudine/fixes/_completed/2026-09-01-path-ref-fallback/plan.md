---
$schema:
    yolo: boolean -> you only live once
total_phases: 5
created: 2026-09-01
phase: 1
agent: codex/default
yolo: true
---

# Execution Plan: Preserve Explicit Operation-File No-Match Diagnostics

Reference specification: [`spec.md`](../../../claudine/fixes/2026-09-01-path-ref-fallback/spec.md)

## Goal

Restrict operation-file autocomplete to unresolved bare discovery names and
report every explicit `compose`, `inline-compose`, and `sequence` reference miss
through the existing `composition.invalid_file_reference` diagnostic, retaining
the shared `FileReference` probe record and adding bounded repository-local
basename suggestions without changing resolution or shared matching semantics.

## Scope and dependency summary

The dependency order is: retain typed detailed-resolution data in the Claudine
library; build the pure recovery classifier and bounded suggestion service in
the CLI completion layer; route both compose entry seams through one policy;
then close L1/L2 and documentation parity before running package and repository
gates.

GitNexus reports **CRITICAL** upstream risk for
`resolve_composition_source_in_context` (5 direct callers and 200 symbols within
depth 3), **HIGH** risk for the compose preparation resolver (4 direct callers,
10 symbols within depth 3), and **HIGH** transitive risk for
`autocomplete_operation_file`. The implementation should therefore preserve
the public resolver's success and non-no-match contracts, review every direct
caller before editing, and avoid broadening shared `FileReference` resolution.

## Phase 1 — Baseline, impact audit, and failing regression matrix

**Outcome:** the current laundering defect is reproduced at the two entry seams,
the high-risk shared callers are audited, and unchanged behavior has explicit
controls.

- [ ] Record `git status --short` and preserve the existing modified specification and unrelated moved plan; limit this fix to the specified Claudine library/CLI, focused tests, synchronized completion docs, and this plan.
- [ ] Re-run GitNexus upstream impact analysis before editing `resolve_composition_source_in_context`, the CLI `resolve_composition_source`, `resolve_sequence_source`, `autocomplete_operation_file`, and any shared walker symbol; review all depth-1 callers, record the known CRITICAL/HIGH results, and stop for direction if new callers expand the specification's scope.
- [ ] Add focused L1 characterization tests for the current operation-file routing seams: direct compose, inline-compose, Markdown sequence, and YAML sequence must each expose the pre-fix autocomplete error for an explicit clean no-match.
- [ ] Add a table-driven eligibility regression matrix covering bare `access` and `access.md`; multi-component implicit paths; POSIX and Windows explicit-relative paths; POSIX absolute, Windows drive-absolute, and UNC forms; home, magic, package, vault, URL, recursive, and interpolation-bearing references.
- [ ] In the matrix, distinguish `ResolutionFailure::NoMatch` from invalid syntax, missing context, I/O, and unsupported remote outcomes; assert only a clean no-match can reach recovery policy and only the two bare implicit names are autocomplete-eligible.
- [ ] Capture unchanged autocomplete controls for zero matches, over-cap, cancellation, non-interactive rejection, single-match confirmation, and multi-match choice so later routing changes do not alter picker scopes, frontmatter gates, ranking, or UI behavior.
- [ ] **Validation checkpoint:** run the narrow new L1 tests and existing operation-file autocomplete tests; confirm explicit misses fail through the old autocomplete path for the expected reason while the existing picker controls remain green.

## Phase 2 — Retained no-match model and diagnostic parity

**Outcome:** a top-level operation-file miss retains the exact authored token,
captured resolution context, ordered probes, and suggestions under the existing
diagnostic identity.

- [ ] Evolve `CompositionError::FileNotFound` into a structured no-match representation, or introduce a structured semantic successor, while keeping `composition.invalid_file_reference`, `Disposition::Correctable`, and all existing diagnostic discovery behavior unchanged.
- [ ] Change `resolve_composition_source_in_context` to call `FileReference::resolve_detailed` once and convert its outcome without rebuilding candidates: preserve the matched-path behavior, map only `ResolutionFailure::NoMatch` to the structured no-match, and retain existing typed errors for invalid syntax, missing context, I/O, and unsupported remote references.
- [ ] Reuse or minimally extend `harness::ResolutionDetail` as the single typed projection for authored/effective kinds, base directory, repository root, and ordered `ProbedCandidate` values; do not add a second candidate/provenance vocabulary or parse display prose.
- [ ] Apply the same detailed-resolution conversion to the YAML branch of `resolve_sequence_source` so Markdown and YAML sources retain identical no-match evidence without changing YAML loading or extension validation.
- [ ] Populate every declared `composition.invalid_file_reference` detail key from `null_detail_for`: exact `reference`, `kind`, `effective_kind`, `base_dir`, compatibility fields, nullable `repository_root`, ordered `candidates`, `failure: no_match`, and an always-present `suggestions` array.
- [ ] Add one data owner for the final sorted suggestion strings and make both `Diagnostic::detail()` and terminal rendering consume it, guaranteeing byte-for-byte order parity and preventing render-time rescans.
- [ ] Render the error with `StatusBlock` plus `TerminalRenderable` `Prose`/`UnorderedList` components: name the authored reference and captured base directory, label candidates strictly from `RootProvenance` (explaining `source` as launch directory at this seam), and render all paths through `biscuit_file::to_portable_string`.
- [ ] Preserve TTY-independent diagnostic identity and substantive content; allow TTY state to influence styling only, and retain typed diagnostic discovery through all existing `color-eyre`/composition wrappers.
- [ ] **Parallelizable after the structured variant is fixed:** add library unit tests for detailed-outcome mapping, candidate order/provenance/disposition projection, null compatibility fields, human/detail parity, and portable Windows path rendering while the renderer is implemented.
- [ ] **Validation checkpoint:** run the focused `claudine` composition-error, diagnostic-registry, harness-resolution, and diagnostic-discovery tests; verify the catalog has no new code or keys and every unavailable declared value remains present as `null`.

## Phase 3 — Bare-name recovery policy and bounded suggestions

**Outcome:** one shared policy decides whether to invoke the existing picker or
enrich the explicit no-match, and suggestion discovery is deterministic,
repository-contained, and non-authoritative.

- [ ] Add a pure operation-file autocomplete eligibility helper that parses with `FileReference::new()`, reads `FileReference::class()`, and returns eligible only for non-recursive `ImplicitRelative` references whose original payload contains no `/`, `\\`, or `{{...}}` interpolation.
- [ ] Keep `path_matches_query` byte-for-byte unchanged; do not normalize `./`, reinterpret sigils, or change schema `file(match)` behavior.
- [ ] Define one recovery result/policy seam that accepts the original token plus the structured detailed no-match and returns either “attempt existing autocomplete” or “return enriched explicit no-match”; keep parse failures and all non-`NoMatch` failures outside this branch.
- [ ] Add a best-effort basename suggestion helper that runs only for an explicit no-match with a non-empty filename and captured effective repository root; compare the complete filename with exact case-sensitive equality and never feed results back into resolution.
- [ ] Reuse the completion walker's `.gitignore`, `_`-prefix, curated skip-list, and no-directory-symlink rules, extracting a shared filter/core only where needed; enforce a visited-entry budget of 20,000 and a match cap of five rather than reusing the autocomplete candidate cap.
- [ ] Normalize suggestion output with `to_portable_string`, make paths repository-relative, deduplicate, sort lexically, and return an empty list for absent roots/filenames, no hits, budget exhaustion, or walk errors without replacing the primary diagnostic.
- [ ] Add L1 walker tests for exact filename matching, lexical order, deduplication, five-result cap, 20,000-entry exhaustion, ignored trees, underscore-prefixed trees, curated skips, directory-symlink escape rejection, missing roots, and injected/read failures.
- [ ] Add cross-platform classifier tests that treat both separator styles and foreign absolute forms consistently on macOS/Linux/Windows without attempting to probe foreign-platform absolute paths.
- [ ] **Parallelizable after helper signatures are fixed:** implement the pure eligibility table and bounded-walker fixture matrix independently because neither changes the compose/sequence entry seams.
- [ ] **Validation checkpoint:** run the completion module's classifier/walker tests plus existing `path_matches_query` and schema completion suites; confirm shared matcher assertions remain unchanged and green.

## Phase 4 — Compose/sequence integration and terminal contract

**Outcome:** direct compose, inline-compose, Markdown sequence, and YAML sequence
all make the same recovery decision, and explicit misses behave identically with
and without a PTY.

- [ ] Replace the unconditional `CompositionError::FileNotFound` fallback in `commands/compose/prep.rs` with the shared recovery policy; keep the existing mode-specific picker and selected-file re-resolution for eligible bare names only.
- [ ] Replace the duplicate fallback in `commands/sequence.rs` with the same policy for both Markdown and YAML source branches; retain `ComposeMode::Sequence`, YAML conversion, source-derived context, and existing enrichment of errors after a picker selection.
- [ ] Ensure direct compose and inline-compose share the same explicit no-match object and that the inline reporting branch does not emit a second source-file report or collapse the structured failure into prose.
- [ ] Add L1 route tests proving all four modes return `composition.invalid_file_reference` for explicit misses, carry exact reference/base/candidate order, omit autocomplete error text, and still invoke the picker for `access`/`access.md` after a clean miss.
- [ ] Add the motivating repository fixture with `homelab/docs/unifi/access.md`; invoke missing `./docs/unifi/access.md` from the repository root and assert the human block and detail payload both suggest `homelab/docs/unifi/access.md` without selecting or retrying it.
- [ ] Extend the self-isolating L2 terminal harness to invoke the same explicit inline-compose miss with and without a PTY; assert identical diagnostic code and substantive body, no chooser/confirmation markers, no input wait, and clean process termination without focusing a terminal window.
- [ ] Add focused L2 coverage for at least one explicit sequence-source miss so the separate sequence entry seam cannot regress; retain existing confirmation and chooser L2 scenarios for bare-name discovery.
- [ ] Review and update only behavior comments/doc comments made inaccurate by the new policy, including the operation-file module overview and resolver fallback descriptions; preserve unrelated comments and the shared matcher documentation.
- [ ] **Parallelizable after route integration stabilizes:** build the L2 PTY/non-PTY assertions and the per-mode L1 matrix independently from the terminal renderer snapshot updates.
- [ ] **Validation checkpoint:** from `claudine/`, run focused L1 routing tests and focused L2 operation-file tests with the terminal-test feature; verify owned harness sessions self-isolate and terminate, and no terminal/browser window gains focus.

## Phase 5 — Documentation parity and release gates

**Outcome:** the recovery contract is documented consistently and the complete
Claudine package passes its required quality gates without out-of-scope changes.

- [ ] Update `claudine/docs/topics/completions/shell-completions.md` to distinguish the three operation-file outcomes: omitted positional rejected by argument parsing, unresolved bare discovery name eligible for the picker, and unresolved explicit reference reported as a typed no-match without a picker.
- [ ] Apply the same changed passage to `.claude/skills/claudine/completions/shell-completions.md` and verify the authoritative topic and portable snapshot are byte-consistent for that passage; update composition documentation only if a separate unconditional-fallback claim is found.
- [ ] Review all changed `///`, `//!`, and inline comments against behavior; fix or remove only drift introduced or exposed by this change, assuming code is authoritative where prior comments conflict.
- [ ] From `claudine/`, run `just test`, `just test-l2`, and `just lint`; confirm L1 covers classification, projection, rendering, suggestion bounds, and four-mode routing, while L2 covers the real TTY/non-TTY contract.
- [ ] From the repository root, run `just ci-local claudine` and record any host-limited Windows validation separately; rely on platform-neutral unit fixtures and CI for native Windows execution.
- [ ] Run GitNexus `detect_changes` with `scope: "compare"` and `base_ref: "main"`; review every affected symbol/execution flow and reconcile any scope outside operation-file resolution, completion recovery, diagnostics, focused tests, and synchronized docs.
- [ ] Inspect the final diff for accidental changes to `path_matches_query`, autocomplete scopes/ranking, primary `FileReference` resolution precedence, nested document references, or unrelated user work; remove any such drift before handoff.
- [ ] Report the exact gates run, the explicit-reference and bare-name behaviors proven, suggestion-bound coverage, and any platform checks not executable on the macOS host. Do not run `cargo fmt` and do not commit unless separately requested.

## Completion criteria

The work is complete when only unresolved bare single-component implicit names
can invoke operation-file autocomplete; every explicit clean miss in all four
operation modes reports the existing typed diagnostic with exact captured probe
evidence; bounded basename suggestions are deterministic and advisory; TTY
state does not change failure identity; shared resolution and matching contracts
remain unchanged; docs are synchronized; and all Claudine L1, L2, lint, and
local CI gates pass.
