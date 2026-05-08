---
ready: false
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### Critical: Required shell syntax is still rejected at tokenization

The implementation has not adopted the new command grammar. `tokenize()` still returns hard parse errors for `|`, `<`, `>`, backticks, `$(`, and `&&`; `||` is rejected through the `|` branch. This directly blocks the required `> /dev/null`, `2> /dev/null`, `2>&1`, `>&2`, `&&`, `||`, complex chains, and literal backtick support.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:20` still documents these patterns as rejected.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:132` rejects `|`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:145` rejects `<`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:151` rejects `>`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:157` rejects backticks.
- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:173` rejects `&&`.

Impact: all positive acceptance criteria except keeping unsupported `;`, `<`, and `|` rejected fail before policy or execution can run.

Verification level: none for the new behavior. Existing Level 1 unit tests assert the old behavior instead, e.g. `tokenize_output_redirect_fails`, `tokenize_backtick_fails`, `tokenize_double_ampersand_fails`, and `tokenize_double_pipe_fails` in `tokenize.rs:327`, `tokenize.rs:340`, `tokenize.rs:363`, and `tokenize.rs:411`.

### Critical: No pipeline model exists, so chain policy and execution cannot be correct

The spec calls for `ShellPipeline`, `CommandChain`, `CommandAction`, `ChainOperator`, and `RedirectionConfig`. These types do not exist in implementation code; `rg` only finds them in the spec and plan documents. The parser still flattens the token list into one `ShellDirective { executable, args }` by taking `cmd_tokens[0]` and `cmd_tokens[1..]`.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs:86` sets a single executable.
- `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs:87` stores every remaining token as args.
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` has no pipeline/action/redirection types.

Impact: even if tokenization allowed operators, the parser would not preserve operator boundaries, per-command argv, or redirection metadata.

Verification level: none. There are no parser tests for `cmd1 && cmd2`, `cmd1 || cmd2`, redirection configs, malformed chain placement, or preserving literal backticks.

### Critical: Executor still spawns exactly one process and always captures stdout/stderr

The executor still builds one `std::process::Command`, sets both stdout and stderr to `Stdio::piped()`, and returns success only for that single child status. There is no chain state machine and no configurable stdout/stderr target for nulling or stream merging.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:168` creates one `Command`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:170` passes one args list.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:172` to `executor.rs:174` always pipes stdout and stderr.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:217` waits on one child process.

Impact: `A && B`, `A || B`, `> /dev/null`, `2> /dev/null`, `2>&1`, and `>&2` cannot execute as specified.

Verification level: none for the new behavior. There are no Level 1 unit/integration tests proving chained execution decisions, output suppression, stderr-to-stdout merging, stdout-to-stderr routing, timeout behavior across a chain, or error-handling interaction with the final chain status.

### High: Whole-chain preflight validation and approval are missing

Policy and approval still run against a single normalized command. `prepare_directive()` resolves one executable, checks one blacklist/whitelist decision, and sends one `ShellApprovalRequest`. There is no flattening of a chain, no validation of every command before execution, and no prompt containing the entire chain.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:153` resolves one directive.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:157` normalizes one command.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:184` to `mod.rs:217` checks blacklist/whitelist for one executable.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:227` to `mod.rs:239` builds one approval request.

Impact: the security requirement is not implemented. A future partial parser that allowed chaining without changing this path could approve/whitelist the wrong surface.

Verification level: none. There are no tests for mixed whitelisted/unwhitelisted chains, rejecting the whole chain when any command is blacklisted, approval prompt text containing the full chain, or atomic abort before any child process starts.

### High: Frontmatter shell expansion is also limited to the old single-command tokenizer

Frontmatter `$(...)` commands call the same `tokenize()` function and then store one executable plus args. The new grammar therefore does not work for top-level frontmatter shell expansions, even though the spec says frontmatter and body shell expansion share behavior.

Evidence:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:110` calls `tokenize(inner_command)`.
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:113` and `frontmatter_shell_expansion.rs:114` store a single executable and args.

