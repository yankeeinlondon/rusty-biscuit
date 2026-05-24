---
agent: open_code
model: ""
ready: true
resolution: 2026-05-11
---

# Review: Prompt Reporting (Iteration #1)

## Resolution Summary (2026-05-11)

All findings from this review have been addressed:

- **F1 + F6 [high]**: Resolved. `SystemPromptReportConfig` gained a
  `show_summary: bool` field. Both CLI `--verbose` and frontmatter
  `verbose` now render the Summary line **and** the Full Prompt body.
  Integration tests `compose_verbose_shows_full_prompts` and
  `compose_frontmatter_verbose_shows_full_system_prompt` now assert
  that the token-count summary line appears alongside the body.
- **F2 [high]**: Resolved. `render_system_prompt_summary` now produces
  the spec-mandated prose: `The system prompt was **{action}**; the
  content was _composed_ from <hyperlink>. {token-message}` with
  action = `appended to` / `replaced` and the matching `composed` /
  `replacement` token message. Two new unit tests (`summary_uses_spec_prose_format_*`)
  and a relative-path test (`summary_relative_path_when_base_provided`)
  guard the format. Path-resolution now uses the launch-context's
  repo root (or CWD) so hyperlink labels are relative.
- **F3 [medium]**: Resolved by documentation. The spec referenced
  "biscuit-terminal's FileTree utility" but `FileTree` renders Markdown
  dependency graphs — it does not count tokens. The actual token
  estimator inside biscuit-terminal (`components::filesystem::estimate_tokens`)
  uses the same 4.0 chars/token (prose) and 2.5 chars/token (dense)
  heuristic that `tokens.rs` already implements. This alignment is now
  documented in the module docstring.
- **F4 [low]**: No action required — the spec does not assign
  PartialPrompt to any system-prompt precedence rule.
- **F5 [medium]**: Resolved. New `prompt_reporting::state` module hashes
  the composed prompt (blake3) and stores it under
  `~/.claudine/state/system_prompts/{xxhash}.txt` keyed on the launch
  repo root (or CWD). `resolve_system_prompt_report_config_with_change`
  consults this in the default precedence branch and suppresses the
  header when the prompt is unchanged; CLI flags, env var, frontmatter,
  and length-based short-circuit all override the suppression per spec.
  Four new precedence tests cover the override matrix.
- **F6 [high]**: See F1 above (same fix).
- **F7 [low]**: Resolved. Three new integration tests
  (`compose_env_var_verbose_shows_full_system_prompt`,
  `compose_env_var_quiet_shows_summary_only`,
  `compose_env_var_silent_suppresses_system_prompt`) exercise the
  `CLAUDINE_SYSTEM_PROMPT` env var end-to-end.
- **F8 [low]**: Resolved. Two new integration tests
  (`compose_user_prompt_at_40_lines_renders_full`,
  `compose_user_prompt_at_41_lines_uses_frontback`) exercise the 40/41
  boundary through the compose pipeline.
- **F9 [low]**: No change — implementation is spec-compliant; flagged
  for awareness only.
- **F10 [low]**: No change — acceptable per spec.

**Test status:** 102 lib tests + 12 integration tests pass.

---

## Summary

The implementation provides a well-structured `prompt_reporting` module under `claudine/lib/src/prompt_reporting/` with nine files covering types, precedence, frontmatter, token estimation, truncation, formatting, system-prompt rendering, user-prompt rendering, and a module root. The CLI wires these through `output/mod.rs` into both the compose and direct-wrapper paths. There are 92 unit tests across all library modules and 7 integration tests in `cli/tests/prompt_reporting.rs`. All tests pass.

The implementation is **not production-ready** due to several spec deviations, a missing combined Summary+FullPrompt mode for verbose, incomplete summary text, missing token-estimation fidelity, and missing PartialPrompt path for system prompts.

---

## Findings

### F1: Verbose mode should render Summary + Full Prompt, not Full Prompt alone [severity: high]

**Spec (line 122):** `--verbose` "will always show the **Summary** information and then the **Full Prompt**"

**Implementation:** `resolve_system_prompt_report_config` at `precedence.rs:71` returns `PromptReportFormat::FullPrompt` with no summary. The `report_system_prompt` function at `system_prompt.rs:227` dispatches on format — when `FullPrompt`, it renders only the body, skipping the summary entirely. The same applies to frontmatter `verbose` at line 136 of the spec.

**Impact:** Verbose users see the raw prompt but lose the token count, source hyperlink, and action label that the summary provides. This directly contradicts the spec's "Summary and then Full Prompt" requirement.

**Verification level:** Level 1 only (integration test `compose_verbose_shows_full_prompts` checks body content appears, but does not verify summary accompanies it).

