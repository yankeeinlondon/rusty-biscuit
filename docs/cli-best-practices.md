# CLI Best Practices

This document is derived from surveying five real CLIs in this monorepo and
distilling what they do well, where they diverge, and what a new CLI should
adopt. The CLIs surveyed:

| Area | Crate | Binary | Module split | Color lib | Error reporting |
|------|-------|--------|--------------|-----------|-----------------|
| `darkmatter/cli` | `darkmatter-cli` | `md` | args/commands/output/main | biscuit-terminal `Prose` | `color_eyre` + `Prose` styled |
| `sniff/cli` | `sniff-detection-cli` | `sniff` | args/commands/output/main | biscuit-terminal `Prose` | `Box<dyn Error>`, `"Error: {e}"` |
| `biscuit-terminal/cli` | `biscuit-terminal-cli` | `bt` | args/commands/output/main | biscuit-terminal `Prose` | `color_eyre` default (backtrace) |
| `playa/cli` | `playa-cli` | `playa` | monolithic `main.rs` (~1.8k loc) | biscuit-terminal `Prose` | `Prose` styled, no eyre |
| `biscuit-speaks/cli` | `biscuit-speaks-cli` | `so-you-say` | monolithic `main.rs` (~2.1k loc) | **owo-colors** | owo-colors styled, no eyre |

The companion `.claude/skills/cli/cli-best-practices.md` is the authoritative
*standards* reference. This document is the *survey* — it records the
ground-truth state and the gaps between the standard and the implementations.

---

## Section 1 — Ideas to Include in the Best Practices

A consolidated checklist of practices worth codifying, drawn from the strongest
patterns observed across the five CLIs.

### Structure

1. **Strict lib/cli split.** No business logic in the CLI crate — it parses,
   maps to library types, and formats output. `md`, `sniff`, and `bt` follow
   this; `playa` and `so-you-say` collapse everything into one large `main.rs`.
2. **Modular CLI layout.** Split the CLI crate into `args.rs` (clap structs,
   completers, help strings), `commands.rs` / `commands/` (per-subcommand glue),
   `output.rs` (serialization, tables, styling), and a thin `main.rs` (setup,
   parse, route). Use a `commands/` directory once subcommand count grows
   (`md` does this for its `schema` group).
3. **clap Derive API with subcommands.** All five use Derive; standardize on it.
   Model mutually-exclusive output choices as a single enum value rather than
   a cluster of boolean flags.

### Output & formatting

4. **`--output <target>` is the one output selector.** Its values are a formatted
   family and a data family; `--output json` makes stdout valid JSON only, and
   `--output plain` strips all ANSI. See 2.1.
5. **One output-format mechanism per CLI.** Never mix `--json`, `--yaml`, `--html`,
   `--md` ad hoc per command.
6. **Terminal-optimized default output via biscuit-terminal `Prose`.** Use
   `Prose`, `Table`, `TwoColumns`, lists, and block quotes — never raw escape
   codes, and not an alternate color crate.
7. **STDOUT holds the deliverable.** In a formatted output, diagnostics, progress,
   warnings, and verbose chatter go to stderr. In a data output, stdout is one
   value and stderr stays empty, with timings and warnings as fields in the value
   (see 2.1).

### Color

8. **Respect `NO_COLOR`; honor `FORCE_COLOR`/`CLICOLOR_FORCE`.** `bt` is the
   reference: a `detect()` helper checks `NO_COLOR` and a forced-color helper
   checks `FORCE_COLOR`/`CLICOLOR_FORCE`. Resolve color once, at a single
   boundary, and thread the decision through.

### Verbosity & diagnostics

9. **`--verbose`/`-v` (stackable) for richer human output**, distinct from
   debug logging. `--quiet` for data-only, `--silent` for nothing-to-stdout.
   `bt` implements the quiet/silent pair (`--silent` implies `--quiet`).
10. **`--debug <level>` plus `RUST_LOG`, wired to a `tracing` subscriber.**
    `md` is the model: string levels (`trace`/`debug`/`info`/...), `RUST_LOG`
    takes precedence, diagnostics emit on stderr. Verbose ≠ debug; never let
    `-vv` mean "turn on tracing".
11. **Don't shadow standard flag names.** `playa` uses `--quiet`/`--loud` for
    *volume*, colliding with the conventional verbosity meaning — avoid this.

### Errors & exit codes

