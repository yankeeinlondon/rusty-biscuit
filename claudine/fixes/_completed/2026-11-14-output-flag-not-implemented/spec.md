---
kind: fix
created: 2026-11-14
status: implemented
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
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
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
implemented: true
implemented_by: claude/opus-5.5 (rust-developer sub-agent)
---

# Refuse Claudine's `--output` option until it is designed

## Why this exists

Claudine's universal `-o`/`--output <json|text|stream>` option forwards the
value to the wrapped provider's own output-format flag (Claude
`--output-format json`, Codex `--json`, and so on), through
`WrapperProfile::apply_output_format`. Passing it also turns off Claudine's
own structured stream (`wrapper_stages.rs`: structured mode requires
`args.output.is_none()`), so stdout becomes the provider's raw, per-provider
JSON and Claudine's parsing, rendering, hooks, and semantic log lose their
input.

That is not the intended design. Claudine always chooses the provider output
format that carries the most information, so that it can parse it. A future
`--output` may be fed from Claudine's own logs; whatever its design, Claudine
will **not** proxy the user's choice to the provider.

The option is also undefined in the docs: `--output` appears only as one-line
entries in flag lists (`docs/getting-started/index.md`,
`docs/topics/execution-flow.md`, `docs/cli/sequence.md`, `docs/pipeline.md`).

The review of `2026-11-14-edit-action` surfaced it: under `--output json` an
`initialize` failure leaves stdout empty, and the `edit` action's eligibility
read the option (a terminal editor was refused "because structured output
reserves the streams").

## How `md --output` works today (for comparison)

Darkmatter's `md` already has a working `--output`, and its design is the
nearest precedent for whatever Claudine adopts. Canonical description:
`darkmatter/docs/cli/output-and-width.md`; field contracts:
`darkmatter/docs/cli/data-contracts.md`.

- **One selector, two families.** `--output <target>` is the only way to
  choose what a command prints. *Formatted* targets (`terminal`, `plain`,
  `markdown`, `markdown-plus`, `html`) are for a person or a document; *data*
  targets (`json`, `json5`, `yaml`, `toml`) are the command's result as one
  value, for a program. Each command declares a default and the values it
  accepts; `md compose` accepts only `markdown` (its default) and the data
  family.
- **A script can trust stdout and never parse stderr.** A data output writes
  exactly one value to stdout whether the run succeeded or failed, and
  nothing to stderr: warnings and errors become `diagnostics`, `--perf`
  timings become `perf`, and lifecycle and shell output becomes
  `lifecycle_output`. A shell command that would need interactive approval
  fails the run rather than prompting. A formatted output writes the
  deliverable to stdout and everything else to stderr.
- **The exit status means the same thing in every output mode.** `0`
  succeeded, `1` ran and failed (the data value has `outcome: "failure"`),
  `2` usage error with nothing run and stdout empty, `130` interrupted.

  ```sh
  md compose broken.md                # stdout: nothing; stderr: the problem report; exit 1
  md compose broken.md --output json  # stdout: {"outcome": "failure", "diagnostics": [...], ...}; stderr: nothing; exit 1
  ```
- **The value is `md`'s own,** built from its result and diagnostics. It is
  never another program's output passed through, which is the property this
  fix protects for Claudine.
- **Lifecycle `edit` reads the host's streams, not the flag directly.** Under
  a data output `md compose` tells the shared editor service that stdout is
  reserved for that one value, so a terminal editor is refused and a GUI
  editor runs with its output discarded; under `markdown` the editor's
  output goes to stderr so it never lands in the document.

Claudine has none of this today: its stdout in a non-interactive run is the
provider's answer as rendered by Claudine, and there is no Claudine-level
result value. A future `--output` would need to decide whether to follow
`md`'s two-family, one-value-on-stdout contract.

## Author rulings (2026-11-14)

- **Refuse, do not ignore.** Any `-o`/`--output` value makes the command fail
  with a usage error before anything runs: no lifecycle event, shell, editor,
  or provider. The message is
  `the --output <target> CLI option has not been implemented yet in Claudine`
  (with the value the user passed in place of `<target>`). A caller that
  asked for JSON must never receive text with exit 0.
- **Remove the dead plumbing.** Keep only the declared option, so the refusal
  is Claudine's message and not clap's "unexpected argument". Delete the
  forwarding (`apply_output_format` and its callers and tests), the
  composition and wrapper `OutputFormat` types and every field that carries
  them (`PrepareOptions.output`, `CommandOutputPolicy.format`,
  `LaunchInputs.output_format`, `LaunchWarningSource::OutputFormat`), and the
  `From` conversion between them. The provider catalog's output-format data
  (`claudine::provider::OutputFormat`, `OutputFormatSupport`,
  `OutputFormatSelector`) stays: Claudine uses it to pick the richest stream
  (`apply_structured_stream`).
- **`edit` eligibility has no relationship to `--output`.** Delete
  `claudine::composition::lifecycle::editor::set_structured_output` and its
  process-wide flag; Claudine's editor service always reports
  `EditorOutput::Terminal`. Eligibility follows only the process's real
  facts (terminals, foreground, display session, parallel group leg).
  Darkmatter's `md compose` keeps its own `MarkdownDocument`/`Structured`
  cases, because there stdout really does carry the document or the JSON
  result.

## Author rulings, review round 1 (2026-11-14)

The first implementation refused `--output` only where the wrapper's
argument parsing happened to read it, and let help lose to the refusal. The
author ruled:

- **`--output` is never passed to the provider, wherever it is written.**
  Argument position never changes meaning: Claudine parses and consumes its
  options, setters, and switches, and only then acts. A command line whose
  meaning depends on where a Claudine option sits is a bug.
  - Found during review: wrapper commands (`claudine claude …`) capture every
    token from the first positional onward
    (`trailing_var_arg`), and a hand-written list
    (`wrap/flags.rs::extract_wrapper_flags_from_passthrough`) takes back only
    `-y`, `-i`, `--edit`, `--repo`, `-q`, `--silent`, `-v`, `--perf`, and
    `--operation`. Every other Claudine option after the prompt (`-o`, `-m`,
    `-t`, `--dry-run`, `--sandbox`, `--asp`/`--rsp`, `--include`, `--use`,
    `--strict`, `--stall-timeout`, `--help`) is forwarded to the provider.
    `claudine claude --dry-run -o json hi` is refused;
    `claudine claude --dry-run hi -o json` is not.
  - Fix: wrapper commands own every option Claudine declares on them,
    anywhere before the first `--`, using an owned-flag surface derived from
    the clap definitions (as `OwnedFlags::for_composition` already does for
    the composition commands), replacing the hand-written list. Only tokens
    after the first `--` belong to the provider.
- **Any command line containing `--help` (or `-h`) before the first `--`
  shows help and exits 0.** Help wins over the `--output` refusal and over
  every other usage error. The remaining arguments only select which help is
  shown (for example the subcommand). Showing help never requires loading or
  converting configuration.
  - Found during review: `claudine goose -o json --help` was refused (exit 2)
    in either order; `claudine claude hi --help` forwarded `--help` to the
    provider; `claudine compose -o json --help` printed help but exited 1;
    `claudine claude --help` exited 1 with `config.invalid` on a host whose
    configuration needs conversion.

## Requirements

1. `claudine <provider> -o <v>`, `--output <v>`, and `--output=<v>`, and the
   same on `compose`, `inline-compose`, `sequence`, and loop runs, exit with
   status 2 and print the refusal on stderr, for every `<v>` (including
   `text` and an unknown value). Nothing runs first: no provider launch, no
   lifecycle action, no `edit`, no shell approval prompt, nothing created.
   `--dry-run` is refused the same way.
2. A value after the provider-argument boundary is the provider's, not
   Claudine's (`claudine claude -- --output-format json` is unchanged by this
   fix; see Out of scope).
