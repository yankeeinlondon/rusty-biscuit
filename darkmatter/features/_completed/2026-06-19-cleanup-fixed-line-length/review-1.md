---
ready: true
agent: codex
model: ""
created: 2026-06-20T13:32:13
---

# Review 1 - Cleanup Fixed Line Length

## Findings

No production-blocking findings.

The current implementation satisfies the reconciled specification's required fixes: hard line breaks are preserved, `===` / `---` setext heading boundaries are protected, separator selection uses `unicode-script`, ZWSP boundaries join without an added space, `--fixed-width` reflows from canonical unwrapped prose, and spaceless fixed-width runs are pinned as atomic overflow tokens.

## Notes

- Documentation for the default `md clean` behavior and the `--ignore-incidental-newlines` opt-out is present in `darkmatter/cli/README.md`, `darkmatter/docs/cli/clean.md`, and the compose-pipeline docs.
- The appropriate verification level for every user-observable requirement here is Level 1. This feature is deterministic Markdown transformation plus clap argument handling; it does not depend on terminal emulator rendering, terminal input encoding, keyboard behavior, mouse/paste/IME handling, or SGR/glyph rendering.

## Verification Level Matrix

| Requirement | Appropriate level | Current strongest level | Status |
|---|---:|---:|---|
| Default `md clean` strips incidental single newlines | L1 | L1 CLI + library | Adequate |
| Preserve blank lines, code, tables, HTML, blockquotes, list markers, directives | L1 | L1 library | Adequate |
| Hard line breaks and setext headings preserved | L1 | L1 library | Adequate |
| Unicode-script separator selection and ZWSP behavior | L1 | L1 library | Adequate |
| `--fixed-width` wraps by display columns and preserves protected blocks | L1 | L1 CLI + library | Adequate |
| Reflow strips source wrapping before applying fixed width | L1 | L1 compose + CLI/library path | Adequate |
| `--ignore-incidental-newlines` preserves source single newlines | L1 | L1 CLI + compose | Adequate |
| CLI conflict between `--fixed-width` and `--ignore-incidental-newlines` | L1 | L1 clap/CLI | Adequate |
| Spaceless fixed-width runs stay atomic and may overflow | L1 | L1 library | Adequate |

## Test Run

- `cargo nextest run --color=never -p darkmatter cleanup -E 'test(/strip_incidental|reflow_to_width|cleanup_to_fixed_width|cleanup_with_fixed_width|compose_cleanup_fixed_width|compose_cleanup_strips|compose_cleanup_can_preserve/)'` - passed, 45 tests.
- `cargo nextest run --color=never -p darkmatter-cli clean` - passed, 19 tests.

## Recommendation

Ready for production for the reviewed scope.
