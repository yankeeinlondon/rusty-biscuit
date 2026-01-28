# Subagent Recommendations: biscuit-terminal Discovery/Metadata Implementation

## Executive Summary

Implementing the `biscuit-terminal` library and CLI discovery/metadata features requires coordinated work across three specialized domains: library implementation (Rust), CLI architecture (clap), and testing. This document recommends a three-phase approach with specialized subagents handling each phase.

**Timeline**: 3 phases over ~5-7 days with parallel execution where possible

---

## Current State Analysis

### Library Status
- **discovery/mod.rs**: Module scaffolding complete, public API defined
- **discovery/detection.rs**: ~679 lines of working code covering:
  - `color_depth()` - terminfo + COLORTERM detection (complete)
  - `color_mode()` - placeholder returning Dark only
  - `is_tty()` - using std::io::IsTerminal (complete)
  - `get_terminal_app()` - TERM_PROGRAM + TERM detection (complete)
  - `terminal_width/height()` - terminal_size crate (complete)
  - `dimensions()` - terminal_size crate (complete)
  - `image_support()` - Kitty protocol detection (complete)
  - `osc8_link_support()` - OSC8 standard detection (complete)
  - `multiplex_support()` - Tmux/Zellij/Native detection (complete)
  - `underline_support()` - Extended underline detection (complete)
  - `italics_support()` - Italic support detection (complete)
- **discovery/eval.rs**: Three stub functions requiring implementation:
  - `line_widths(content)` - strip escape codes, measure lines
  - `has_escape_codes(content)` - detect ANSI sequences
  - `has_osc8_link(content)` - detect OSC8 link sequences

### CLI Status
- **main.rs**: Placeholder "Hello, world!"
- **Cargo.toml**: Dependencies defined (clap 4.5 with derive feature, biscuit-terminal)
- **README.md**: Usage specification documented but not implemented

### Reference Implementations
- **queue/lib/src/terminal.rs**: 826 lines with comprehensive terminal detection
  - TerminalKind enum, TerminalCapabilities struct
  - TerminalDetector with 8 terminal types
  - Priority-based detection logic
  - 70+ unit tests with environment variable isolation
  - TUI layout setup and pane creation
- **sniff/lib**: Hardware/network/OS discovery (less relevant but shows pattern)

### Dependencies Available
- `clap` 4.5 with derive API (local skill available: `.claude/skills/clap`)
- `terminal_size` 0.4.3
- `termini` 1.0 (terminfo database)
- `serde`/`serde_json` for serialization
- `tracing` for instrumentation
- `assert_cmd` for CLI testing

---

## Phase Breakdown & Subagent Assignments

### Phase 1: Library Development (2-3 days)

**Primary Goals**:
1. Complete `eval` module for ANSI escape sequence detection
2. Improve `color_mode()` detection using termbg strategy
3. Create public metadata aggregation API
4. Add comprehensive unit tests

#### Phase 1.1: Eval Module Implementation
**Subagent**: **Rust Specialist** (rust skill)

**Deliverables**:
- `has_escape_codes()` - Detect ANSI sequences (CSI sequences, OSC sequences)
  - Pattern: `\x1b\[[0-9;]*m` (SGR), `\x1b\].*?[\x07\x1b\\]` (OSC)
  - Reference: syntect library uses regex for similar operations
- `has_osc8_link()` - Detect OSC 8 hyperlink sequences
  - Pattern: `\x1b]8;;(.*?)\x1b\\` (OSC 8 standard)
  - Must account for path/file components
- `line_widths()` - Calculate visual width of each line
  - Strip ANSI codes before measuring
  - Handle Unicode character widths (grapheme clusters vs. display width)
  - Use `unicode-width` or `textwrap` crate (need to evaluate dependencies)
  - Return Vec<u16> for each newline-separated line

**Key Considerations**:
- ANSI sequence detection must handle both CSI (8-bit) and OSC 8 variants
- Line width calculation needs to account for terminal rendering (some terminals support Mode 2027 for grapheme clustering)
- All functions should be marked `#[tracing::instrument]` for observability
- Tests should cover: empty strings, strings with codes, multi-line content

