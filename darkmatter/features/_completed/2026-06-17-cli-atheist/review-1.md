---
ready: true
agent: codex
model: ""
resolved: 2026-06-17
---

# Review 1

## Findings

### High: JSON compatibility baselines are captured but not asserted

The spec makes `md validate refs --json` and `md graph --json` compatibility a public API gate: serde-backed output must match the previous hand-rolled JSON exactly unless the change explicitly documents a migration. The implementation adds baseline JSON fixtures under `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`, but the test suite does not compare current output against those fixtures. `validate_refs_json_output` only checks that stdout parses as JSON, and `test_graph_json_output` only checks for `{`, `"references"`, and `example.com`.

Relevant code:

- `darkmatter/cli/tests/validate_refs.rs:19`
- `darkmatter/cli/tests/validate_refs.rs:36`
- `darkmatter/cli/tests/graph.rs:209`
- `darkmatter/cli/tests/graph.rs:220`
- `darkmatter/features/2026-06-17-cli-atheist/plan.md:615`

Verification level: Level 1 is the right level for JSON shape compatibility, but the strongest current coverage is only parseability/substrings. That is not enough to prove the spec's byte-for-byte JSON requirement.

Fix: add deterministic Level 1 tests that normalize expected temp paths where needed and compare the full JSON values, including local paths, remote URLs, fragments, data URIs, inline CSS/script/meta records, validation errors, and graph insertions.

**Resolution (2026-06-17):** Resolved. A shared `common::baseline` helper
now lives at `darkmatter/cli/tests/common/mod.rs`; it loads the fixtures
under `baseline/json/`, normalizes environment-specific values (temp dir
path, its canonicalized form, and the FNV-1a reference-id hash prefix),
and exposes a `normalize(value, paths_to_redact)` walker. New Level 1
tests:

- `validate_refs.rs` — `validate_refs_json_local_baseline`,
  `_remote_baseline`, `_fragment_baseline`, `_datauri_baseline`,
  `_inline_baseline`, `_errors_baseline` cover local paths, remote URLs,
  fragments, data URIs, inline CSS/script/meta records, and validation
  errors. Each test reproduces the content used to capture the baseline
  and asserts the normalized JSON equals the normalized fixture.
- `graph.rs` — `graph_json_local_baseline`, `graph_json_follow_baseline`,
  `graph_json_validate_baseline` cover graph insertions, transclusion
  expansion (`--follow`), and validation overlay (`--validate`).

### High: Delta text renderer extraction lacks byte-for-byte compatibility coverage

`DeltaReport` replaces the CLI delta printer and claims to preserve the previous visual shape, but the integration test only asserts that stdout contains `Modified`. The new renderer emits raw ANSI/Unicode strings from the library, ignores the `Terminal` passed to `TerminalRenderable::render`, and switches verbose frontmatter serialization to `serde_yaml_ng`; those are all plausible output-drift points under a spec that requires behavior to stay byte-for-byte equal.

Relevant code:

- `darkmatter/lib/src/markdown/delta/report.rs:71`
- `darkmatter/lib/src/markdown/delta/report.rs:81`
- `darkmatter/lib/src/markdown/delta/report.rs:123`
- `darkmatter/cli/tests/delta.rs:7`
- `darkmatter/cli/tests/delta.rs:21`

Verification level: Level 1 full-output fixtures are required for this CLI text surface. Level 2 is only needed if the review goal is to verify the styled ANSI bytes through a real terminal capture. Current Level 1 coverage is a substring smoke test, so the migration is not proven behavior-preserving.

Fix: add Level 1 golden tests for normal and verbose `md delta` output across frontmatter, preamble, added/removed/moved sections, whitespace-only changes, code block changes, broken links, and visual diff output. Add Level 2 only for requirements that claim real-terminal rendering/styling behavior.

**Resolution (2026-06-17):** Resolved. `darkmatter/cli/tests/delta.rs`
now carries 14 Level 1 golden tests that assert the full stdout bytes
produced by `md delta` (and `md -v delta` for the verbose branch) against
captured expected output. Coverage:

- no-change, frontmatter-only, frontmatter scalar-type change,
  preamble-only, section added/removed/modified/moved, whitespace-only,
  code-block content modification, code-block language change;
- verbose no-change, verbose frontmatter+content (asserts the statistics
  block, the frontmatter visual diff header, and the highlighted
  `Old`/`New` and `Hello`/`Goodbye` runs), and verbose content-only
  (asserts the body visual diff is emitted and the frontmatter block is
  suppressed).

Output determinism was verified across `NO_COLOR`, `FORCE_COLOR`,
`COLUMNS`, and `CI` before pinning the expected bytes.

### Medium: The no-god-files goal is still visibly unmet

The feature's first goal says each file under `darkmatter/cli/src/` and `darkmatter/cli/tests/` should stay under the ~500-line soft cap. The new `just lint-files` helper is useful, but it reports several files still over the cap:

- `darkmatter/cli/src/commands/compose.rs`: 1021 lines
- `darkmatter/cli/src/commands/schema/about.rs`: 626 lines
- `darkmatter/cli/src/commands/schema/assignment.rs`: 537 lines
- `darkmatter/cli/tests/code_block.rs`: 747 lines
- `darkmatter/cli/tests/schema_validate.rs`: 555 lines

Relevant spec/code:

- `darkmatter/features/2026-06-17-cli-atheist/spec.md:55`
- `darkmatter/features/2026-06-17-cli-atheist/spec.md:372`
- `darkmatter/justfile:119`

Verification level: structural check, not L1/L2/L3. The helper reports the issue but does not fail. Because the cap is explicitly soft this is not a production blocker by itself, but it means the feature is not complete against its own primary maintainability objective.

Fix: either split the remaining over-cap files along the responsibilities already named in the spec, or update the spec/plan with explicit accepted exceptions and why they are not god-files despite the line count.

**Resolution (2026-06-17):** Resolved via documented exceptions. Per the
review's second remediation path, the spec gains an **Accepted Over-Cap
Exceptions** section (after § Out of Scope) with a per-file rationale
table. None of the five files matches the god-file pattern this feature
targets (hundreds of unrelated top-level symbols); each is a single-
responsibility module whose line count is driven by coverage of that
responsibility. `just lint-files` (`darkmatter/justfile`) now carries
the accepted list inline and prints `(accepted: <reason>)` next to
those entries while still flagging any *new* over-cap file. ADR-5 still
holds: the cap remains a soft signal, not a CI gate.

## Requirement Coverage

- Args, commands, output, `tests/cli.rs`, and `tests/level2_layout.rs` are broadly decomposed into the intended module/test layout.
- Level 2 split tests keep `level2_` names and real-terminal harness usage. No Level 3 coverage is required by this feature because it does not define keyboard, paste, mouse, or terminal input-encoder behavior.
- JSON and delta output compatibility are now pinned by Level 1 baseline / golden tests; the feature is production-ready against its byte-for-byte compatibility contract.

## Verification Run

- `cargo test -p darkmatter-cli --test delta --test validate_refs --test graph -- --nocapture` passed (42 tests).
- `cargo test -p darkmatter-cli --tests` passed (573 tests across all binaries).
- `cargo clippy -p darkmatter-cli --tests --no-deps` clean.
- `just lint-files` reports the five accepted exceptions with reasons and exits cleanly.
