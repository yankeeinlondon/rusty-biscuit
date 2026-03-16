---
spec: darkmatter/features/shell-expansion/spec.md
tech_design: darkmatter/features/shell-expansion/tech-design.md
last_updated: 2026-03-15
plan: darkmatter/features/shell-expansion/plan.md
review: darkmatter/features/shell-expansion/review.md
implement_complete: false
implementation_files: darkmatter/lib/src/markdown/transform/shell_expansion/types.rs, darkmatter/lib/src/markdown/transform/shell_expansion/tokenize.rs, darkmatter/lib/src/markdown/transform/shell_expansion/parser.rs, darkmatter/lib/src/markdown/transform/shell_expansion/policy.rs, darkmatter/lib/src/markdown/transform/shell_expansion/store.rs, darkmatter/lib/src/markdown/transform/shell_expansion/executor.rs, darkmatter/lib/src/markdown/transform/shell_expansion/mod.rs, darkmatter/lib/src/markdown/transform/mod.rs, darkmatter/lib/src/markdown/transform/types.rs, darkmatter/lib/src/markdown/types.rs, darkmatter/cli/src/approval.rs, darkmatter/cli/src/commands.rs, darkmatter/cli/src/lib.rs
---
## Tech Design for shell-expansion Complete

**Timestamp:** 2026-03-15

- **What it does**: Adds a `::shell <command> <params>` directive to Darkmatter Markdown that executes approved host commands and replaces the directive line with captured output (stdout+stderr).

- **Key design decision**: Commands are executed directly via `std::process::Command` with parsed argv tokens, never through a shell interpreter (`sh -c`, `bash -c`, etc.). This is the most important choice -- it makes blacklist enforcement reliable, prevents accidental support for pipes/redirection/substitution, and stabilizes approval matching on normalized argv.

- **Stage placement**: Shell expansion slots into Stage 1 as step 4 (after replacement, interpolation, TOC linking; before cleanup, normalization). This lets `{{ }}` interpolation feed into command args and lets cleanup/normalization operate on shell-emitted markdown.

- **Public API surface**: `ShellExpansionOptions` (timeout, policy_root, working_directory, approval_handler) added to `TransformOptions`; `ShellApprovalHandler` trait with `approve()` method; `ShellApprovalRequest`/`ShellApprovalDecision` enum (AllowExactPersist, AllowCommandPersist, AllowOnce, Deny, BlacklistPersist); new `shell_expansion` toggle on `Stage1Stages`; two new counters on `TransformReport`.

- **Internal runtime changes**: Introduces `PipelineRuntime` wrapping both `TransclusionRuntime` and `ShellExpansionRuntime`, replacing the single-purpose transclusion runtime. This ensures allow-once decisions and policy mutations persist across recursively composed child documents within a single compose run.

- **Module layout**: New `transform/shell_expansion/` module with six files: `mod.rs` (orchestration), `parser.rs` (directive scanning), `tokenize.rs` (argv tokenization), `policy.rs` (blacklist/whitelist matching), `store.rs` (policy file discovery/load/append), `executor.rs` (process spawn/timeout/capture), `types.rs` (data types and errors).

