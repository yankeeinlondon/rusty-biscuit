---
total_phases: 6
created: 2026-07-16
phase: 6
yolo: "true"
packages:
  - darkmatter
source_code:
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
  - darkmatter/lib/benches/reference_graph.rs
documentation:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
  - darkmatter/fixes/2026-07-16-redundant-walk/phase-1-evidence.md
  - darkmatter/fixes/2026-07-16-redundant-walk/results.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_1:
  - darkmatter/fixes/2026-07-16-redundant-walk/phase-1-evidence.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/reference/validate.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/reference/validate.rs
docs_updated_during_phase_4:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/benches/reference_graph.rs
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_5:
  - darkmatter/fixes/2026-07-16-redundant-walk/results.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/fixes/2026-07-16-redundant-walk/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
---

# Execution Plan — Eliminate Redundant Reference-Graph Verification

Derived from [`spec.md`](./spec.md). This fix splits `validate_with_graph` into a
freshness gate and a shared validation engine, so the two internally fresh callers
(`Markdown::validate_references` and `FileTree::ensure_built`) stop reopening and
rehashing every transcluded child that the graph builder loaded microseconds earlier.
The public checked-prebuilt contract on `Markdown::validate_references_with_graph`
is unchanged, and no public signature, error, report, or CLI byte moves.

## Ground Truth (verified against the worktree at `74e0fdc90`)

