# CLI Output Testing

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Test more than success or failure. Spawn the binary through the area's command
builder ([spawning-binaries.md](spawning-binaries.md)), and put shared tempdir,
config, git, and ANSI helpers in `tests/common/` (wired per
[consolidated-binaries.md](consolidated-binaries.md) where `autotests = false`).

## What to cover

- **Channels.** Machine-readable or pipeable output goes to stdout; status,
  progress, warnings, and errors go to stderr. Assert both sides: a success
  path such as `completions bash` has empty stderr, and a failure path has
  empty stdout.
- **Color modes, each independently.** `NO_COLOR=1` emits no ANSI;
  `FORCE_COLOR=1` emits ANSI even without a TTY; `--plain` is the explicit
  no-style mode and should usually override forced color. Assert "no ANSI" as
  `stdout == strip_ansi(&stdout)` with `biscuit_test_harness::strip_ansi`,
  which handles the full escape-sequence shape (including `ESC ( B`) rather
  than a hand-rolled stripper.
- **Shell completions.** Assert a shell-specific marker rather than snapshot the
  script: bash `_my_cli()`, zsh `#compdef my-cli`, fish `complete -c my-cli`,
  elvish `edit:completion:arg-completer[...]`.
- **Snapshots** for grouped help, stable dry-run output, and tables or
  summaries without live values, after redaction
  ([snapshot-redaction.md](snapshot-redaction.md)).

## Host-dependent output

Pull requests run Linux and macOS only; native Windows runs in the nightly,
after merge. So before asserting on text a spawned binary prints, ask of each
expected string: *does any part of it come from the host rather than the
code?* Each item below has turned a test red on exactly one OS.

| Host-supplied part | Example of the trap | Do this |
|---|---|---|
| Program name | clap fills `bin_name` from argv[0], so a parsed `--help` or usage error reads `Usage: claudine.exe mcp` on Windows, while help rendered without argv reads `claudine` | Fix the product: set `bin_name` on the root `#[command(...)]` when two help paths would otherwise disagree. Normalize in the test (as `sniff`'s snapshots do) only when the `.exe` is correct output |
| Path separators and spelling | `\` separators, `\\?\` verbatim prefixes, drive letters in an expected path | Compare against what the surface actually emits; see the `os` skill's [windows-paths.md](../os/windows-paths.md#separators-and-spellings) |
| Line endings | a native tool such as `jq.exe` emits CRLF, and a checkout with `core.autocrlf` has CRLF fixtures | Trim or normalize at the boundary that reads them; see the `os` skill's [windows.md](../os/windows.md#environment-and-processes) |
| Wrap points | a temp path's length differs between hosts, so wrapped text breaks at different words | Assert on unwrapped content or pin the width; see the `os` skill's [linux.md](../os/linux.md#temp-path-length-changes-word-wrapping) |
| Ambient color | the macOS shell exports `COLORTERM`, so SGR depth differs between hosts | Pin the color environment on the child (`NO_COLOR`, `FORCE_COLOR`, `CLICOLOR_FORCE`), never inherit it |

**A Unix-only twin is not coverage.** If a similar assertion already exists in
a file gated by `#[cfg(unix)]` or one using `std::os::unix` (for example
`PermissionsExt`), it has never run on Windows. Its passing says nothing about
your assertion there.

When you hit a new host-supplied trap, add a row here, and add the symptom to
the `os` skill's "Red on one environment only" table.
