---
phases: 4
created: 2026-04-21
start_phase: 2
---

# Horizontal Rule Feature Execution Plan

## Phase 1: Core Component Implementation (biscuit-terminal)

### Steps

1. **Create BrowserRenderable trait** - Add `BrowserRenderable` trait to `biscuit-terminal/lib/src/components/renderable.rs` with `render_to_browser()` and `render_to_browser_with_inline_variables()` methods
2. **Implement HorizontalRule data structures** - Create `biscuit-terminal/lib/src/components/horizontal_rule.rs` with `RuleStyle`, `RulePlacement`, `RuleWeight` enums and `HorizontalRule` struct
3. **Implement terminal rendering logic** - Add `Renderable` implementation with progressive enhancement tiers (Image → Unicode → ASCII fallbacks)
4. **Implement browser rendering logic** - Add `BrowserRenderable` implementation generating SVG strings with CSS variables and `currentColor` support
5. **Create visual style SVG templates** - Implement SVG generation for all 7 visual styles (`dashes`, `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`)

### Validation Checkpoints

- ✅ All enums and structs compile without errors
- ✅ Terminal rendering produces correct output for all 3 tiers
- ✅ Browser rendering generates valid SVG with proper CSS variables
- ✅ All visual styles render correctly in both terminal and browser contexts

## Phase 2: Darkmatter Integration

### Steps

1. **Extend event model** - Add `HorizontalRule` variant to `InlineEvent` enum in `darkmatter/lib/src/markdown/inline/types.rs`
2. **Create RuleProcessor** - Implement `RuleProcessor` iterator adapter in `darkmatter/lib/src/markdown/inline/mod.rs` to parse attribute syntax (`--- { ... }`)
3. **Implement attribute parsing** - Add regex pattern matching and JSON-like attribute parsing for horizontal rule markers
4. **Integrate terminal renderer** - Update `darkmatter/lib/src/markdown/output/terminal.rs` to handle `InlineEvent::HorizontalRule` and map to `biscuit_terminal::HorizontalRule`
5. **Integrate HTML renderer** - Update `darkmatter/lib/src/markdown/output/html.rs` to handle `InlineEvent::HorizontalRule` using browser rendering

### Validation Checkpoints

- ✅ All markdown syntax variants (`---`, `***`, `___`) with attributes parse correctly
- ✅ Attribute parsing handles all supported properties (style, placement, weight, width, color)
- ✅ Terminal output correctly renders through biscuit-terminal component
- ✅ HTML output generates valid SVG with proper styling

## Phase 3: Testing and Quality Assurance

### Steps

1. **Write unit tests for biscuit-terminal** - Test all `HorizontalRule` rendering tiers and visual styles
2. **Write unit tests for darkmatter** - Test attribute parsing from all markdown marker types
3. **Write integration tests** - Verify end-to-end rendering from markdown input to terminal/HTML output
4. **Create snapshot tests** - Establish baseline SVG and ANSI output snapshots for visual consistency
5. **Run comprehensive test suite** - Execute all tests and verify no regressions

### Validation Checkpoints

- ✅ All unit tests pass for both libraries
- ✅ Integration tests verify correct orchestration between darkmatter and biscuit-terminal
- ✅ Snapshot tests capture expected visual output
- ✅ No existing functionality is broken

## Phase 4: Documentation and Skill Updates

### Steps

1. **Create darkmatter documentation** - Write `darkmatter/docs/topics/horizontal-rules.md` usage guide
2. **Create biscuit-terminal component docs** - Write `biscuit-terminal/docs/components/horizontal-rule.md` API documentation
3. **Create trait documentation** - Write `biscuit-terminal/docs/components/browser-renderable-trait.md` trait documentation
4. **Update darkmatter agent skill** - Update `.claude/skills/darkmatter/SKILL.md` to include horizontal rule capability
5. **Update biscuit-terminal agent skill** - Update `.claude/skills/biscuit-terminal/SKILL.md` to include `HorizontalRule` component and `BrowserRenderable` trait

### Validation Checkpoints

- ✅ All documentation files are created with comprehensive coverage
- ✅ Agent skills reflect new functionality accurately
- ✅ Documentation examples work correctly when tested

## Parallelizable Work

- **Phase 1 Steps 1-2** can be done in parallel with **Phase 2 Step 1**
- **Phase 3 Steps 1-2** can be done in parallel once their respective components are implemented
- **Phase 4 Steps 1-3** can be done in parallel once implementation is complete
- **Phase 4 Steps 4-5** can be done in parallel once documentation is written

