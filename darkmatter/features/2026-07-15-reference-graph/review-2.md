---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T03:42:38-07:00
spec: 2026-07-15-reference-graph/spec.md
implemented: false
description: "A **feature** review of `2026-07-15-reference-graph/spec.md`"
feature: 2026-07-15-reference-graph/review-2.md
previous: 2026-07-15-reference-graph/review-1.md
---

# Review 2 — Reference Graph

## Verdict

Not ready for production. The first review's complete-context fingerprint, typed
length-delimited encoding, structured public mismatch classification, lifecycle cases, and
cross-commit benchmark evidence are present. The public graph remains opaque and the core
provenance design is intact.

However, path-valued options are still encoded through `Path::display()`, which is lossy on
macOS and Linux. Distinct non-UTF-8 paths can therefore receive the same graph fingerprint even
when they resolve references differently. The performance report also measures candidate commit
`b425fb466`, before the current identity-encoder fixes, so it does not establish AC13 for the
implementation reviewed here. Two first-review test requests are represented only by
identity/provenance checks rather than the required graph-building lifecycle.

## Findings

### High — Native paths are still encoded lossily and can false-match

The new encoder preserves field and collection boundaries, but every path is first converted with
`display().to_string()` (`compose/context/options.rs:1577-1579`, `1627-1629`, `1645-1655`,
`1716-1718`, `1826-1828`; the cache subset repeats this at `1881` and `1940`). `Display` is a
presentation API. On Unix, `Path`/`OsStr` may contain arbitrary non-UTF-8 bytes, and lossy display
replaces different invalid byte sequences with the same replacement character.

This is a false-success path through the core invariant. Two `magic_paths`, shell roots, cache
roots, or fallback directories can be unequal as `PathBuf` values and select different files while
producing the same `graph_value_fingerprint`. Prebuilt validation can then accept a graph built
from different resolution inputs, violating AC4 and AC5. Because the same conversion is used by
the compose-cache product, the collision can also select an unrelated persistent cache entry.

Encode native path data without a UTF-8 presentation conversion. A dedicated cross-platform path
encoder should use length-delimited Unix bytes and Windows wide units (with an explicit platform
tag), or an equivalently exact representation. Add a Unix-gated regression using two distinct
invalid-UTF-8 paths that currently display identically, plus portable separator-bearing path
cases for both graph and cache fingerprints.

### High — The construction-regression report does not measure the reviewed implementation

The new cross-commit table is useful evidence for commit `b425fb466`
(`results.md:146-163`), but the current review includes substantial uncommitted changes to
`classify_options`, complete context fingerprinting, both identity encoders, and graph tests. A
byte-identical benchmark source does not make those two implementations identical. AC13 applies
to the final implementation, not to the commit immediately before the review fixes.

A partial current-worktree run measured `small/construct` at 234.66 us
`[232.79 us, 234.66 us, 236.83 us]`, versus the recorded pre-opacity 167.18 us median: +67.48 us
and +40.4%. That fixture still passes the two-part budget because the absolute delta is below
100 us, but the large and multi-transclusion current-worktree measurements did not complete, so
the overall gate remains unknown.

Freeze the final candidate revision, rerun all three `construct` fixtures against the same
pre-opacity baseline on the same host/toolchain, and update `results.md` with the final revision,
medians, dispersion, and deltas. Retain the existing reuse-win measurements, but rerun them when
the final identity implementation changes their timed path.

### Medium — Volatile-context regressions do not build or reuse a real graph

The new tests prove that two option identities differ and that a directly assembled
`ReferenceGraphProvenance` returns `Options`
(`compose/context/options.rs:2319-2362`; `reference/provenance.rs:475-526`). They do not call
`Markdown::reference_graph`, do not demonstrate that interpolation or `when=` produces different
graph contents, and do not call the public `validate_references_with_graph` path. This falls short
of review 1's requested Level 1 regression that constructs two contexts and proves real graph
reuse is rejected for both behaviors.

Add builder/validator tests that use a timestamp-interpolated reference target and a volatile
context-controlled transclusion. Assert the built graphs differ as expected, then assert the
public validation call rejects cross-context reuse with the typed
`ReferenceGraphMismatchKind::Options` classification.

### Medium — Remote-fetch lifetime coverage does not prove graph non-retention

`prebuilt_graph_rejects_recreated_shared_remote_fetch` builds a graph and confirms that a fresh
runtime mismatches (`tests/reference_integration.rs:1801-1840`). That result is identical whether
the graph retained the old runtime strongly or retained only a `Weak`: a different live allocation
must mismatch in both cases. The unit test at `compose/context/options.rs:2451-2470` captures an
options identity without constructing a graph, so it also does not establish AC6's graph-ownership
claim. The same logical weakness exists in the public preflight lifetime test, although the
crate-private preflight strong-count test gives better identity-level evidence.

