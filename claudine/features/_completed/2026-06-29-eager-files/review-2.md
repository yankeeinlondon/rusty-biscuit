---
ready: true
agent: codex/default
created: 2026-06-29T20:31:00
---

# Review 2 - Eager Files

## Findings

No blocking findings.

The implementation now matches the feature spec:

- Bare SimplifiedSchema `file` lowers to lazy `format: darkmatter-file-reference` and validates syntax only.
- `file(eager)` lowers to the existing eager `format: darkmatter-file` and preserves raw JSON Schema compatibility.
- `required` remains presence-only and orthogonal to `eager`.
- `file(eager)[]` applies eagerness per array item.
- `match(...)` is no longer emitted as `x-darkmatter-match` and is treated as completion/chooser metadata only.
- The motivating Claudine compose case is covered end-to-end: eager `review` must exist, lazy `plan` may be a future output path.

One non-blocking follow-up: `darkmatter/docs/inline/schema-validation.md` still uses `spec: "file(required)"` and `spec: "file(match('*.md'); required)"` in early examples. The later "How Validation Works" section explains the new lazy default correctly, so this is not a functional gap, but those examples are input-shaped and would teach the old existence-checking mental model unless readers continue to the later section. Consider switching them to `file(eager; required)` / `file(eager; match('*.md'); required)` in a documentation cleanup.

## Verification Levels

This feature is schema-validation and completion-candidate behavior. It does not add terminal rendering, keyboard handling, paste, IME, mouse, or scroll UX requirements, so Level 1 is the appropriate minimum.

- Bare `file` accepts syntactically valid missing paths: Level 1 present.
- `file(eager)` rejects missing paths and accepts existing paths: Level 1 present.
- `required` is presence-only and orthogonal to `eager`: Level 1 present.
- Malformed file references fail for lazy and eager declarations: Level 1 present.
- `file(eager)[]` applies existence validation per item: Level 1 present.
- Raw JSON Schema `format: darkmatter-file` remains eager and `darkmatter-file-reference` is lazy: Level 1 present.
- `match(...)` is metadata only, while completion still receives patterns: Level 1 present.
- Claudine compose succeeds for lazy output + present eager input and aborts before provider launch for missing eager input: Level 1 compiled-binary coverage present.

## Verification Run

- `cargo nextest run --color=never -p darkmatter -E 'test(/file|eager|match|schema_validation/)'` passed: 501 run, 501 passed, 4448 skipped. Nextest retried one leaked-handle case and it passed on retry.
- `cargo nextest run --color=never -p claudine-cli -E 'test(/compose_lazy_plan|compose_missing_eager|schema_completion|file_match|completion_file/)'` passed: 41 run, 41 passed, 1959 skipped. Nextest retried one leaked-handle case and it passed on retry.

## Decision

Production ready.
