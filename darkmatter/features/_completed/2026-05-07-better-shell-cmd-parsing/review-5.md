---
ready: false
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### High: Shell-block commands still reject the new pipeline grammar

The plan's Phase 6 explicitly requires shell-block lines to accept the same pipeline grammar as body `::shell` and frontmatter `$()` commands: each logical shell-block line may now be a pipeline. The current shell-block parser still tokenizes each logical command, strips only `Word` tokens, and hard-rejects `&&`, `||`, and all redirection tokens.

Evidence:

- `darkmatter/features/2026-05-07-better-shell-cmd-parsing/plan.md` Phase 5 says shell-block lines may now be pipelines and Phase 6 calls for shell-block integration coverage.
- `darkmatter/lib/src/markdown/compose/shell_blocks/body.rs:70` rejects non-word tokens for shell blocks.
- `darkmatter/lib/src/markdown/compose/shell_blocks/body.rs:79` and `body.rs:82` explicitly return "Chain operators ... are not allowed in shell blocks".
- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:106` builds shell-block `ShellDirective`s with only `executable`/`args` and `pipeline: None`.
- Existing tests assert the old rejection behavior, for example `double_ampersand_rejected` and `rejects_redirect`.

Impact: a documented public compose surface is missing. Body directives and frontmatter support `echo a && echo b`, but the equivalent shell-block command fails during parsing before policy or execution. This also means discovery cannot emit one entry per action for shell-block pipelines because those commands never parse as pipelines.

Verification level: Level 1 gap. This is parser/execution behavior, so Level 1 is appropriate, but the strongest current tests verify rejection rather than the required user-observable behavior.

### Medium: Pipeline timeout fallbacks lose their compose warning

Single-command timeout fallback returns `CommandExecution::timeout_fallback(timeout)`, and `execute_prepared_directive()` converts that marker into a compose warning. The multi-action pipeline path drops that marker: `execute_pipeline_detailed()` receives `Ok(exec)` from an action, copies only stdout/stderr into accumulators, and returns `CommandExecution::from_streams(...)` at the end.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:591` returns `CommandExecution::timeout_fallback(timeout)` when `ShellTimeoutBehavior::EmptyString` is active.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:367` to `executor.rs:381` handles successful pipeline actions but ignores `exec.timeout_fallback`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:439` returns `CommandExecution::from_streams(...)`, which clears the timeout marker.
- Existing timeout fallback tests cover single-command frontmatter behavior, for example `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:761`, but I did not find a pipeline timeout fallback test.

Impact: `--allow-shell-timeout` still suppresses the timed-out pipeline action, but the user loses the warning that the directive timed out and was replaced with an empty string. That is a behavioral regression from the existing timeout contract and makes pipeline failures harder to audit.

Verification level: Level 1 gap. Timeout fallback and compose warnings are in-process behavior; add Level 1 tests for body and frontmatter pipelines such as `sleep 1 && echo never` with a short timeout and `ShellTimeoutBehavior::EmptyString`.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `> /dev/null` executes with no captured stdout | Level 1 compose tests | ok |
| `2> /dev/null` suppresses stderr | Level 1 compose tests | ok |
| `2>&1` and `>&2` merge/route streams with stable ordering | Level 1 compose tests | ok |
| combined accepted redirections have coherent shell-style semantics | Level 1 parser and compose tests | ok |
| `A && B` and `A || B` execute conditionally | Level 1 compose tests | ok |
| complex `A && B || C` chains | Level 1 compose tests | ok |
| every command in a chain is policy checked before execution | Level 1 compose regression tests | ok |
| approval prompt presents the entire chain | Level 1 approval-request tests | ok |
| dry-run shell command report presents every body/frontmatter chain action | Level 1 discovery tests | ok |
| shell-block line may be a pipeline | Level 1 tests assert rejection | gap |
| unsupported `;`, `<`, bare `|`, arbitrary redirection remain rejected | Level 1 tokenizer/parser tests | ok |
| literal backticks are preserved | Level 1 tokenizer test | ok |
| frontmatter `$()` supports the new chain grammar and rejects interpolated executables after operators | Level 1 parser/compose/discovery tests | ok |
| timeout fallback emits warnings for pipeline directives | no targeted pipeline test; implementation drops marker | gap |

No Level 2 or Level 3 verification appears necessary for this feature as specified. The user-observable behaviors here are command parsing, process orchestration, captured output, policy prompts, and compose warnings rather than terminal emulator rendering or OS keyboard input.

## Recommendations

- Change shell-block parsing to preserve `ShellPipeline` from `parse_pipeline()` instead of reducing commands to word tokens, and pass that pipeline through the `ShellDirective` built in `shell_blocks/mod.rs`.
- Replace the current shell-block rejection tests for `&&` and supported redirections with acceptance tests, while keeping rejection tests for `;`, `<`, bare `|`, arbitrary file redirection, and command substitution.
- Carry `timeout_fallback` out of `execute_pipeline_detailed()`, likely by tracking the first timed-out action while accumulating stdout/stderr, then returning a `CommandExecution` with that marker set.
- Add Level 1 integration tests for pipeline timeout fallback in body `::shell`, frontmatter `$()`, and shell-block pipelines once shell-block support is wired.

## Validation Run

- `cargo test -p darkmatter shell_expansion` passed: 294 passed, 1 ignored.
- `cargo test -p darkmatter shell_block` passed: 77 unit-filtered tests plus 15 `shell_block_integration` tests passed.
- `cargo test -p darkmatter frontmatter_shell` passed: 42 passed.

Note: the feature plan still references `cargo test -p darkmatter-lib ...`, but the workspace package name from `cargo metadata` is `darkmatter`.

## Production Readiness

Not ready. The prior review findings appear addressed, but shell-block pipeline support is still absent and pipeline timeout fallbacks silently lose their warning marker.
