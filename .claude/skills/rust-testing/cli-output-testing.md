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
