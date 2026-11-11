# Implementation record: composition subcommand help

Status: implementation complete, ready for review. All evidence below was
produced on macOS (the author's development host); no claim is made here that
a local run proved Linux, native Windows, or WSL2. CI covers Linux and macOS on
the pull request and Windows/WSL2 in the nightly.

## What changed

- **Help is selected from the original argv** (`cli/src/argv/rule4_help_hoist.rs::select_composition_help`),
  called in `cli/src/main.rs::run` right after the completion intercept and
  before `--perf` bootstrap, normalization, the ownership partition,
  configuration, tracing, and the execution runtime. Help before the command
  renders the grouped root overview; help after it renders that command's
  compact clap help; the process exits 0 through `shutdown::exit_before_runtime`.
- **One command model.** `build_cli_command(is_wrapper)` is shared by normal
  parsing and the help renderer; the renderer builds the root (`Command::build`)
  before extracting the child so usage is parent-qualified and global options
  are inherited. The completion tree (`completion::completion_command`) is
  unchanged.
- **Complete reference.** `SharedComposeArgs` declares the missing `--kilo`,
  `--pi`, and `--antigravity` shortcuts (all ten catalog shortcuts now show);
  positional help describes the file / setters / provider-argument grammar;
  each command has a forwarding example footer; YOLO history text was
  removed; the `--dry-run` and `--dry-run-shell` warnings are in their first
  help paragraph.
- **Docs and skill** (Phase 3): `docs/cli/index.md` (planned marker replaced by
  a "Help for the composition commands" section), `cli/README.md` (one-line
  overview + link), `docs/topics/cli-pre-parsing.md` and
  `docs/topics/argv-normalization.md` (pipeline diagrams, rule 4 rewritten as
  selection-first plus a retained normalization contract, stale provider
  shortcut claim removed), `.claude/skills/claudine/SKILL.md` (pointer), and
  `.claude/skills/claudine/cli-commands.md` (argv row). The skill's `topics/`
  directory is a symlink to `claudine/docs/topics/`, so the two topic "copies"
  are the same files.

## Deviations from the specification and plan

Recorded in detail in the implementation log's `decisions_for_review`:

1. `build_cli_command` returns the model unbuilt; only the help renderer calls
   `Command::build` (building inside the factory would break the existing
   `root_help_request` fallback's `ignore_errors` propagation).
2. A root `-V`/`--version` before the command and before any help token leaves
   the argv to the ordinary parser; a help token that comes first wins.
3. Tests place `--perf` after the command. Placed before it,
   `claudine --perf compose -h` also prints compose help (exit 0, empty
   stderr), because the command recognizer skips flag-shaped root tokens. An
   earlier version of this record wrongly said it exits 2; corrected after
   review 1.
4. The help footer comes from one `composition_help_footer!` macro rather than
   three literal strings.

## Verification

| Command (macOS, from `claudine/` unless noted) | Result |
| --- | --- |
| `just test` (includes every `claudine-cli` L1 test, i.e. `just test-cli`'s set) | 11917 run, 11917 passed (150 slow), 15 skipped, 618.7 s |
| `just lint` | exit 0 |
| `just check-tier-coverage claudine` (repo root) | 0 stranded |
| `claudine --plain <cmd> --help` for all three commands (debug build) | exit 0, empty stderr |

The host's load average was about 200 during the test run (from other work on
the host); no test failed or timed out.

Covered behavior (Phase 2 tests, `cli/tests/l1/argv_normalization.rs`,
`completion_ownership.rs`, `sequence_errors_cli.rs`, and the
`main.rs::composition_help` and `rule4_help_hoist` unit modules): command
help with and without a file for `-h`/`--help` on all three commands; help
beside an unknown provider, missing provider value, malformed `--set`,
`--dry-run-shell` alone, conflicting interactive switches, and an unknown
switch before the file; root help before the command; plain mode on both
sides; `--quiet`/`--silent`/`--perf`; no provider launch, lifecycle action,
input mutation, config creation, or audio for `inline-compose` help on a real
document; `--help` after `--` forwarded to the provider; completion
(`__complete`, `COMPLETE=bash`) never printing help; all ten shortcuts in the
clap and completion models.

Confirmed unchanged: no `Cargo.toml`/`Cargo.lock`, CI workflow, or script
changes; wrapper help layout, configuration parsing, and provider argument
ownership untouched.

## Captured help output

### `claudine --plain compose --help`

```text
Compose a Markdown document and send as prompt to an agentic CLI

Usage: claudine compose [OPTIONS] <ARG>...

Arguments:
  <ARG>...  One required file reference, then optional `key=value` setters (overriding `--set`) and
            arguments for the provider

Options:
      --provider <PROVIDER>           Use a specific provider [possible values: claude, codex,
                                      gemini, goose, kimi, opencode, qwen, kilo, pi, antigravity]
  -v, --verbose...                    Increase presentation detail for human-facing output
      --claude                        Use Claude Code
      --debug <LEVEL>                 Set Claudine's diagnostic tracing level [possible values:
                                      trace, debug, info, warn, error]
      --codex                         Use Codex CLI
      --plain                         Strip ANSI escape codes from all output
      --gemini                        Use Gemini CLI
      --goose                         Use Goose
      --kimi                          Use Kimi Code
      --opencode                      Use OpenCode
      --qwen                          Use Qwen Code
      --kilo                          Use Kilo
      --pi                            Use Pi
      --antigravity                   Use Antigravity
      --exclude <PROVIDER>            Exclude providers from automatic selection (repeatable)
                                      [possible values: claude, codex, gemini, goose, kimi,
                                      opencode, qwen, kilo, pi, antigravity]
  -y, --yolo                          Enable the provider's auto-approval (YOLO) mode [env:
                                      CLAUDINE_YOLO=]
  -i, --interactive                   Run the provider session in interactive mode
      --no-interactive                Run the provider session in non-interactive mode
      --include <ENV_NAME>            Preserve this env var even when it matches sensitive-name
                                      filters
  -m, --model <MODEL>                 Override the model used by the provider
  -o, --output <FORMAT>               Set the output format (json, text, stream)
      --append-system-prompt <FILE>   Append a system prompt from a file [aliases: --asp]
      --replace-system-prompt <FILE>  Replace the provider's system prompt with contents from a file
                                      [aliases: --rsp]
  -t, --timeout <DURATION>            Wall-clock timeout (e.g. `30s`, `5m`, `2h`). Sends SIGTERM
                                      then SIGKILL. Only valid in non-interactive mode
      --step-timeout <DURATION>       Step-silence timeout (e.g. `30s`, `5m`). Kills the child when
                                      no stream event is observed for this long. Only valid in
                                      non-interactive structured-stream mode
      --stall-timeout <DURATION>      OpenCode stalled-generation backstop budget (e.g. `10m`).
                                      Aborts when OpenCode repeatedly drops generations with no
                                      progress for this long. OpenCode-scoped, structured-stream
                                      only; `0s` disables. Default `10m`
      --operation <OP>                Set the OPERATION env var for the composed session [aliases:
                                      --op]
      --sandbox                       Enable provider-specific sandboxing
      --repo                          Use only repo-scoped skills, commands, and agents via a
                                      provider overlay
      --dry-run                       Show the composed result without launching a provider;
                                      composition `$( … )` shell expansions still run
      --dry-run-shell                 Run each `shell:` task for real once during the preview;
                                      requires `--dry-run`
  -q, --quiet                         Suppress env details and info messages, but still show the
                                      system prompt when set
      --silent                        Suppress all output except the composition result
      --set <JSON>                    Override frontmatter values as JSON/JSON5 (e.g. `--set
                                      '{"key":"val"}'`)
      --mcp                           Enable Claudine-managed MCP session composition
      --use <ID>                      Activate specific MCP servers by ID or alias (comma-separated)
      --strict                        Treat unresolved or ambiguous MCP tags as hard errors
      --perf                          Emit a performance report to stderr after command completion
      --max-iterations <N>            Maximum number of loop iterations (overrides `loop.max`
                                      frontmatter)
      --on-rate-limit <POLICY>        What to do when a completed loop iteration reports a provider
                                      rate-limit signal. Overrides `loop.on_rate_limit` frontmatter
                                      [possible values: pause, abort, continue]
  -h, --help                          Print help

Example:
  claudine compose prompt.md name=Ken -- --help

Everything after `--` is forwarded to the provider unchanged, so this example asks
the provider for its own help. Provider-native options are not listed above.

```

### `claudine --plain inline-compose --help`

```text
Inline composition: use frontmatter prompt to generate and replace body

Usage: claudine inline-compose [OPTIONS] <ARG>...

Arguments:
  <ARG>...  One required file reference, then optional `key=value` setters (overriding `--set`) and
            arguments for the provider

Options:
      --provider <PROVIDER>           Use a specific provider [possible values: claude, codex,
                                      gemini, goose, kimi, opencode, qwen, kilo, pi, antigravity]
  -v, --verbose...                    Increase presentation detail for human-facing output
      --claude                        Use Claude Code
      --debug <LEVEL>                 Set Claudine's diagnostic tracing level [possible values:
                                      trace, debug, info, warn, error]
      --codex                         Use Codex CLI
      --plain                         Strip ANSI escape codes from all output
      --gemini                        Use Gemini CLI
      --goose                         Use Goose
      --kimi                          Use Kimi Code
      --opencode                      Use OpenCode
      --qwen                          Use Qwen Code
      --kilo                          Use Kilo
      --pi                            Use Pi
      --antigravity                   Use Antigravity
      --exclude <PROVIDER>            Exclude providers from automatic selection (repeatable)
                                      [possible values: claude, codex, gemini, goose, kimi,
                                      opencode, qwen, kilo, pi, antigravity]
  -y, --yolo                          Enable the provider's auto-approval (YOLO) mode [env:
                                      CLAUDINE_YOLO=]
  -i, --interactive                   Run the provider session in interactive mode
      --no-interactive                Run the provider session in non-interactive mode
      --include <ENV_NAME>            Preserve this env var even when it matches sensitive-name
                                      filters
  -m, --model <MODEL>                 Override the model used by the provider
  -o, --output <FORMAT>               Set the output format (json, text, stream)
      --append-system-prompt <FILE>   Append a system prompt from a file [aliases: --asp]
      --replace-system-prompt <FILE>  Replace the provider's system prompt with contents from a file
                                      [aliases: --rsp]
  -t, --timeout <DURATION>            Wall-clock timeout (e.g. `30s`, `5m`, `2h`). Sends SIGTERM
                                      then SIGKILL. Only valid in non-interactive mode
      --step-timeout <DURATION>       Step-silence timeout (e.g. `30s`, `5m`). Kills the child when
                                      no stream event is observed for this long. Only valid in
                                      non-interactive structured-stream mode
      --stall-timeout <DURATION>      OpenCode stalled-generation backstop budget (e.g. `10m`).
                                      Aborts when OpenCode repeatedly drops generations with no
                                      progress for this long. OpenCode-scoped, structured-stream
                                      only; `0s` disables. Default `10m`
      --operation <OP>                Set the OPERATION env var for the composed session [aliases:
                                      --op]
      --sandbox                       Enable provider-specific sandboxing
      --repo                          Use only repo-scoped skills, commands, and agents via a
                                      provider overlay
      --dry-run                       Show the composed result without launching a provider;
                                      composition `$( … )` shell expansions still run
      --dry-run-shell                 Run each `shell:` task for real once during the preview;
                                      requires `--dry-run`
  -q, --quiet                         Suppress env details and info messages, but still show the
                                      system prompt when set
      --silent                        Suppress all output except the composition result
      --set <JSON>                    Override frontmatter values as JSON/JSON5 (e.g. `--set
                                      '{"key":"val"}'`)
      --mcp                           Enable Claudine-managed MCP session composition
      --use <ID>                      Activate specific MCP servers by ID or alias (comma-separated)
      --strict                        Treat unresolved or ambiguous MCP tags as hard errors
      --perf                          Emit a performance report to stderr after command completion
      --max-iterations <N>            Maximum number of loop iterations (overrides `loop.max`
                                      frontmatter)
      --on-rate-limit <POLICY>        What to do when a completed loop iteration reports a provider
                                      rate-limit signal. Overrides `loop.on_rate_limit` frontmatter
                                      [possible values: pause, abort, continue]
  -h, --help                          Print help

Example:
  claudine inline-compose prompt.md name=Ken -- --help

Everything after `--` is forwarded to the provider unchanged, so this example asks
the provider for its own help. Provider-native options are not listed above.

```

### `claudine --plain sequence --help`

```text
Run a serial sequence of composition steps from a single document

Usage: claudine sequence [OPTIONS] <ARG>...

Arguments:
  <ARG>...  One required file reference, then optional `key=value` setters (overriding `--set`) and
            arguments for the provider

Options:
      --provider <PROVIDER>           Use a specific provider [possible values: claude, codex,
                                      gemini, goose, kimi, opencode, qwen, kilo, pi, antigravity]
  -v, --verbose...                    Increase presentation detail for human-facing output
      --claude                        Use Claude Code
      --debug <LEVEL>                 Set Claudine's diagnostic tracing level [possible values:
                                      trace, debug, info, warn, error]
      --codex                         Use Codex CLI
      --plain                         Strip ANSI escape codes from all output
      --gemini                        Use Gemini CLI
      --goose                         Use Goose
      --kimi                          Use Kimi Code
      --opencode                      Use OpenCode
      --qwen                          Use Qwen Code
      --kilo                          Use Kilo
      --pi                            Use Pi
      --antigravity                   Use Antigravity
      --exclude <PROVIDER>            Exclude providers from automatic selection (repeatable)
                                      [possible values: claude, codex, gemini, goose, kimi,
                                      opencode, qwen, kilo, pi, antigravity]
  -y, --yolo                          Enable the provider's auto-approval (YOLO) mode [env:
                                      CLAUDINE_YOLO=]
  -i, --interactive                   Run the provider session in interactive mode
      --no-interactive                Run the provider session in non-interactive mode
      --include <ENV_NAME>            Preserve this env var even when it matches sensitive-name
                                      filters
  -m, --model <MODEL>                 Override the model used by the provider
  -o, --output <FORMAT>               Set the output format (json, text, stream)
      --append-system-prompt <FILE>   Append a system prompt from a file [aliases: --asp]
      --replace-system-prompt <FILE>  Replace the provider's system prompt with contents from a file
                                      [aliases: --rsp]
  -t, --timeout <DURATION>            Wall-clock timeout (e.g. `30s`, `5m`, `2h`). Sends SIGTERM
                                      then SIGKILL. Only valid in non-interactive mode
      --step-timeout <DURATION>       Step-silence timeout (e.g. `30s`, `5m`). Kills the child when
                                      no stream event is observed for this long. Only valid in
                                      non-interactive structured-stream mode
      --stall-timeout <DURATION>      OpenCode stalled-generation backstop budget (e.g. `10m`).
                                      Aborts when OpenCode repeatedly drops generations with no
                                      progress for this long. OpenCode-scoped, structured-stream
                                      only; `0s` disables. Default `10m`
      --operation <OP>                Set the OPERATION env var for the composed session [aliases:
                                      --op]
      --sandbox                       Enable provider-specific sandboxing
      --repo                          Use only repo-scoped skills, commands, and agents via a
                                      provider overlay
      --dry-run                       Show the composed result without launching a provider;
                                      composition `$( … )` shell expansions still run
      --dry-run-shell                 Run each `shell:` task for real once during the preview;
                                      requires `--dry-run`
  -q, --quiet                         Suppress env details and info messages, but still show the
                                      system prompt when set
      --silent                        Suppress all output except the composition result
      --set <JSON>                    Override frontmatter values as JSON/JSON5 (e.g. `--set
                                      '{"key":"val"}'`)
      --mcp                           Enable Claudine-managed MCP session composition
      --use <ID>                      Activate specific MCP servers by ID or alias (comma-separated)
      --strict                        Treat unresolved or ambiguous MCP tags as hard errors
      --perf                          Emit a performance report to stderr after command completion
      --max-iterations <N>            Maximum number of loop iterations (overrides `loop.max`
                                      frontmatter)
      --on-rate-limit <POLICY>        What to do when a completed loop iteration reports a provider
                                      rate-limit signal. Overrides `loop.on_rate_limit` frontmatter
                                      [possible values: pause, abort, continue]
  -h, --help                          Print help

Example:
  claudine sequence sequence.md name=Ken -- --help

Everything after `--` is forwarded to the provider unchanged, so this example asks
the provider for its own help. Provider-native options are not listed above.

```

