---
phases: 6
created: 2026-05-07
start_phase: 1
---

# Execution Plan: Better Shell Command Parsing

## Phase 1: Baseline and Compatibility Map

Goal: lock down the current behavior and identify every call site that assumes a single executable plus argv.

1. Inventory shell expansion entry points and write down the exact ownership boundaries:
   - `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs`
   - `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`
   - `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
   - `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`
   - `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`
   - `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`
   - `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
   - `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs`
2. Run the current targeted shell tests to establish a baseline:
   - `cargo test -p darkmatter-lib shell_expansion`
   - `cargo test -p darkmatter-lib shell_block`
   - `cargo test -p darkmatter-lib frontmatter_shell`
3. Confirm existing policy semantics for exact whitelist, prefix whitelist, blacklist, allow-once, pre-approved commands, alias resolution, timeout handling, and error handling.

Validation checkpoint:
- Baseline test results are recorded in the implementation notes or commit message.
- No code behavior has changed in this phase.

Parallelizable:
- A second engineer can inspect CLI/pre-approval UX while the main implementer maps library call sites.

## Phase 2: Shared Pipeline Model

Goal: introduce a representation that can express a command chain without losing single-command compatibility.

1. Add pipeline-oriented types in `types.rs`:
   - `ShellPipeline`
   - `CommandAction`
   - `CommandLink` or equivalent ordered chain edge
   - `ChainOperator::{And, Or}`
   - `RedirectionConfig`
   - `StdoutTarget::{Capture, Null, Stderr}`
   - `StderrTarget::{Capture, Null, Stdout}`
2. Extend `ShellDirective` to carry the parsed pipeline while preserving the existing `raw_command`, `executable`, and `args` fields until all call sites are migrated.
3. Add helpers on `ShellPipeline`:
   - `actions()` or `iter_actions()` for policy/discovery flattening.
   - `first_action()` for backwards-compatible display and origin errors.
   - `normalized_commands()` using existing `normalize_command`.
   - `raw_chain()` or use `ShellDirective::raw_command` for approval prompts.
4. Add unit tests for the model helpers using simple commands, redirections, and `A && B || C`.

Validation checkpoint:
- `cargo check -p darkmatter-lib`
- Existing single-command tests still compile with temporary compatibility fields.

Parallelizable:
- Unit tests for type helpers can be written in parallel with tokenizer work after the type names are finalized.

## Phase 3: Tokenizer and Parser

Goal: parse only the supported shell-like syntax and reject everything else before policy or execution.

1. Replace the tokenizer output with a structured token enum, for example:
   - `Word(String)`
   - `AndIf`
   - `OrIf`
   - `RedirectStdoutNull`
   - `RedirectStderrNull`
   - `RedirectStderrToStdout`
   - `RedirectStdoutToStderr`
2. Preserve the current quoting and escaping rules:
   - single quotes are literal
   - double quotes support current backslash behavior
   - backslash escaping outside quotes remains unchanged
3. Allow literal backticks as ordinary word characters. Continue rejecting `$(` command substitution.
4. Recognize only the allowed redirection forms:
   - `> /dev/null`
   - `2> /dev/null`
   - `2>&1`
   - `>&2`
5. Keep unsupported operators as parse errors:
   - `;`
   - `<`
   - single `|`
   - unsupported pipe-like sequences
   - arbitrary file redirection such as `> out.txt` and `2> err.log`
   - append redirection such as `>> /dev/null`
6. Build `ShellPipeline` in `parser.rs`, including validation for:
   - no empty command before or after `&&` / `||`
   - redirection belongs to the current command
   - duplicate/conflicting stdout or stderr redirections fail clearly
   - directive options and `::timeout:<N>` are parsed outside the command pipeline contract as they are today
7. Update body `::shell`, frontmatter `$()`, and shell-block command parsing to construct directives from the same pipeline parser.

Validation checkpoint:
- `cargo test -p darkmatter-lib tokenize`
- `cargo test -p darkmatter-lib parser`
- Manual parse assertions cover:
  - `echo ok && echo done`
  - `false || echo fallback`
  - `ls > /dev/null`
  - `python -c "..." 2>&1`
  - literal backticks in arguments
  - rejected `echo ok | cat`, `echo ok; echo bad`, `cat < file`, `echo ok > file`

Parallelizable:
- Rejection-case tests and acceptance-case tests can be split once the token enum is available.

## Phase 4: Policy, Approval, and Discovery

Goal: validate the entire chain before any process starts, while preserving existing approval semantics.

1. Add pipeline flattening helpers for effective commands after alias resolution:
   - each `CommandAction` resolves aliases independently
   - resolved alias args are prepended to that action only
   - redirection metadata is not treated as argv for policy matching
2. Replace single-command preparation with pipeline preparation in `shell_expansion/mod.rs`:
   - built-in blacklist checks every action
   - user blacklist checks every normalized action
   - whitelist and allow-once checks every normalized action
   - strict pre-approved mode requires every normalized action to be present
3. Implement atomic approval:
   - if any action needs approval, call the approval handler once with the full raw chain as the prompt command
   - approval rejection aborts the full pipeline
   - no action executes until every action has passed blacklist/whitelist/pre-approval/handler checks
