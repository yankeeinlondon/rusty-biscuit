---
ready: false
agent: "codex/default"
created: "2026-06-29T04:33:46"
implemented: true
---

# Review 6

## Findings

### High - `md compose` still lacks Level 2 coverage for the `$schema` focused-excerpt contract

The spec's reference failure requires the invalid-file-reference report to render identically in `md compose` and `claudine compose`, including the focused excerpt shape (`$schema` / `spec` / `iteration`) and excluding unrelated frontmatter keys. The Claudine CLI now has Level 2 coverage for that path, but the Darkmatter CLI Level 2 fixture explicitly avoids `$schema` and only asserts `spec:` plus a bare `iteration` occurrence (`darkmatter/cli/tests/level2_errors.rs`:340-417). A bare `iteration` can come from the scope sentence, so it does not prove the receiving key is present as a focused YAML excerpt line, and the test does not assert `$schema:` or absence of `agent:`.

The full `$schema` parent / involved-keys / unrelated-key exclusion contract is currently covered by an in-process renderer unit test (`darkmatter/lib/src/markdown/errors/blocks.rs`:682-731). That is useful Level 1 coverage, but it does not exercise the `md compose` binary path or the real terminal renderer. Under the review rubric, this user-visible terminal output requirement needs Level 2 verification for both binaries because the acceptance criterion says the report is identical in `md compose` and `claudine compose`.

Add a Darkmatter Level 2 case mirroring `level2_invalid_file_reference_excerpt_includes_schema_parent_in_tmux`: drive `md compose` against a prompt that declares `$schema:` and fails through the top-level `iteration` interpolation, then assert `invalid file path`, `$schema:`, `spec:`, `iteration:`, no `agent:`, OSC8 link/styling, and the prompt filename in the captured pane.

Verification level present: Level 1 for the full `$schema` excerpt in the renderer, and partial Level 2 for `md compose` without `$schema`.

Required verification level: Level 2 for the `md compose` `$schema` focused-excerpt path.

## Notes

The two review-5 blockers appear addressed in the staged implementation:

- `composition.shell_expansion` now projects the authored command via `ShellExpansionError::command()` and leaves command-less variants as `null`, instead of leaking the Markdown source path.
- `LifecycleMultipleLifecycleActions` and `LifecycleActionOrder` now map to `composition.lifecycle_invalid` and project `property` / `message`, with per-variant conformance coverage.

I attempted to run `cargo nextest run -p claudine --test diagnostic_detail_conformance --color never`, but stopped it at about 60 seconds while it was still compiling dependencies, per the non-interactive session rule. No completed test result is available from this review run.

Production ready: **no**.
