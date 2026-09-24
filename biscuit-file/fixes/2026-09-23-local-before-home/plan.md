---
total_phases: 4
created: 2026-09-23
phase: 2
agent: "codex/gpt-6-sol"
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/magic_local_roots.rs
    - biscuit-file/lib/tests/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
    - biscuit-file
---

# Implementation plan — local `@` roots before home

Source: [`spec.md`](./spec.md). Task numbers express dependencies; a wave groups tasks that can have separate owners and run concurrently. Wave numbers are unique across this plan.

## Phase 1 — Rulings and success contract

### Necessary Rules

The specification leaves two design choices for human review. Record the reviewed decisions in the specification before Phase 2; if either recommendation changes, revise the dependent tasks and acceptance checks here first.

1. **Overlapping home and local trees:** Recommend inferred tiers for `add_magic_path`, with an explicit user-tier override for Claudine's `~/.claudine/prompts` and `~/.claudine` registrations. This keeps those roots after local roots even when the launch directory or repository root equals `$HOME`. Keep the ordinary builder signature. A path outside the local root, including `/opt/configs`, is a user-tier fallback.
2. **Nested prompts from another tree:** Recommend an immutable, request-scoped launch `@` snapshot, separate from source repository, package, and authoring-base anchors. A nested `@x.md` searches the launch tree first; `./x.md`, bare `x.md`, `&x.md`, and `^x.md` retain their source-specific meanings. Source derivation and completion must not rediscover the launch scope.

### Work and completion criteria

Implement one ordered `@` root chain in biscuit-file, then apply it to Claudine's registration, composition, completion, and human-readable miss report. Update Darkmatter's identity and provenance handling for the new context inputs. Finish with focused regressions and documentation. Defect 1 from the specification is already fixed; retain its behavior.

Success means local `@` matches win over every home or other fallback match in repositories and plain directories, including trees under or equal to `$HOME`; nested external prompts retain the launch `@` scope; completion and recursive search follow resolution order; a direct `@` miss lists ordered search directories while structured probe details remain intact; and affected package tests and lint pass on supported platforms. Other reference forms keep their established anchors and diagnostics.

### Tasks

**Wave 1 — parallel, read-only spikes.** The two spikes have separate owners and can run together before the rulings are finalized.

- [x] **1.1 Trace contexts**
  - Trace launch capture, `FileResolutionContext` derivation, Claudine `InvocationContext::derive_source`, the composition compatibility path, and CLI completion. Record where a fresh source context replaces launch state and sketch the smallest API that transfers an immutable launch `@` scope without changing other reference kinds.
  - Check repository and package scope selection when a prompt comes from `~/.claudine` or a second repository; use these findings to verify ruling 2.
- [x] **1.2 Audit identities**
  - Trace `RootProvenance` exhaustive matches, Darkmatter graph identity, the separate compose cache key, and configured `magic_paths` consumers. Record which new fields and variants need encoding, without changing existing provenance codes.
  - Check Windows normalized path spelling and containment expectations, including 8.3 and verbatim paths; use these findings to constrain ruling 1 and the tests.

**Sequential after Wave 1.**

- [x] **1.3 Record rulings**
  - Obtain human review of both recommendations, record the chosen policies in `spec.md`, and settle the launch-scope API contract before implementation. If a ruling differs, update this plan's ordering and tests accordingly.
- [x] **1.4 Freeze assertions**
  - Confirm that tests distinguish the reported failures from the fix: both `@x.md` and `@prompts/x.md` local-wins cases, plain-directory lookup, home overlap, nested external prompts, and the miss report. Record that path-shaped nonmatching joins remain legitimate probes and that Defect 1's five CLI tests stay green.

**Checkpoint:** Phase 2 starts only when both rulings and their observable acceptance cases are settled.

## Phase 2 — Biscuit-file resolution foundation

These tasks are sequential because each establishes an API or root chain used by the next.

- [x] **2.1 Capture launch scope**
  - In `biscuit-file/lib/src/file_reference/context.rs`, capture the request directory and its selected launch repository, package, and package-area roots for `@` independently of the authoring base and source-specific anchors. Preserve that snapshot through `for_source`, `for_base`, and trusted external derivations; let `&`, `^`, bare, and explicit-relative references retain their current anchors.
  - Add an explicit configured-root tier override while retaining inferred `add_magic_path`. Interpret relative configured roots against the captured request directory, including ambient `resolve_from(base)`. Document this behavior change and expose the inputs needed by cache identity.
- [x] **2.2 Unify root ordering**
  - Add `RootProvenance::LocalRoot` for the request directory when no launch repository exists. Build the `@` chain once in the specified order: local prepends; package, package area, local root; local appends; user prepends; home; user appends. Classify by normalized lexical containment in the launch local root unless explicitly overridden; keep registration order within each position.
  - Normalize and deduplicate roots after ordering, retaining the first provenance. Use the same chain for direct `candidate_plan`, actual resolution, `%@` traversal, and completion; append a completion path segment only after root selection. Expose ordered roots with path and provenance for diagnostics. Keep non-`@` ordering unchanged.