4. Decide and implement persistence semantics for chain approval:
   - `AllowExactPersist`: persist each normalized action, not the raw chain containing operators
   - `AllowCommandPersist`: persist each executable prefix from the unapproved actions
   - `AllowOnce`: reserve and allow all normalized actions for this pipeline
   - `BlacklistPersist`: persist the full rejected normalized action that triggered the prompt, or all unapproved actions if the UI cannot identify one
5. Update `ShellApprovalRequest` only if necessary. Prefer preserving the existing public shape by setting:
   - `raw_command` to the full chain
   - `executable` to the first action executable
   - `args` to the first action args
   - `normalized_exact` to a stable full-chain display or the first missing normalized command, with documented behavior
6. Update `collect_shell_commands` to emit entries for every action in every chain while preserving source provenance and deduplicating by per-action normalized command.
7. Remove or narrow `RawToken(">")` and `RawToken(">>")` blacklist rules only after redirection tokens are no longer passed as argv.

Validation checkpoint:
- `cargo test -p darkmatter-lib policy`
- `cargo test -p darkmatter-lib discovery`
- Add tests proving:
  - a blacklisted second command aborts before the first command executes
  - pre-approved mode rejects a chain if any action is missing
  - approval handler receives the full raw chain once
  - discovery returns all commands in `A && B || C`

Parallelizable:
- Discovery tests can be written alongside preparation/approval tests after the pipeline flattening helper exists.

## Phase 5: Executor State Machine and Redirection

Goal: execute approved pipelines with Rust-side orchestration and no system shell.

1. Split the current process execution into a reusable single-action runner:
   - input: `CommandAction`, origin, raw display command, timeout, shell options, source
   - output: stdout, stderr, and exit code
   - still uses `which::which`, resolved working directory, null stdin, timeout polling, ANSI stripping, and rich errors
2. Implement redirection behavior:
   - `> /dev/null`: set stdout to `Stdio::null()` and return empty stdout
   - `2> /dev/null`: set stderr to `Stdio::null()` and return empty stderr
   - `2>&1`: merge stderr into the stdout result deterministically enough for tests; prefer a single captured stream if available, otherwise document possible ordering limits
   - `>&2`: route stdout into stderr result
3. Implement `execute_pipeline_detailed`:
   - run the first action
   - for `&&`, run the next action only when the previous exit code is `0`
   - for `||`, run the next action only when the previous exit code is non-zero
   - append captured outputs from executed actions in command order using the existing body contract
   - skipped actions produce no output and no error
4. Preserve error-handling behavior at directive level:
   - an unhandled non-zero final/action failure produces `ExecutionFailed`
   - existing `ErrorHandling` rules are applied to the pipeline failure result once
   - timeout fallback still returns empty output and warning for the directive
5. Preserve frontmatter behavior:
   - frontmatter shell expansion stores trimmed stdout only
   - stderr remains excluded except where `2>&1` intentionally routes it to stdout
6. Preserve shell-block behavior:
   - each logical shell-block line may now be a pipeline
   - shell-block lines still execute sequentially
   - partial output is preserved when a later pipeline fails

Validation checkpoint:
- `cargo test -p darkmatter-lib executor`
- Add executor tests for:
  - `ls > /dev/null`
  - stderr suppression with `2> /dev/null`
  - stderr-to-stdout with `2>&1`
  - stdout-to-stderr with `>&2`
  - `true && echo yes`
  - `false && echo skipped`
  - `true || echo skipped`
  - `false || echo fallback`
  - `false && echo skipped || echo fallback`

Parallelizable:
- Redirection tests and chaining tests can be developed in parallel once the single-action runner exists.

## Phase 6: Integration, Documentation, and Acceptance Closure

Goal: prove the feature works through public compose paths and update public-facing documentation where behavior changes.

1. Add integration coverage in `darkmatter/lib/tests/shell_pipeline_integration.rs` or the closest existing shell integration file:
   - body `::shell`
   - frontmatter `$()`
   - shell-block line
   - composed markdown with policy root and whitelist fixtures
2. Add policy/approval integration tests:
   - chain approval prompt includes the full chain
   - no command executes when a later command fails policy preflight
   - allow-once allows all actions in the approved chain for that compose run
3. Run acceptance criteria exactly:
   - `ls > /dev/null` executes with no captured output
   - `cmd1 && cmd2` executes `cmd2` only after success
   - `cmd1 || cmd2` executes `cmd2` only after failure
   - `npm install && npm run build || echo "Build failed"` parses and preflights as one chain
   - unsupported `;`, `<`, and `|` still fail during tokenization/parsing
   - literal backticks are preserved in arguments
4. Update docs when behavior changes:
   - `darkmatter` README or shell expansion docs, if present
   - `.claude/skills/darkmatter/SKILL.md` shell expansion notes
   - `docs/dependencies.md` only if a new crate is added for stream merging
5. Run final validation:
   - `cargo fmt --all`
   - `cargo check -p darkmatter-lib`
   - `cargo test -p darkmatter-lib shell_expansion`
   - `cargo test -p darkmatter-lib shell_block`
   - `cargo test -p darkmatter-lib frontmatter_shell`
   - `cargo test -p darkmatter-lib --test shell_pipeline_integration` if a new integration target is added

Validation checkpoint:
- All acceptance criteria have direct tests or documented manual verification.
- No shell command is executed before full-chain policy and approval succeeds.
- Existing single-command behavior remains compatible.

Parallelizable:
- Documentation updates can happen while final integration tests run.
- Manual acceptance notes can be collected in parallel with full targeted test runs.
