---
$schema:
    ready: "boolean(required) -> is the feature production ready?"
ready: false
implemented: true
agent: codex/default
created: 2026-07-09T20:25:32
---

# Review 2: DMLS Interpolation Assistance

## Findings

### High: the feature fails the canonical lint gate

Acceptance criterion 8 requires the DMLS lint suite to pass. `just lint` fails
in the new cursor-to-expression helper because `expression_at` declares a
needless explicit lifetime:

- `darkmatter/dmls/src/overlay/expressions.rs:152`

The workspace treats Clippy warnings as errors, so DMLS does not currently pass
its production gate. Elide the lifetime as Clippy suggests and rerun the full
area lint recipe.

### High: the canonical Level 1 area gate remains red

Acceptance criterion 8 also requires the package area's canonical Level 1 suite
to pass. `just test` still fails reproducibly on:

- `layout::page::tests::render_code_block_center_aligned_with_max_fill`
  (`darkmatter/lib/src/layout/page.rs:2679`); and
- `layout::page::tests::render_code_block_with_pad_fill`
  (`darkmatter/lib/src/layout/page.rs:2745`).

Both tests failed all four nextest attempts. The failures are outside this
feature's DMLS implementation, but the canonical area gate stops after them and
does not reach the CLI or DMLS packages. They must be fixed, or the release
candidate must establish that they are not part of its production gate, before
the feature can satisfy its stated acceptance criteria.

### Medium: the native-stdio Level 1 test is still excluded by its name

The stdio smoke test was correctly moved out of the Level 2 file tier, but its
function is named `native_binary_speaks_lsp_over_real_stdio`:

- `darkmatter/dmls/tests/stdio_subprocess.rs:67`
- `darkmatter/justfile:72`

The substring `real_` matches the canonical Level 1 exclusion
`test(/real_/)`. Consequently, `just _test dmls` reports one skipped test, and
the area's `test-real` recipe does not run DMLS tests at all. The test passes
when selected directly, so this is a routing defect rather than a runtime
failure. Rename it without a reserved tier prefix, such as
`native_binary_speaks_lsp_over_stdio`, and confirm it is included in the normal
Level 1 count.

## Prior-review closure

The earlier function-hover defect is fixed. Function metadata is now limited
to the identifier span; `ctx.*` and frontmatter arguments inside calls reach
their own hover paths, while parentheses and commas retain generic expression
hover. Focused provider tests cover all four cases.

The in-memory LSP and no-side-effects suites have also been moved into the
Level 1 gate. They now verify the initialize capability, catalog-backed hover
and completion response shapes, eager Markdown documentation, UTF-16 edits
after an astral character, the prose-period guard, and passive behavior without
claiming real-terminal coverage.

## Verification-level matrix

| User-facing requirement | Strongest effective verification | Assessment |
|---|---:|---|
| Catalog-backed `ctx.*` interpolation hover and shared frontmatter block | Level 1 provider + in-memory LSP | Appropriate |
| Bare versus explicitly qualified `ctx.*` classification | Level 1 provider | Appropriate |
| `ctx.*` completion metadata and UTF-16 `textEdit` | Level 1 provider + in-memory LSP | Appropriate |
| `.` capability trigger and no prose completions | Level 1 provider + in-memory LSP | Appropriate |
| Catalog-backed function completions, including all six formatters | Level 1 provider + in-memory LSP | Appropriate |
| Known function-identifier hover; generic unknown-function hover | Level 1 provider + in-memory LSP | Appropriate |
| Function argument, punctuation, and nested-call hover precedence | Level 1 provider | Appropriate |
| Passive/no-execution behavior | Level 1 in-memory LSP sentinel test | Appropriate |
| Native binary stdio lifecycle | Level 1 subprocess | Correct level, but excluded from the canonical gate by its name |
| Terminal rendering or terminal input encoding | Not applicable | No requirement needs Level 2 or Level 3 |

No specification requirement concerns terminal glyphs/SGR/layout, injected
terminal bytes, or OS keyboard events. Requiring Level 2 or Level 3 for these
LSP JSON response contracts would not exercise any additional relevant layer.

## Verification performed

- `just _test dmls`: passed 335 tests; one test skipped because
  `native_binary_speaks_lsp_over_real_stdio` matches the `real_` filter.
- Direct nextest selection of the skipped stdio test: passed 1/1.
- `just test-l2`: passed 19/19 Darkmatter and 69/69 darkmatter-cli real-terminal
  tests; DMLS correctly selected 0 tests because it has no real-terminal cases.
- `just lint`: failed on `clippy::needless_lifetimes` in
  `overlay/expressions.rs:152` after Darkmatter and darkmatter-cli passed.
- `just test`: failed on the two Darkmatter layout tests above after 271 tests
  passed; nextest canceled the remaining area run.
- Testing was executed on macOS. The feature code uses platform-neutral LSP and
  Rust APIs, but Windows and Linux were not available for execution in this
  review environment.

## Production readiness

Not ready for production. The reviewed DMLS behavior and its requirement-level
coverage are substantially corrected, but the feature fails its lint criterion,
the canonical Level 1 area gate remains red, and the native-stdio Level 1 smoke
test is still omitted from canonical execution.
