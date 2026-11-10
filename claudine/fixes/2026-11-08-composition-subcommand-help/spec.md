---
kind: fix
created: 2026-11-08
status: draft-spec
area: claudine
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    implemented: boolean -> indicates whether this spec's plan has been implemented
human_review: false
reviewed: false
implemented: false
---

# Per-command help for `compose`, `inline-compose`, and `sequence`

## The defect

`claudine compose -h`, `claudine inline-compose --help`, and
`claudine sequence -h` print the **top-level** grouped help screen. That
screen lists the commands and five global options. None of the switches the
three commands accept appears anywhere in `claudine`'s help:

- the provider switches (`--provider`, `--claude`, …, `--exclude`);
- `--set`;
- `--dry-run` and `--dry-run-shell`;
- `--max-iterations` and `--on-rate-limit`;
- the wrapper-grade switches they share.

Every other subcommand shows its own help: `claudine claude -h`,
`claudine mcp -h`, `claudine actions --help`. `claudine/docs/cli/index.md`
tells users that `claudine compose -h` gives "help on the **compose**
command".

```text
$ claudine compose -h
Claudine
cross-agent abstraction and meta-agent harness

Usage: claudine <command> [options]
Shared Resources:
  skills   …
```

Users currently have to read the docs or the source
(`commands/compose/mod.rs::SharedComposeArgs`) to find a switch.

## Why it happens

The behavior is deliberate, from argv normalization rule 4
(`cli/src/argv/rule4_help_hoist.rs`):

- The three composition subcommands collect their file reference, setters,
  and agent arguments with a greedy positional. A trailing `--help` would be
  swallowed by it.
- So rule 4 moves `-h`/`--help` to argv position 1.
- `main.rs::dispatch` then sees `cli.help` and calls
  `commands::help::run()`, which takes no arguments and always renders the
  root screen.

The hoist fixed a real bug: a misleading clap tip, and later a "second file
reference" error. The root screen it lands on is the part to correct.
`cli/tests/l1/argv_normalization.rs` currently pins the root-screen outcome:
`headline_compose_with_interleaved_flag_renders_help` asserts the
`Composition` group heading appears.

## What should happen

- `claudine compose -h` (and `--help`, `inline-compose`, `sequence`, with or
  without a file, setters, or provider switches before the help token) prints
  that command's own help. That means its usage line, its arguments, and
  every user-facing switch, with a one-line description each.
- Exit status `0`, help on stdout, nothing on stderr. This matches the
  other subcommands.
- A help token after `--` is still forwarded to the agent untouched. Rule
  4's existing `--` boundary is kept.
- `claudine --help` and bare `claudine` still print the grouped screen.

## Constraints and risks

- **Doc comments become user text.** Clap renders the `///` comments on
  `SharedComposeArgs` fields as help. Several of them are maintainer notes,
  for example the `yolo` field's history of the retired `YOLO` variable,
  and `set_records`/`caller_arguments` explain internals (they are
  `#[arg(skip)]`, so they don't render). Every rendered field needs a short
  user-facing first line, and probably an explicit `help = "…"` /
  `long_help` split, so `-h` stays compact.
- **One help surface per command.** Render clap's help for the selected
  subcommand, the way the wrappers' `print_wrapper_help` does. Don't write a
  second hand-maintained list: the grouped root screen already drifted once,
  and `commands/help.rs::grouped_help_lists_exactly_the_visible_subcommands`
  now guards it.
- **Tests to update.** In `argv_normalization.rs`, the two
  `headline_compose_*_renders_help` tests change from "grouped screen" to
  "compose screen". They keep their assertions that no clap tip and no
  "expected at most one file reference" error leaks.
  `sequence_errors_cli.rs::removed_fail_fast_is_absent_from_sequence_help`
  should then be checking a screen that actually lists switches.
- **Shell completion** reads the same clap model. Confirm that help changes
  alter no completion candidates.

## Acceptance

1. Each of these prints the compose (or inline-compose, or sequence)
   screen, exits 0, and lists `--dry-run`, `--set`, and `--provider`:
   - `claudine compose -h`
   - `claudine compose x.md --gemini name=Ken --help`
   - `claudine inline-compose --help`
   - `claudine sequence -h`
2. `claudine compose x.md -- --help` forwards `--help` to the agent, as today.
3. `claudine --help` is unchanged.
4. No rendered help line is a maintainer note: no history, no type paths,
   no references to internal modules.
5. `claudine/docs/cli/index.md` and `claudine/cli/README.md` describe the
   result accurately.
