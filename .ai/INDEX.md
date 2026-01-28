# biscuit-terminal Implementation: Documentation Index

## Quick Navigation

**Start Here**: [`EXEC_SUMMARY.md`](./EXEC_SUMMARY.md) - 2-min overview of entire project

---

## Documentation Files

### Executive & Planning
1. **[EXEC_SUMMARY.md](./EXEC_SUMMARY.md)** ⭐ START HERE
   - Overview of entire project
   - Subagent assignments
   - Phase schedule
   - Success criteria
   - **Read time**: 5 minutes

2. **[subagent-recommendations.biscuit-terminal.md](./subagent-recommendations.biscuit-terminal.md)**
   - Comprehensive analysis of all work required
   - Current state assessment
   - Complete phase breakdown with deliverables
   - Risk mitigation strategies
   - Code quality standards
   - **Read time**: 30 minutes (reference document)

### Subagent Briefings

3. **[phase-1-library-briefing.md](./phase-1-library-briefing.md)** - For Subagent 1
   - Detailed library implementation requirements
   - Line-by-line code examples
   - Testing patterns
   - Implementation strategies for each function
   - **Read time**: 45 minutes (working document)

4. **[phase-2-cli-briefing.md](./phase-2-cli-briefing.md)** - For Subagent 2
   - *To be created by project lead after Phase 1.3 complete*
   - Clap argument structure design
   - Output formatter implementation
   - Integration testing approach

5. **[phase-3-testing-briefing.md](./phase-3-testing-briefing.md)** - For Subagent 3
   - *To be created by project lead after Phase 2.1 complete*
   - Unit test strategy and patterns
   - Integration test design
   - Test isolation and mocking

---

## Key Concepts & Decisions

### ANSI Escape Code Detection
- **Approach**: Simple byte scanning for ESC (0x1b) character
- **Rationale**: Minimal dependencies, sufficient accuracy
- **Variants detected**: CSI sequences (`[...m`), OSC sequences (`]...ST`)
- **Reference**: detection.rs lines 238-270 (existing terminal app detection)

### Line Width Calculation
- **Approach**: Use `unicode-width` crate after stripping ANSI codes
- **Handles**: Fullwidth characters (emoji, CJK), zero-width combining marks
- **Output**: Vec<u16> with width for each newline-separated line
- **Dependency**: `unicode-width = "0.1"` (add to Cargo.toml)

### Color Mode Detection
- **Approach**: termbg crate + COLORFGBG fallback + luminance calculation
- **Method**: Query OSC 11 for background RGB, calculate relative luminance
- **Threshold**: L > 0.5 = Light, L <= 0.5 = Dark
- **Timeout**: 100ms to prevent hanging
- **Dependency**: `termbg = "0.5"` (add to Cargo.toml)

### Public API Design
- **Aggregation**: TerminalMetadata struct collecting all detection results
- **Serialization**: Serde support for JSON output in CLI
- **Convenience**: Prelude module for `use biscuit_terminal::prelude::*;`
- **Pattern**: Follows queue-lib TerminalDetector design

### CLI Architecture
- **Framework**: clap with derive macros
- **Main command**: `terminal meta` (or default)
- **Formats**: human-readable (colored), JSON, table
- **Subcommands**: individual metadata queries (color-depth, app, etc.)
- **Output**: human-readable by default, disable colors in non-TTY

---

## Project Structure

```
biscuit-terminal/
├── lib/                              # Library crate
│   ├── src/
│   │   ├── lib.rs                   # ⬅️ Exports & prelude
│   │   ├── discovery/
│   │   │   ├── mod.rs               # ⬅️ Module re-exports
│   │   │   ├── detection.rs         # 👉 Enhance color_mode()
│   │   │   ├── eval.rs              # 👉 Implement 3 functions
│   │   │   └── metadata.rs          # ⬅️ Create new file
│   │   ├── components/
│   │   ├── terminal.rs
│   │   └── utils/
│   ├── Cargo.toml                   # ⬅️ Add unicode-width, termbg
│   └── README.md                    # ⬅️ Update with examples
│
├── cli/                              # CLI crate
│   ├── src/
│   │   └── main.rs                  # 👉 Implement full CLI
│   ├── Cargo.toml
│   └── README.md                    # ⬅️ Add usage guide
│
└── README.md                         # ⬅️ Update overview

Legend:
👉 = Significant new implementation
⬅️ = Modifications/additions to existing files
```

