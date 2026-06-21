---
ready: true
agent: codex/default
created: 2026-06-20T16:45:48
---

# Review: System Prompt Mode

## Findings

No blocking findings.

The prior review's two remaining gaps are addressed:

- The discovered `mode: replace` wrapper behavior is now covered by platform-neutral Level 1 CLI dry-run tests. The tests verify that Codex dry-run uses the replace delivery key (`model_instructions_file=`) rather than the append key (`developer_instructions=`), and that the user-visible dry-run report includes `mode: replace`.
- The documented `mode: null` behavior now has Level 1 unit coverage and resolves to `SystemPromptMode::Append`.

## Requirement Verification Matrix

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Absent `mode` defaults to append | Level 1 unit | OK |
| `mode: null` defaults to append | Level 1 unit | OK |
| `mode: append` resolves append | Level 1 unit | OK |
| `mode: replace` resolves replace in prepare | Level 1 unit | OK |
| Invalid string rejected through schema validation | Level 1 unit | OK |
| Non-string value rejected through schema validation | Level 1 unit | OK |
| Full resolve/prepare pipeline preserves discovered replace mode | Level 1 unit | OK |
| Wrapper/provider delivery uses replace path | Level 1 CLI dry-run, platform-neutral | OK |
| Explicit `--replace-system-prompt` ignores frontmatter | Level 1 unit | OK |
| Non-interactive session preserves discovered replace mode | Level 1 unit | OK |
| Empty body with `mode: replace` disables prompt | Level 1 unit | OK |
| Document `$schema` conflict falls back to append | Level 1 unit | OK |
| Prompt report/dry-run shows effective mode | Level 1 CLI dry-run, platform-neutral | OK |
| System prompt docs updated | Manual review | OK |
| Frontmatter property catalog updated | Manual review | OK |

## Verification Run

- `cargo nextest run --color never -p claudine -E 'test(/discovered_.*mode|explicit_replace_flag_ignores_frontmatter_mode|non_interactive_session_preserves_discovered_replace_mode/)'` — passed, 11 tests.
- `cargo nextest run --color never -p claudine-cli -E 'test(codex_dry_run_discovered_replace)'` — passed, 2 tests.

No Level 2 or Level 3 tests are required for this feature as implemented. The behavior is frontmatter parsing, composition-time schema validation, launch-plan construction, and plain dry-run/report text. It does not depend on terminal emulator rendering or OS keyboard encoding.

## Readiness

Production ready: **yes**.