**Estimated Time**: 1-1.5 days
**Blocks**: Phase 1.3, Phase 2 (eval module used in CLI output formatting)

---

#### Phase 1.2: Color Mode Detection Enhancement
**Subagent**: **Rust Specialist** (rust skill, terminal skill)

**Deliverables**:
- Replace `color_mode()` placeholder with termbg-based detection
- Detection strategy (priority order):
  1. Query OSC 10/11 (foreground/background colors) via terminal
  2. Parse RGB values using `termbg` crate (if available, else manual RGB parsing)
  3. Calculate luminance using relative luminance formula: L = 0.2126*R + 0.7152*G + 0.0722*B
  4. Threshold: L > 0.5 = Light mode, L <= 0.5 = Dark mode
  5. Fallback: Check `COLORFGBG` environment variable (some terminals set this)
  6. Final fallback: Default to Dark

- Integration considerations:
  - OSC query requires synchronous I/O to terminal (potential deadlock risk)
  - Consider timeout (e.g., 100ms) to avoid hanging
  - Cache result since theme doesn't change during session
  - This function may need to be marked as potentially blocking

**Key Considerations**:
- termbg crate (0.5.x) provides safe RGB parsing with timeout
- OSC 11 queries background color specifically
- Must handle non-TTY contexts gracefully
- Test with both light and dark terminal themes

**Estimated Time**: 1 day
**Dependencies**: Phase 1.1 (completion), queue-lib example code
**Blocks**: Phase 1.3

---

#### Phase 1.3: Library Exports & Metadata Aggregation
**Subagent**: **Rust Specialist** (rust skill, dockhand-library skill)

**Deliverables**:
- Create a `TerminalMetadata` struct aggregating all detection results
  ```rust
  pub struct TerminalMetadata {
      pub app: TerminalApp,
      pub color_depth: ColorDepth,
      pub color_mode: ColorMode,
      pub dimensions: (u32, u32),
      pub image_support: ImageSupport,
      pub osc8_link_support: bool,
      pub multiplex: MultiplexSupport,
      pub underline_support: UnderlineSupport,
      pub italics_support: bool,
      pub is_tty: bool,
  }
  ```
- Implement `TerminalMetadata::detect()` convenience method
- Update `lib.rs` to expose:
  - All enum types (TerminalApp, ColorDepth, etc.)
  - All detection functions
  - TerminalMetadata struct and methods
- Add prelude module with common exports
- Ensure all public items have doc comments

**Key Considerations**:
- Follow queue-lib pattern of separating detection logic from aggregation
- TerminalMetadata should be Serialize/Deserialize for CLI JSON output
- Methods should use `#[must_use]` for detection functions
- Update module documentation

**Estimated Time**: 0.5-1 day
**Dependencies**: Phase 1.1, 1.2 completion
**Blocks**: Phase 2 (CLI implementation)

---

### Phase 2: CLI Implementation (1.5-2 days)

**Primary Goals**:
1. Implement clap-based argument parsing
2. Create `--meta` flag for comprehensive metadata output
3. Add subcommands for individual metadata queries
4. Support multiple output formats (JSON, human-readable, table)

#### Phase 2.1: Clap CLI Architecture
**Subagent**: **CLI Specialist** (clap skill, rust skill)

**Deliverables**:
- Design clap Args structure supporting:
  - Subcommand pattern: `terminal <command> [--format json|human|table]`
  - Subcommands:
    - `meta` / `metadata` (full metadata)
    - `color-depth`, `color-mode`, `dimensions`, `image-support`
    - `app`, `multiplexer`, `underline-support`, `italics`, `osc8`, `tty`
    - `eval <content>` (evaluate content for escape codes, line widths)
  - Global flags: `--format json` (JSON output for all subcommands)
  - Allow shorthand: `terminal` or `terminal meta` (default to meta)

- Implement Help/Usage text
- Add version support (from Cargo.toml)

