---
ready: false
agent: codex
model: ""
created: 2026-06-20T07:42:24
---

# Review 1 - Cleanup Fixed Line Length

## Findings

### High - Required fixed-width CJK atomic-overflow regression is missing

The reconciled spec requires a regression test proving that CJK / spaceless runs under `--fixed-width` are treated as one atomic token and may overflow the requested width instead of being split between ideographs (`darkmatter/features/2026-06-19-cleanup-fixed-line-length/spec.md:159`). The current implementation likely behaves that way because `reflow_tokens` only splits on whitespace (`darkmatter/lib/src/markdown/cleanup.rs:1032`), but the tests only pin ASCII long-word overflow and Latin display-width wrapping (`darkmatter/lib/src/markdown/cleanup.rs:2330`, `darkmatter/lib/src/markdown/cleanup.rs:2344`, `darkmatter/lib/src/markdown/cleanup.rs:2353`). There is no fixture with a wide Han/Thai/spaceless run whose display width exceeds the target.

Required verification level: Level 1 is appropriate because this is deterministic Markdown string transformation, not terminal rendering or terminal input behavior. Current strongest verification: absent. Add a Level 1 cleanup test such as a contiguous Han run with `UnicodeWidthStr::width(run) > width`, call `cleanup_to_fixed_width(run, width)`, and assert the output remains exactly one unbroken line. A CLI-flavored Level 1 test for `md clean --fixed-width` would also be useful, but the library regression is the blocker because the spec explicitly says this contract must be pinned.

## Notes

- The previous review's structural-safety findings are resolved in the current branch. There are now Level 1 tests for two-space and trailing-backslash hard breaks, `===` / `---` setext heading preservation, Unicode-script joins, ZWSP boundaries, and compose fixed-width ordering.
- The `darkmatter::prelude` addition is unrelated to this cleanup feature. I am not marking it as a production blocker, but it should ideally be split out of this feature branch because it adds public API and a new test module outside the reviewed specification.

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
| Spaceless fixed-width runs stay atomic and may overflow | L1 | Missing | Gap |

No Level 2 or Level 3 tests are required for this feature as specified. The behavior under review is Markdown text transformation and CLI argument handling, not terminal emulator rendering, terminal input encoding, keyboard behavior, mouse/paste/IME handling, or SGR/glyph rendering.

## Test Run

- `cargo nextest run --color=never -p darkmatter cleanup -E 'test(/strip_incidental|reflow_to_width|cleanup_to_fixed_width|cleanup_with_fixed_width|compose_cleanup_fixed_width|compose_cleanup_strips|compose_cleanup_can_preserve/)'` - passed, 43 tests.
- `cargo nextest run --color=never -p darkmatter-cli clean` - passed, 19 tests.

## Recommendation

Not ready for production until the required Level 1 CJK/spaceless fixed-width atomic-overflow regression is added. The implementation otherwise appears aligned with the reconciled spec, and the appropriate verification level for the implemented behaviors is present.
