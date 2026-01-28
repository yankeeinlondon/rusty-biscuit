# Executive Summary: biscuit-terminal Discovery/Metadata Implementation

## Overview

**Project**: Implement first release of `biscuit-terminal` library and CLI with metadata/discovery features
**Scope**: Complete terminal capability detection, package as library, expose via CLI with `--meta` flag
**Timeline**: 5-7 days (3 phases)
**Team**: 4 specialized subagents
**Status**: Ready for execution

---

## What We're Building

### Library (`biscuit-terminal`)
- Complete terminal metadata detection covering:
  - Terminal application identification (WezTerm, iTerm2, etc.)
  - Color depth (8-bit, 24-bit, etc.)
  - Color mode (light/dark background)
  - Terminal dimensions
  - Image support (Kitty protocol)
  - OSC8 hyperlink support
  - Multiplexing capabilities (Tmux, Zellij, native)
  - Underline style support
  - Italic text support
  - TTY detection

- Public API aggregates all detection into single `TerminalMetadata` struct
- Helper functions for analyzing terminal content (escape codes, line widths)

### CLI (`biscuit-terminal-cli`)
- Binary: `terminal` (or `bt`)
- Main subcommand: `terminal meta` or `terminal --meta`
- Output formats: human-readable (default), JSON, table
- Subcommands for individual metadata queries
- Integration testing with assert_cmd

---

## Current State vs. Target State

### Library Status
| Component | Current | Target | Work |
|-----------|---------|--------|------|
| detection.rs | ~679 lines, mostly complete | All functions working | 1. Enhance color_mode() |
| eval.rs | 3 stub functions | Fully implemented | 2. Implement line_widths, has_escape_codes, has_osc8_link |
| metadata.rs | ❌ Does not exist | Public API struct | 3. Create TerminalMetadata aggregation |
| lib.rs | 5 lines, minimal | Full exports, prelude | 4. Add public exports |
| Cargo.toml | Basic deps | +unicode-width, termbg | 5. Add required deps |
| Unit tests | 0% | >90% coverage | 6. Comprehensive tests |

### CLI Status
| Component | Current | Target | Work |
|-----------|---------|--------|------|
| main.rs | "Hello, world!" | Full CLI | 1. Implement with clap |
| Subcommands | ❌ None | 8+ commands | 2. Add meta, color-depth, app, etc. |
| Output formats | ❌ None | JSON + human + table | 3. Implement formatters |
| Integration tests | ❌ None | Full coverage | 4. Test all subcommands |
| Documentation | Partial | Complete | 5. README, rustdoc |

---

## Subagent Assignments

### Subagent 1: Rust Library Specialist
**Skills**: rust, terminal, dockhand-library
**Assignments**: Phase 1 (Library Development) - Days 1-3
**Deliverables**:
- eval module with ANSI detection and line width calculation
- Enhanced color_mode detection
- TerminalMetadata struct and public API
- Unit tests for all detection functions

**Unblocks**: Subagents 2 & 3

### Subagent 2: CLI Specialist
**Skills**: clap, rust, terminal
**Assignments**: Phase 2 (CLI Development) - Days 2-4 (starts after Phase 1.3)
**Deliverables**:
- Clap argument structure with subcommands
- Meta output (human-readable, JSON, table formats)
- Eval subcommand for content analysis
- Integration tests

**Blocked by**: Subagent 1 Phase 1.3

### Subagent 3: Testing Specialist
**Skills**: rust-testing, nextest, rust
**Assignments**: Phase 3.1-3.2 (Testing) - Days 3-5 (overlaps with Phase 2)
**Deliverables**:
- Comprehensive unit tests for library (>90% coverage)
- Integration tests for CLI
- Cross-platform validation
- Test isolation patterns

**Blocked by**: Subagent 1 Phase 1.3 (can start immediately after)

### Subagent 4: Documentation Specialist
**Skills**: rust, terminal, technical-writing
**Assignments**: Phase 3.3 (Documentation) - Days 5-7 (final phase)
**Deliverables**:
- Updated README files (library, CLI, root)
- Rustdoc examples for all public items
- Usage guides and examples

**Blocked by**: Subagents 2 & 3

---

## Phase Schedule