**Key Considerations**:
- Use clap derive macros for cleaner code
- Consider compatibility with scripts/automation (JSON output)
- Each subcommand should accept `--format` flag independently
- Default format: human-readable with color/styling
- JSON format should serialize entire structure consistently

**Estimated Time**: 0.75 days
**Dependencies**: Phase 1.3 completion
**Blocks**: Phase 2.2, 2.3

---

#### Phase 2.2: Metadata Output Implementation
**Subagent**: **CLI Specialist** (clap skill) + **Terminal/Rendering Specialist** (biscuit-terminal skill)

**Deliverables**:
- Implement output formatters:
  1. **Human-readable format** (default)
     - Use biscuit-terminal's prose/text block components where applicable
     - Styled output with colors, bold, indentation
     - Example:
       ```
       Terminal Metadata
       ─────────────────
       App:              WezTerm
       Color Depth:      TrueColor (16M colors)
       Color Mode:       Dark
       Dimensions:       120w × 40h
       Image Support:    Kitty Graphics Protocol
       OSC8 Links:       Yes
       Italic Support:   Yes
       ```

  2. **JSON format**
     - Serialize TerminalMetadata directly
     - Pretty-print with indentation
     - Include all enum variants as strings

  3. **Table format** (optional, nice to have)
     - Use biscuit-terminal Table component if available
     - Rows for each metadata item

- Implement `eval` subcommand output
  - `terminal eval <string>` shows:
    - Has escape codes: yes/no
    - Has OSC8 links: yes/no
    - Line widths: [width1, width2, ...]
    - Visual preview with escape codes highlighted

**Key Considerations**:
- Human-readable format should be visually appealing and use terminal capabilities
- JSON output should be machine-parseable (no fancy formatting that breaks parsing)
- Handle empty/null cases gracefully
- Use tracing for debug info (e.g., detection method used)
- Consider color output override with NO_COLOR env var

**Estimated Time**: 1 day
**Dependencies**: Phase 2.1, Phase 1.3
**Blocks**: Phase 3 (testing)

---

#### Phase 2.3: CLI Integration & Error Handling
**Subagent**: **CLI Specialist** (clap skill) + **Error Handling Specialist** (color-eyre skill)

**Deliverables**:
- Main CLI entry point integration
- Error handling for:
  - Invalid format argument
  - Terminal detection failures (graceful degradation)
  - Pipe/redirection scenarios (auto-detect JSON format)
  - Non-TTY output (suppress styling)
- Tracing setup (use subscriber with env-filter)
- Exit codes: 0 for success, 1 for errors

**Key Considerations**:
- Errors should be user-friendly
- Non-TTY detection: disable colors in human-readable output
- Match existing CLI patterns from research-cli or sniff-cli if any
- Terminal detection should never crash the program

**Estimated Time**: 0.5 days
**Dependencies**: Phase 2.1, 2.2
**Blocks**: Phase 3

---

### Phase 3: Testing & Documentation (1.5-2 days)

**Primary Goals**:
1. Comprehensive unit tests for library detection functions
2. Integration tests for CLI subcommands
3. Update README files with examples
4. Cross-platform validation

#### Phase 3.1: Library Unit Tests
**Subagent**: **Testing Specialist** (rust-testing skill, nextest skill)

**Deliverables**:
- Unit tests for all detection functions in `detection.rs`:
  - Test each enum variant path (e.g., all TerminalApp types)
  - Environment variable mocking (follow queue-lib pattern with Mutex for isolation)
  - Test fallback chains (e.g., COLORTERM → terminfo → default)
  - Test error cases (missing terminfo, dumb terminal)

- Unit tests for `eval.rs` functions:
  - `has_escape_codes()`: test CSI codes, OSC codes, mixed content, empty strings
  - `has_osc8_link()`: test valid OSC8, invalid sequences, empty content
  - `line_widths()`: test single line, multi-line, Unicode widths, escape code stripping

- Unit tests for TerminalMetadata aggregation

- Tests for color_mode() detection (mock termbg responses)

