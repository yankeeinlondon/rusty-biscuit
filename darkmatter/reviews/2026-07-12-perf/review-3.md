---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-15T07:30:42-07:00
spec: 2026-07-12-perf/spec.md
implemented: false
description: "A **feature** review of `2026-07-12-perf/spec.md`"
feature: 2026-07-12-perf/review-3.md
---

# Review 3 — Performance

## Verdict

Not ready for production. The implementation retains the major measured gains: Darkmatter no
longer requests NTP during ordinary compose, TOC construction is no longer quadratic, schema
validation and ownership costs are materially lower, and the rendering reductions preserve the
current real-terminal frames. The Finding 29 follow-up is especially strong: its same-source
Criterion comparison supports the approved `Arc<Value>` compatibility exception and the built-in
baseline now avoids deep clones on its common paths.

Those wins do not close the feature's delivery contract. The bare `sniff::detect_timezone()` API
still has the semantic change the specification expressly forbids, the OSC/terminal caching claims
still have no requirement-matched Level 2 measurement, and the original command/TOC closeout still
uses different, unhashed fixtures. Several findings also remain partially implemented or deferred
without the disposition the specification requires.

## Findings

### High — The bare Sniff timezone API still violates Finding 1's compatibility ruling

Darkmatter correctly calls `detect_timezone_with_options(false)` at
`darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:129`, so ordinary compose no longer
requests NTP. The cross-package change goes farther: `sniff::os::detect_timezone()` still delegates
to `detect_timezone_with_options(false)` at `sniff/lib/src/os/time.rs:508-509`, and its rustdoc now
advertises that new behavior.

Finding 1 explicitly requires the bare function to remain the full NTP-reporting convenience API;
only Darkmatter was authorized to select the local-only path. Existing Sniff callers that relied on
the previous contract now receive `NtpStatus::Unknown` without opting into that behavior. Restore
the bare function to `detect_timezone_with_options(true)` and retain Darkmatter's explicit `false`
call. A broader Sniff default change needs its own caller audit and compatibility decision.

### High — Terminal caching still lacks the required Level 2 query-count and latency evidence

The implementation caches OSC 10 with `TEXT_COLOR_CACHE` and shares a terminal lazily within one
compose invocation. The only cache-specific tests remain
`text_color_is_stable_across_calls`, `bg_color_is_stable_across_calls`, and
`color_mode_is_stable_across_calls` (`biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:130-147`
and `discovery/detection/color.rs:357`). These are Level 1 value-equality tests. They can pass even
if the implementation performs two identical OSC queries.

The specification's Phase 3 checkpoint requires a Level 2 case that constructs multiple terminals
and proves one OSC request. The current plan explicitly substitutes unit tests because an L2
counter is considered fragile (`plan.md:332-347`). The 91 Darkmatter-area L2 tests exercised real
terminal rendering in this review, but none counted OSC traffic, measured repeated construction,
or exercised the multi-branch compose reuse path. The non-TTY Hyperfine data cannot fill this gap
because OSC detection short-circuits when piped.

Add a checked-in L2 helper/benchmark that runs inside a supported real terminal, observes the OSC
10 request count, and measures repeated `Terminal` construction in one process. Add a CLI case that
exercises verbose/perf/warning report branches and proves one per-invocation terminal detection.
No Level 3 test is needed because this feature does not involve the terminal input encoder.

### High — The CLI and TOC performance closeout still fails the reproducibility contract

The specification requires byte-identical baseline/post-change fixtures, a checked-in deterministic
generator or committed fixtures, recorded sizes and content hashes, and predeclared thresholds.
`results.md:21-25` acknowledges that the post-change TOC generator produced different content, and
its reproduction instructions still say to use "any deterministic generator" (`results.md:127-132`).
Neither `baseline.md` nor `results.md` records fixture hashes. The measurements also used non-TTY
stdout/stderr, so they cannot support the interactive terminal claims.

The algorithmic TOC fix and reported speedup are credible, but the saved artifacts do not satisfy
the feature's release gate. Check in the exact generator and a fixture manifest containing byte
sizes plus Darkmatter/xxHash identities, then rerun baseline and post-change commands with identical
bytes and declared thresholds. `results-2.md` already meets the same-source standard for Finding 29;
that evidence is valid for the ownership finding only.

### Medium — Residual findings remain open without an approved disposition

The specification says residual work remains part of this active review and may close only through
implementation, a requirement-matched no-win measurement, a correctness rejection, or an
owner-approved move to a linked active/unscheduled spec. The plan still leaves Findings 11, 12, 13,
17, 32, and 35 unchecked (`plan.md:594-626`, `:654-666`, `:688-710`, `:720-734`) and records only
subsets for Findings 7, 14, 16, 23, 25, 33, and 35.

Some individual implementation choices are sound. In particular, parallel body-shell execution is
correctly rejected because it would violate source-ordered side effects. That rejection does not
close Finding 17's independent 10 ms process-polling sub-item. Likewise, condition-aware rendering
explains why prepared child bodies cannot simply be copied from preflight, but the duplicate
cross-pass work remains an open design problem. Complexity, blast radius, and rarity are scheduling
reasons, not the closeout evidence required by this specification.

Reopen each residual item explicitly. Implement and benchmark it, record a requirement-matched
no-material-win result, or obtain the required owner-approved scope move. The draft
`2026-07-12-faster-compose` fix concerns eager context capture and does not own these residuals; the
draft `2026-07-15-perf-tweaks` feature concerns only additional baseline-schema ownership work.