Expose a test-only weak/strong-count or drop-probe handle for the remote runtime, build and drop a
real graph while retaining an external observation handle, and assert graph construction and
ownership never increase the final strong count. Use the same graph-level pattern for preflight so
the tests prove the stated ownership contract rather than infer it from a fresh-instance mismatch.

## Prior-review closure

| Review 1 finding | Current status |
|---|---|
| Context values omitted from graph identity | Value-level fix present; end-to-end graph regression still missing |
| Delimiter collisions and `Debug` enum encoding | Fixed for strings, collections, and enums; native path conversion remains non-injective |
| Missing pre-opacity construction baseline | Baseline added, but candidate predates the current fixes |
| Incomplete stateful lifecycle coverage | Preflight/fetch cases added; remote graph-retention assertion remains non-proving |
| Public tests depend only on message text | Fixed: public fingerprint-free mismatch kinds are exposed and asserted |

## Requirement-to-verification assessment

| Requirement | Strongest current evidence | Assessment |
|---|---|---|
| AC1-3 — opaque immutable graph and one construction route | Source/compile-time privacy plus Level 1 builder tests | Appropriate and present |
| AC4 — reject document, source, mode, options, and descendant mismatches | Level 1 provenance/integration tests | Most dimensions are covered, but non-UTF-8 path options can falsely match; volatile-context coverage is not end-to-end |
| AC5 — canonical, exhaustive, non-`Debug` identity | Exhaustive destructure, tagged encoder, ordering/collision Level 1 tests | Fails for path values because `Path::display()` is lossy |
| AC6 — graph ownership does not extend stateful lifetimes | Level 1 shell graph count; preflight identity count; recreated-runtime mismatch | Remote graph ownership is not actually observed; preflight graph-level evidence should also be strengthened |
| AC7 — one dependency per local child and stale-child rejection | Level 1 manifest and filesystem integration tests | Appropriate and present |
| AC8 — graph mode solely controls extraction | Source inspection plus Level 1 mode tests | Appropriate and present |
| AC9 — clone-stable identity | Level 1 original/clone/further-clone integration test | Appropriate and present |
| AC10 — accessor/view migration | Source and compile-time API inspection | Appropriate and present |
| AC11 — graph/file-tree/Mermaid/DOT/terminal/JSON compatibility | Level 1 snapshots, model tests, and spawned CLI baselines | Appropriate tier: no renderer, terminal-emulator, or input-encoder behavior changed |
| AC12 — required gates | Current library check passed; focused L1/lint gates did not complete in this review | Not established for the reviewed worktree |
| AC13 — reuse win without material construction regression | Full measurements for `b425fb466`; one partial current-worktree fixture | Not established for the final implementation |

Level 2 and Level 3 are not required. The feature changes graph construction, validation, and
deterministic string/JSON adapters; it does not change terminal glyph layout, SGR styling,
scrolling, paste/mouse/IME behavior, hotkeys, or terminal input encoding. The CLI JSON checks are
spawned-binary Level 1 tests, which is the appropriate verification level for that contract.

## Verification performed for this review

- Read the complete specification, review 1, performance report, graph/provenance/validation
  implementation, option classifier, public errors, graph views, file-tree/CLI call sites, and
  focused tests.
- GitNexus reported HIGH impact for `ComposeOptions::classify_options`: two direct consumers,
  25 upstream symbols, three modules, and the `run_compose_pipeline` execution flow. The TOC-link
  cache change was LOW risk with one direct caller and four impacted symbols.
- Sniff established the affected package area as `darkmatter`; direct area consumers are
  `darkmatter-cli` and `dmls`, while the library also has broader downstream workspace users.
- `cargo check -p darkmatter --lib --color=never`: passed on macOS.
- The focused Nextest command initially found stale `ComposeWarning.line` test accesses; those
  changed concurrently to `line_number`. The rerun repeatedly rebuilt the shared target and did
  not reach test execution inside the non-interactive ceiling, so no current L1 pass is claimed.
- Current-worktree Criterion evidence completed only `small/construct`: 234.66 us median
  `[232.79 us, 236.83 us]`. Large and multi-transclusion measurements did not complete.
- `cargo fmt --check` could not run because `rustfmt` is not installed for the pinned stable
  toolchain. No formatter was installed and no write-mode formatting was run.
- `git diff --check` passed for the reviewed implementation and result-report paths before this
  review file was written.
- Tests and benchmarks ran on macOS only; Windows and Linux execution was unavailable. The path
  collision finding specifically affects Unix-native non-UTF-8 paths and therefore applies to both
  macOS and Linux.
