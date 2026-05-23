---
phases: 4
created: 2026-05-23
start_phase: 1
---

# Execution Plan: Context Subcommand

## Overview

Add a `claudine context` subcommand that provides a full overview of Darkmatter's runtime context variables, expression engine, and side effects. The command renders structured terminal output using `biscuit-terminal` components.

## Dependencies

- `biscuit-terminal` (tables, prose, rendering)
- `darkmatter` (context capture API)
- `sniff` (git repo detection)
- `clap` (CLI argument parsing)

---

## Phase 1: CLI Wiring and Command Skeleton

**Goal:** Register the new subcommand and route it through the CLI parser.

- [ ] Add `Context(ContextArgs)` variant to `Commands` enum in `claudine/cli/src/args.rs`
- [ ] Define `ContextArgs` struct with `--values`, `--expressions`, and `--side-effects` boolean flags
- [ ] Add `pub mod context;` to `claudine/cli/src/commands/mod.rs`
- [ ] Create `claudine/cli/src/commands/context.rs` with a stub `run` function that prints the selected mode
- [ ] Wire the command in `claudine/cli/src/main.rs` (add to the non-wrapper command match arm)

**Validation Checkpoint:**
- [ ] `cargo build -p claudine-cli` succeeds
- [ ] `claudine context --help` displays the new subcommand and its flags

---

## Phase 2: Context Variable Reporting (`claudine context` and `--values`)

**Goal:** Implement the default context variable report and the `--values` variant.

**Parallelizable:** Phases 2a and 2b can be worked on in parallel by different implementers, but 2b depends on 2a's data structures.

### Phase 2a: Data Model and Parsing

- [ ] Create a `ContextVariable` struct holding `property`, `type_name`, and `description`
- [ ] Create a `ContextSection` struct holding a heading name and a list of `ContextVariable` entries
- [ ] Implement a parser that reads `darkmatter/docs/topics/context-variables.md` and extracts:
  - H3 headings as section titles
  - H4 headings as subsection titles (or falls back to H3 when no H4 exists)
  - Table rows under each heading as `ContextVariable` instances

**Validation Checkpoint:**
- [ ] Unit test: parse the context-variables.md file and assert the expected number of sections and variables are discovered

### Phase 2b: Report Rendering

- [ ] Implement `render_default_report(sections: &[ContextSection])`:
  - Render each section heading with `Prose`
  - Render each subsection heading with `Prose`
  - Render a `Table` with columns: Property, Type, Description
- [ ] Implement `render_values_report(sections: &[ContextSection])`:
  - Same structure as default report but with columns: Property, Type, Value
  - Integrate with `darkmatter::markdown::compose::context::capture` (or equivalent public API) to resolve live values for each variable
  - Handle `null` values gracefully (render as `<dim>null</dim>`)
- [ ] Add the two required `StatusInfo::Info` stderr messages at the bottom of every report path:
  - "use `--expressions` to see the expression engine's operations and functions"
  - "use `--side-effects` to see the available safe side effects that Claudine provides without need for being white listed."

**Validation Checkpoint:**
- [ ] Run `claudine context` and visually verify section headings, tables, and footer messages
- [ ] Run `claudine context --values` and verify live values appear in the Value column
- [ ] Run `claudine context --plain` and verify plain output aligns correctly

---

## Phase 3: Expressions Reporting (`--expressions`)

**Goal:** Implement the `--expressions` flag that renders the expression engine reference.

- [ ] Implement a parser that reads `darkmatter/docs/topics/darkmatter-expressions.md` and extracts:
  - H2/H3 headings for sections (e.g., "Operators", "Functions")
  - H4 headings or bold text for subsections
  - Tables and code blocks for operator/function details
- [ ] Implement `render_expressions_report()` that produces a concise, well-structured terminal output:
  - Use `Prose` for headings and descriptions
  - Use `Table` for operator precedence, truthiness rules, and function listings
  - Keep verbosity low; this is a reference, not a tutorial
- [ ] Add the two required `StatusInfo::Info` stderr footer messages

**Validation Checkpoint:**
- [ ] Run `claudine context --expressions` and verify the report is structured, readable, and includes operator tables and function listings
- [ ] Verify footer messages appear on stderr

---

## Phase 4: Side Effects Stub (`--side-effects`)

**Goal:** Implement the `--side-effects` flag as a placeholder.

- [ ] Implement `render_side_effects_report()` that outputs a single line: "not implemented yet"
- [ ] Add the two required `StatusInfo::Info` stderr footer messages (even though one refers to itself)

**Validation Checkpoint:**
- [ ] Run `claudine context --side-effects` and verify "not implemented yet" is printed
- [ ] Verify footer messages appear on stderr

---

## Final Integration and Validation

**Goal:** Ensure the complete feature is polished and tested.

- [ ] Run `cargo clippy -p claudine-cli` and resolve all warnings
- [ ] Run `cargo test -p claudine-cli` and ensure all existing tests pass
- [ ] Add integration tests for the `context` command:
  - Default invocation exits 0 and produces non-empty stdout
  - `--values` invocation exits 0 and produces non-empty stdout
  - `--expressions` invocation exits 0 and produces non-empty stdout
  - `--side-effects` invocation exits 0 and produces non-empty stdout
  - Footer messages are written to stderr in all modes
- [ ] Update `claudine/cli/README.md` or help text if there is a command listing

**Validation Checkpoint:**
- [ ] All integration tests pass
- [ ] Manual end-to-end test: `claudine context`, `claudine context --values`, `claudine context --expressions`, `claudine context --side-effects`
- [ ] Confirm terminal width changes are handled gracefully (tables wrap or truncate appropriately)