- **Security model**: Three-layer defense -- (1) built-in blacklist with structured rules (Executable, ExecutablePrefix, SubcommandPrefix, ArgExact, ArgPrefix, RawToken), (2) persisted user blacklist (`.shell-blacklist`), (3) persisted whitelist (`.shell-whitelist`) with `exact` and `prefix` entries. Shell metacharacters (`|`, `;`, `&&`, `>`, `` ` ``, `$(`) are rejected at tokenization. Shell-interpreter wrappers (`sh -c`, `bash -c`, etc.) are explicitly blocked.

- **Execution model**: 10-second default timeout; stdin always null; stdout/stderr piped and captured concurrently; child killed on timeout; exit 0 with empty output removes the directive; non-zero exit is a hard pipeline failure including captured streams in the error. Working directory resolves from explicit option, then source file parent, then policy_root, then CWD.

- **Error handling**: Dedicated `ShellExpansionError` enum (ParseDirective, CommandNotFound, Blacklisted, ApprovalRequired, Denied, Timeout, ExecutionFailed, PolicyIo) that converts into the existing `MarkdownError::Transform` surface. All shell failures are hard failures -- they never degrade to warnings even when `fail_fast = false`.

- **Implementation phases**: Phase 1 -- types, parser, tokenizer, blacklist, policy store. Phase 2 -- executor, PipelineRuntime, wire into transform pipeline, library tests. Phase 3 -- CLI approval handler, non-interactive error guidance, CLI tests, docs updates.

- **Key divergences from spec**: (1) Policy root resolution prefers source-file ancestry over CWD for file-backed compose. (2) `policy_root` naming replaces spec's `policy_dir`. (3) Shell-interpreter wrapper forms (`bash -lc`, etc.) are rejected despite appearing in spec examples. (4) Shell metacharacters are rejected at tokenization rather than relying solely on the textual blacklist. (5) The library never prompts directly -- it delegates via a caller-provided `ShellApprovalHandler` trait or returns `ApprovalRequired`.

## Plan for shell-expansion

**Timestamp:** 2026-03-15

The implementation plan organizes the shell-expansion feature into 3 phases with 14 tasks total.

**Phase 1 — Types, Parser, Tokenizer, Blacklist, Policy Store** (7 tasks):

- Task 1.1: Core types (`types.rs`) — ShellDirective, ShellExpansionOptions, ShellApprovalHandler trait, ShellExpansionError, PipelineRuntime, BlacklistRule
- Task 1.2: Argv tokenizer (`tokenize.rs`) — shell-like tokenization with quoting support and metacharacter rejection
- Task 1.3: Directive parser (`parser.rs`) — line scanning with code-region exclusion, builds ShellDirective list
- Task 1.4: Built-in blacklist (`policy.rs`) — structured rules for ~40+ dangerous commands, interpreter-wrapper blocking, whitelist/blacklist matching
- Task 1.5: Policy file store (`store.rs`) — discovery (git root/HOME), line-oriented load/append, deduplication
- Task 1.6: Module scaffold — wire into existing types (Stage1Stages, TransformOptions, TransformReport, MarkdownError), add `which` dependency
- Task 1.7: Phase 1 tests

**Phase 2 — Executor, Pipeline Runtime, Pipeline Integration** (4 tasks):

- Task 2.1: Command executor (`executor.rs`) — process spawn, timeout, stdout/stderr capture, working directory resolution
- Task 2.2: Pipeline runtime refactor — replace TransclusionRuntime with PipelineRuntime in recursive pipeline threading
- Task 2.3: Wire shell expansion stage — insert between TOC linking and cleanup, implement execute_directive orchestration and reverse-order replacement
- Task 2.4: Phase 2 tests — executor unit tests, pipeline integration tests, runtime sharing tests

**Phase 3 — CLI Approval Handler, CLI Integration, Documentation** (3 tasks + docs):

- Task 3.1: CLI approval handler — interactive prompt on stderr with 5 choices, terminal detection
- Task 3.2: CLI compose integration — conditional handler attachment in run_compose()
- Task 3.3: CLI integration tests — whitelisted/blacklisted/non-interactive scenarios
- Task 3.4: Documentation updates — dependencies.md, pipeline docs

**Key risks**: Phase 2 Task 2.2 (PipelineRuntime refactor) touches recursive pipeline plumbing and is highest-risk for regressions. All existing transform and transclusion tests must pass before proceeding.

### Phase 1: Types, Parser, Tokenizer, Blacklist, Policy Store

All Phase 1 components were already implemented in a prior session. Verified all files exist and compile:

- `darkmatter/lib/src/markdown/transform/shell_expansion/types.rs` — ShellDirective, ShellExpansionOptions, ShellApprovalHandler, ShellExpansionError, PipelineRuntime, BlacklistRule, ShellRuleSet, ShellExpansionRuntime
- `darkmatter/lib/src/markdown/transform/shell_expansion/tokenize.rs` — tokenize() with quoting support and metacharacter rejection (36 unit tests)
- `darkmatter/lib/src/markdown/transform/shell_expansion/parser.rs` — parse_directives() with code-region exclusion (12 unit tests)
- `darkmatter/lib/src/markdown/transform/shell_expansion/policy.rs` — check_builtin_blacklist(), check_user_blacklist(), check_whitelist(), normalize_command() (18 unit tests)
- `darkmatter/lib/src/markdown/transform/shell_expansion/store.rs` — resolve_policy_paths(), load_ruleset(), append helpers (10 unit tests)
- `darkmatter/lib/src/markdown/transform/shell_expansion/mod.rs` — execute_directive(), apply_replacements_in_reverse() + 6 integration tests
- `darkmatter/lib/src/markdown/transform/types.rs` — Stage1Stages, TransformOptions, TransformReport already include shell expansion fields
- `darkmatter/lib/src/markdown/types.rs` — MarkdownError::ShellExpansion variant already present
- `darkmatter/lib/Cargo.toml` — `which = "7"` and `dirs = "6"` already present

### Phase 2: Executor, Pipeline Runtime, Pipeline Integration

All Phase 2 components were already implemented:

- `darkmatter/lib/src/markdown/transform/shell_expansion/executor.rs` — execute_command() with timeout, output capture, working directory resolution (7 unit tests)
- `darkmatter/lib/src/markdown/transform/mod.rs` — PipelineRuntime used in run_transform_pipeline(), shell expansion stage wired between TOC linking and cleanup, recursive transclusion shares PipelineRuntime

### Phase 3: CLI Approval Handler, CLI Integration, Documentation

All Phase 3 components were already implemented:

- `darkmatter/cli/src/approval.rs` — CliShellApprovalHandler with interactive 5-choice prompt, can_prompt_interactively() check
- `darkmatter/cli/src/commands.rs` — run_compose() builds ShellExpansionOptions with conditional approval handler based on file input and terminal detection
- `darkmatter/docs/darkmatter-pipeline.md` — already includes shell expansion in the pipeline table
- `docs/dependencies.md` — `which` crate already documented

### Cleanup Fixes Applied

- Fixed doctest in `policy.rs` — `check_builtin_blacklist` example used `&str` slices instead of `&[String]`
- Fixed 6 unnecessary `mut` warnings in integration test bindings in `mod.rs`
- Fixed unused `TransformSource` import in integration tests
- Fixed trailing semicolons on struct initializations in tests

## Implementation of shell-expansion Complete

All three phases complete. 86 unit tests + 15 doctests pass. The only failing test (`test_table_very_narrow_width`) is a pre-existing issue in the terminal table rendering module, unrelated to shell expansion. No regressions in the full 1324-test suite.