- [x] **2.3 Prove resolver parity**
  - Add synthetic L1 context tests for all R1–R3 cases: repo under home, no repo under home, launch equal to home, repo equal to home, external fallback root, relative configured root and `resolve_from(base)`, trusted external source, recursive search, and first-seen deduplication.
  - Compare completion roots for partial tokens with normalized `candidate_plan` paths for corresponding complete tokens across every supported completion entry form. Assert ordered root provenance, `LocalRoot` versus `Source`, and a real first-match result where duplicate files exist. Use host-absolute temporary paths or platform-specific literals; never depend on ambient CWD or HOME.

**Checkpoint:** Biscuit-file L1 tests and `just lint` pass; the exported `@` root API and launch-scope behavior are stable enough for the consumer work in Wave 2.

## Phase 3 — Consumer integration

**Wave 2 — parallel after Phase 2.** Assign separate owners to these tasks. Keep shared Claudine CLI regression edits for Phase 4 to avoid conflicting writes.

- [ ] **3.1 Wire Claudine scopes**
  - Make `with_prompt_magic_roots` register local conventions against the launch local root, even without a repository, in composition, `InvocationContext`, and CLI completion. Explicitly mark the two `~/.claudine` roots as user-tier; keep their Start/End positions and Defect 1's path-shaped fallback registrations.
  - Carry the launch `@` snapshot into contexts rebuilt for external or other-repository sources in both `derive_source` paths and completion. Preserve source-specific behavior for `./`, bare, `&`, and `^`. Update `composition/resolve/tests.rs` for the registration shape.
  - Review `composition/sequence/expr.rs` and composition and sequence preflight tests for changed `@` winners; adjust only expectations governed by the new order.
- [ ] **3.2 Render search roots**
  - Store the ordered `@` root list beside Claudine's existing `ResolutionDetail` probe record on direct `@` misses. Render the payload once and list directories in priority order, marking only configured roots `(*)`; remove joined candidates and provenance labels from that human-readable branch.
  - Update the provider renderer and any other renderer of the same error. Preserve structured concrete candidates, dispositions, and provenance, as well as existing reports for bare and absolute references. Update exhaustive `RootProvenance` mappings and add focused renderer tests using `biscuit-terminal` renderable components.
- [ ] **3.3 Update Darkmatter identity**
  - Update exhaustive provenance mappings and append a new cache encoding code for `LocalRoot` without renumbering prior codes. Encode `package_root`, the captured launch `@` scope, and configured-root tier overrides in graph identity; inspect the separate compose cache key for the same winner-changing inputs.
  - Review `ComposeOptions.magic_paths` callers in compose utilities, transclusion, reference graph and validation, and type tests. Update tests that expected an outside configured Start root to beat the repository. Prove cache identities differ when only launch scope or tier changes; retain `AuthoringBaseFirst` behavior for bare references.

**Checkpoint:** Claudine, claudine-cli, and Darkmatter compile against the new biscuit-file API; each task's focused tests pass before shared integration tests begin.

## Phase 4 — Regression proof and documentation

**Wave 3 — parallel after Wave 2.** The test and documentation tasks own different files and can proceed together; final validation follows both.

- [ ] **4.1 Exercise CLI behavior**
  - Extend `claudine/cli/tests/l1/compose_prompt_tiers.rs` with both local-wins conflicts, both `@` forms from a plain `$HOME/scratch`, home-overlap cases, and a nested prompt loaded from home or another repository. Assert the nested non-`@` anchors remain source-specific.
  - Verify the `@` miss report's payload count, ordered directories, configured `(*)` markers, and absence of joined paths or provenance labels; separately assert that structured probes remain concrete and a bare or absolute miss retains its current report. Keep the existing five path-shaped tests green.
  - Extend completion coverage so duplicate local and user matches for `@prompts/` offer and execute the local result. Isolate HOME, launch directory, repository discovery, and completion state in fixtures; keep these tests at L1 because they need no focused terminal or browser window.
- [ ] **4.2 Refresh reference docs**
  - Update the biscuit-file file-reference topic, README examples, and biscuit-file skill reference for the local/user tiers, request-directory fallback, overlap override, relative configured-root base, and shared completion order.
  - Update Claudine's documentation and skill copies of `shell-completions.md`, and finish the “Local Wins” section in `compose-prompt-rules.md` with the selected external-prompt policy. Review affected resolver, context, registration, and Darkmatter comments for drift; update public API docs that change behavior.

**Sequential after Wave 3.**

- [ ] **4.3 Validate packages**
  - Run `just test` and `just lint` from `biscuit-file/`, `claudine/`, and `darkmatter/`; run focused claudine-cli and cache identity regressions through their area recipes. Resolve failures attributable to the changed root chain, without adding unrelated tests or CI cells.
  - Obtain available macOS, Linux, native Windows, and WSL2 evidence for the affected packages, using existing cross-check hosts or scheduled CI cells as appropriate. Treat cross-compilation as compile evidence only; record any platform still pending. Check Windows path spelling and the no-repository fixture explicitly.
- [ ] **4.4 Review completion**
  - Recheck R1–R6 and the spec's test matrix against the final diff, including Defect 1 retention, all affected call sites, and no change to non-`@` ordering. Confirm frontmatter phase count and the Wave 1–3 dependencies. Leave the fix active and report implementation complete, ready for author review; the author handles lifecycle closure.

**Checkpoint:** All required behavior has observable passing evidence, documentation matches the implementation, and any OS evidence gap is stated precisely for review.
