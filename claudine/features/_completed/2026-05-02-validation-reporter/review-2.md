---
ready: true
agent: ""
model: ""
---

# Review 2: Validation Reporter

This is the second review pass. Review 1 raised one High and three Medium
findings; all four have been addressed. Remaining items are smaller and not
shipping blockers.

## Summary of prior-review fixes

| Prior finding | Status | Evidence |
|---|---|---|
| H: User output only smoke-tested | **Resolved** | `LineSink`/`StringSink` seam at `claudine/lib/src/harness/report.rs:26-67`; ten asserted Level-1 tests including `failure_state_full_block_contains_all_four_sections`, `pass_state_renders_exactly_one_compact_row`, `osc8_hyperlink_target_present_in_raw_output`, `failure_block_omits_line_range_suffix_when_unknown`, `legacy_failure_path_renders_two_lines_only`. Level-2 PTY test in `claudine/cli/tests/validation_reporter_pty.rs` exercises a real PTY through `validation_reporter_pty_harness`. |
| M: Snippet reconstructed instead of original YAML | **Resolved** | `original_yaml_slice` (`parse.rs:544-569`) returns the authored slice when a span is recoverable; `reconstruct_yaml_snippet` is the fallback. Tested by `parse_rules_yaml_snippet_preserves_authored_quoting` and `parse_rules_yaml_snippet_preserves_inline_comments`. |
| M: Line ranges never attempted | **Resolved** | `span::find_rule_spans` (`parse.rs:1315-1333`) walks list and map forms, ambiguity-rejects on duplicate names. Covered by four `find_rule_spans_*` unit tests plus `parse_rules_carry_line_range_when_recoverable`. |
| M: YAML rendering shape (boxed/labeled) | **Resolved** | `render_failure_block_to` switched to `darkmatter::markdown::highlighting::highlight_yaml_lines` (`report.rs:266-279`), no fence chrome. Asserted by `render_failure_block_yaml_section_has_no_yaml_label_or_box` and the Level-2 PTY check `transcript.contains("\nyaml\n")`. |

## Test verification

I ran:

```text
cargo test -p claudine harness::report::    # 37 passed
cargo test -p claudine harness::parse::     # 56 passed
cargo test -p claudine-cli --test validation_reporter_pty -- --ignored
                                            # 1 passed (Level 2)
```

All green.

## Findings

### Medium: Level 2 verifies absence of YAML chrome but not presence of YAML styling — **Resolved**

Resolved by two new positive-styling assertions:

- **Level 1**: `render_failure_block_yaml_region_contains_sgr_styling` in
  `claudine/lib/src/harness/report.rs` renders the canonical happy-path
  snippet (`file_exists: "x"\nmessage: "y"\n`), locates the YAML region
  between the source-location and `Reason:` lines, and asserts at least one
  line carries a `\x1b[38;...m` truecolor foreground SGR — the exact
  introducer `highlight_yaml_lines` emits per highlighted token.
- **Level 2**: the PTY test
  `pty_pre_check_failure_emits_osc8_link_and_styled_header` in
  `claudine/cli/tests/validation_reporter_pty.rs` slices the transcript
  between the `:3-5` source-line marker and the `Reason:` marker and
  asserts the slice contains `\x1b[38;`, guaranteeing the styling survives
  end-to-end through a real PTY.

A degraded highlighter (broken theme load, missing yaml grammar, empty
palette) that triggers the unstyled fallback in `report.rs:266-279` now
fails both tests instead of slipping through silently.

### Low: OSC-8 hyperlink target carries no line anchor — **Resolved**

`render_failure_block_to` in `claudine/lib/src/harness/report.rs` now
constructs the link target separately from the display text. When
`line_range` is `Some(N..=M)`, the target is `<abs-path>:N:1` (clangd
style); when it is `None`, the target stays the bare absolute path. The
displayed text continues to use the human-friendly `:N-M` range format.

Coverage:

- `osc8_hyperlink_target_includes_line_anchor_when_range_present` asserts
  the target contains `:42:1` for a `42..=44` range.
- `osc8_hyperlink_target_omits_line_anchor_when_range_none` asserts the
  target is the bare path (after stripping the optional `file://` scheme)
  when no range is known.
- The existing `osc8_hyperlink_target_present_in_raw_output` and the
  Level-2 PTY assertion both use `contains(abs_path)` so they continue to
  pass with the anchored target format.

### Low: Silent unstyled-fallback path is untested — **Resolved**

The line-count selection logic was extracted from `render_failure_block_to`
into two private helpers in `claudine/lib/src/harness/report.rs`:

- `yaml_snippet_lines(snippet: &str) -> Vec<String>` — owns trimming,
  empty-input handling, and dispatch to the highlighter.
- `select_yaml_lines(plain: &[&str], highlighted: &[String]) -> Vec<String>`
  — owns the count-match decision and the four-space indent.

Because `select_yaml_lines` takes both slices as parameters, the fallback
branch is now directly unit-testable without standing up a degraded
highlighter. New tests:

