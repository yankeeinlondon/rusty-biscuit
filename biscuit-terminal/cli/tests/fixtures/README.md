# Test Fixtures for biscuit-terminal-cli Level-2 tests

These small files exist solely to drive `bt` real-terminal tests.

## Images

- `tiny.png` — 1x1 PNG. Smallest possible bitmap; used for "image protocol bytes
  are emitted at all" assertions where pixel-to-cell rounding doesn't matter.
- `tiny.jpg` — 1x1 JPEG. Same role as `tiny.png` but exercises the JPEG
  decode path.
- `13x13.png` — 13x13 red PNG. Sized so that `13 / cell_height_px` yields a
  fractional cell count (typically ~0.8 cells for a 16-px font). Used to
  prove the difference between `ceil` (default rounding) and `floor` (Warp
  rounding) branches in `bt image`. Both rounding strategies must produce a
  non-zero, but visibly different, row count.

## Other

- `unicode_dir/` — directory containing files named with CJK and emoji
  glyphs. Drives the `bt dir` Level-2 unicode-width test.
