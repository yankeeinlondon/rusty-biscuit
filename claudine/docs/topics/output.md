# What Claudine Writes, and Where

This page tells a caller (a person at a shell, or a script reading Claudine's
streams) what a non-interactive `claudine` run prints to stdout, what it prints
to stderr, and how to tell success from failure. It applies to every command
that runs a provider: the wrappers (`claudine claude`, `claudine codex`, …),
`claudine compose`, `claudine inline-compose`, and `claudine sequence`.

A run is _non-interactive_ when Claudine starts the provider with a prompt and
no conversation, for example `claudine claude "summarize README.md"` or
`claudine compose task.md`. An interactive session hands the terminal to the
provider, so none of the rules below apply to it. See
[Non-Interactive Sessions](non-interactive-sessions.md) for how a session's mode
is chosen.

## stdout is the answer; stderr is everything else

Claudine writes only the provider's answer text to stdout. Everything Claudine
or the provider says _about_ the run goes to stderr: the execution header, the
session and model line, the provider's reasoning, tool calls and their results,
warnings, errors, and the closing summary.

```sh
claudine claude "Name the license in LICENSE" > answer.txt
# answer.txt:  The project is licensed under the MIT License.
# terminal:    the header, any tool activity, and the summary line (stderr)
```

When stdout is a terminal, the answer is rendered as Markdown; when it is
redirected to a file or a pipe, the answer is written as the provider's plain
text.

Three things change what reaches stdout, each one because you asked for it:

- `--dry-run` on `compose`, `inline-compose`, or `sequence` runs nothing, so
  stdout carries each composed prompt instead of an answer
  (`claudine compose --dry-run task.md > prompt.md`). See
  [A dry run's report](#a-dry-runs-report).
- `inline-compose` has the provider rewrite the document itself; the
  rewritten document, not stdout, is its result.
- A lifecycle action that writes to the `stdout` channel adds its line to
  stdout, between or beside the answer's lines. See
  [Lifecycle](flow-control/lifecycle.md). The order and layout of the stderr sections are described in
[Non-Interactive Sessions → The 9-Section Model](non-interactive-sessions.md#the-9-section-model).

## Claudine always asks the provider for its richest stream

Each provider can report its work in several formats: plain text, one JSON
document at the end, or a stream of JSON events. Claudine always asks for the
format that carries the most information (usually the event stream) and parses
it. That stream is what lets Claudine show tool activity and reasoning as they
happen, run lifecycle events and hooks, record the semantic log, and decide
whether the run actually succeeded.

```text
you type:       claudine claude "hi"
Claudine runs:  claude --print --output-format stream-json … "hi"
```

The caller never sees that raw stream. The table of formats Claudine selects
for each provider is in
[Non-Interactive Sessions → Structured JSON Streaming](non-interactive-sessions.md#structured-json-streaming).

A provider option you pass after the first `--` is the provider's, not
Claudine's, and reaches it unchanged. If that option itself
selects an output format, Claudine leaves the provider's format alone and stops
parsing the stream:

```sh
claudine claude "hi" -- --output-format json
# Claude prints its own JSON to stdout; Claudine does not render or parse it
```

## When no provider runs

A run can stop before the provider starts: the composition file is missing, a
shell command in it is refused, an `initialize` event fails, or no installed
provider can run the document. Then:

- stdout is empty, because there is no answer;
- stderr carries the reason;
- the exit status is nonzero.

```sh
claudine compose missing.md > answer.txt
echo $?        # 1
wc -c answer.txt  # 0 answer.txt
```

A script can therefore treat a nonempty stdout together with exit status `0` as
an answer, and anything else as a run that produced none.

## Exit status

| Status | Meaning |
|---|---|
| 0 | the run succeeded |
| 1 | the run failed, including a failure before the provider started, and a provider that exited 0 without completing its work |
| 2 | a usage error: Claudine refused the command line and ran nothing |
| 130 | interrupted with Ctrl-C |

Exit 0 from the provider alone is not success; see
[Non-Interactive Sessions → Exit Code 0 Is Not Success](non-interactive-sessions.md#exit-code-0-is-not-success).

## A dry run's report

`--dry-run` on `compose`, `inline-compose`, or `sequence` checks the document
without running anything (see
[Composition → Dry Run](composition.md#dry-run)). Its streams follow the same
split:

- **stderr** carries the report: each problem found, each check that could
  not run, and a closing summary line.
- **stdout** carries each composed preview.
- With `--json` (accepted only beside `--dry-run`), stdout carries one JSON
  report and stderr stays empty:

  ```sh
  claudine compose task.md --dry-run --json | jq -r '.diagnostics[].code'
  # composition.provider_unresolved
  ```

The exit status is `2` when the report holds any error or warning, and `0`
when it holds none; a dry run never exits `1`. A script can therefore treat
any non-zero status as "fix the document first".

## `--output` is refused

Claudine has no output selector. `-o` and `--output` are still recognized, so
that they get a clear answer instead of being forwarded to the provider, but any
value is refused before anything runs: no lifecycle event, shell approval,
editor, or provider. The command exits with status `2`, prints one line to
stderr, and leaves stdout empty. `--dry-run` is refused the same way.

Where you write the option does not matter: before or after the prompt or
file, between other options, in any spelling (`-o json`, `--output json`,
`--output=json`). Only after the first `--` is `--output` the provider's.

```sh
claudine compose task.md --output json
claudine claude "fix it" -o json
# stderr: the --output json CLI option has not been implemented yet in Claudine
# stdout: nothing
echo $?   # 2
```

`--help` (or `-h`) anywhere before `--` still shows help and exits `0`, even
beside `--output`.

The refusal names whatever value you passed, including values Claudine never
accepted (`--output=yaml`). It is deliberate: a caller that asks for JSON must
never receive text with exit status `0`. The option is not shown in `--help`.

## For comparison: `md --output`

Darkmatter's `md` CLI, which Claudine uses to compose Markdown, does have a
working `--output`. Claudine has no equivalent today beyond a dry run's
`--json`, and whether it will adopt
the same design is not decided. The `md` contract, in short:

- **One selector, two families.** _Formatted_ targets (`terminal`, `plain`,
  `markdown`, `markdown-plus`, `html`) are for a person or a document; _data_
  targets (`json`, `json5`, `yaml`, `toml`) are the command's result as one
  value, for a program.
- **A data output is exactly one value on stdout and nothing on stderr,**
  whether the run succeeded or failed: warnings and errors become diagnostics
  inside the value.
- **The exit status means the same in every mode:** `0` succeeded, `1` ran and
  failed, `2` usage error with nothing run, `130` interrupted.
- **The value is `md`'s own,** built from its result and diagnostics, never
  another program's output passed through.

```sh
md compose broken.md                # stdout: nothing; stderr: the problem report; exit 1
md compose broken.md --output json  # stdout: {"outcome": "failure", "diagnostics": [...], ...}; stderr: nothing; exit 1
```

The full description is in Darkmatter's
[Output and Width](../../../darkmatter/docs/cli/output-and-width.md) and the
field contracts in
[Data Contracts](../../../darkmatter/docs/cli/data-contracts.md).

## Related

- [Non-Interactive Sessions](non-interactive-sessions.md): the stderr sections,
  provider streams, and success classification.
- [Wrapped Execution Switches](wrapped-execution-switches.md): the options every
  wrapper accepts.
