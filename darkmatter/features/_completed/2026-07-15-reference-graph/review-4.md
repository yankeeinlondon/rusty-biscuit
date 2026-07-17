---
$schema: feature-review.yaml
ready: true
agent: claude/default
created: 2026-07-16T12:49:06-07:00
spec: 2026-07-15-reference-graph/spec.md
implemented: false
description: "A **feature** review of `2026-07-15-reference-graph/spec.md`"
feature: 2026-07-15-reference-graph/review-4.md
previous: 2026-07-15-reference-graph/review-3.md
---

# Review 4 — Reference Graph

## Verdict

**Ready for production.** Review 3's sole blocker — the red Darkmatter Level-1 test gate — is
resolved, and every required gate is now green on this revision. All thirteen acceptance criteria
are satisfied, and I found no correctness defect in the reference-graph implementation.

I am recording one **medium** finding: the ordinary `validate_references` path pays the full
provenance check and re-reads every visited descendant from disk, even though the specification's
own reasoning says compatibility there is "guaranteed by construction". This is wasted work, not
wrong work — it does not violate any acceptance criterion as written, so it does not block release.
It should be fixed as a focused follow-up, because the regression it causes lands on the one
benchmark function that was never compared against baseline.

## Review 3 Closure

| Prior finding | Status | Evidence |
|---|---|---|
| Required Darkmatter Level-1 test gate was red (`compose_redirected_does_not_spawn_appearance_defaults`) | **Closed** | Commit `74e0fdc90` scoped the `defaults` shim to the appearance-probe argv. The test now passes on the first attempt: `PASS [0.179s] (265/559) darkmatter-cli::compose_terminal_detection compose_redirected_does_not_spawn_appearance_defaults`. The full area run completes with 6,876 passing and **zero** failures across `darkmatter`, `darkmatter-cli`, and `dmls`. |
| `just lint` was inconclusive (exceeded the session's 60-second limit with no output) | **Closed** | `just lint` now completes cleanly across all three packages (exit 0). |

## Findings

### Medium — `validate_references` re-reads every descendant it just built the graph from

`validate` builds a fresh graph and immediately delegates to `validate_with_graph`
(`lib/src/markdown/reference/validate.rs:336`), which unconditionally calls
`verify_graph_compatibility` (`validate.rs:362`). That in turn calls `verify_descendants`
(`validate.rs:588`), which re-reads and re-hashes **every** entry in the dependency manifest
straight from disk via `Markdown::try_from` (`validate.rs:612`).

On the prebuilt path this is exactly the point of the feature. On the ordinary `validate_references`
path it is redundant by the specification's own argument:

> The ordinary `validate_references` path remains unchanged conceptually: it builds a full graph and
> immediately validates it, so compatibility is guaranteed by construction.

The graph was built from those same children microseconds earlier, so every read is re-reading a file
the builder just loaded, to confirm it still matches an identity derived from that very load.

**Why this matters, and why the evidence misses it.** The regression lands entirely on
`build_and_validate`, and that is the one bench function never compared against baseline. The
baseline run was taken with `Filter: -- construct` (`results.md:157`), and the report states plainly
that "only the `construct` medians are used for this comparison" (`results.md:136`). So no
pre-opacity `build_and_validate` measurement exists on either the cross-commit or the same-session
paired run.

The recorded numbers bound the cost. The `small` and `large` fixtures are in-memory with no on-disk
children, so `verify_descendants` is a no-op loop for them — which is why `small`'s validate-side
delta is negligible (`build_and_validate` 241.21 µs − `construct` 204.78 µs ≈ 36 µs ≈
`validate_prebuilt` 34.685 µs). Only `multi_transclusion` has children
(`TRANSCLUSION_CHILD_COUNT = 12`, `lib/benches/reference_graph.rs:44`), and its `validate_prebuilt`
is **4.1522 ms versus `large`'s 105.17 µs — roughly 40× — despite `large` being the larger
document**. `results.md:95` attributes that elevated floor to exactly this cause: "the prebuilt path
still re-reads and re-hashes all 12 children from disk (descendant verification), so its floor is
higher than the single-document fixtures." That whole cost is now added unconditionally to
`validate_references`, whose `multi_transclusion` total is 10.455 ms — so the redundant share is on
the order of tens of percent for transclusion-heavy documents, and it has never been measured.

**Secondary effect.** Because the check runs on the ordinary path, `validate_references` gains a new
failure mode the specification does not sanction: if a child document changes in the narrow window
between graph construction and validation, the call now returns a hard
`ReferenceError::ReferenceGraphMismatch` where it previously returned a report. The window is small
and the practical risk is low, but it is a public-API behavior change on a path the spec describes as
conceptually unchanged.

**Suggested fix.** Keep the single funnel but let the internal caller state that compatibility is
already established — for example an internal `validate_with_graph_checked(..., verify: bool)` (or a
private `validate_with_prebuilt_graph` wrapper) so `validate` skips `verify_graph_compatibility`
while `Markdown::validate_references_with_graph` continues to run it in full. The public signatures
and the fail-closed contract on the prebuilt path are unaffected. Whichever shape is chosen, the
follow-up should also record a baseline-vs-candidate `build_and_validate` comparison so the ordinary
path is no longer an unmeasured blind spot.

### Low — two small robustness/efficiency notes

Neither is worth blocking on; both are cheap to address if the code is touched again.

1. `whole_state_fingerprint` folds frontmatter values with
   `serde_json::to_string(value).unwrap_or_default()` (`provenance.rs:97`). A serialization failure
   silently contributes an empty string, so two distinct unserializable values would fold
   identically. Serialization of a frontmatter value effectively cannot fail today, so this is
   theoretical — but `unwrap_or_default()` fails *open* in a fail-closed identity, which is the wrong
   default direction for this module.
2. `ReferenceDependencyManifest::record` dedupes by linear scan (`provenance.rs:134`), making manifest
   assembly O(n²) in unique children. Irrelevant at 12 children; worth revisiting only if graphs with
   very large fan-out appear.

## Requirement-to-Verification Assessment

Verification levels are recorded per the review's Level-1/2/3 taxonomy. This feature defines
deterministic library and CLI behavior — it changes no terminal query, real-terminal rendering,
glyph/SGR output, keyboard/paste/IME/mouse handling, or other OS-input behavior. Per `spec.md:636`,
Level 2 and Level 3 coverage are **not required**, and I agree: no requirement here asserts
user-observable terminal or OS-input behavior, so there is no level mismatch to report.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | No public or `pub(crate)` data fields | L1: source + compile-time API | **Pass.** Fields private (`types.rs:452`); manual `Debug` omits provenance (`types.rs:464`). |
| 2 | Inspectable but not constructible/mutable | L1: accessor tests | **Pass.** Only read-only accessors; no `new`/`from_parts`/`Default`/`DerefMut`. |
| 3 | One provenance-computing internal constructor | L1: unit tests | **Pass.** `from_build` is the sole funnel and computes identity itself (`types.rs:601`), so identities cannot be mislabeled. |
| 4 | Reject root/descendant/source/mode/options before flattening | L1: `prebuilt_graph_rejects_{edited_root_body,changed_frontmatter,different_source,different_url_source,changed_graph_options,edited_child_before_flatten,missing_child,unreadable_child}`, `prebuilt_transclusion_only_graph_rejected_by_full_validation` | **Pass.** `verify_graph_compatibility` (`validate.rs:362`) precedes `flatten_graph` (`validate.rs:364`). |
| 5 | Canonical, compact, exhaustive options identity, no `Debug` | L1: 20+ identity tests incl. insertion-order, element-boundary injectivity, non-UTF-8 paths | **Pass.** No-`..` destructure of all 43 fields (`options.rs:1549`) is a genuine compile-time guard. |
| 6 | No extended stateful lifetimes | L1: strong-count + weak-drop probes | **Pass.** `Weak` handles only; `graph_ownership_does_not_extend_{shell_handler,preflight_graph}_lifetime` and `graph_build_does_not_retain_remote_fetch_runtime` all pass. |
| 7 | One identity per unique child; changed/missing/unreadable reject | L1: `dependency_manifest_*`, `unchanged_child_via_multiple_insertions_passes`, `prebuilt_graph_bypasses_cache_for_descendant_edit` | **Pass.** Dedup by resolved source; verification re-reads disk, bypassing both caches, so a cache-stale build cannot mask an edit. |
| 8 | Mode is sole extraction switch | L1: mode/provenance tests | **Pass.** `let extract_references = matches!(mode, ReferenceGraphMode::Full)` (`graph.rs:83`) — no parallel Boolean. |
| 9 | Clone-stable identity (Finding 18 reuse) | L1: `graph_from_cloned_options_passes_original_and_further_clone`, `file_tree_validate_reuses_graph_without_spurious_mismatch` | **Pass.** This is the load-bearing guard; it passes. |
| 10 | Callsites use accessors | L1: compile-time | **Pass.** CLI serializes via `graph.view(follow)` (`cli/src/commands/graph.rs:29`); no residual field access. |
| 11 | Preserve graph/file-tree/Mermaid/DOT/terminal/JSON behavior | L1: serializer + fixture + spawned-CLI tests | **Pass.** Incl. `prebuilt_graph_view_json_omits_provenance`. |
| 12 | Focused tests + build/test/lint/whitespace/scope gates | L1 + tooling | **Pass.** All green — see below. |
| 13 | Reuse win preserved without material construction regression | Criterion, same-session paired | **Pass as written.** No fixture trips both the 5% and 100 µs gates; reuse win holds on all three. See the medium finding — AC13 gates *construction* only, so the ordinary-path regression is out of its scope by construction, not by oversight in the measurement method. |

## Verification Performed

All work was done on macOS (Darwin 25.5.0, Apple Silicon) in the `darkmatter` worktree.

- Read the specification, review 3, `results.md`, and the implementation:
  `reference/{provenance,types,graph,validate}.rs`, `compose/context/options.rs`,
  `cli/src/commands/graph.rs`, and `lib/benches/reference_graph.rs`.
- **`just test`: PASS** (exit 0) — 6,876 tests passing, **zero failures**, across `darkmatter`,
  `darkmatter-cli`, and `dmls`. Review 3's blocking CLI test passes on the first attempt.
- **`just lint`: PASS** (exit 0) — all three packages; this closes review 3's inconclusive result.
- **`just build`: PASS** (exit 0) — all three packages.
- **`git diff --check`: clean.**
- Traced the build→validate call graph by hand to confirm `verify_graph_compatibility` runs strictly
  before `flatten_graph`, and that `load_markdown` returns raw (cached) parsed Markdown so the
  build-time identity and the disk re-read compare like-for-like — the property that makes both
  `unchanged_child_via_multiple_insertions_passes` and
  `prebuilt_graph_bypasses_cache_for_descendant_edit` meaningful rather than vacuous.
- Confirmed the documentation obligations: `ReferenceGraph` rustdoc records the opaque
  builder-produced contract, the Darkmatter skill documents accessors and JSON-invisible provenance,
  and the 2026-07-12 performance review records link to this feature.

Two limitations worth stating plainly:

- **`detect_changes()` was not used as feature-specific evidence.** As review 3 found, the
  branch-wide comparison against `main` is dominated by hundreds of unrelated changes on this
  long-lived branch. It remains a developer pre-commit gate.
- **Cross-platform execution was not exercised.** Windows and Linux behavior was reviewed in source
  only. The relevant surface is `verify_descendants`'s `path.exists()` split between `Missing` and
  `Unreadable` (`validate.rs:626`): on Windows a permission-denied child may report `Missing` rather
  than `Unreadable`. Both are mismatches that reject reuse, so this is a diagnostic-wording nuance,
  not a correctness difference.
</content>
</invoke>