12. **`color_eyre` + styled `Error:` prefix, no raw backtraces by default.**
    `md` is the gold standard: `color_eyre::install()`, a bold red `Error:`
    rendered through `Prose`, and a deduplicated cause chain. Backtraces are
    opt-in via the usual env vars, never the default.
13. **Conventional exit codes.** `0` success, `1` the command ran and failed, `2`
    usage error and nothing else, `130` for SIGINT during interactive prompts. A
    check that finds a problem (such as a hash mismatch) exits `1`.

### Completions, signals, help

14. **Dynamic shell completions with value hints.** Use `clap_complete`'s
    `CompleteEnv`, and supply completers for files, enum values, and
    `biscuit-file` `FileReference` (`@`/`!`) paths. Offer both the env-driven
    `COMPLETE=<shell>` activation and an explicit `--completions <shell>`
    discovery path (`sniff`/`bt` expose the flag).
15. **Reset `SIGPIPE` to default on Unix.** `md` does this so piping into
    `head` doesn't panic; every data-emitting CLI should.
16. **Handle `SIGINT`/`SIGTERM` for in-flight work.** Clean up temp files,
    flush, release locks. Long synchronous work (e.g. `playa` audio playback)
    should be cancellable, not just killable.
17. **Depth-adapted `--help`.** Help shows only details relevant to the current
    subcommand level; a `help` subcommand is fine but should stay hidden from
    help output.

---

## Section 2 — Inconsistencies Found, and How to Standardize Them

Each item lists what diverges across the five CLIs and a concrete path to
convergence.

### 2.1 Output-format flags are inconsistent and overlapping

**Observed:**
- `md` — one `--output <target>` on every command, built on the shared contract
  below (the model).
- `sniff` — global `--json` and `--plain`.
- `bt` — global `--json` but it only affects metadata commands; rendering
  commands instead use per-command `--html` / `--md` / `--md-plus`.
- `playa`, `so-you-say` — no machine-readable output format at all; voice/effect
  lists are emitted as rendered Markdown tables only.

**How to standardize:** every CLI moves to one contract, implemented by the shared
libraries so no CLI hand-writes a renderer per format
([`CommandOutput`](../renderable/docs/command-output.md),
[`write_output`](../biscuit-terminal/docs/output.md)).

1. **Two sources, two families.** A library exposes `as_prose()`, a render-tree
   document, and `as_data()`, a serializable value. Every output comes from one of
   them, and this split is a design principle to preserve. A command accepts whole
   families (both, or the data family only) plus any command-specific extras its
   `--help` lists, and never part of a family.

   | Family | `--output` values | Source |
   |--------|-------------------|--------|
   | formatted | `terminal`, `plain`, `markdown`, `markdown-plus`, `html` (alias `browser`) | `as_prose()` |
   | data | `json`, `json5`, `yaml`, `toml` | `as_data()` |

2. **`--output` is the one selector.** Retire `--json`, `--plain`, `--format`,
   and per-format flags. The enum prevents `--json --yaml` nonsense without manual
   `conflicts_with` wiring. `playa` and `so-you-say` gain the data family for their
   list commands (`list-effects`, `list-providers`, `list-voices`) this way, so
   they become scriptable.
3. **Each command declares its default**, and `--help` shows it. There is no
   `auto`, and an explicit `--output` always wins.
   - A command whose deliverable is a Markdown document defaults to `markdown` and
     always writes Markdown, whatever stdout is, so `tool doc.md > out.md` is
     predictable.
   - Every other command defaults to `terminal`, which falls back to the same bytes
     as `plain` when stdout is not a terminal or the terminal lacks color.
4. **Streams.** Stdout holds the deliverable.
   - A data output writes exactly one value to stdout, on success and on failure,
     and leaves stderr empty. Warnings, timings, and problems are fields in the
     value, or are not said. A script therefore never has to separate two streams.
   - In a formatted output, anything that is not the deliverable goes to stderr, as
     `terminal` with the `plain` fallback, judged by stderr's own TTY and color
     detection. When a report is what the command was run to produce, it is the
     deliverable and goes to stdout.
   - Stderr has its own width: its terminal's columns, or unlimited when it is not a
     terminal. `--width` and `OUTPUT_COLUMNS` never apply to it, because the two
     streams can go to different places.