| Thing | Location |
|---|---|
| `validate` (one-step) | `darkmatter/lib/src/markdown/reference/validate.rs:332` |
| `validate_with_graph` (checked prebuilt) | `darkmatter/lib/src/markdown/reference/validate.rs:349` |
| `verify_graph_compatibility` | `darkmatter/lib/src/markdown/reference/validate.rs:563` |
| `verify_descendants` (the redundant walk) | `darkmatter/lib/src/markdown/reference/validate.rs:604` |
| Engine body to extract | `validate.rs:364` (`flatten_graph`) through `validate.rs:548` (`Ok(report)`) |
| `build_reference_graph` | `darkmatter/lib/src/markdown/reference/graph.rs:132` |
| `FileTree::ensure_built` | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:225` |
| Public `validate_references` / `..._with_graph` | `darkmatter/lib/src/markdown/reference/mod.rs:535` / `:568` |
| Prebuilt compatibility suite | `darkmatter/lib/tests/reference_integration.rs:1215`–`1907` |
| Parity unit test | `validate.rs:1410` (`validate_with_graph_matches_validate_with_fragments`) |
| Flatten-ordering assertion | `reference_integration.rs:1349` (`prebuilt_graph_rejects_edited_child_before_flatten`) |
| Benchmark | `darkmatter/lib/benches/reference_graph.rs` (group `reference_graph/{fixture}`) |

Two facts that shape the plan:

- **`reference::validate` is a public module** (`mod.rs:15` declares `pub mod validate`).
  A bare `pub fn` there would export the unchecked seam to the world. `pub(super)` in
  `validate.rs` resolves to `pub(in crate::markdown::reference)`, which is visible
  throughout the `reference` subtree — including the `file_tree` child module — and is
  invisible outside it. That is exactly the spec's requested visibility, and it is the
  only Phase-2 choice that satisfies Goal 5 and the "MUST NOT be publicly exported" rule.
- **The worktree is already dirty** in unrelated source and documentation. Do not hard-code a
  partial file list here: Phase 1 records the complete porcelain status and benchmark-input
  fingerprint. Preserve every pre-existing edit, and keep all files that can affect the benchmark
  stable between the saved baseline and candidate run except for this fix's intended
  `reference/` changes. If the unrelated state changes, record it and recapture the pair rather
  than claiming the measurements are comparable.

---

## Phase 1 — Pre-Implementation Evidence

Nothing in this phase changes source. The spec requires the baseline to exist *before*
any implementation edit, so this phase gates everything after it. Tasks 1.1 and 1.2 are
independent and **can run in parallel**; 1.3 depends on 1.1.

- [x] **1.1 — Record the exact pre-fix worktree state.** Capture `git rev-parse HEAD`,
      `git status --porcelain`, `rustc --version`, `cargo --version`, OS + architecture
      (`uname -mrs`), and the xxHash of `darkmatter/lib/benches/reference_graph.rs`
      (`bh --file darkmatter/lib/benches/reference_graph.rs`) as the benchmark-source
      fingerprint. `md hash` is Markdown-aware and MUST NOT be used for this Rust file. Park
      these values in scratch notes for `results.md` in Phase 5.
- [x] **1.2 — Refresh impact analysis on the current index.** Run GitNexus `impact` with
      `direction: "upstream"` for each of `validate`, `validate_with_graph`,
      `verify_graph_compatibility`, `validate_references`, `validate_references_with_graph`,
      and `FileTree::ensure_built`. Compare against the spec's recorded risk
      (`validate` HIGH / `ensure_built` MEDIUM / rest LOW). **Report any HIGH or CRITICAL
      result to the user before editing**. Review depth-1 production callers individually;
      integration tests and the benchmark are expected outside `darkmatter/lib`, so their mere
      presence is not a reason to stop. Stop only if a production caller cannot prove the fresh
      invariant, or if a public/cross-area effect appears that invalidates the recorded scope.
      Also save the current worktree-scoped `detect_changes` result as a before snapshot: this
      dirty worktree makes a later unstaged result meaningful only as a delta from that snapshot.
- [x] **1.3 — Quiesce the host, then capture the named Criterion baseline.** Close other
      workloads; do not run the baseline on a loaded machine. Run exactly:
      ```
      cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
        --save-baseline redundant-walk-before --warm-up-time 1 --measurement-time 4
      ```
      The filter is a regex over the full benchmark id, so it selects
      `build_and_validate` for all three fixtures (`small`, `large`, `multi_transclusion`)
      and skips `validate_prebuilt` / `construct`.
- [x] **1.4 — Transcribe baseline numbers.** For each of the three fixtures record sample
      count, median, and confidence interval from the Criterion output. Judge stability from the
      current run's dispersion and repeatability, not proximity to Review 4's ~10.5 ms: the old
      absolute came from a different host state/toolchain run and is not a pass/fail baseline.
      Rerun if the current intervals are wide or repeated medians are unstable.

**Checkpoint 1:** A `redundant-walk-before` baseline exists on this host for all three
`build_and_validate` fixtures, the three medians are written down, impact analysis is
refreshed, and no implementation source has been modified. Do not start Phase 2 until this holds.

---

## Phase 2 — Split Freshness From Validation (`validate.rs`)

The core refactor. Serial — every task touches the same file.

- [x] **2.1 — Extract the shared engine.** Add a private
      `fn validate_graph_contents(md: &Markdown, options: &ReferenceValidationOptions, graph: &ReferenceGraph) -> Result<ReferenceValidationReport, ReferenceError>`
      holding the current `validate_with_graph` body from `let ref_set = super::graph::flatten_graph(graph);`
      (`validate.rs:364`) through `Ok(report)` (`validate.rs:548`) **verbatim** — flattening,
      the heading-slug cache, fragment preparation, the per-record local/remote/fragment match,
      every `fail_fast` early return, and the batched remote pass. Move nothing else and
      change no logic. Leave it private (no `pub`): it is reached only through the two seams below.
- [x] **2.2 — Reduce `validate_with_graph` to the checked seam.** Its body becomes
      `verify_graph_compatibility(md, options, graph)?;` followed by
      `validate_graph_contents(md, options, graph)`. Keep it `pub(crate)` and keep the
      `verify` call **first** — that ordering is the fail-closed contract asserted by
      `prebuilt_graph_rejects_edited_child_before_flatten`.
- [x] **2.3 — Add the fresh seam.** Add
      `pub(super) fn validate_fresh_graph(md, options, graph) -> Result<ReferenceValidationReport, ReferenceError>`
      that calls `validate_graph_contents` directly with no verification. Do **not** add a
      `verify: bool` parameter to a single entry point — the spec rejects that shape because a
      wrong `false` at the prebuilt boundary is easy to write and invisible in review.
- [x] **2.4 — Rewrite `validate` to use it.** Keep the `build_reference_graph` call and its
      existing `ReferenceError::Validation` error mapping exactly as-is, then call
      `validate_fresh_graph` instead of `validate_with_graph`. The build and the validate call
      must stay visibly adjacent so a future refactor cannot slip an external handoff between them.
- [x] **2.5 — Preserve tracing behavior, including rejected prebuilt graphs.**
      `info!("validate: starting reference validation")` currently runs before
      `verify_graph_compatibility`, so a rejected prebuilt graph still emits it. Do **not** move
      this event into `validate_graph_contents`, where a failed compatibility check would skip it.
      Keep the event at the start of `validate_with_graph` and emit the same event at the start of
      `validate_fresh_graph`; keep `debug!(ref_count = …)` inside the shared engine. Add
      `#[instrument(skip_all)]` to `validate_fresh_graph` so the two existing call shapes retain
      their span counts (`validate` → fresh seam has two; `FileTree` → fresh seam has one), and do
      not instrument `validate_graph_contents` or create a third nested span. The fresh path's
      span name may accurately become `validate_fresh_graph`; the checked public path retains the
      existing `validate_with_graph` span and log ordering.
