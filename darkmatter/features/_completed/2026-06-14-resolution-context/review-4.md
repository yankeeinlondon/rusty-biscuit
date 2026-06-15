---
ready: true
agent: codex
model: ""
---

# Review: Resolution Context & Token Resolution, Iteration 4

## Findings

No blocking findings.

The Iteration 3 blocker in Claudine hook `doc.*` lookup has been addressed: `doc.<path>` now traverses the same event object returned by bare `doc`, and tests cover both positive traversal (`doc.git.branch`) and the important non-leakage case (`doc.env.*` does not fall through to process environment).

## Test Rigor

- Frontmatter read-side functions and the motivating optional `spec: file` ternary: **Level 1** CLI/in-process coverage is appropriate. These requirements are expression evaluation, file reference validation, and compose output behavior; no real terminal rendering or OS keyboard encoder is involved.
- `$()` token resolution, no-command diagnostics, PATH/property precedence, and branch approval discovery: **Level 1** parser/execution coverage is appropriate. The suite exercises classification, selected branch execution, safe function exclusion from approval, command branch preapproval, and the `{{ }}` suggestion path.
- Remote URL behavior split between body-side remote reads and local-only frontmatter surfaces: **Level 1** compose coverage is appropriate. Tests verify successful body reads, loud frontmatter failures, no frontmatter fetches, and the `$()` ternary condition failure path.
- `doc.*` namespace: **Level 1** coverage is appropriate and now checks the reserved namespace invariant across darkmatter frontmatter/effective-state/public-condition paths and Claudine loop/hook adapters.
- Reference-graph `when=` and public `evaluate_condition_against`: **Level 1** coverage is appropriate. Tests verify read-side functions resolve against the intended document/work directory.
- Claudine loop/hook read-side base-dir behavior: **Level 1** coverage is appropriate. Tests verify loop conditions and hook conditions root `file_exists(...)` at the prompt/hook base directory.

No Level 2 or Level 3 tests are required for this feature because none of the user-observable requirements depend on terminal emulator rendering, terminal input encoding, or OS keyboard injection.

## Verification

Focused checks run:

- `cargo test --color=never -p claudine --lib doc_namespace`
- `cargo test --color=never -p claudine --lib read_side_functions_resolve_against_base_dir`
- `cargo test --color=never -p darkmatter --lib remote_transclusion_tests`
- `cargo test --color=never -p darkmatter --lib frontmatter_shell_expansion`
- `cargo test --color=never -p darkmatter-cli --test cli motivating_spec_ternary`
- `cargo test --color=never -p darkmatter --lib when_condition_uses_read_side_functions_against_document_dir`
- `cargo test --color=never -p darkmatter --lib shortcut_read_side_functions_resolve_against_work_dir`
- `cargo test --color=never -p claudine --lib condition_lookup_read_side_function_resolves_against_base_dir`
- `cargo test --color=never -p claudine --lib until_file_exists_resolves_against_prompt_parent`

All passed.

## Notes

The live prompt migration also appears complete: the known prompt/documentation usages now refer to `doc.doc`. The only remaining bare `{{doc}}` matches are in the dated completed-feature design doc, which the spec explicitly allowed to leave as a historical artifact.
