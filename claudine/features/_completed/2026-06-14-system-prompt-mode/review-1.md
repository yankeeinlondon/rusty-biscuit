---
ready: false
implemented: true
agent: codex/default
created: 2026-06-20T15:58:50
---

# Review: System Prompt Mode

## Summary

The implementation is close to the specification. The core library path now attaches a baseline `SimplifiedSchema` only for `StandardDiscovered` sources, reads `mode` from composed frontmatter, preserves explicit flag authority, falls back defensively for document-owned schema overrides, and updates the system-prompt/frontmatter docs.

I do not consider the feature production ready yet because the user-facing contract "a discovered `mode: replace` file triggers the same replace delivery path as `--replace-system-prompt`" is not verified through the wrapper/provider delivery surface.

## Findings

### High: discovered `mode: replace` is not tested through provider delivery

- Requirement: A discovered `system-prompt.md` with `mode: replace` must propagate through to provider delivery so it uses the same replace path as `--replace-system-prompt`.
- Implementation evidence: `prepare_system_prompt*` computes `PreparedSystemPrompt.mode` from composed frontmatter, and `apply_system_prompt_via_spec` dispatches on `SystemPromptMode`.
- Coverage present: Level 1 unit tests in `claudine/lib/src/system_prompt/prepare.rs` verify `PreparedSystemPrompt.mode == Replace` for discovered files and non-interactive sessions. Provider delivery unit tests verify replace mechanics when `SystemPromptMode::Replace` is supplied directly.
- Gap: no test joins those two halves through the wrapper path. A regression in `resolve_and_apply_system_prompt`, dry-run planning, or provider application could still report/launch append semantics for discovered `mode: replace` while all current tests pass.
- Required fix: add a Level 1 CLI/wrapper integration test with a discovered `system-prompt.md` containing `mode: replace` and a provider whose replace path is distinguishable from append. Assert the dry-run or staged launch plan uses the replace mechanism, for example Codex `model_instructions_file` instead of `developer_instructions`, Claude `--system-prompt-file` instead of `--append-system-prompt-file`, or Qwen `--system-prompt` instead of `--append-system-prompt`.

### High: effective mode report/dry-run rendering lacks user-facing coverage

- Requirement: Prompt reporting/dry-run output should surface the effective mode from `PreparedSystemPrompt.mode`, so a discovered `mode: replace` file visibly reports `mode: replace`.
- Coverage present: Level 1 unit coverage checks that the prepared data model has `mode == Replace`.
- Gap: there is no CLI integration test asserting the actual dry-run/report text contains `mode: replace` for a discovered file. The current `discovered_replace_mode_propagates_to_prepared_mode_field` test only proves the field value, not the user-visible rendering path through `describe_effective` and `log_dry_run`.
- Required fix: add a Level 1 CLI integration test for `claudine <provider> --dry-run` with a discovered `system-prompt.md` containing `mode: replace`, and assert stderr/plain output includes the system-prompt section with `mode: replace`. Level 2 is not required here unless asserting terminal-specific styling, width, OSC8 links, or color.

## Requirement Verification Matrix

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Absent `mode` defaults to append | Level 1 unit | OK |
| `mode: append` resolves append | Level 1 unit | OK |
| `mode: replace` resolves replace in prepare | Level 1 unit | OK |
| Invalid string rejected through schema validation | Level 1 unit | OK |
| Non-string value rejected through schema validation | Level 1 unit | OK |
| `mode: null` behaves as absent | Darkmatter nullable optional enum supports this, but no feature-specific test | Low test gap |
| Full wrapper/provider delivery uses replace path | Split Level 1 unit coverage only; no joined wrapper test | Gap |
| Explicit `--replace-system-prompt` ignores frontmatter | Level 1 unit | OK |
| Non-interactive session preserves discovered replace mode | Level 1 unit | OK |
| Empty body with `mode: replace` disables prompt | Level 1 unit | OK |
| Document `$schema` conflict falls back to append | Level 1 unit | OK |
| Prompt report/dry-run shows effective mode | Data model unit only; no rendered CLI output test | Gap |
| System prompt docs updated | Manual review | OK |
| Frontmatter property catalog updated | Manual review | OK |

## Notes

The implementation comments are heavier than this code needs in a few places, but they mostly encode spec decisions around schema defaults, document-side schema overrides, and performance-sensitive shared compose context. I would not block production on comment volume.

No Level 2 or Level 3 tests are required for the core mode-selection behavior because it is not a terminal encoder/decoder or OS-keyboard requirement. If future tests assert colors, wrapping, or OSC8 links for the rendered report, those assertions should move to Level 2.