### Medium — The public prebuilt-graph validation API cannot enforce its correctness contract

`Markdown::validate_references_with_graph` is public, but its only guard is documentation saying
that the supplied graph must correspond to the same document and `options.graph`
(`darkmatter/lib/src/markdown/reference/mod.rs:528-539`). The implementation flattens whatever graph
it receives while separately using the supplied `Markdown` and options for fragment preparation.
A caller can pair a graph from another document or different graph options and receive a plausible
but incorrect successful report.

The only focused regression proves parity for a matching graph. Keep this optimization crate-private
for `FileTree`/CLI reuse, or make `ReferenceGraph` carry and validate document/options identity.
Before keeping a public entry point, add negative tests for mismatched document bytes, source path,
and graph options.

### Medium — Finding 22 changes directory-hash membership without the required compatibility ruling

`collect_markdown_files` now unconditionally excludes `node_modules`, `target`, and `vendor`
(`darkmatter/lib/src/markdown/fs.rs:3-43`). That changes the aggregate returned by `md hash <dir>`
for existing directory trees. The specification lists directory-hash membership changes as a
non-goal without an explicit compatibility ruling (`spec.md:109-118`) and otherwise permits public
behavior changes only for Finding 1 and the approved Finding 29 ownership exception.

The plan calls this deliberate and the hash documentation was updated, but no compatibility
exception is recorded in the specification or results, and the new test covers the collector rather
than the user-facing CLI hash/exit-status behavior. Either add an explicit approved exception with
the intended aggregate-hash migration semantics, or preserve prior membership. If retained, add an
end-to-end Level 1 CLI test that pins the aggregate result for included and skipped subtrees.

## Requirement-to-verification assessment

| Requirement | Strongest current verification | Assessment |
|---|---|---|
| F1 — Darkmatter compose performs no NTP probe | Level 1 source path, targeted timezone tests, and non-TTY timing | Darkmatter's explicit local-only call is correct; the bare Sniff API compatibility contract is still broken |
| F2/F3/F21 — OSC/color probes are cached and CLI detection is reused | Level 1 cache-value tests; unrelated Level 2 frame captures | **Wrong level:** no L2 OSC request count or repeated-construction/CLI latency proof |
| F4 — TOC line lookup is non-quadratic and line-compatible | Level 1 line/span tests plus non-TTY command timing | Mechanism is covered; benchmark fixture identity and threshold gates remain unsatisfied |
| F5/F6/F8/F9/F26-F29 — schema resolution, coercion, cache reuse, hashing, and ownership | Level 1 schema/compose tests plus same-source Criterion A/B for F29 | Behavior passes; F29's explicit `Arc<Value>` exception is measured and supported |
| F7/F16/F18 — graph/preflight work reuse | Level 1 graph, fragment, and compose tests | Safe graph/Arc reuse landed; cross-pass prepared-content work remains open and the public graph API is unchecked |
| F10-F17/F30-F35 — allocation, scanning, shell, and copy reductions | Level 1 unit/integration tests and Criterion smoke data for landed subsets | Multiple findings remain deferred or partial; no approved closeout disposition |
| F19/F20/F23/F24 — render-path reductions preserve output | Level 1 snapshots/units plus Level 2 real-terminal frames | Appropriate output verification; F23's environment-resolution sub-item remains deferred |
| F22 — directory hashing prunes vendored trees | Level 1 collector unit test | User-facing CLI aggregate semantics and compatibility ruling are missing |
| F25 — cleanup pass reduction preserves canonical output | Level 1 cleanup suite | Placeholder fusion is covered; specified line-pass fusion remains open |
| Cross-platform contract | macOS compile/test execution and target-gated source inspection | No current Linux/Windows CI evidence was recorded for this review |

No requirement concerns keyboard events, modifier presses, paste, IME, or mouse input, so Level 3
is not applicable.

## Verification performed

- Read the complete specification, plan, baseline, both result documents, prior review, relevant
  implementation history, and the current Darkmatter, Sniff, and Biscuit Terminal code paths.
- Used the current Darkmatter GitNexus index to trace compose, schema, TOC, and prebuilt-reference-
  graph surfaces. The index matched this worktree's `e0030b854` HEAD.
- `darkmatter/just test`: 5,616 library, 555 CLI, and 566 DMLS tests passed.
- `darkmatter/just test-l2`: 19 library, 69 CLI, and 3 DMLS real-terminal tests passed.
- `darkmatter/just lint`: passed for the library, CLI, and DMLS.
- Sniff's timezone/NTP slice: 25 tests passed; `sniff/just lint` passed. The full Sniff L1 run
  reached 1,332/1,333 passing before the known host/worktree-sensitive
  `detect_area_errors_when_not_in_repo` test timed out at 30 seconds; its retry was stopped under
  the non-interactive 60-second rule.
- Biscuit Terminal's cache slice: 3 tests passed; `biscuit-terminal/just lint` passed. Its full L1
  run reached 2,767/2,768 passing; the known width-sensitive `layout_matrix_snapshots` failure
  reproduced on every retry and is unrelated to OSC caching.
- `md schema validate` could not validate this review because the repository's
  `schemas/feature-review.yaml` is itself rejected as a tagged standalone schema: it contains the
  unsupported `$schema` and `description` keys. The requested review frontmatter is retained.
- Final whitespace checks passed for both requested documents. No write-mode formatter was run.

The broad green suites establish compatibility for the landed mechanisms. They do not substitute
for the missing requirement-matched performance artifacts described in the findings above.