**Test Pattern** (from queue-lib):
```rust
#[test]
fn test_wezterm_detection() {
    with_env("WEZTERM_PANE", "0", || {
        assert_eq!(detect_app(), TerminalApp::Wezterm);
    });
}
```

**Test Coverage Goals**:
- All public functions tested
- All enum variants covered
- Edge cases (empty, None, error paths)
- Priority order validation (e.g., TERM_PROGRAM before TERM)

**Estimated Time**: 1 day
**Dependencies**: Phase 1.3 completion
**Blocks**: Phase 3.2

---

#### Phase 3.2: CLI Integration Tests
**Subagent**: **Testing Specialist** (rust-testing skill, nextest skill)

**Deliverables**:
- Integration tests for CLI using `assert_cmd` and `predicates`
  - Test each subcommand: `terminal meta`, `terminal color-depth`, etc.
  - Test output formats: JSON, human-readable, table
  - Test `--format json` flag
  - Test `eval` subcommand with various escape codes
  - Test help text: `terminal --help`
  - Test version: `terminal --version`
  - Test error cases: invalid subcommand, invalid format

- Example test:
  ```rust
  #[test]
  fn test_meta_command_produces_valid_json() {
      let mut cmd = Command::cargo_bin("terminal").unwrap();
      cmd.arg("meta").arg("--format").arg("json");
      cmd.assert().success()
          .stdout(predicate::str::contains("\"color_depth\""));
  }
  ```

- Snapshot tests for output consistency (consider insta crate)

**Estimated Time**: 0.75 days
**Dependencies**: Phase 2.3 completion
**Blocks**: Phase 3.3

---

#### Phase 3.3: Documentation & README Updates
**Subagent**: **Documentation Specialist** (rust skill, terminal skill)

**Deliverables**:
- Update **lib/README.md**:
  - Add discovery/metadata section with examples
  - Document all detection functions
  - Show TerminalMetadata usage
  - Link to detailed docs

- Update **cli/README.md**:
  - Add subcommand reference
  - Show example outputs
  - Document format options
  - Add use cases (scripts, debugging)

- Update **README.md** (root):
  - Mention discovery module in overview
  - Quick start for CLI usage

- Add rustdoc examples to all public functions
- Ensure all public API documented per CLAUDE.md standards

**Estimated Time**: 0.5 days
**Dependencies**: Phase 2.3, Phase 3.1, 3.2 completion
**Blocks**: Release readiness

---

## Dependency Graph

```
Phase 1.1 (Eval Module)
    ↓
Phase 1.2 (Color Mode) ─────→ Phase 1.3 (Exports/Aggregation)
                                ↓
                            Phase 2.1 (CLI Architecture)
                                ↓
                    Phase 2.2 (Output) ─→ Phase 2.3 (Integration)
                                            ↓
                        Phase 3.1 (Unit Tests) ─→ Phase 3.2 (Integration Tests)
                                                    ↓
                                            Phase 3.3 (Documentation)
```