5. **`--width <N | N% | auto | source | unlimited>` is the CLI-wide width option.**
   Every command has it, and it has no effect on a command that prints no prose, so
   one rule covers every command. `auto` is `OUTPUT_COLUMNS`, else the terminal's
   columns, else unlimited; it is never a guessed 80. A command whose deliverable is
   a Markdown document defaults to `source`, which keeps authored line breaks.
   `--max-width` is **not** a general pattern and is not added to a command.
6. **Typed data outputs, enforced by tests.** Each command's data output has a schema
   file. A conformance test per command runs the real binary and validates stdout
   strictly; a guard test enumerates the clap definitions and fails on a command
   with no schema or conformance test; a parity test proves `yaml`, `json5`, and
   `toml` carry the same data as `json`. A new command cannot ship an untyped data
   output. See `darkmatter/cli/schemas/output/`.
7. **Exit statuses mean the same in every output mode.** A script decides what
   happened from the status, never from stderr.

   | Status | Meaning | Data output |
   |--------|---------|-------------|
   | `0` | the command ran and succeeded | one value on stdout |
   | `1` | the command ran and failed | one value on stdout, reporting the failure |
   | `2` | a usage error, and nothing else: no command ran | nothing on stdout; the parser's message on stderr |

8. **A CLI with subcommands never has a separate root command.** A root alias is fine
   for ergonomics, but it must be an exact alias of one named subcommand, so that
   every flag and output rule has one meaning.

### 2.2 Two CLIs are monolithic

**Observed:** `playa` (~1.8k-line `main.rs`) and `so-you-say` (~2.1k-line
`main.rs`) put args, command logic, and output formatting in one file. The
other three follow the args/commands/output/main split.

**How to standardize:** Refactor `playa` and `so-you-say` to the four-module
layout. Mechanical and low-risk: move clap structs to `args.rs`, per-subcommand
handlers to `commands.rs` (or `commands/`), and table/list rendering to
`output.rs`. Verify business logic actually lives in `playa/lib` and
`biscuit-speaks/lib`, not in the CLI.

### 2.3 `so-you-say` uses a different color library

**Observed:** `so-you-say` styles output with **owo-colors**; the other four use
biscuit-terminal `Prose`. This is the single largest stylistic outlier — it
means inconsistent markup, its own color-detection, and no `Prose` components.

**How to standardize:** Migrate `so-you-say` to `Prose`/`Table` from
biscuit-terminal, matching the other CLIs. This also gives it the shared
color-mode resolution (2.4) and `plain` output (2.1) for free. Drop the
`owo-colors` dependency afterward.

### 2.4 `NO_COLOR` / `FORCE_COLOR` handling is uneven

**Observed:**
- `bt` — explicit `NO_COLOR` + `FORCE_COLOR`/`CLICOLOR_FORCE` (best).
- `sniff` — sets `NO_COLOR` from `--plain`, no explicit `FORCE_COLOR`.
- `md` — relies on darkmatter's implicit `detect_color_mode()`.
- `playa`, `so-you-say` — no env-var color control at all.

**How to standardize:** Extract a shared color-resolution helper (a small
crate-level or biscuit-terminal function) that resolves `--plain` → `NO_COLOR` →
`FORCE_COLOR`/`CLICOLOR_FORCE` → TTY detection into a single `ColorMode`,
resolved once at startup. Every CLI calls it; no CLI re-implements the
precedence.

### 2.5 Verbosity / debug story is missing in two CLIs and inconsistent across the rest

**Observed:**
- `md` — `--verbose` (count) + `--debug <level>` (string levels) + `RUST_LOG` +
  `tracing` (the reference design).
- `sniff` — `--verbose` (count, user output) + `--debug` (count, **raw
  tracing**) + `RUST_LOG`. The `--debug`-as-count semantics differ from `md`'s
  `--debug <level>`.
- `bt` — `--verbose` (count) + `--quiet`/`--silent` + per-command `--debug` +
  conditional `RUST_LOG`.
- `playa` — none (and `--quiet`/`--loud` are volume flags, a name collision).
- `so-you-say` — none; only an ad-hoc `DEBUG` env var, no `tracing`.

**How to standardize:**
1. Adopt `md`'s `--debug <level>` string form everywhere; deprecate `sniff`'s
   `--debug` counting form for consistency.
2. Add `--verbose`/`--quiet`/`--silent` + `--debug <level>` + `RUST_LOG` +
   a `tracing` subscriber to `playa` and `so-you-say`; remove `so-you-say`'s
   bespoke `DEBUG` env var.