Impact: `$(cmd1 && cmd2)`, `$(cmd1 || cmd2)`, and `$(cmd > /dev/null)` fail in frontmatter before approval/execution.

Verification level: none. There are no frontmatter tests for the new syntax.

### Medium: Unsupported syntax coverage is partial but not aligned with the new grammar

The only acceptance criterion currently covered at Level 1 is that unsupported metacharacters continue to fail. However, those tests are coupled to the old “reject all redirection/chaining/backticks” grammar and now assert incorrect behavior for `>`, `&&`, `||`, and backticks.

Impact: the existing tests would prevent the feature from being implemented until rewritten.

Verification level: Level 1 present for `;`, `<`, and `|` rejection. Needs Level 1 tests that distinguish allowed `||` from unsupported bare `|`, allowed curated redirections from arbitrary file redirection, and literal backticks from unsupported subshell expansion.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `ls > /dev/null` executes with no captured stdout | none; old Level 1 test rejects `>` | gap |
| `2> /dev/null` suppresses stderr | none | gap |
| `2>&1` merges stderr into stdout | none | gap |
| `>&2` routes stdout to stderr | none | gap |
| `A && B` executes `B` only when `A` succeeds | none; old Level 1 test rejects `&&` | gap |
| `A || B` executes `B` only when `A` fails | none; old Level 1 test rejects `||` via pipe error | gap |
| complex `A && B || C` chain state machine | none | gap |
| every command in a chain is policy checked before any execution | none | gap |
| approval prompt presents entire chain | none | gap |
| policy rejection aborts entire chain atomically | none | gap |
| `;`, `<`, bare `|` remain unsupported | Level 1 tokenizer tests | partially ok |
| backticks are literal arguments, not subshell expansion | none; old Level 1 test rejects backticks | gap |
| body `::shell` and frontmatter `$()` both support the grammar | none | gap |

No Level 2 or Level 3 terminal-emulator testing is required for these requirements as written; the observable behavior is command parsing, policy, process orchestration, and captured output. Level 1 unit and CLI/in-process integration tests are the appropriate minimum here, but they are mostly absent or currently inverted.

## Recommendations

Implement the feature as a structural change, not as extra token exceptions:

- Replace `Vec<String>` tokenization with a token enum that preserves arguments, `&&`, `||`, and curated redirection tokens.
- Add pipeline/action/redirection types in `types.rs` and make `ShellDirective` carry a parsed pipeline, while keeping display/raw text for approval and diagnostics.
- Parse and validate redirections explicitly: allow only `/dev/null`, `2>&1`, and `>&2`; reject arbitrary paths and unsupported forms such as `>>`, `<`, and bare `|`.
- Change policy preparation to flatten all command actions, resolve aliases per action, check blacklists/whitelists/pre-approved sets for every action, and prompt once with the raw full chain.
- Add an executor path that runs the chain state machine and applies per-command stream config. Define and test how chain exit status interacts with existing `--when-error` and timeout behavior.
- Update discovery and frontmatter shell expansion to use the same pipeline parser so approval previews and execution agree.

Minimum test additions before production:

- Level 1 tokenizer/parser tests for every allowed redirection, every unsupported redirection/metacharacter, operator placement errors, quoting around operators, and literal backticks.
- Level 1 executor tests with helper commands/scripts for `&&`, `||`, `A && B || C`, stdout suppression, stderr suppression, stderr merge, stdout-to-stderr routing, timeout, and non-zero exit handling.
- Level 1 policy/approval tests proving whole-chain approval, preflight of all commands, blacklisted command aborts before any earlier command executes, and whitelist/pre-approved behavior for chains.
- CLI integration tests for body `::shell` and frontmatter `$()` covering successful chain execution and parse failures with useful diagnostics.

## Production Readiness

Not ready. The implementation in the current worktree has not implemented the core parser, policy, or executor model described by the specification, and the existing tests still encode the pre-feature behavior.