- [x] **2.6 — Fix the drifted docs in the same change** (CLAUDE.md authoring discipline;
      spec "Comments and documentation"):
      - `validate`'s docblock (`validate.rs:329-330`) currently says "delegates to
        [`validate_with_graph`]" — now false. Restate it as: builds the graph once and validates
        that fresh snapshot, no compatibility check.
      - The in-body comment at `validate.rs:356-361` must drop "A freshly built graph from
        `validate` matches by construction" and describe only the caller-supplied prebuilt contract.
      - `validate_with_graph`'s docblock (`:341-347`) and `verify_graph_compatibility`'s (`:551-562`)
        must describe only the prebuilt contract.
      - `validate_fresh_graph`'s new rustdoc must state the freshness precondition explicitly:
        graph produced by `build_reference_graph`/`Markdown::reference_graph` in this operation,
        same `Markdown`, same (or clone-stable-cloned) `ReferenceGraphOptions`, no caller-controlled
        work in between — and that any path failing to prove this must use `validate_with_graph`.
      - Leave the public `validate_references_with_graph` rustdoc (`mod.rs:542-567`) and the
        Darkmatter skill's prebuilt guidance **unweakened**; both remain correct.

**Checkpoint 2:** `cargo check -p darkmatter` is clean; `verify_descendants` still exists and
is still reached only from `verify_graph_compatibility`; source inspection confirms
`validate_fresh_graph` is `pub(super)` and no public re-export was added. Do not run `cargo fmt`.

---

## Phase 3 — Route `FileTree::ensure_built` Through the Fresh Seam

Depends on Phase 2 (needs `validate_fresh_graph` to exist).

- [x] **3.1 — Swap the call.** In `file_tree/mod.rs:243-250`, replace
      `self.md.validate_references_with_graph(&graph, val_opts)` with a direct
      `validate::validate_fresh_graph(&self.md, &val_opts, &graph)`. The error mapping
      simplifies: the seam returns `ReferenceError` directly, so the two-arm
      `MarkdownError::Reference(re) => FileTreeError::Reference(*re)` match collapses to
      `.map_err(FileTreeError::Reference)?`. Update the existing import to
      `use crate::markdown::reference::validate::{self, ...};` so the module-qualified call
      resolves. `FileTreeError::Reference` is declared in `file_tree/mod.rs:55` and holds an
      unboxed `ReferenceError`; the current public-method adapter is what boxes it inside
      `MarkdownError` and forces the two-arm mapping.
- [x] **3.2 — Keep the freshness invariant visible.** `val_opts.graph = self.graph_options.clone()`
      must remain immediately above the validate call, and the graph must keep being built from
      `self.graph_options.clone()` — identity is clone-stable, which is what makes this eligible.
      Do not reorder these statements.
- [x] **3.3 — Replace the stale Finding-18 comment.** The comment at `file_tree/mod.rs:241-242`
      explains only why a second *build* is avoided. Rewrite it to state why this graph is
      eligible for the fresh seam: it was built from this `FileTree`'s own `md` and
      `graph_options` in this same method with no caller handoff, so descendant re-verification
      would only re-confirm identities captured a few lines above.
- [x] **3.4 — Review `ensure_built`'s rustdoc for drift.** Its public contract (lazy,
      idempotent build; graph/validation errors) remains correct and SHOULD stay concise. The
      freshness precondition belongs at the surprising internal call in 3.3; do not add
      implementation narration to public rustdoc unless the public behavior actually needs it.

**Checkpoint 3:** `cargo check -p darkmatter` clean; `rg` confirms `FileTree` is no longer a
production caller of `validate_references_with_graph` (remaining matches are the public adapter,
benchmark, tests, or rustdoc references).