---

## Phase Dependencies

```
PHASE 1 (Subagent 1: Library Development - 2-3 days)
├─ 1.1: Implement eval module (line_widths, has_escape_codes, has_osc8_link)
├─ 1.2: Enhance color_mode detection (COLORFGBG + termbg)
└─ 1.3: Create TerminalMetadata & update exports ← UNBLOCKS PHASES 2 & 3
    │
    ├─→ PHASE 2 (Subagent 2: CLI Implementation - 1.5-2 days)
    │   ├─ 2.1: Design clap argument structure
    │   ├─ 2.2: Implement metadata output formatters
    │   └─ 2.3: Integration & error handling ← UNBLOCKS PHASE 3.2
    │       │
    │       └─→ PHASE 3.2 (Subagent 3: CLI Integration Tests - 0.75d)
    │
    └─→ PHASE 3.1 (Subagent 3: Library Unit Tests - 1d)
        │
        └─→ PHASE 3.3 (Subagent 4: Documentation - 0.5d)
            Starts after Phases 2.3 & 3.2 complete

Timeline: 5-7 days total with parallel execution
```

---

## Success Checkpoints

### After Phase 1.3 Complete
- [ ] `cargo check -p biscuit-terminal` passes
- [ ] `cargo test -p biscuit-terminal` passes
- [ ] `cargo clippy -p biscuit-terminal` has no warnings
- [ ] TerminalMetadata::detect() returns all fields
- [ ] Library can be imported from other crates

### After Phase 2.3 Complete
- [ ] `terminal meta` command runs without errors
- [ ] `terminal meta --format json` produces valid JSON
- [ ] All subcommands work: color-depth, app, multiplex, eval, etc.
- [ ] Help text is clear and accurate
- [ ] Non-TTY output disables colors

### After Phase 3.2 Complete
- [ ] Library test coverage >90%
- [ ] CLI integration tests pass all subcommands
- [ ] Tests run in parallel without interference
- [ ] Cross-platform validation (macOS, Linux, Windows Terminal)

### After Phase 3.3 Complete
- [ ] All README files updated with examples
- [ ] Rustdoc examples compile: `cargo test --doc`
- [ ] Public API documented completely
- [ ] Usage patterns clear from documentation

---

## Common Tasks Reference

### Run Tests
```bash
# Library tests
cargo test -p biscuit-terminal

# CLI tests
cargo test -p biscuit-terminal-cli

# All tests with verbose output
cargo test --workspace -- --nocapture

# Run with coverage (requires tarpaulin)
cargo tarpaulin -p biscuit-terminal --out Html
```

### Build & Install
```bash
# Build library
cargo build -p biscuit-terminal

# Build CLI
cargo build -p biscuit-terminal-cli

# Install CLI binary
cargo install --path biscuit-terminal/cli

# Run CLI in debug mode
cargo run -p biscuit-terminal-cli -- meta
```

### Code Quality
```bash
# Check for errors
cargo check -p biscuit-terminal

# Lint warnings
cargo clippy -p biscuit-terminal

# Format code
cargo fmt -p biscuit-terminal

# Doc tests
cargo test --doc -p biscuit-terminal
```

### Testing with Environment Variables
```bash
# Run tests with output
RUST_LOG=debug cargo test -p biscuit-terminal -- --nocapture

# Run serial tests only (for env var isolation)
cargo test -p biscuit-terminal -- --test-threads=1
```

---

## Dependencies Overview

### Current (Already in Cargo.toml)
- `serde` 1.0.228 - Serialization/deserialization
- `serde_json` 1.0 - JSON support
- `terminal_size` 0.4.3 - Get terminal dimensions
- `termini` 1.0 - Terminfo database access
- `tracing` 0.1 - Structured logging
- `clap` 4.5 - CLI argument parsing (lib + cli)

### New (To Add)
- `unicode-width` 0.1 - Calculate visual width of text
- `termbg` 0.5 - Safe OSC 11 terminal color query

### Test Dependencies (Already in Cargo.toml)
- `assert_cmd` 2 - CLI integration testing
- `predicates` 3 - Assertion predicates

---

## Technical Standards (from CLAUDE.md)

### Rust Documentation
- ❌ NO explicit H1 (`# `) in rustdoc blocks (item name auto-added)
- ✅ Use H2 (`## `) for sections: Examples, Returns, Errors, Panics, Notes
- ✅ Use H3 (`### `) only for subsections
- ✅ Examples should have `no_run` if they can't compile standalone