3. Rename `playa`'s volume `--quiet`/`--loud` to non-colliding names (e.g.
   `--volume <level>` or `--soft`/`--loud`, matching `so-you-say`'s `--soft`).
4. Keep the verbose-vs-debug split everyone should respect: `--verbose` is
   styled user output, `--debug`/`RUST_LOG` is raw tracing on stderr.

### 2.6 Error reporting uses three different strategies

**Observed:**
- `md` — `color_eyre` + bold red `Error:` via `Prose`, deduped cause chain
  (best; never shows backtraces by default).
- `playa`, `so-you-say` — `Prose`/owo styled errors, but **no** `color_eyre`.
- `sniff` — `Box<dyn Error>` with a plain `"Error: {e}"`, no styling, no eyre.
- `bt` — `color_eyre` **default** formatter, which prints a backtrace —
  violating "no raw backtraces by default".

**How to standardize:** Adopt `md`'s pattern as the house standard:
`color_eyre::install()`, catch at `main`, render a styled `Error:` prefix
through `Prose`, dedupe the cause chain, and suppress backtraces unless the
user opts in. `bt` should add the styled rendering layer instead of leaning on
the default; `sniff` should add `color_eyre` + styling; `playa`/`so-you-say`
should adopt `color_eyre` underneath their existing styled output.

### 2.7 Exit-code discipline varies

**Observed:**
- `md` — `0`/`1`/`2` plus a domain `2` (hash mismatch).
- `playa` — `0`/`1`/`2`/`130` (most complete).
- `so-you-say` — `0`/`1`/`130`.
- `sniff` — mostly `0`/`1`, some inline `std::process::exit` scattered across
  subcommands (no unified layer).
- `bt` — `0`/`1` only; no distinct usage-error code, no `130`.

**How to standardize:** Define the convention once (`0` success, `1` general,
`2` usage, `130` SIGINT) and apply it uniformly. Give `bt` a `2` usage path and
`130` handling. Centralize `sniff`'s scattered `process::exit` calls behind a
single error-to-exit-code mapping in `main`.

### 2.8 Completion activation paths and value hints differ

**Observed:**
- All five use `clap_complete` dynamic `CompleteEnv`.
- `sniff` and `bt` also expose an explicit `--completions <shell>` flag for
  discovery; `md`, `playa`, `so-you-say` only support the env-driven path.
- `md`, `sniff`, `bt`, `playa` provide rich value hints (files, enums,
  package names, audio files); **`so-you-say` provides no value hints**.

**How to standardize:**
1. Expose both activation paths everywhere: `COMPLETE=<shell>` and an explicit
   `--completions <shell>` (with `--completions --help` showing per-shell
   install snippets).
2. Add value hints to `so-you-say` (providers, voices, languages) and adopt
   `biscuit-file` `FileReference` completers (`@`/`!`) where the CLI accepts
   file paths.

### 2.9 Signal handling is minimal and inconsistent

**Observed:**
- `md` — resets `SIGPIPE` to default (good for pipes); no SIGINT/SIGTERM.
- `playa`, `so-you-say` — exit `130` on `inquire` interrupt only; the actual
  work (audio playback) is synchronous and not cancellable.
- `sniff`, `bt` — no signal handling at all.

**How to standardize:**
1. Add the `SIGPIPE`-reset (`md`'s pattern) to every data-emitting CLI so
   piping into `head`/`less` never panics.
2. Add `SIGINT`/`SIGTERM` handlers (`tokio::signal` where async is present,
   `ctrlc` otherwise) for any CLI that holds resources or runs long operations;
   make `playa` playback cancellable rather than relying on process kill.

---

## Quick Convergence Priority

If standardizing incrementally, this is the suggested order (highest
value-to-effort first):

1. One `--output` on all five CLIs, through the shared contract (2.1).
2. `color_eyre` + styled `Error:` everywhere; kill `bt`'s default backtrace (2.6).
3. Shared color-mode resolver respecting `NO_COLOR`/`FORCE_COLOR` (2.4).
4. Migrate `so-you-say` off owo-colors to `Prose` (2.3).
5. Add verbosity/debug/tracing to `playa` and `so-you-say`; fix the `playa`
   flag-name collision (2.5).
6. Modularize `playa` and `so-you-say` (2.2).
7. Uniform exit codes and completion parity (2.7, 2.8).
8. `SIGPIPE` reset + cancellable long operations (2.9).