### Phase 1: Library Development (2-3 days)
**Owner**: Subagent 1
**Tasks**:
- 1.1: Implement eval module (has_escape_codes, has_osc8_link, line_widths)
- 1.2: Enhance color_mode detection (COLORFGBG + termbg)
- 1.3: Create TerminalMetadata struct and public exports

**Deliverables**:
- Working library with full terminal detection
- ✅ `cargo check` passes
- ✅ `cargo test` passes
- ✅ All public items documented

**Unblocks**: Phases 2 & 3

---

### Phase 2: CLI Implementation (1.5-2 days)
**Owner**: Subagent 2
**Starts**: After Phase 1.3 complete
**Tasks**:
- 2.1: Design clap argument structure
- 2.2: Implement metadata output (human/JSON/table)
- 2.3: Integration and error handling

**Deliverables**:
- Working CLI with metadata output
- ✅ `terminal meta` command works
- ✅ `--format json` produces valid JSON
- ✅ All subcommands implemented

**Unblocks**: Phase 3.2

---

### Phase 3: Testing & Documentation (1.5-2 days)
**Owner**: Subagents 3 & 4 (parallel)
**Starts**: After Phase 1.3 (testing) / Phase 2.3 (docs)
**Tasks**:
- 3.1: Unit tests for library
- 3.2: Integration tests for CLI
- 3.3: Documentation updates

**Deliverables**:
- ✅ >90% test coverage
- ✅ All subcommands tested
- ✅ README files updated
- ✅ Rustdoc examples working

---

## Key Technical Decisions

### 1. ANSI Escape Code Detection
**Decision**: Simple byte scanning (no regex dependency)
**Rationale**: 
- Reduces dependencies
- Sufficient for OSC 8 and CSI detection
- Easy to understand and maintain

### 2. Line Width Calculation
**Decision**: Use `unicode-width` crate
**Rationale**:
- Handles fullwidth characters (CJK, emoji)
- Handles zero-width combining marks
- Well-tested industry standard

### 3. Color Mode Detection
**Decision**: Use `termbg` crate with timeout
**Rationale**:
- Safe OSC 11 querying with timeout
- Fallback to COLORFGBG env var
- Non-blocking approach

### 4. CLI Output Format
**Decision**: Support human-readable (default), JSON, and table
**Rationale**:
- Human-readable for terminal inspection
- JSON for scripting/automation
- Table for visual presentation
- All formats serializable for consistency

### 5. Public API Design
**Decision**: Aggregate all detection in `TerminalMetadata` struct
**Rationale**:
- Single point of reference for all capabilities
- Serializable for CLI output
- Easy to extend with new fields
- Follows queue-lib pattern

---

## Dependencies (New)

| Crate | Version | Purpose | Required |
|-------|---------|---------|----------|
| unicode-width | 0.1 | Line width calculation with fullwidth support | Yes |
| termbg | 0.5 | Safe OSC 11 terminal color query | Yes* |

*termbg can be optional if color_mode() detection is deferred

**Dependency Justification**:
- Both are lightweight, well-maintained crates
- unicode-width is already used by many terminal libraries
- termbg has minimal dependencies (only std::io, std::time)
- No conflicts with existing workspace dependencies

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| OSC 11 query timeout | Medium | 100ms latency per terminal creation | Use termbg with timeout, fallback gracefully |
| ANSI detection false negatives | Low | Some escape codes missed | Test with syntect output, LSP diagnostic sequences |
| Test environment variable conflicts | Medium | Tests interfere in parallel execution | Use Mutex pattern from queue-lib |
| Cross-platform color_mode differences | High | Different detection on macOS/Linux | Document limitations, test on multiple platforms |
| Unicode width calculation edge cases | Low | Emoji/CJK width miscalculation | Test with unicode-width test suite examples |

---

## Success Metrics

### Phase 1 Success
- ✅ All detection functions implemented
- ✅ `cargo test -p biscuit-terminal` passes
- ✅ No clippy warnings
- ✅ >90% test coverage

### Phase 2 Success
- ✅ CLI accepts `terminal meta` command
- ✅ JSON output is valid and complete
- ✅ Human-readable format is readable
- ✅ All subcommands work independently

### Phase 3 Success
- ✅ Integration tests pass for all CLI subcommands
- ✅ >90% overall test coverage
- ✅ README files are complete and accurate
- ✅ Rustdoc examples compile without errors