### Error Handling
- ❌ No `unwrap()` or `expect()` in production code
- ✅ All public functions return `Result` where applicable
- ✅ Use `thiserror` for custom error types (if creating)

### Testing
- ✅ Both runtime behavior AND type tests (if complex types)
- ✅ Use `describe`/`it` pattern for test groups
- ✅ Isolation required for environment-variable-dependent tests
- ✅ Use Mutex pattern from queue-lib for env var tests

### Tracing
- ✅ Libraries emit events, never configure subscribers
- ✅ Use structured fields: `tool.name`, `tool.duration_ms`
- ✅ Skip sensitive data: `#[tracing::instrument(skip(secret))]`

---

## Reference Implementations

### queue-lib Terminal Detection
**File**: `/Volumes/coding/personal/dockhand/queue/lib/src/terminal.rs` (826 lines)
**Use for**:
- Terminal detection pattern (TerminalKind enum, TerminalDetector struct)
- Test isolation with Mutex + env var helpers
- Comprehensive test coverage (70+ tests)
- Capabilities struct design

### Local Skills in .claude/skills/
- **rust/** - General Rust best practices
- **terminal/** - Terminal-specific knowledge
- **clap/** - Argument parsing patterns
- **rust-testing/** - Testing patterns and strategies
- **dockhand-library/** - Project-specific patterns

---

## Quick Start for Subagents

### For Subagent 1 (Library Specialist)
1. Read [EXEC_SUMMARY.md](./EXEC_SUMMARY.md) - 5 min overview
2. Read [phase-1-library-briefing.md](./phase-1-library-briefing.md) - 45 min detailed tasks
3. Start with Phase 1.1: eval module implementation
4. Use `.claude/skills/rust/` for best practices
5. Reference queue-lib for patterns

### For Subagent 2 (CLI Specialist)
1. Wait for Phase 1.3 completion from Subagent 1
2. Read [EXEC_SUMMARY.md](./EXEC_SUMMARY.md) - context overview
3. Review [phase-2-cli-briefing.md](./phase-2-cli-briefing.md) when available
4. Use `.claude/skills/clap/` for argument parsing
5. Reference existing CLI in research-cli or sniff-cli

### For Subagent 3 (Testing Specialist)
1. Read [EXEC_SUMMARY.md](./EXEC_SUMMARY.md) - project overview
2. Can start Phase 3.1 after Subagent 1 Phase 1.3 complete
3. Review [phase-3-testing-briefing.md](./phase-3-testing-briefing.md) when available
4. Use queue-lib as test pattern reference
5. Leverage `.claude/skills/rust-testing/` and nextest

### For Subagent 4 (Documentation Specialist)
1. Can start Phase 3.3 after Phase 2.3 complete
2. Review [EXEC_SUMMARY.md](./EXEC_SUMMARY.md) for context
3. Use CLAUDE.md standards for documentation
4. Reference existing .claude/skills/ documentation patterns

---

## Escalation Contacts

- **Project Lead**: Ken Snyder
- **Library Lead** (Subagent 1): [Assign]
- **CLI Lead** (Subagent 2): [Assign after Phase 1.3]
- **Testing Lead** (Subagent 3): [Assign after Phase 1.3]
- **Docs Lead** (Subagent 4): [Assign after Phase 2.3]

**Blockers should be raised immediately to project lead**

---

## File Manifest

All documentation created for this project:

| File | Purpose | Audience | Read Time |
|------|---------|----------|-----------|
| EXEC_SUMMARY.md | Project overview, phase schedule, assignments | All | 5 min |
| INDEX.md | This file - navigation and quick reference | All | 10 min |
| subagent-recommendations.biscuit-terminal.md | Complete technical analysis | Project lead | 30 min |
| phase-1-library-briefing.md | Subagent 1 detailed requirements | Subagent 1 | 45 min |
| phase-2-cli-briefing.md | Subagent 2 requirements (TBD) | Subagent 2 | 45 min |
| phase-3-testing-briefing.md | Subagent 3 requirements (TBD) | Subagent 3 | 45 min |

---

## Last Updated

- **Created**: 2026-01-27
- **Status**: Ready for Phase 1 execution
- **Next Update**: After Phase 1.3 completion

---

**Start execution with Subagent 1 reading [phase-1-library-briefing.md](./phase-1-library-briefing.md)**

