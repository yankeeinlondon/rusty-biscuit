# `windows_captured_stdout` (F5, R4): where its one test exists and runs

Manifest row: old target `windows_captured_stdout` becomes module `windows_captured_stdout` in
`level2` (`required-features = ["terminal-tests"]`), declared `#[cfg(windows)] mod windows_captured_stdout;`,
with the inner `#![cfg(windows)]` kept (`keep_inner_cfg: true`, R17). Identity: `captured_stdout_receives_only_value_no_tui_bytes` becomes `windows_captured_stdout::captured_stdout_receives_only_value_no_tui_bytes`
(basis: source-scan).

## Platform-absent on macOS and Linux

| Capture | Host | Feature set | `captured_stdout_…` listed | `level3_*` tests listed |
|---|---|---|---:|---:|
| capture-before | darwin | `none` | 0 | 0 |
| capture-before | darwin | `terminal-tests` | 0 | 4 |
| capture-after | darwin | `none` | 0 | 0 |
| capture-after | darwin | `terminal-tests` | 0 | 4 |
| capture-linux-before | linux | `none` | 0 | 0 |
| capture-linux-before | linux | `terminal-tests` | 0 | 0 |
| capture-linux-after | linux | `none` | 0 | 0 |
| capture-linux-after | linux | `terminal-tests` | 0 | 0 |

Zero everywhere on macOS and Linux, before and after, as before the move (the file is
`#![cfg(windows)]`). The four `level3_chord_select` tests are listed only on darwin under
`terminal-tests` (`#[cfg(target_os = "macos")]`), unchanged by the move.

## Compiled and run on Windows

`cross-check-windows.txt` (archive mode, CI feature union `terminal-tests`, L1 filter):

```
        PASS [   0.085s] (133/393) biscuit-tui-cli::level2 windows_captured_stdout::captured_stdout_receives_only_value_no_tui_bytes
     Summary [   2.206s] 393 tests run: 393 passed, 7 skipped
```

The test compiles into the `level2` binary with `terminal-tests`, and the L1 filter selects it,
because its path carries no tier marker (spec criterion 1 for this hazard).