**Parallelizable**:
- Phase 1.1 and 1.2 can start simultaneously (different modules)
- Phase 3.1 can start as soon as Phase 1.3 complete (doesn't require CLI)
- Phase 2.1 can start as soon as Phase 1.3 complete (doesn't require Phase 1.2)

---

## Subagent Specialization Recommendations

### Subagent 1: Rust Library Specialist
**Skills**: rust, terminal, dockhand-library
**Assignments**: Phase 1.1, 1.2, 1.3
**Rationale**:
- Deep Rust knowledge required for eval module ANSI parsing
- Terminal domain expertise for color_mode detection
- Library design patterns from dockhand-library skill
- Can complete entire library implementation independently

**Key Deliverables**:
- Eval module with line width calculation
- Enhanced color_mode detection
- Public API design and exports
- Library unit tests

---

### Subagent 2: CLI & Argument Parsing Specialist
**Skills**: clap, rust, terminal
**Assignments**: Phase 2.1, 2.2, 2.3
**Rationale**:
- Clap skill provides comprehensive argument parsing patterns
- Rust for implementation
- Terminal skill for output formatting/styling
- Can handle complete CLI pipeline

**Key Deliverables**:
- Clap-based argument structure
- Subcommand implementation
- Output formatters (human-readable, JSON, table)
- CLI error handling and integration

---

### Subagent 3: Testing & QA Specialist
**Skills**: rust-testing, nextest, rust
**Assignments**: Phase 3.1, 3.2
**Rationale**:
- rust-testing skill for comprehensive test patterns
- nextest for parallel test execution
- Experience with environment isolation patterns (from queue-lib reference)
- Test-driven verification of all functionality

**Key Deliverables**:
- Unit tests for detection logic
- Integration tests for CLI
- Test coverage reporting
- Cross-platform validation

---

### Subagent 4: Documentation Specialist
**Skills**: rust, terminal, technical-writing
**Assignments**: Phase 3.3, README updates
**Rationale**:
- Rust doc conventions
- Terminal knowledge for contextual examples
- Clear communication of API usage

**Key Deliverables**:
- Updated README files
- Rustdoc examples
- Usage patterns and tutorials

---

## Execution Strategy

### Recommended Timeline

**Week 1**:
- Day 1-2: Subagent 1 completes Phase 1.1 & 1.2 (eval, color_mode)
- Day 1-1.5: Subagent 1 completes Phase 1.3 (exports/aggregation)
- Day 2-2.5: Subagent 2 starts Phase 2.1 (CLI architecture)
- Day 3: Subagent 2 completes Phase 2.1, starts Phase 2.2 (output)
- Day 3-4: Subagent 3 can start Phase 3.1 (unit tests) once Phase 1.3 complete

**Week 2**:
- Day 1: Subagent 2 completes Phase 2.2, starts Phase 2.3 (integration)
- Day 1-2: Subagent 3 completes Phase 3.1, starts Phase 3.2 (CLI tests)
- Day 2: Subagent 2 completes Phase 2.3
- Day 3: Subagent 3 completes Phase 3.2
- Day 3-4: Subagent 4 completes Phase 3.3 (documentation)

**Release**: End of week 2

---

## Risk Mitigation

### High-Risk Items

1. **Color Mode Detection Timeout**
   - OSC 11 queries to terminal may hang if terminal doesn't respond
   - Mitigation: Use `termbg` crate with built-in timeout, or implement 100ms timeout
   - Subagent 1 should validate with multiple terminal emulators

2. **ANSI Sequence Detection Accuracy**
   - Complex regex patterns may miss edge cases
   - Mitigation: Test against real-world terminal output from syntect and LSP clients
   - Consider using regex crate's verbose mode for documentation

3. **Environment Variable Isolation in Tests**
   - Tests modifying env vars can interfere with each other
   - Mitigation: Use Mutex pattern from queue-lib (already documented)
   - Subagent 3 should validate test isolation

4. **Cross-Platform CLI Compatibility**
   - Different terminal emulators have different capabilities
   - Mitigation: Test on macOS (Terminal.app, iTerm2, WezTerm), Linux (Gnome, Konsole), Windows (Windows Terminal)
   - Document platform-specific behavior in CLI output

### Mitigation Assignments

| Risk | Owner | Action |
|------|-------|--------|
| Color mode timeout | Subagent 1 | Validate with multiple terminals |
| Sequence detection | Subagent 1 | Test with syntect examples |
| Test isolation | Subagent 3 | Follow queue-lib pattern |
| Cross-platform | Subagent 2 | Include platform notes in help |

---

## Code Quality Standards (from CLAUDE.md)

### Documentation Standards
- No explicit H1 headings in rustdoc (item name is auto-added)
- Use H2 for primary sections (Examples, Returns, Errors, Notes)
- Use H3 only for subsections
- Section order: brief summary, Examples, Returns, Errors, Panics, Safety, Notes

### Error Handling
- No `unwrap()` or `expect()` in production paths (only tests)
- All public functions return `Result` types where applicable
- Use `thiserror` for error types (if needed)

### Testing Standards
- Both runtime behavior AND type tests for complex functions
- Use `describe` and `it` blocks pattern (describe for groups, it for tests)
- Test isolation for environment-variable-dependent code

### Tracing Standards
- Libraries emit only, never install subscribers
- Use structured fields over messages (tool.name, tool.duration_ms)
- Skip sensitive data: `#[tracing::instrument(skip(api_key))]`

---

## Success Criteria

### Phase 1 Complete When
- [ ] All functions in eval.rs implemented and tested
- [ ] color_mode() returns Light or Dark based on terminal
- [ ] TerminalMetadata struct defined and exported
- [ ] All detection functions have comprehensive unit tests
- [ ] Library exports verified with `cargo check -p biscuit-terminal`

### Phase 2 Complete When
- [ ] CLI accepts `terminal meta` and displays metadata
- [ ] `--format json` flag produces valid JSON
- [ ] All subcommands work: color-depth, color-mode, app, multiplex, etc.
- [ ] `eval` subcommand analyzes content for escape codes
- [ ] Help text is informative
- [ ] Non-TTY output disables styling
- [ ] CLI passes integration tests with assert_cmd

### Phase 3 Complete When
- [ ] Unit test coverage >90% for library code
- [ ] Integration tests cover all CLI subcommands
- [ ] README files updated with usage examples
- [ ] All public functions have rustdoc examples
- [ ] Cross-platform testing validates behavior
- [ ] `cargo test -p biscuit-terminal -p biscuit-terminal-cli` passes

---

## Deliverables Checklist

### Library (biscuit-terminal)
- [ ] eval.rs: line_widths(), has_escape_codes(), has_osc8_link()
- [ ] detection.rs: improved color_mode()
- [ ] lib.rs: TerminalMetadata struct and aggregation method
- [ ] Module documentation and rustdoc examples
- [ ] Unit tests for all detection functions
- [ ] Prelude module for convenient imports

### CLI (biscuit-terminal-cli)
- [ ] Clap argument structure with subcommands
- [ ] Meta subcommand (full metadata output)
- [ ] Format options: JSON, human-readable, table
- [ ] Eval subcommand for content analysis
- [ ] Error handling and user-friendly messages
- [ ] Integration tests using assert_cmd
- [ ] Binary name: `terminal` (from Cargo.toml [[bin]])

### Documentation
- [ ] lib/README.md with discovery examples
- [ ] cli/README.md with subcommand reference
- [ ] Root README.md updated overview
- [ ] Rustdoc examples for all public functions

---

## Follow-Up Tasks (Not in Scope)

These items should be tracked separately:

1. **Terminal Color Query**
   - Implement OSC 10/11/12 queries for dynamic color detection
   - More complex than current color_mode detection
   - Consider for v0.2.0

2. **Grapheme Cluster Support**
   - Implement Mode 2027 support for proper Unicode width calculation
   - Depends on terminal capabilities negotiation
   - Nice-to-have for eval module

3. **Configuration File Detection**
   - Detect terminal config location (e.g., ~/.wezterm.lua)
   - Beyond scope of discovery/metadata
   - Consider for v0.2.0

4. **Terminal Screenshot/Recording**
   - Advanced feature for terminal state capture
   - Out of scope for v0.1.0

---

## Conclusion

The biscuit-terminal implementation is well-suited for a four-subagent model:

1. **Library Specialist** (Phases 1.1-1.3) - Core detection and aggregation
2. **CLI Specialist** (Phases 2.1-2.3) - User-facing argument parsing and output
3. **Testing Specialist** (Phases 3.1-3.2) - Comprehensive validation
4. **Documentation Specialist** (Phase 3.3) - Clear communication

This approach leverages specialized skills while maintaining clear dependencies and allowing parallelization where appropriate. The estimated 5-7 day timeline allows for thorough implementation, testing, and documentation of a production-quality first release.

**Recommended Start Date**: Next available sprint
**Recommended Approach**: Assign subagents in parallel, with Subagent 1 unblocking the others in sequence
