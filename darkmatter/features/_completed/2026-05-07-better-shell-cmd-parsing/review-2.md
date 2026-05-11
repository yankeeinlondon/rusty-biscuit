---
ready: true
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### Critical: Pipeline whitelist checks use the first executable for every chained command

`prepare_directive()` intends to whitelist every command in a chain, but the whitelist path calls `executable_for_normalized()` for each normalized command, and that helper always returns `directive.executable` / `directive.args`. For a pipeline, those are the first action only.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:212` checks every normalized command.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:214` asks `executable_for_normalized()` for the executable.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:586` ignores the normalized command and returns the directive's first executable.

Impact: a prefix whitelist for the first command can authorize later commands that are not whitelisted. I reproduced this with a file containing `::shell echo ok && pwd` and a whitelist containing only `prefix echo`; `md compose` executed both `echo` and `pwd` without approval. This violates the spec's whole-pipeline preflight requirement.

Verification level: Level 1 gap. Existing policy tests cover single commands, but there is no Level 1 policy/CLI test proving that each pipeline action is independently whitelisted.

### High: Body `::shell` approval/reporting loses redirection syntax

The body directive parser replaces the original command text with `pipeline.display_string()`. That formatter serializes executables, args, and chain operators, but it never serializes redirection configuration. As a result, body commands such as `echo hidden > /dev/null` are reported and approved as `echo hidden`.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs:86` sets `raw_command` from `pipeline.display_string()`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:130` to `types.rs:148` formats commands without redirection tokens.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:245` to `mod.rs:252` uses `effective.raw_command` / normalized commands in approval requests.

Impact: the user is not shown the entire chain/syntax for upfront approval, and `md compose --shell` underreports the command surface. I reproduced `::shell echo hidden > /dev/null` reporting as `echo hidden`.

Verification level: Level 1 gap. There are tokenizer/parser tests for redirection config, but no CLI/discovery/approval test verifying that the whole user-authored command, including redirections, is surfaced.

### High: Trailing chain operators are accepted and silently ignored

`parse_pipeline()` builds an action when it sees `&&` or `||`, stores the pending operator, and then only builds a final action if there are words or a redirection. For `cmd &&` or `cmd ||`, the pending operator is never validated, so the command executes as if the operator was absent.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:360` to `tokenize.rs:368` accepts an operator after building the previous action.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:385` only builds a final action when there are words/redirection.
- There is a test for a leading operator, but none for trailing operators.

Impact: malformed shell syntax is treated as a successful partial command. I reproduced `::shell echo ok &&` composing successfully and outputting `ok`.

Verification level: Level 1 gap. Add parser and CLI tests for trailing `&&`, trailing `||`, doubled operators, and operator followed only by redirection.

### Medium: Execution coverage is too light for the implemented chain/redirection behavior

The new tokenizer/parser tests cover allowed tokens and parsed redirection fields, but the executor tests still exercise only single-command `ShellDirective`s with `pipeline: None`. I did not find focused Level 1 executor tests for:

- `A && B` skip/execute decisions.
- `A || B` skip/execute decisions.
- `A && B || C` final status and output behavior.
- `> /dev/null`, `2> /dev/null`, `2>&1`, and `>&2` through the compose path.
- timeout and `--when-*` error handling on chained commands.

Verification level: Level 1 partial. Level 2/Level 3 terminal testing is not required for this feature as specified; the observable behavior is parsing, approval, process orchestration, and captured output. But Level 1 needs to cover the executor and CLI/in-process compose paths, not just tokenization.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `ls > /dev/null` executes with no captured stdout | Level 1 tokenizer/parser only | gap: compose/CLI output and approval reporting incomplete |
| `2> /dev/null` suppresses stderr | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| `2>&1` merges stderr into stdout | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| `>&2` routes stdout to stderr | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| `A && B` executes `B` only if `A` succeeds | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| `A || B` executes `B` only if `A` fails | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| complex `A && B || C` chains | Level 1 tokenizer/parser only | gap: no executor/compose verification |
| every command in a chain is policy checked before execution | partial Level 1, broken for whitelist prefix | gap, critical |
| approval prompt presents the entire chain | partial Level 1, redirections omitted for body directives | gap |
| policy rejection aborts entire chain atomically | Level 1 blacklist path for commands, not whitelist/approval chains | partial |
| `;`, `<`, bare `|`, arbitrary redirection remain unsupported | Level 1 tokenizer tests | ok |
| literal backticks are preserved | Level 1 tokenizer test | ok |
| frontmatter `$()` supports the new grammar | Level 1 parser support present | needs compose/execution coverage |

## Recommendations

- Change `executable_for_normalized()` to operate by action index, or remove it and reuse the already-indexed action helper for whitelist checks.
- Preserve the authored body command string separately from normalized display output. Approval and `--shell` reporting should show the full command including redirections.
- Make `parse_pipeline()` reject a pending operator at end-of-input.
- Add Level 1 compose/CLI tests for chained execution, redirection output behavior, malformed trailing operators, whole-chain approval request contents, and independent whitelist checks for every action.

## Production Readiness

Not ready. The core parser and executor model exists now, but there is a security-significant whitelist bug and approval/reporting does not present the full redirection surface for body directives.
