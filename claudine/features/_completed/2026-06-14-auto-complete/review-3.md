---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-26T11:42:07"
---

# Review 3 — Auto Complete

Not ready for production. This iteration closes the specific review-2 gap for `compose` operation-file autocomplete: there are now Level 2 real-terminal tests for the single-match confirmation dialog and multi-match chooser, plus Level 3 OS-keyboard tests for accepting/canceling the confirmation and navigating/submitting the multi-match chooser.

One user-visible spec surface is still under-verified at the required level: `sequence` operation-file autocomplete, especially YAML sequence candidate detail rendering.

## Findings

### High: sequence/YAML operation-file autocomplete detail has only Level 1 verification

Requirement: operation-file autocomplete applies to `claudine compose|inline-compose|sequence <partial>`, and YAML sequence candidates must be accepted when `.yaml`/`.yml` files define a top-level `sequence` key. For those YAML sequence files, the badge/name/description/schema block must be populated from top-level `name`, `description`, and `$schema` exactly like Markdown frontmatter, with the `Sequence` badge and the same fallbacks.

Implementation: the operation-file path has a mode-specific branch for YAML sequence details in [operation_file.rs](../../cli/src/completion/operation_file.rs:185), and the YAML extractor has independent logic in [file_detail.rs](../../lib/src/composition/file_detail.rs:86). That is not just a different input fixture; it is separate parsing and detail-population code from the Markdown compose path.

Coverage present at the time: Level 1 unit tests verified `gather_candidates(..., ComposeMode::Sequence)` included YAML sequence files and `extract_yaml_sequence_detail` read YAML top-level keys, while the terminal tests staged only Markdown `compose` prompts. The later test-tier consolidation moved the complete operation-file and YAML interaction matrix into [level2_auto_complete_operation_file.rs](../../cli/tests/level2_auto_complete_operation_file.rs) and retained one focused OS-input smoke test in [level3_auto_complete_chooser.rs](../../cli/tests/level3_auto_complete_chooser.rs).

Why this is a gap: the spec requires the YAML sequence detail block to render in the same real terminal UI surfaces as Markdown details. Level 1 parsing assertions cannot catch real-terminal rendering regressions in the `Sequence` badge, top-level YAML field display, schema list rendering, or the YAML candidate path flowing through the operation-file chooser/confirmation.

Fix direction: add at least Level 2 operation-file tests for `claudine sequence <partial>` where the candidate is a YAML sequence file with top-level `name`, `description`, and `$schema`. Cover both single-match confirmation and multi-match chooser detail. If the user presses keys in those presentations, keep the existing Level 3 pattern and add a small `sequence` YAML accept/navigation case there as well.

## Verification Matrix

- Shared bounded walker, query-filtered cap counting, mode contract filtering, no-match/over-cap/non-TTY errors: Level 1 coverage present.
- Dynamic completion contract, no config override, bare `file`/`file[]` fallback, comma continuation: Level 1 subprocess/unit coverage present.
- Missing `file` and `file[]` frontmatter property chooser behavior: Level 2 and Level 3 coverage present for single-select, multi-select, layout, and OS keyboard input.
- Operation-file `compose` single-match and multi-match presentations: Level 2 and Level 3 coverage present.
- Operation-file `sequence` YAML candidate presentation/detail: strongest coverage is Level 1; required Level 2 real-terminal coverage is missing.

## Notes

I attempted a focused `cargo nextest run --color=never -p claudine-cli operation_file`, but the host was still compiling a cold dependency graph after roughly 60 seconds, so I interrupted it with exit code 130 per the non-interactive session limits. No test result is available from that run.
