---
phases: 6
created: 2026-04-28
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/block_pairs.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/compose/perf.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/body.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/render.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
  - darkmatter/lib/src/markdown/types.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages:
  - darkmatter
---

# Shell Blocks Implementation Plan

Generated from:
- Functional Specification: `darkmatter/features/_unscheduled/shell-block/spec.md`
- Technical Design: `darkmatter/features/_unscheduled/shell-block/tech-design.md`

---

## Phase 1: Foundation & Shared Infrastructure

**Goal**: Lay the groundwork shared by page blocks and shell blocks, and update compose metadata.

### Step 1.1 — Add `block_pairs.rs` shared scanner
- Create `darkmatter/lib/src/markdown/compose/block_pairs.rs`
- Define `BlockOpenKind { Page, Shell }` and `BlockPair` types
- Implement linear scanner that:
  - Skips fenced code regions via `parse_utils::find_code_regions()`
  - Pushes `::block` and `::shell-block` openers onto one stack
  - Pops on every `::end-block`
  - Validates `::end-block` has no trailing content
- Returns ordered `Vec<BlockPair>` with spans and source locations

### Step 1.2 — Update `ComposeOperation` enum and metadata
- Add `ShellBlocks` variant to `ComposeOperation` in compose mod
- Increment `ComposeOperation::COUNT`
- Assign stable index after `ShellExpansion`
- Include in `default_order()`
- Add `Display` text `ShellBlocks`
- Add `PerfMetricKind::ShellBlocks`
- Add `ComposeReport::shell_blocks_applied`

### Step 1.3 — Add `MarkdownError::ShellBlock` variant
- Add `ShellBlock(#[from] ShellBlockError)` to `MarkdownError`
- Ensure it delegates status-block rendering like `ShellExpansion` and `PageBlock`

**Checkpoint**: `cargo check` passes with new types wired but no shell block implementation yet.

---

## Phase 2: Shell Block Core Module

**Goal**: Build the `shell_blocks/` module end-to-end, independently runnable in unit tests.

### Step 2.1 — Define shell block types (`types.rs`)
- `ShellBlockRegion`: span, body_span, start/end lines, `ErrorHandling`, timeout, `SourceExcerpt`
- `ShellBlockCommand`: raw_command, executable, args, physical_span, start/end lines
- `ShellBlockError` enum with `Parse`, `Unterminated`, `Command` variants
- `SourceExcerpt`: compact owned structure with surrounding body lines
- Implement `biscuit_terminal::errors::BlockError` for `ShellBlockError`

### Step 2.2 — Implement parameter parsing (`parser.rs`)
- Parse `::shell-block <params>` line using `parse_utils::Cursor`
- Map key-value options to `ErrorHandling` fields:
  - `when_error="text"` → `when_error`
  - `when_exit_code="N,text"` → `when_exit_code.push((N, text))`
  - `except_exit_code="N,text"` → `except_exit_code.push((N, text))`
  - `stderr_contains="find,text"` → `stderr_contains.push((find, text))`
  - `stderr_lacks="find,text"` → `stderr_lacks.push((find, text))`
  - `enrich_error="text"` → `enrich_error`
  - `enrich_error_on="N,text"` → `enrich_error_on.push((N, text))`
  - `timeout=N` → `timeout_override`
- Return targeted parse errors for wrong-style options (`--when-error`, `--when-exit-code`, `::timeout:N`, `when-error`) with hints
- Treat unknown key-value options as hard parse errors