---

## Phase 4 — Correctness Verification

Depends on Phase 3. Task 4.1 must be written before 4.2/4.3 run, but **4.2 and 4.3 are
independent of each other and can run in parallel**.

- [x] **4.1 — Add the focused mechanism test** in `validate.rs`'s `#[cfg(test)] mod tests`.
      Name it for what it proves, e.g. `fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph`.
      Shape (mirror the tempdir/`Markdown::try_from` pattern already used by
      `reference_integration.rs:1349`):
      1. Tempdir with `root.md` transcluding `child.md` via `::file`; load the root with
         `Markdown::try_from`.
      2. Construct one `ReferenceValidationOptions::default()` value and build the graph once via
         `build_reference_graph(&md, &opts.graph)`. Reuse that same `opts` for both calls; do not
         construct a second options value, which could make the checked path fail for an options
         mismatch instead of the edited child.
      3. Rewrite `child.md` on disk, adding a broken local reference.
      4. Assert `validate_fresh_graph(&md, &opts, &graph)` returns `Ok` — **not**
         `Err(ReferenceError::ReferenceGraphMismatch(_))` — and that the report reflects the
         **build-time** snapshot (the post-edit broken reference is absent from the issues).
      5. Assert `validate_with_graph(&md, &opts, &graph)` returns
         `Err(ReferenceError::ReferenceGraphMismatch(mismatch))` and assert
         `mismatch.kind()` is `ReferenceGraphMismatchKind::Dependency` with
         `DependencyMismatchKind::Changed`, so the test cannot pass for the wrong identity
         dimension.
      Keep `validate_remote: false` so the test does no network I/O. Add **no** production
      global counter and **no** filesystem abstraction for this test — the spec forbids both.
- [x] **4.2 — Run the focused reference suites.**
      `cargo nextest run -p darkmatter -E 'test(/reference|validate|file_tree|graph/)'`.
      The whole prebuilt compatibility suite (`reference_integration.rs:1215`–`1907`) must stay
      green, especially: document/source/mode/options mismatch rejection; edited, missing,
      unreadable, and cache-stale child rejection; clone-stable options reuse; checked-vs-one-step
      parity; and file-tree validation/report behavior.
- [x] **4.3 — Confirm the two load-bearing existing assertions still hold.**
      `prebuilt_graph_rejects_edited_child_before_flatten` (`reference_integration.rs:1349`) must
      still pass unmodified — it is the guard that the checked path was not accidentally weakened.
      `validate_with_graph_matches_validate_with_fragments` (`validate.rs:1410`) must still pass:
      for a graph that *is* fresh, both seams must produce an identical report, which is the
      Goal-4 "one engine" evidence.
- [x] **4.4 — Prove the fresh seam is unreachable from the public API.** Verify by inspection
      (and, if `reference_integration.rs` lacks it, by keeping the existing stale-graph rejection
      test) that `Markdown::validate_references_with_graph` still routes through
      `validate_with_graph` and can never reach `validate_fresh_graph`.

**Checkpoint 4:** New mechanism test passes; the full reference/file-tree/graph selection is
green; no existing test was weakened or deleted to make it pass. If a compatibility test now
fails, the refactor is wrong — do not relax the test.

---

## Phase 5 — Candidate Measurement and `results.md`

Depends on Phase 4 (measure only correct code) and on the Phase 1 baseline. Serial; the host
must be as quiet as it was in 1.3.

- [x] **5.1 — Run the paired candidate benchmark** in the same session, on the same host, with
      the same parameters and an unmodified `reference_graph.rs`:
      ```
      cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
        --baseline redundant-walk-before --warm-up-time 1 --measurement-time 4
      ```
      Do not edit fixture content, `TRANSCLUSION_CHILD_COUNT`, `sample_size(30)`, or the timed
      boundaries between the two runs. Re-verify the Phase-1.1 benchmark-source hash is unchanged.
      Phase 1 and this task form one measurement session; if the work is interrupted or host/build
      inputs change materially, the old baseline remains provenance evidence but is not acceptance
      evidence — recapture a matched pre-fix/candidate pair before drawing a conclusion.
