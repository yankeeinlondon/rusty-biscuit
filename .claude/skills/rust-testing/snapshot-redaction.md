# Snapshot Redaction

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

A snapshot must hold on every OS and in a CI archive cell, so redact
host-specific and time-varying values before snapshotting: temp directories,
`$HOME`-derived paths, timestamps, UUIDs and session IDs, process IDs, ANSI
sequences, and generated filenames. Use string helpers for text and insta
redaction selectors for structured output.

## Path redaction traps

- **Replace both the given and the canonicalized path, longest first.** macOS
  hands out `/var/folders/…` temp dirs and reports them back as
  `/private/var/folders/…`; replacing only one form, or the shorter one first,
  leaves a stray absolute prefix.
- **Recognize native and portable spellings** so Windows separators and
  verbatim-path simplification cannot leave an absolute fixture path behind.
- **Normalize the binary name in help output.** Clap derives the usage line
  from `argv[0]`, which carries `.exe` on Windows.

Reference helpers: `redact_workspace_paths`, `redact_temp_home`, and
`redact_session_id` in `claudine/cli/tests/common/wrap.rs`; `redact_base_paths`
and `normalize_help_output` in `sniff/cli/tests/l1/snapshots.rs`.

## TUI buffer snapshots

Use a fixed terminal size, disable color unless the snapshot needs style data,
and snapshot a cloned `Buffer`, not the terminal handle
([tui-testing.md](tui-testing.md)).

## Reviewing changes

Never accept a changed snapshot to go green. Confirm the diff is a real output
change rather than a leaked temp path or timestamp, and run moved snapshots
with `INSTA_UPDATE=no` ([consolidated-binaries.md](consolidated-binaries.md)).