### Final Release Success
- ✅ All tests pass: `cargo test --workspace`
- ✅ Binaries built: `cargo install --path biscuit-terminal/cli`
- ✅ Works on macOS, Linux, Windows Terminal
- ✅ Consistent output across terminal emulators

---

## Estimated Effort

| Phase | Component | Estimate | Owner |
|-------|-----------|----------|-------|
| 1.1 | Eval module | 1-1.5d | Subagent 1 |
| 1.2 | Color mode detection | 1d | Subagent 1 |
| 1.3 | Exports & aggregation | 0.5-1d | Subagent 1 |
| 2.1 | CLI architecture | 0.75d | Subagent 2 |
| 2.2 | Metadata output | 1d | Subagent 2 |
| 2.3 | Integration | 0.5d | Subagent 2 |
| 3.1 | Library tests | 1d | Subagent 3 |
| 3.2 | CLI tests | 0.75d | Subagent 3 |
| 3.3 | Documentation | 0.5d | Subagent 4 |
| **Total** | **All** | **5-7 days** | **All** |

---

## Start Conditions

✅ All conditions met:
- Library has ~679 lines of detection code already
- Partial eval.rs stub functions in place
- Reference implementations exist (queue-lib, sniff-lib)
- Dependencies already exist in workspace
- CLI scaffolding ready (clap already in Cargo.toml)
- Testing infrastructure in place (assert_cmd, cargo test)
- Documentation template exists (README files)

**Ready to start immediately**

---

## Documentation Deliverables

### Library Documentation
- [ ] **discovery/mod.rs**: Module overview with examples
- [ ] **detection.rs**: Rustdoc for all detection functions
- [ ] **eval.rs**: Rustdoc with examples for ANSI analysis
- [ ] **metadata.rs**: TerminalMetadata struct documentation
- [ ] **lib/README.md**: Library usage guide with examples
- [ ] **Rustdoc examples**: `cargo test --doc` must pass

### CLI Documentation
- [ ] **cli/README.md**: CLI usage guide, subcommands, examples
- [ ] **main.rs**: Code comments for CLI structure
- [ ] **Help text**: Built into clap derive macros
- [ ] **Usage examples**: In CLI README

### Root Documentation
- [ ] **README.md**: Update with discovery/metadata sections
- [ ] **.claude/skills/**: Consider creating a biscuit-terminal skill

---

## Next Immediate Action

**Create subagent assignments and begin Phase 1**

1. Assign Subagent 1: Review `/Volumes/coding/personal/dockhand/.ai/phase-1-library-briefing.md`
2. Assign Subagent 2: Queue to review Phase 2 briefing (not yet created, Subagent 1 unblocks)
3. Assign Subagent 3: Queue to review Phase 3.1 briefing (can start once Subagent 1 completes Phase 1.3)
4. Assign Subagent 4: Queue to review Phase 3.3 briefing (starts after Phase 2 complete)

---

## Reference Documents

All detailed briefs are in `/Volumes/coding/personal/dockhand/.ai/`:

- **subagent-recommendations.biscuit-terminal.md** - Complete analysis and recommendations
- **phase-1-library-briefing.md** - Detailed Phase 1 requirements (created)
- **phase-2-cli-briefing.md** - Phase 2 requirements (to be created by lead after Phase 1 starts)
- **phase-3-testing-briefing.md** - Phase 3 requirements (to be created by lead after Phase 2 starts)

---

## Contact & Escalation

**Project Lead**: Ken Snyder (Project owner)
**Subagent 1 Lead**: Rust Library Specialist
**Subagent 2 Lead**: CLI Specialist
**Subagent 3 Lead**: Testing Specialist
**Subagent 4 Lead**: Documentation Specialist

**Daily Standup Items**:
- Phase completion progress
- Blockers or dependency issues
- Test coverage metrics
- Documentation completion

---

## Conclusion

This is a well-scoped, achievable project with:
- ✅ Clear phase breakdown
- ✅ Specialized subagent assignments
- ✅ Minimal dependencies (only 2 new crates)
- ✅ Strong reference implementations
- ✅ Reasonable timeline (5-7 days)
- ✅ High-quality standards per CLAUDE.md
- ✅ Comprehensive testing and documentation

**Recommendation**: Proceed with Phase 1 execution immediately. Subagent 1 can begin library development today.