- [x] **5.2 — Check the acceptance thresholds** against the Criterion comparison:
      - `multi_transclusion/build_and_validate` MUST improve by **both ≥10% and ≥500 µs** at
        the median. **RESULT: NOT MET** — same-run decomposition (`vp − (b&v − construct)`)
        shows the removed walk costs ≈159 µs (≈1.5%); the 4.15 ms `validate_prebuilt` floor
        is the shared engine, not descendant re-verification. See `results.md`.
      - **No** `build_and_validate` fixture regressed by both >5% and >100 µs at the median —
        **PASS** (small −16.6%, large −2.1%, multi −4.4%).
      Host contention forced three discarded paired captures; final numbers come from
      load-gated quiet windows with tight CIs, corroborated by `construct`/`validate_prebuilt`
      reproducing Review 4's quiet-host medians within 0.3%.
- [x] **5.3 — Run the final unfiltered benchmark** (run as two filtered invocations covering
      all nine groups) and confirm `validate_prebuilt` is still materially faster than
      `build_and_validate` for each fixture: **PASS** — small 31.5 µs vs 208 µs (6.6×),
      large 427 µs vs 6.31 ms (≈15×), multi 4.162 ms vs 10.066 ms (2.4×).
- [x] **5.4 — Write `darkmatter/fixes/2026-07-16-redundant-walk/results.md`** containing, per
      the spec: complete baseline and candidate commit/worktree state (including every
      pre-existing dirty path, with confirmation that benchmark-affecting unrelated edits stayed
      unchanged); the exact commands; the
      benchmark-source fingerprint; OS, architecture, Rust toolchain, and Criterion parameters;
      sample count, median, and confidence interval per fixture; host-load observations; and
      absolute + percentage deltas.
- [x] **5.5 — Verify the benchmark's own comments are still accurate.** `reference_graph.rs:10-22`
      describes what `build_and_validate` and `validate_prebuilt` include. The current claim that
      "only provenance checking, descendant re-verification, and flattening are measured" is
      incomplete: `validate_prebuilt` also performs the ordinary per-reference validation and
      report work; only graph construction is outside its timed loop. Correct that comment, and
      state that `build_and_validate` now measures construction plus the shared validation engine
      without a compatibility walk. Make comment-only benchmark edits **after** 5.1/5.3 so the
      recorded timed-source xxHash remains identical across the pair, and record that timed hash
      in `results.md`. Do not alter historical results in the original feature directory.

**Checkpoint 5:** `results.md` exists with a same-session paired measurement; the
`multi_transclusion` improvement clears both thresholds; no fixture regressed past both limits;
`validate_prebuilt` still beats `build_and_validate` everywhere.

