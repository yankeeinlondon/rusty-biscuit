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

### Low: OSC-8 hyperlink target carries no line anchor

The displayed source line includes `:42-44` but the OSC-8 link target
constructed at `report.rs:241-255` is the bare absolute file path. Modern
editors support `file:///path:42:1` and `file:///path#L42` jump anchors;
clicking the rendered link in iTerm2/VS Code/Ghostty/Kitty opens the file
at line 1, not at the offending rule.

Recommended fix: when `line_range` is `Some`, append `:N:1` (clangd-style)
to the link target. Keep the display-text format unchanged. Add a
`render_failure_block_to_string` test asserting the OSC-8 target contains
the line suffix when a range is present.

### Low: Silent unstyled-fallback path is untested

`report.rs:270-278` has two parallel emission branches: styled when
highlighter line count matches plain count, unstyled otherwise. The
unstyled branch has zero coverage. If `highlight_yaml_lines` ever returns
extra lines (trailing newline edge case, multi-line scalars), users see
unstyled output and no test catches the regression.

Recommended fix: extract the line-count check into a single function that
returns a `Result<Vec<String>, Vec<String>>` (or similar) so each branch
can be unit-tested in isolation. Or, more simply, add one test that
constructs a `RuleSource` whose `yaml_snippet` is empty after `trim_end`
and asserts no YAML lines emit; and one that uses a single-line snippet
that round-trips through both highlighter and plain path identically.

### Low: `prose_escape` allocates 6 strings per call

`report.rs:92-99` chains six `String::replace` calls; every failure block
runs this 2–3 times (path display, abs path, suffix, reason). On a single
failure this is invisible; on a harness with 50 failing checks it is 200+
unnecessary allocations. A single-pass implementation that reserves
`s.len() + small` and walks bytes once would be both shorter and faster.

Recommended fix: replace with a single linear scan. Bonus: the current
`replace('"', "&quot;")` is asymmetric with the other replacements (HTML
entity vs backslash escape) — confirm Prose's grammar treats both as
equivalent and document why if so.

### Low: `report_check_outcomes_to_string` is `#[cfg(test)]`-only

`report.rs:181-189` gates the captured-string entry point behind `cfg(test)`.
This is fine today but blocks any future JSON / SARIF / JSONL exporter or
non-TTY snapshot consumer (called out as out-of-scope in the spec but
plausible follow-up). The `LineSink` trait is also private. If a future
consumer needs the rendered transcript, this becomes a gratuitous
re-export refactor.

Recommendation: leave `cfg(test)` for now; revisit when the JSON exporter
spec lands.

### Nit: File-level `#![allow(deprecated)]`

`report.rs:6` blanket-allows deprecated for the whole file. The original
reason isn't documented at the suppression site. Either narrow to the
specific item (`#[allow(deprecated)] fn foo`), document why a file-wide
allow is needed, or remove if the deprecation is no longer present in the
referenced API. A `git blame` on this line would clarify, but the
suppression should carry its own justification.

### Nit: `ValidationCheckOutcome` clones `RuleSource` per outcome

`validate.rs:181` clones `rule.source` (which contains the full
`yaml_snippet` String) into every outcome. For the current code paths
(one outcome per rule) this is exactly one clone per rule and harmless.
If the future ever runs the same rule against multiple subjects (e.g.
fan-out validations like `frontmatter_prop_equals` over a list), the
snippet text gets duplicated N times. An `Arc<RuleSource>` on the outcome
would make this cheap. Not actionable today.

## Spec-vs-implementation traceability

Each spec goal maps to verified tests:

| Spec goal | Implementation | Verification |
|---|---|---|
| 1. Failure stated plainly (no positive assertion + red glyph) | `failure_header_text` (`report.rs:214-221`) | L1: `failure_state_pre_phase_uses_pre_validation_failed`, `failure_state_post_phase_uses_post_validation_failed`. L2: PTY transcript contains `Pre-validation failed`. |
| 2. Surface `evaluate_single` diagnostic | Section 4 reason line (`report.rs:283-288`) | L1: `failure_state_full_block_contains_all_four_sections` checks `Reason: file does not exist:` + path. L2: stripped transcript contains it. |
| 3. Source path with OSC-8 link + line range | `render_failure_block_to` source-line section (`report.rs:241-255`) | L1: `osc8_hyperlink_target_present_in_raw_output`, `failure_block_omits_line_range_suffix_when_unknown`. L2: OSC-8 introducer + `file://` target asserted. |
| 4. Syntax-highlighted YAML snippet | `highlight_yaml_lines` path + four-space indent (`report.rs:266-279`) | L1 negative (no chrome): `render_failure_block_yaml_section_has_no_yaml_label_or_box`. L1 indent: `render_failure_block_yaml_section_indents_each_line_by_four_spaces`. **No L1/L2 positive styling assertion** — see Medium finding above. |
| 5. Pass-state stays compact (one line) | `report_check_outcomes_to::passed` branch (`report.rs:198-199`) | L1: `pass_state_renders_exactly_one_compact_row`, `report_check_outcomes_pass_path_unchanged`. |

## Readiness

Ready for production. The Medium finding (positive YAML styling assertion)
is a tightening recommendation, not a behavioral defect — manual smoke
through the PTY harness shows the styling is present. The Low and Nit
items are quality-of-life improvements. None of them block merge.

If the YAML styling fallback ever becomes a regression risk (e.g. after a
darkmatter highlighter API change), revisit the Medium finding before
shipping that change.
