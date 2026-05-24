---
ready: false
agent: ""
model: ""
---

# Review: Validation Reporter

## Findings

### High: User-visible terminal output is only smoke-tested, not verified at the required level

The spec is primarily about user-observable terminal rendering: failing checks must show a plain failure header, source location with OSC-8 link, syntax-highlighted YAML, and the underlying reason, while passing checks stay compact. The current reporter tests exercise the branch but explicitly do not capture or assert stderr:

- `claudine/lib/src/harness/report.rs:392` says the failure-block test is "primarily a smoke test".
- `claudine/lib/src/harness/report.rs:415` only calls `report_check_outcomes(&report, &term)`.

Verification level present:

- Failure header text: Level 1 smoke only; no asserted rendered output.
- Source location and OSC-8 hyperlink: Level 1 smoke only; should have Level 2 real-terminal capture because OSC-8 and terminal rendering are user-visible.
- YAML syntax highlighting / SGR styling: Level 1 smoke only; should have Level 2 capture because glyphs, colors, widths, and rendered code-block chrome are terminal-emulator behavior.
- Reason line visibility: Level 1 smoke only; should at least have Level 1 asserted stderr or PTY/binary output.
- Pass-state compact output: Level 1 smoke only; should at least assert exactly one status row for pass cases.

Under the review rubric, this is not production-ready: the strongest tests do not verify the user-facing requirements at the appropriate level.

Recommended fix: split pure formatting into a testable writer or render-to-string helper and add asserted Level 1 tests for exact text/structure. Add Level 2 tests that run a failing validation inside `tmux`, WezTerm, or Kitty and capture pane text/escape output enough to verify the source link/styling path and YAML block render as intended.

### Medium: The YAML snippet is reconstructed, not the offending YAML rule the author wrote

The spec requires showing "the offending YAML rule itself" and says the rich block should show the YAML block "the author wrote." The implementation builds `RuleSource` from the already-parsed `serde_json::Value`:

- `claudine/lib/src/harness/parse.rs:424` calls `build_rule_source(source_path, name, value)`.
- `claudine/lib/src/harness/parse.rs:446-453` re-serializes a new one-key mapping with `serde_yaml_ng::to_string`.

That drops comments, original quoting, anchors, ordering/formatting nuances, and any exact author syntax. It also means the displayed snippet can differ from the markdown the user needs to edit.

Recommended fix: carry raw frontmatter text, locate the actual rule span, and store the original source slice as `yaml_snippet`. If a span cannot be recovered, then fall back to reconstructed YAML and label that behavior internally in tests.

### Medium: Line ranges are never attempted

The spec made line ranges best-effort and the proposed output includes `path:42-44` when possible. The current implementation always leaves them absent:

- `claudine/lib/src/harness/parse.rs:439-442` documents that `line_range` is always `None`.
- `claudine/lib/src/harness/parse.rs:456` sets `line_range: None`.
- `claudine/lib/src/harness/parse.rs:1630` asserts that parsed rule sources have no line range.

This is an implementation gap, not just an unavailable parser capability, because no recovery is attempted from the source markdown/frontmatter text.

Recommended fix: implement a conservative span finder for `pre_checks` / `post_checks` blocks and rule keys. It can be imperfect, but it should populate a range for common list and map forms and omit only when ambiguous.

### Medium: The YAML rendering shape does not match the requested failure block

The spec asks for an indented syntax-highlighted YAML snippet with no surrounding box. The current renderer uses `darkmatter::markdown::YamlBlock` directly:

- `claudine/lib/src/harness/report.rs:165-170`

In the observed targeted test output, that renderer emits a `yaml` label plus blank dark-background lines around the snippet. That may be acceptable for fenced markdown elsewhere, but it is heavier than the requested inline failure block and makes failures noisier.

Recommended fix: expose or add a syntax-highlighting helper that can render raw YAML lines without fence chrome, then assert the captured output shape.

## Test Run

I ran the following targeted tests:

- `cargo test -p claudine parse_rules_carry_source_with_yaml_snippet -- --nocapture`
- `cargo test -p claudine outcome_carries_rule_source_clone -- --nocapture`
- `cargo test -p claudine report_check_outcomes_failure_with_source_emits_block -- --nocapture`

All three passed. The first attempted combined `cargo test` command used multiple test filters in one invocation and failed with Cargo's expected "unexpected argument" error; the corrected individual invocations passed.

## Readiness

Not ready for production. The core plumbing exists, but the feature is user-facing terminal presentation work and lacks asserted output tests plus Level 2 real-terminal verification for the styling/link/rendering requirements.