- `yaml_snippet_lines_returns_empty_for_empty_input`
- `yaml_snippet_lines_returns_empty_for_whitespace_only_input`
- `yaml_snippet_lines_styled_path_indents_each_line_by_four_spaces`
- `select_yaml_lines_styled_branch_uses_highlighted_when_counts_match`
- `select_yaml_lines_fallback_branch_uses_plain_when_counts_differ`
- `select_yaml_lines_fallback_branch_handles_extra_highlighted_entry`
  (covers the trailing-newline edge case the original review called out)
- `select_yaml_lines_styled_branch_handles_empty_inputs`

### Low: `prose_escape` allocates 6 strings per call — **Resolved**

`prose_escape` in `claudine/lib/src/harness/report.rs` now performs one
linear scan with `String::with_capacity(s.len() + 8)`, replacing the
six-step `String::replace` chain. The escape table is unchanged: `\\`,
`<`, `>`, `{`, `}` go to backslash-escaped form; `"` goes to `&quot;`.

The doc comment now records why `"` uses the HTML entity form rather than
a backslash escape: Prose's attribute parser (e.g. inside `href="..."`)
treats `\"` as a literal quote and not as a string delimiter, so the
entity form is the documented mechanism for embedding a quote inside an
attribute value. The asymmetry the review flagged is intentional and now
load-bearing in the comment.

The two existing tests (`prose_escape_handles_special_chars` and
`prose_escape_escapes_double_quotes`) still pass and continue to exercise
the full escape table.

### Low: `report_check_outcomes_to_string` is `#[cfg(test)]`-only — **Deferred**

Per the original review's own recommendation: leave `cfg(test)` for now;
revisit when the JSON / SARIF / JSONL exporter spec lands. No code change.

### Nit: File-level `#![allow(deprecated)]` — **Resolved**

The `#![allow(deprecated)]` at the top of `claudine/lib/src/harness/report.rs`
existed only to silence `StatusState::Failure` (deprecated in
`biscuit-terminal/lib/src/components/status.rs:66` in favor of
`StatusState::Error`). Every `StatusState::Failure` reference in
`report.rs` was swapped to `StatusState::Error`, and the file-level
suppression was removed. The two states share an identical
`(StatusTheme::Circular, …)` icon definition (red 500, same nerd glyph
and emoji fallback), so the rendered output is byte-for-byte unchanged.

### Nit: `ValidationCheckOutcome` clones `RuleSource` per outcome — **Deferred**

Per the original review: "Not actionable today." No code change.

## Spec-vs-implementation traceability

Each spec goal maps to verified tests:

| Spec goal | Implementation | Verification |
|---|---|---|
| 1. Failure stated plainly (no positive assertion + red glyph) | `failure_header_text` (`report.rs:214-221`) | L1: `failure_state_pre_phase_uses_pre_validation_failed`, `failure_state_post_phase_uses_post_validation_failed`. L2: PTY transcript contains `Pre-validation failed`. |
| 2. Surface `evaluate_single` diagnostic | Section 4 reason line (`report.rs:283-288`) | L1: `failure_state_full_block_contains_all_four_sections` checks `Reason: file does not exist:` + path. L2: stripped transcript contains it. |
| 3. Source path with OSC-8 link + line range | `render_failure_block_to` source-line section (`report.rs:241-255`) | L1: `osc8_hyperlink_target_present_in_raw_output`, `failure_block_omits_line_range_suffix_when_unknown`. L2: OSC-8 introducer + `file://` target asserted. |
| 4. Syntax-highlighted YAML snippet | `yaml_snippet_lines` + `select_yaml_lines` helpers, four-space indent | L1 negative (no chrome): `render_failure_block_yaml_section_has_no_yaml_label_or_box`. L1 indent: `render_failure_block_yaml_section_indents_each_line_by_four_spaces`. L1 positive styling: `render_failure_block_yaml_region_contains_sgr_styling`, `yaml_snippet_lines_styled_path_indents_each_line_by_four_spaces`. L1 fallback branch: `select_yaml_lines_fallback_branch_uses_plain_when_counts_differ` and `select_yaml_lines_fallback_branch_handles_extra_highlighted_entry`. L2 positive styling: `\x1b[38;` introducer asserted in the YAML region of the PTY transcript. |
| 5. Pass-state stays compact (one line) | `report_check_outcomes_to::passed` branch (`report.rs:198-199`) | L1: `pass_state_renders_exactly_one_compact_row`, `report_check_outcomes_pass_path_unchanged`. |

## Readiness

Ready for production. All Medium and Low findings have been resolved
except the two explicitly deferred per the original review's own
recommendations (`report_check_outcomes_to_string` `cfg(test)` gating —
revisit when JSON exporter spec lands; `ValidationCheckOutcome` snippet
cloning — not actionable today). The file-level
`#![allow(deprecated)]` Nit is also resolved by switching all
`StatusState::Failure` references to `StatusState::Error`. The remaining
deferred Nit (`ValidationCheckOutcome` clones `RuleSource` per outcome)
is a future-only concern with no current impact.

Verification: `cargo test -p claudine --lib` (2037 passed) and
`cargo test -p claudine-cli --test validation_reporter_pty -- --ignored`
(1 passed) both green.