### Step 2.3 — Implement body splitting (`body.rs`)
- `split_logical_commands(body: &str) -> Vec<ShellBlockCommand>`
- Rules:
  - Ignore blank physical lines
  - Treat non-empty line as new logical command
  - Trailing unescaped `\` joins with next non-blank line (one space separator)
  - Preserve start/end lines per logical command
  - Tokenize each logical command via `shell_expansion::tokenize::tokenize`
- Reject pipes, chaining, redirects, command substitution via existing tokenizer

### Step 2.4 — Implement output rendering (`render.rs`)
- `render_block_output(results: &[ShellBlockCommandResult]) -> String`
- Contract:
  - Trim each command's combined output
  - Drop empty command outputs entirely
  - Append one newline after each non-empty command output
  - Insert one blank line between non-empty command outputs
  - Return empty string if all outputs empty

### Step 2.5 — Implement orchestration entrypoint (`mod.rs`)
- `run_shell_blocks_stage(content: &str, ...) -> Result<(String, ComposeReport), MarkdownError>`
- Flow:
  1. Parse shell-block regions from current content
  2. Resolve policy paths and load runtime policy once if any block exists
  3. Prepare every logical command in a block before executing the first one
  4. Execute prepared commands sequentially
  5. Render replacement per block
  6. Apply replacements in reverse span order
  7. Merge warnings and approval counts into `ComposeReport`

**Checkpoint**: All `shell_blocks/` unit tests pass (parser, body splitting, rendering).

---

## Phase 3: Integration Points

**Goal**: Wire shell blocks into the existing compose pipeline, discovery, and execution machinery.

### Step 3.1 — Add `Markdown::run_shell_blocks_stage`
- Place beside `run_shell_expansion_stage`
- Call `shell_blocks::run_shell_blocks_stage`
- Integrate with `ShellExpansionRuntime` and `PipelineRuntime`

### Step 3.2 — Add `ShellCommandOrigin::ShellBlock` variant
- Extend `ShellCommandOrigin` with `ShellBlock { start_line: usize, command_line: usize }`
- Update approval prompts and diagnostics to distinguish `::shell` from shell-block commands

### Step 3.3 — Update `shell_expansion::discovery`
- Extend `collect_shell_commands` to include shell-block logical commands
- Run same pre-execution compose subset plus shared block scanner
- Exclude false page blocks
- Produce `ShellCommandEntry` for each logical command with:
  - `raw_command`: folded logical command
  - `executable` and `args`: after tokenization and alias passthrough
  - `normalized`: existing `normalize_command`
  - `source_file`: same source attribution
  - `origin`: `ShellCommandOrigin::ShellBlock { ... }`

### Step 3.4 — Integrate `block_pairs.rs` with `page_blocks::parser`
- Refactor `page_blocks::parser` to use shared scanner or share opener/closer tokenization
- Maintain existing region-tree builder if wrapping
- Ensure one stack for both opener kinds

**Checkpoint**: Compose pipeline runs successfully; `cargo test` for compose module passes.

---

## Phase 4: Error Handling & Edge Cases

**Goal**: Implement spec-compliant error presentation and resilient edge-case behavior.

### Step 4.1 — Implement `ShellBlockError` rendering
- Error display includes:
  - Error class and short title
  - Source file when available
  - Block opener line and command line
  - Code excerpt with failing logical command highlighted
  - Existing shell-expansion details (command not found, denial, timeout, execution failure)
  - Hint when author likely used `::shell` flag syntax instead of key-value syntax

### Step 4.2 — Implement partial output demotion on unhandled failure
- When command fails with no matching handler:
  - Preserve output from already-succeeded commands
  - Visually demote partial output (dimmed, commented, etc.)
  - Present error after demoted partial output
- Implement in `BlockError` renderer rather than mutating replacement text

### Step 4.3 — Handle all edge cases
- Unterminated shell block
- Unmatched `::end-block`
- Nested `::block` containing `::shell-block`
- Nested `::shell-block` inside false page block
- Empty block body
- All commands produce empty output
- Mixed empty and non-empty command outputs
- Continuation lines with escaped backslash (`\\`)

**Checkpoint**: Error rendering tests pass; edge case tests pass.

---

## Phase 5: Testing

**Goal**: Achieve high confidence through comprehensive tests.

### Step 5.1 — Parser tests
- Simple one-command block
- Multiple sibling blocks
- Body continuation folding
- Blank line ignoring
- Rejected pipes, chaining, redirects, command substitution
- Nested `::block` containing `::shell-block`
- Nested `::shell-block` inside false page block
- Unmatched and unterminated `::end-block`
- Wrong-style parameter hints

### Step 5.2 — Execution tests
- All commands approved before first execution
- Denial of command 2 prevents command 1 execution in same block
- Successful multiple commands render with exactly one blank line between non-empty outputs
- Empty command output omitted without extra separators
- `when_error` fallback applies only to failing command; subsequent commands still run
- Unhandled failure preserves earlier outputs in `ShellBlockError`
- Timeout override on block applies to every command in block
- Multiple commands with mixed empty/non-empty outputs

### Step 5.3 — Discovery tests
- Shell-block commands appear in `collect_shell_commands`
- Discovery preserves source file and origin
- False page blocks exclude nested shell-block commands
- Transcluded documents contribute shell-block commands once conditions pass

### Step 5.4 — Integration tests
- End-to-end compose with shell blocks
- CLI integration (light, since approval handler unchanged)
- Performance: verify linear parsing, no excessive allocations

**Checkpoint**: Test coverage for shell_blocks/ module >90%; all tests pass.

---

## Phase 6: Documentation & Final Validation

**Goal**: Update docs and perform final validation.

### Step 6.1 — Update documentation
- Create/update `darkmatter/docs/inline/shell-blocks.md` or extend `shell-expansion.md`
- Update `darkmatter/docs/darkmatter-compose-pipeline.md` for new Inline Pre operation
- Update `darkmatter/README.md` if public examples mention shell expansion
- Update `.claude/skills/darkmatter/SKILL.md` to include Shell Blocks

### Step 6.2 — Final validation
- Run full test suite: `cargo test` for darkmatter
- Run linting: `cargo clippy` for darkmatter
- Verify no breaking changes to existing `::shell` behavior
- Verify no breaking changes to page blocks
- Check that all new public types have rustdoc

**Checkpoint**: All tests pass, lint passes, docs complete, ready for merge.

---

## Parallelizable Work

Within each phase, steps can often proceed in parallel when they touch different files:

- **Phase 1**: Steps 1.1, 1.2, and 1.3 are independent (different modules)
- **Phase 2**: Steps 2.1 (types) must come first; 2.2, 2.3, 2.4 can be developed in parallel once types exist; 2.5 (orchestration) comes last
- **Phase 3**: Steps 3.1 and 3.2 are independent; 3.3 and 3.4 can be done in parallel
- **Phase 4**: Steps 4.1, 4.2, 4.3 can be developed in parallel
- **Phase 5**: Test categories (5.1, 5.2, 5.3) can be written in parallel; 5.4 comes after
- **Phase 6**: Step 6.1 (docs) and 6.2 (validation) are sequential

## Dependency Graph (Simplified)

```
Phase 1: block_pairs.rs + ComposeOperation + MarkdownError
    |
Phase 2: types -> {parser, body, render} -> mod.rs
    |
Phase 3: run_shell_blocks_stage + discovery + page_blocks integration
    |
Phase 4: error rendering + partial output + edge cases
    |
Phase 5: comprehensive tests
    |
Phase 6: documentation + final validation
```

## Risk Areas

1. **Shared block scanner**: Must not break existing page block behavior. Extensive regression tests required.
2. **Approval discovery**: Flattening shell-block commands into existing discovery must preserve normalization and source attribution exactly.
3. **Error rendering**: Source excerpts must be accurate even after interpolation changes line numbers.
4. **Parameter syntax confusion**: Authors will mix `::shell` flags with `::shell-block` key-value syntax; error hints are critical.

## Estimated Effort

- Phase 1: 1–2 days
- Phase 2: 3–4 days
- Phase 3: 2–3 days
- Phase 4: 2–3 days
- Phase 5: 2–3 days
- Phase 6: 1–2 days

**Total: 11–17 days** (with parallel work, calendar time ~8–12 days)