**Fix:** Add a `SummaryAndFull` variant or a `show_summary: bool` field to `SystemPromptReportConfig`, and render both in `report_system_prompt` when verbose.

---

### F2: Summary text does not match the spec's required format [severity: high]

**Spec (lines 100-111):** The summary must render as:
- `The system prompt was **{action}**; the content was _composed_ from <a href={absolute-path-to-prompt}>{relative-path-to-prompt}</a>. {token-message}`
- `{action}` is `appended to` or `replaced` (note: "appended to", not "appended")
- `{token-message}` is `The composed system prompt is roughly {#} tokens.` or `The replacement system prompt is roughly {#} tokens.`

**Implementation:** `render_system_prompt_summary` at `system_prompt.rs:75-96` renders:
- `source: <blue><dim>{path}</dim></blue> · {token_count} tokens`
- This is a compact `source · N tokens` format, not the prose sentence the spec requires.
- The action word ("appended to" vs "replaced") is not included in the summary.
- The action is never rendered as "appended to" (with "to") — only "appended" appears in the header.

**Verification level:** Level 1 (unit tests check for "tokens" and "built-in" strings, not spec-format text).

**Fix:** Rewrite `render_system_prompt_summary` to produce the exact prose format from the spec, including action word, hyperlink, and token-message sentence.

---

### F3: Token estimation uses character-count heuristic instead of FileTree [severity: medium]

**Spec (line 107):** "token estimation uses **biscuit-terminal's FileTree utility** (not a simple character-count heuristic)"

**Implementation:** `tokens.rs` uses a 4-chars-per-token heuristic (`estimate_tokens`). No `FileTree` integration exists anywhere in `prompt_reporting/`.

**Note:** `FileTree` actually lives in `darkmatter` (not `biscuit-terminal`), but regardless, the implementation does not use it. The spec explicitly calls out that this should not be a simple character-count heuristic.

**Verification level:** Level 1 (unit tests verify the heuristic arithmetic, not FileTree integration).

**Fix:** Integrate `darkmatter::markdown::reference::file_tree::FileTree` for token estimation, or document and get explicit approval that the simpler heuristic is acceptable for v1. The limitation note in the spec ("Claudine cannot measure the agent platform's original/default system prompt") is already acknowledged in the doc comment on `estimate_system_prompt_tokens`.

---

### F4: No PartialPrompt path exists for system prompts [severity: medium]

**Spec (lines 112-116):** The spec defines PartialPrompt with both Truncate and FrontBack variants. The precedence chain does not mention when PartialPrompt should be used for system prompts, but the format is defined as an available body variant.

**Implementation:** `resolve_system_prompt_report_config` never returns `PromptReportFormat::PartialPrompt`. The only format values returned are `Summary`, `FullPrompt`, and (implicitly via the `TruncationMode` field) support for truncation. But `PartialPrompt` is never selected by the precedence resolver.

**Assessment:** The spec does not define a precedence rule that selects PartialPrompt for system prompts (it only uses Summary and FullPrompt). However, the types and rendering code support it. This is likely deferred work. The `PartialPrompt` rendering path in `render_system_prompt_body` does exist and works correctly.

**Verification level:** Level 1 (partial_format tests exist but are unreachable through the precedence resolver).

**Status:** Low concern — the spec doesn't assign PartialPrompt to any system-prompt precedence rule, so this is correctly deferred. The user prompt path does exercise PartialPrompt when line count > 40.

---

### F5: System prompt "unchanged" conditional not implemented [severity: medium]

**Spec (line 92):** "the default condition (when no CLI flag, ENV variable, or frontmatter has selected a body mode) is to show Line 1 when the system prompt has changed or when the caller has used the `verbose` flag"

**Implementation:** `resolve_system_prompt_report_config` always returns `show_header: true` for the default case. There is no detection of whether the system prompt has "changed" versus a previous session. The "unchanged" suppression is not implemented — the header always appears when a system prompt is present.

**Assessment:** Detecting "changed" would require comparing the current composed prompt against a previous session's prompt, which is non-trivial and would need persistent state. This may be a v2 concern, but the spec does list it as a requirement.

**Verification level:** No test coverage for unchanged behavior.

---

### F6: Frontmatter verbose for system prompt should also show Summary + Full [severity: high]

**Spec (line 136):** Frontmatter `verbose` "suggests that this system prompt should report the Summary and the Full Prompt"

**Implementation:** `config_from_verbosity(Verbose)` at `precedence.rs:118` returns `FullPrompt` only, same issue as F1.

**Verification level:** Integration test `compose_frontmatter_verbose_shows_full_system_prompt` verifies body appears but also asserts `!plain.contains("tokens")` — i.e., it **verifies** that no summary appears, which contradicts the spec.

---

### F7: CLAUDINE_SYSTEM_PROMPT env var not tested at integration level [severity: low]

