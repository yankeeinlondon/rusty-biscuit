---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-15T19:09:49-07:00
spec: 2026-07-15-reference-graph/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-reference-graph/spec.md`"
feature: 2026-07-15-reference-graph/review-1.md
next: 2026-07-15-reference-graph/review-2.md
---

# Review 1 — Reference Graph

## Verdict

Not ready for production. The opaque graph shape, single construction path, provenance check,
dependency re-verification, accessor migration, clone behavior, and JSON presentation are present,
and the focused Level 1 suites pass. However, the options provenance can accept incompatible graph
inputs: it deliberately drops captured context values that graph construction consumes, and its
delimiter-based collection encoding has collisions. Both defects can let validation reuse a stale
graph rather than reject it.

The required performance gate is also incomplete. The recorded Criterion run establishes that
prebuilt validation is faster than rebuilding, but it contains no pre-opacity construction
baseline and therefore cannot establish the specified construction-regression threshold.

## Findings

### High — Options identity excludes context values that can change graph contents

`ReferenceGraphOptionsIdentity::capture` incorporates `context_hash(context)`, but that shared
cache-oriented hash removes `now`, `now_utc`, `utc`, `time`, `time_military`, `timestamp`,
`timestamp_ms`, `memory_used`, and `memory_avail`
(`compose/cache/hashing.rs:98-128`; `compose/context/options.rs:1507-1510`). Graph construction does
consume the captured context: InlinePre preparation performs interpolation, and the same context
is placed into the effective state used by transclusion `when=` conditions
(`reference/graph.rs:266-269`, `291-343`).

Consequently, two independently captured `ComposeOptions` values can have equal graph identities
while producing different references or descendant sets. A document whose link/transclusion
contains `{{ ctx.timestamp }}`, or whose `when=` expression depends on one of the omitted values,
can build one graph and later pass compatibility with options that would build another. The
validator trusts that comparison before flattening (`reference/validate.rs:572-588`), so this is a
false-success path through the feature's core invariant and violates options-identity requirement
5 and acceptance criteria 4–5.

Give graph identity its own complete context encoding instead of reusing the persistent-cache
context hash. Include every captured context/environment value graph preparation can observe, and
add a Level 1 regression that constructs two contexts differing only in a currently omitted value
and proves graph reuse is rejected for both interpolation and conditional transclusion.

### High — The claimed canonical options encoding is non-injective and uses `Debug`

The graph fingerprint serializes collections by joining raw values with commas, then joins fields
with NUL (`compose/context/options.rs:1444-1510`, `1532-1559`, `1656-1665`). This does not preserve
element boundaries. For example, `pre_approved_commands = {"a,b"}` and
`pre_approved_commands = {"a", "b"}` both encode as
`pre_approved_commands=Some:[a,b]`; the equivalent collision exists for `exclude_keys`, and paths,
hosts, and ordered vectors have similar delimiter ambiguity. These option values are not
equivalent and can change shell preparation, reference contents, or resolution behavior, yet the
graph check can accept them as identical.

The same encoder formats `PathPosition`, `ignore_invalid_references`, timeout/list/cache modes, and
other enums with `{:?}` (`compose/context/options.rs:1437-1439`, `1469`, `1474`, `1493-1498`),
despite the specification explicitly prohibiting `Debug` as canonical encoding
(`spec.md:374-389`, AC5).

Replace the ad hoc strings with a versioned tagged encoder that writes explicit discriminants and
length-prefixed byte fields (or an equivalently unambiguous canonical DTO). Keep the exhaustive
no-`..` destructure, but encode enums with stable explicit tags. Add collision regressions using
separator-bearing collection values and paths, in addition to the existing insertion-order tests.

### High — Construction-regression evidence required by the release gate is missing

The benchmark records candidate-only `build_and_validate`, `validate_prebuilt`, and `construct`
medians and demonstrates a meaningful reuse win on all three fixtures
(`results.md:60-98`). It does not compare construction against the pre-opacity implementation.
The report explicitly states that no baseline exists, provides a future cross-commit procedure,
and substitutes an analytical claim that the delta cannot exceed a few microseconds
(`results.md:118-140`).

That is insufficient for the specification's quantitative gate: a regression is unacceptable
when it exceeds both 5% and 100 microseconds, and acceptance criterion 13 requires evidence that
the intended reuse win did not introduce a material construction regression
(`spec.md:612-624`, `653-678`). Complexity reasoning cannot establish either measured threshold.

Run the identical, fingerprinted fixture and benchmark binary against a pre-opacity commit and the
candidate worktree on the same host/toolchain, recording both median distributions and deltas.
Keep the existing prebuilt-versus-rebuild numbers; they answer the other half of the gate.

### Medium — Stateful-resource lifecycle coverage is incomplete

The implementation uses weak identity handles, and the tests cover shell-handler recreation,
shell-handler strong counts, and clone/fresh-instance behavior for the shared remote-fetch runtime
(`compose/context/options.rs:1919-1965`; `tests/reference_integration.rs:1511-1598`). The
specification separately requires recreated callback, preflight, and fetch instances to be
rejected and graph ownership not to increase the final strong count of any of them
(`spec.md:577-580`).

There is no equivalent dropped-and-recreated preflight-graph case, no preflight strong-count
assertion, and no remote-fetch drop/lifetime assertion. The remote-fetch unit test compares two
live independently created instances, which does not exercise the expired-weak-handle path. Add
the missing Level 1 lifecycle cases through graph construction and validation, not only direct
identity comparison.

### Medium — Public mismatch tests still depend on diagnostic text

The crate-private provenance unit tests assert the structured `ReferenceGraphMismatch` variants,
but the public `ReferenceGraphMismatchError` keeps its reason private and exposes no typed
classification (`reference/errors.rs:9-59`). Public integration tests therefore distinguish
document/source/options/dependency failures with `err.to_string().contains(...)`, including the
stateful mismatch cases (`tests/reference_integration.rs:1511-1562`). This is exactly the brittle
message-substring testing the validation contract asks to avoid (`spec.md:442-448`).

Expose a public, fingerprint-free mismatch kind (and dependency kind/source where applicable)
through an accessor on `ReferenceGraphMismatchError`, then assert those values in integration
tests while retaining separate rendering assertions for the human-readable diagnostic.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| AC1–3 — opaque immutable graph and one provenance-computing construction route | Source/compile-time privacy plus Level 1 builder tests | Appropriate and passing |
| AC4 — reject root, descendant, source, mode, and options mismatches before flattening | Level 1 provenance and integration tests | Root/source/mode/descendant cases pass; options can falsely match because context and collection identity are unsound |
| AC5 — canonical, compact, exhaustive, non-`Debug` options identity | Exhaustive-destructure and ordering unit tests | Gap: delimiter collisions and `Debug`-derived enum encoding violate the requirement |
| AC6 — graph ownership does not extend stateful lifetimes | Level 1 shell-handler strong-count test | Gap: preflight and remote-fetch lifetime/drop paths are not verified |
| AC7 — one dependency per unique local child and stale-child rejection | Level 1 production-builder manifest tests plus changed/missing/unreadable integration tests | Appropriate and passing; validation iterates the deduplicated manifest once |
| AC8 — graph mode solely controls extraction | Source inspection plus Level 1 mode tests | Appropriate and passing |
| AC9 — clone-stable identity | Level 1 original/clone/further-clone integration test | Appropriate and passing |
| AC10 — callers use read-only accessors/views | Source and compile-time API inspection | Appropriate and passing |
| AC11 — graph, file-tree, Mermaid, DOT, terminal, and JSON compatibility | Level 1 unit/snapshot tests and spawned CLI JSON baselines | Appropriate for the non-interactive behavior reviewed; focused CLI baseline passes |
| AC12 — required gates | Focused Level 1, area lint, area build, and whitespace checks pass; full area test and GitNexus checks were attempted but did not complete within the non-interactive ceiling | Partially established; no complete current area test or GitNexus result is claimed |
| AC13 — prebuilt win without material construction regression | Candidate-only Criterion medians | Reuse win is demonstrated; construction regression is not measured against a baseline |

Level 2 and Level 3 verification are not required for this feature. It changes no real-terminal
rendering contract or terminal input encoding, and the specification explicitly excludes those
tiers. The CLI JSON tests are Level 1 spawned-binary baselines, which is the appropriate tier for
their deterministic output contract.

## Verification performed for this review

- Inspected the complete specification, implementation, result report, graph/provenance/options
  paths, public accessors and views, CLI graph command, and focused unit/integration coverage.
- `cargo nextest run -p darkmatter --color=never -E
  'test(/prebuilt_graph/) + test(/reference_graph/) + test(/options_identity/) +
  test(/document_identity/) + test(/graph_ownership/) + test(/unchanged_child/)'`: 31 passed,
  5,771 skipped by the filter. One leaked-handle retry passed on retry.
- `cargo nextest run -p darkmatter-cli --test graph --color=never`: 14 passed.
- Full `just test` reached 1,206 of 5,667 tests with no failures before it was terminated at the
  non-interactive command-time ceiling; no complete area Level 1 result is claimed.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls` with no warnings.
- `just build`: passed for the Darkmatter library, CLI, and language server.
- `git diff --check -- darkmatter/features/2026-07-15-reference-graph/spec.md
  darkmatter/features/2026-07-15-reference-graph/review-1.md`: passed.
- Review-frontmatter validation could not run because the repository's
  `schemas/feature-review.yaml` is itself rejected by `md schema validate` as a standalone tagged
  schema: it contains unsupported `description` and `$schema` keys. The requested frontmatter was
  retained exactly.
- GitNexus index refresh and query were attempted, but neither completed within the non-interactive
  ceiling. Direct source inspection and executable tests were used instead of treating a stale
  index as evidence.
- Tests ran on macOS only. No platform-specific implementation defect was found, but Windows and
  Linux execution was not available in this review.