**Checkpoint 5 outcome:** `results.md` exists ✔; regression guard ✔; `validate_prebuilt` gap ✔;
**`multi_transclusion` ≥10%/≥500 µs improvement ✘** — the removed walk measures ≈159 µs (≈1.5%),
so the threshold (derived from the spec's falsified ~4.15 ms walk-cost premise) is unreachable
on this fixture. Full evidence and interpretation in `results.md`. The ≥10%/≥500 µs threshold
was superseded by the 2026-07-18 spec amendment (review 1), which replaces it with a
mechanism-based requirement plus a guard calibrated to the measured effect; under the amended
guards the recorded evidence satisfies AC8.

---

## Phase 6 — Area Gates and Closure

Depends on Phase 5. Tasks 6.1–6.3 are the scoped area gates; **6.1 and 6.4 can run in parallel**
with each other once the code is final.

- [x] **6.1 — `just build`** from `darkmatter/`. Covers `darkmatter`, `darkmatter-cli`, and `dmls`.
- [x] **6.2 — `just test`** from `darkmatter/`. Full area unit suite must be green.
- [x] **6.3 — `just lint`** from `darkmatter/`. Clippy must be clean — watch for a
      `dead_code` warning if any helper ended up unreachable, which would signal a bad split.
- [x] **6.4 — `git diff --check`** for whitespace damage.
- [x] **6.5 — GitNexus change analysis.** Run worktree-scoped `detect_changes()` and compare its
      changed symbols/flows with the before snapshot from 1.2; the delta attributable to this fix
      must be limited to reference validation and file-tree flows. Also record
      `detect_changes({scope: "compare", base_ref: "main"})` for the required regression audit,
      but do not misrepresent that branch-wide result as fix-specific evidence: this long-lived,
      dirty branch is expected to include unrelated work. Investigate any *new delta* outside
      `markdown::reference`.
- [x] **6.6 — Confirm the no-change surfaces.** Public Rust signatures, `ReferenceError` variants,
      report contents, serialized graph views, and CLI output are unchanged — the diff should
      contain no edit to a `pub fn` signature, no new/removed error variant, and no
      `darkmatter/cli/src` change attributable to this fix. Use the existing
      `darkmatter/cli/tests/graph.rs` validation cases and `graph_json_validate_baseline` as the
      CLI-output evidence; do not claim byte identity from source inspection alone.
- [x] **6.7 — Acceptance-criteria sweep.** Walk spec §"Acceptance Criteria" 1–9 and tick each
      against concrete evidence produced above (AC1→2.4, AC2→3.1, AC3→2.2, AC4→2.1+4.3,
      AC5→4.1, AC6→4.2, AC7→6.6, AC8→5.4, AC9→6.1-6.5).
- [x] **6.8 — Report and stop.** Summarize the outcome for the user. **Do not commit** — commits
      are a separate, explicitly requested operation.

**Checkpoint 6:** All nine acceptance criteria satisfied with named evidence; area build/test/lint
green; `detect_changes` scope matches expectation; nothing committed.

**Checkpoint 6 outcome:** Area gates green — build (darkmatter, darkmatter-cli, dmls) ✔;
test (5763 + 559 + 566 passed) ✔; lint (no warnings, no `dead_code`) ✔; `git diff --check` ✔.
Worktree-scoped `detect_changes` delta vs the 1.2 before-snapshot is limited to `validate`,
`validate_with_graph`, `FileTree::ensure_built`, and the benchmark comment (plus this fix's docs);
the `performance-followup` and `CLAUDE.md` entries are the pre-existing drift recorded in
`phase-1-evidence.md`. Branch-wide `compare` vs `main` (1892 symbols / 559 files) reflects the
long-lived branch's unrelated work and is regression-audit context only, not fix-specific
evidence. AC1–7, AC9 satisfied; AC8 partially — regression thresholds met and `results.md`
complete, but the `multi_transclusion` ≥10%/≥500 µs improvement threshold is unmet per the
Checkpoint 5 outcome (falsified ~4.15 ms walk-cost premise; actual removed walk ≈159 µs).
Nothing committed.

---

## Explicitly Out of Scope

Per spec §Non-goals — do not, while in here: weaken/cache/optionalize the public prebuilt
freshness contract; remove provenance or the dependency manifest; change what enters the manifest
or how identities hash; touch extraction, traversal, flattening, fragment/remote validation, or
report rendering; expose an unchecked public API; fix Review 4's `whole_state_fingerprint`
serialization fallback or linear manifest dedup notes; add watchers/locks/retries; or edit
historical results under `features/2026-07-15-reference-graph/`.

Level 2 and Level 3 tests are **not** required — this fix changes no terminal query, rendering
bytes, browser behavior, or host input handling. Downstream package gates beyond the area are
required only if Phase 1.2 impact or Phase 6.5 `detect_changes` surfaces a public or cross-area
effect.

## Risks

| Risk | Signal | Mitigation |
|---|---|---|
| Baseline taken after an implementation edit | `results.md` deltas look impossibly small | Phase 1 gates Phase 2; Checkpoint 1 requires the before snapshot and confirms no fix implementation edit preceded it |
| Unrelated dirty files change mid-measurement | Paired run noncomparable | Record the full worktree state in 1.1; keep benchmark-affecting unrelated inputs stable or recapture the pair |
| `pub(super)` accidentally widened to `pub` in a public module | Unchecked seam becomes public API | Checkpoint 2 inspects visibility and exports; Goal 5 forbids public surface change |
| Engine extraction silently drops a `fail_fast` early return | Reports change on error-heavy docs | 2.1 says extract verbatim; `validate_with_graph_matches_validate_with_fragments` parity test (4.3) catches divergence |
| Mechanism test's options identity mismatched | Checked path fails for the wrong reason, test proves nothing | 4.1 reuses one `ReferenceValidationOptions::default()` and builds with `&opts.graph` |
| Noisy host makes `multi_transclusion` miss the 10%/500 µs bar | Wide/overlapping CIs | Quiesce host; rerun baseline+candidate as a pair, never against Review 4's numbers |
| Start log moves behind the compatibility gate | Rejected prebuilt graphs lose their validation-start event | Keep the log at the beginning of each seam; never put it only in the shared engine |