The env var is parsed in `output/mod.rs:194` and unit-tested in precedence tests. No integration test sets `CLAUDINE_SYSTEM_PROMPT=verbose|quiet|silent` and verifies the compose pipeline behavior. This is a test coverage gap.

**Verification level:** Level 1 (unit tests only).

---

### F8: No test for user prompt exactly at 40-line boundary [severity: low]

The user prompt switches from `FullPrompt` to `PartialPrompt` at > 40 lines. The precedence test checks `<= 40` → Full and `41` → Partial. Good boundary coverage. However, there is no integration test exercising a user prompt with exactly 40 lines vs 41 lines through the compose pipeline.

**Verification level:** Level 1 only for the boundary condition.

---

### F9: Leading whitespace stripping for user prompt may be too aggressive [severity: low]

**Spec (line 164):** "all leading whitespace should be removed in all cases"

**Implementation:** `strip_leading_whitespace` at `truncation.rs:85` strips leading whitespace from every line via `trim_start()`. This removes indentation from code blocks and nested lists. If the user prompt contains indented content (e.g., YAML, code), this will mangle it.

**Assessment:** The spec is explicit, so the implementation is spec-compliant. Flagging for awareness — this may need refinement if user prompts contain structured indented content.

---

### F10: The `verbose` flag on compose uses `u8` count but precedence takes `bool` [severity: low]

In `composition/mod.rs:1456`, the call is `crate::output::log_system_prompt(&effective_sp, detail_requested, silent, quiet, &term)` where `detail_requested` is `verbose > 0` (a bool). This is correct but means the compose path cannot distinguish between `-v` and `-vv`. The spec only defines boolean behavior, so this is fine for now.

---

## Test Coverage Assessment

| Module | Unit Tests | Integration Tests | Verification Level |
|--------|-----------|-------------------|--------------------|
| `types.rs` | Implicit via other tests | — | Level 1 |
| `precedence.rs` | 12 tests covering all precedence branches | — | Level 1 |
| `frontmatter.rs` | 6 tests (case variants, missing, invalid) | 1 test (compose with frontmatter) | Level 1 |
| `tokens.rs` | 5 tests (empty, simple, dense, with/without appendix) | 1 test (token count in compose output) | Level 1 |
| `truncation.rs` | 12 tests (boundary, blank-line skip, overlap, strip) | 1 test (long user prompt FrontBack) | Level 1 |
| `formatting.rs` | 13 tests (blank-line collapse, markdown, block quotes) | — | Level 1 |
| `system_prompt.rs` | 12 tests (header, summary, body, top-level reporter) | 5 tests | Level 1 |
| `user_prompt.rs` | 14 tests (header, body, stripping, reporter) | Covered by compose integration tests | Level 1 |
| CLI integration | — | 7 tests | Level 1 |

**All verification is Level 1 (in-process / PTY).** No Level 2 or Level 3 tests exist. For this feature (text rendering, not keyboard/terminal-protocol interaction), Level 1 is appropriate for the data-path logic. However, the spec requirement about the block-quote "vertical line should be a centered line which aligns with the center of the icon" is a visual layout concern that can only be verified at Level 2 (real-terminal capture).

---

## Positive Observations

1. **Clean module decomposition.** The nine-file split is logical and each file has a single, clear responsibility.
2. **Comprehensive precedence tests.** All priority-chain interactions (CLI > env > length > frontmatter > default) are exercised.
3. **Blank-line advancement in truncation.** The `truncate_front_back` function correctly handles blank boundary lines and overlap fallback — well-tested edge case.
4. **Good separation of library vs CLI.** All logic is in the library; the CLI merely wires parameters and calls render functions.
5. **Proper use of biscuit-terminal Prose and BlockQuote.** The rendering pipeline uses the correct components from the monorepo's terminal library.
6. **Darkmatter integration.** Markdown rendering with blank-line collapsing is correctly implemented via the darkmatter library.

---

## Recommended Actions

1. **[High] Implement combined Summary+Full mode** for verbose (both CLI `--verbose` and frontmatter `verbose`). This requires either a new format variant or a `show_summary` flag on `SystemPromptReportConfig`. Update the integration test that currently asserts `!contains("tokens")` in verbose mode.
2. **[High] Rewrite summary format** to match the spec's prose format: "The system prompt was **appended to**; the content was _composed_ from [hyperlink]. The composed system prompt is roughly N tokens."
3. **[Medium] Evaluate FileTree token estimation** or get explicit sign-off that the character-count heuristic is acceptable for v1.
4. **[Medium] Implement "unchanged" detection** or explicitly scope this to v2 in the spec.
5. **[Low] Add integration test for CLAUDINE_SYSTEM_PROMPT** env var.
6. **[Low] Add integration test for user prompt 40/41-line boundary**.
