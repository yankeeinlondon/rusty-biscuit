---
ready: false
implemented: true
agent: codex/default
created: 2026-06-20T16:18:00
---

# Review: System Prompt Mode

## Summary

The implementation now covers the previous review's main Unix/macOS gaps: a discovered `mode: replace` file is verified through Codex dry-run delivery, and the dry-run report is verified to show `mode: replace`. The core prepare path also matches the spec: discovered files get the baseline schema, explicit flags remain authoritative, invalid values fail through `SystemPromptComposition`, document-owned schema overrides fall back to append, and docs were updated.

I do not consider the feature production ready yet because two user-observable contracts are still under-verified: the joined wrapper/dry-run behavior has no Windows Level 1 coverage, and explicit `mode: null` is documented as valid but lacks an automated feature test.

## Findings

### High: wrapper delivery and dry-run mode reporting are still unverified on Windows

- Requirement: a discovered `system-prompt.md` with `mode: replace` must trigger the same provider replace delivery path as `--replace-system-prompt`, and dry-run/report output must show the effective `mode: replace`.
- Implementation evidence: `prepare_system_prompt*` computes `PreparedSystemPrompt.mode` from composed frontmatter, and `apply_system_prompt_via_spec` dispatches on `SystemPromptMode`.
- Coverage present: Level 1 CLI integration tests in `claudine/cli/tests/wrap_basics.rs` now assert Codex dry-run emits `model_instructions_file=` and reports `mode: replace`.
- Gap: both joined wrapper tests are gated with `#[cfg(unix)]` (`codex_dry_run_discovered_replace_system_prompt_uses_model_instructions_file` and `codex_dry_run_discovered_replace_system_prompt_reports_effective_mode`). On Windows, the exact production surface that was missing in review #1 is still skipped, so CI would not catch a Windows-only regression in discovery, scoped temp file path handling, or dry-run provider argument construction.
- Required fix: make these Level 1 dry-run tests platform-neutral, or add equivalent Windows-enabled tests. Since the tests use `--dry-run`, they should not need to execute the fake provider; a Windows-safe fake `codex.exe`/`.cmd` or helper abstraction should be enough.

### High: explicit `mode: null` lacks automated Level 1 coverage

- Requirement: the spec's valid-value table says `mode: null` behaves like an absent key and resolves to `Append`.
- Coverage present: absent `mode` has Level 1 unit coverage, and a manual dry-run probe with `mode: null` resolved to append locally.
- Gap: no automated unit or CLI test pins explicit `mode: null`. This is a user-authored frontmatter value with documented behavior, and it depends on the Darkmatter schema/nullability path plus `fm_get::<String>("mode")` read-back semantics.
- Required fix: add a Level 1 unit test beside the existing system-prompt mode tests with `---\nmode: null\n---\n\nBody.` and assert `ResolvedSystemPrompt::Ready(...).mode == SystemPromptMode::Append`.

## Requirement Verification Matrix

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Absent `mode` defaults to append | Level 1 unit | OK |
| `mode: null` defaults to append | Manual probe only; no automated test | Gap |
| `mode: append` resolves append | Level 1 unit | OK |
| `mode: replace` resolves replace in prepare | Level 1 unit | OK |
| Invalid string rejected through schema validation | Level 1 unit | OK |
| Non-string value rejected through schema validation | Level 1 unit | OK |
| Full resolve/prepare pipeline preserves discovered replace mode | Level 1 unit | OK |
| Wrapper/provider delivery uses replace path | Level 1 CLI on Unix only | Gap |
| Explicit `--replace-system-prompt` ignores frontmatter | Level 1 unit | OK |
| Non-interactive session preserves discovered replace mode | Level 1 unit | OK |
| Empty body with `mode: replace` disables prompt | Level 1 unit | OK |
| Document `$schema` conflict falls back to append | Level 1 unit | OK |
| Prompt report/dry-run shows effective mode | Level 1 CLI on Unix only | Gap |
| System prompt docs updated | Manual review | OK |
| Frontmatter property catalog updated | Manual review | OK |

## Verification Run

- `cargo nextest run --color never -p claudine-cli -E 'test(codex_dry_run_discovered_replace)'` — passed, 2 tests.
- `cargo nextest run --color never -p claudine -E 'test(/discovered_.*mode|explicit_replace_flag_ignores_frontmatter_mode|non_interactive_session_preserves_discovered_replace_mode/)'` — passed, 10 tests.
- Manual CLI probe with a temporary discovered `system-prompt.md` containing `mode: null` — resolved to append.

No Level 2 or Level 3 tests are required for this feature as implemented. The behavior is frontmatter parsing, launch-plan construction, and dry-run/report text; it does not depend on terminal emulator rendering or OS keyboard encoding.
