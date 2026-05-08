---
phases: 4
created: 2026-05-04
start_phase: 1
---

# Execution Plan: Better Shell Command Parsing

This plan implements logical command chaining (`&&`, `||`) and specific redirection patterns (`> /dev/null`, `2>&1`, etc.) in `darkmatter` shell expansion, while maintaining security through Rust-side orchestration.

## Phase 1: Core Types and Tokenization

Update the data model and tokenizer to support the richer shell syntax.

- [ ] **Step 1: Define Core Types**
    - Update `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` with:
        - `ShellPipeline`, `CommandChain`, `CommandAction`, `ChainOperator`, `RedirectionConfig`, `StdoutTarget`, `StderrTarget`.
    - Update `ShellDirective` and `ShellCommandEntry` to work with the new pipeline structure.
    - **Validation:** `cargo check -p darkmatter-lib`.

- [ ] **Step 2: Enhance Tokenizer**
    - Introduce `Token` enum in `tokenize.rs`: `Argument(String)`, `And` (`&&`), `Or` (`||`), and redirection tokens.
    - Update `tokenize` function to return `Vec<Token>`.
    - Support literal backticks (preserve instead of error).
    - **Validation:** Run updated tests in `tokenize.rs`.

## Phase 2: Parsing and Validation

Refactor the parser to build pipelines and update the policy engine for multi-command validation.

- [ ] **Step 3: Update Parser**
    - Update `parser.rs` to transform `Vec<Token>` into a `ShellPipeline`.
    - Adjust `extract_error_handling` and `extract_timeout_suffix` to work with tokens.
    - Update `darkmatter/lib/src/markdown/compose/shell_blocks/body.rs` to handle the new tokenizer output.
    - **Validation:** Run tests in `parser.rs`.

- [ ] **Step 4: Enhance Policy Engine**
    - Update `policy.rs` to remove raw redirection tokens from the built-in blacklist.
    - Implement `validate_pipeline` in `policy.rs` that checks every command in a `ShellPipeline`.
    - Update `discovery.rs` to flatten pipelines into `ShellCommandEntry` for upfront user approval.
    - **Validation:** `cargo test -p darkmatter-lib --lib markdown::compose::shell_expansion::policy`.

## Phase 3: Execution Engine

Implement the state machine for conditional execution and redirection handling.

- [ ] **Step 5: Implement Redirection Logic**
    - In `executor.rs`, update process spawning to configure `Stdio` based on `RedirectionConfig`.
    - Handle `2>&1` by merging stderr into stdout or using appropriate piping.
    - **Validation:** Unit test for redirection in `executor.rs`.

- [ ] **Step 6: Implement Pipeline State Machine**
    - Implement `execute_pipeline` in `executor.rs`.
    - Manage exit status and conditional jumps for `&&` and `||`.
    - **Validation:** Unit test for `cmd1 && cmd2 || cmd3` logic.

## Phase 4: Integration and Final Validation

Wire everything together and verify the acceptance criteria.

- [ ] **Step 7: Wire Integration Points**
    - Update `execute_command` and `execute_command_detailed` in `executor.rs` to use the new pipeline engine.
    - Update `darkmatter/lib/src/markdown/compose/mod.rs` if necessary to handle the new directive structure.
    - **Validation:** `cargo test -p darkmatter-lib`.

- [ ] **Step 8: Acceptance Testing**
    - Verify `ls > /dev/null` suppresses output.
    - Verify `echo success && echo done` prints both.
    - Verify `false && echo skipped` only executes `false`.
    - Verify complex chains and upfront approval prompts.
    - **Validation:** Create a new integration test file `darkmatter/lib/tests/shell_pipeline_integration.rs`.