3. `--help` no longer advertises `-o`/`--output` among the universal
   switches; the option stays parseable so requirement 1 can refuse it.
4. Everything listed under "Remove the dead plumbing" is gone, with no
   remaining reference in code, tests, or docs.
5. With no `--output`, behavior is byte-for-byte unchanged: Claudine still
   selects each provider's structured stream.

## Documentation

- New topic page `claudine/docs/topics/output.md`: what Claudine writes to
  stdout and stderr in a non-interactive run (the provider's answer text on
  stdout; Claudine's status, reasoning, tool activity, and diagnostics on
  stderr), that Claudine always asks the provider for its richest structured
  stream and parses it, what a caller sees when no provider runs (a failure
  before the provider: empty stdout, the reason on stderr, nonzero exit), and
  that `--output` is refused with the message above. Written for a reader new
  to the repository, with an example per rule.
- Remove `-o`/`--output` from the flag lists in
  `docs/getting-started/index.md`, `docs/topics/execution-flow.md`,
  `docs/cli/sequence.md`, and `docs/pipeline.md` (C4.7), linking the new page
  where a reader would look for it.
- `docs/topics/flow-control/lifecycle.md`: drop the `--output` row of the
  `edit` host table and the `--output json` sentence under "When `edit`
  fails".
- Skills: `.claude/skills/claudine/cli-reference.md` and
  `architecture.md`; the user-scope `use-claudine` skill's universal-switch
  list.

## Verification

- A CLI test per command shape (wrapper, `compose`, `inline-compose`,
  `sequence`) and per spelling (`-o v`, `--output v`, `--output=v`) asserts
  exit 2, the exact message on stderr, empty stdout, and no side effect (a
  document whose `initialize` would `edit` and `ensure_file` creates nothing
  and launches nothing; no provider launch is recorded).
- The `edit` tests that ran under `--output json` are removed; the GUI and
  terminal eligibility tests that remain need no output flag.
- `cd claudine && just test`, `just test-l2`, `just lint` pass.

## Out of scope

- Designing a working `--output` (a Claudine-level result fed from its logs).
- Native provider flags passed after the provider-argument boundary
  (`flags::has_explicit_native_output_request` still yields to them). Whether
  Claudine should refuse those too is a separate decision.
