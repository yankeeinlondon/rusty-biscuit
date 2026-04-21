---
phases: 4
created: 2026-04-20
start_phase: 1
---

# Execution Plan: Horizontal Rule Component

## Phase 1: biscuit-terminal Core Component

### 1.1 Create BrowserRenderable Trait
- **File:** `biscuit-terminal/lib/src/components/renderable.rs`
- Add `BrowserRenderable` trait with `render_to_browser()` and `render_to_browser_with_inline_variables()` methods
- **Validation:** Trait compiles and can be implemented by `HorizontalRule`

### 1.2 Create HorizontalRule Component
- **File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs` (new)
- Implement data structures:
  - `RuleStyle` enum (Dashes, Dots, Waves, LineStar, LineCircle, InsetLine, CurtainRod)
  - `RulePlacement` enum (Full, Centered, Left, Right)
  - `RuleWeight` enum (Thin, Medium, Thick)
  - `HorizontalRule` struct
- **Validation:** All types compile, serialize/deserialize works

### 1.3 Implement Renderable Trait (Terminal Rendering)
- Implement `Renderable` for `HorizontalRule` with 3-tier progressive enhancement:
  - Tier 1: SVG→PNG via resvg with `TerminalImage`
  - Tier 2: Unicode fallback characters
  - Tier 3: ASCII fallback characters
- **Validation:** All 7 styles render correctly across all 3 tiers

### 1.4 Implement BrowserRenderable Trait
- Implement `BrowserRenderable` for `HorizontalRule`
- SVG generation with `stroke="currentColor"` and CSS variables
- **Validation:** SVG output is valid and uses CSS variables for scaling

### 1.5 Visual Style Definitions
- Implement all 6 visual styles with SVG primitives, Unicode fallbacks, ASCII fallbacks
- **Validation:** Style matrix complete (see tech-design table, lines 160-167)

---

## Phase 2: darkmatter Integration

### 2.1 Create RuleProcessor Iterator Adapter
- **File:** `darkmatter/lib/src/markdown/inline/mod.rs` (or block-level location)
- Implement pattern matching: `^([\-\_\*]{3,})\s*\{(.*)\}\s*$`
- Emit `InlineEvent::HorizontalRule(attrs)` with parsed attributes
- **Validation:** Parses `--- { style: waves }` correctly

### 2.2 Extend InlineEvent Enum
- **File:** `darkmatter/lib/src/markdown/inline/types.rs`
- Add `HorizontalRule(HorizontalRuleAttrs)` variant to `InlineEvent`
- Define `HorizontalRuleAttrs` struct with optional style, placement, weight, width, color
- **Validation:** New variant compiles and integrates with existing event stream

### 2.3 Update Terminal Renderer
- **File:** `darkmatter/lib/src/markdown/output/terminal.rs`
- Handle `InlineEvent::HorizontalRule(attrs)`
- Map `HorizontalRuleAttrs` to `biscuit_terminal::HorizontalRule`
- Render via `rule.render(&term)`
- **Validation:** Renders to terminal with correct style/placement/weight

### 2.4 Update HTML Renderer
- **File:** `darkmatter/lib/src/markdown/output/html.rs`
- Handle `InlineEvent::HorizontalRule(attrs)`
- Render via `rule.render_to_browser()`
- **Validation:** Outputs valid HTML/SVG

---

## Phase 3: Documentation & Maintenance

### 3.1 Create Usage Guide
- **File:** `darkmatter/docs/topics/horizontal-rules.md`
- Document markdown syntax: `--- { style: waves, width: "50%" }`
- Document all style/placement/weight/width/color options
- **Validation:** Guide renders correctly in darkmatter

### 3.2 Create Component API Documentation
- **File:** `biscuit-terminal/docs/components/horizontal-rule.md`
- Document `HorizontalRule` struct, all enums, trait implementations
- **Validation:** Doc compiles (`cargo doc`)

### 3.3 Create Trait Documentation
- **File:** `biscuit-terminal/docs/components/browser-renderable-trait.md`
- Document `BrowserRenderable` trait methods and usage
- **Validation:** Doc compiles

### 3.4 Update Agent Skills
- Update `.claude/skills/darkmatter` to include horizontal rule capability
- Update `.claude/skills/biscuit-terminal` to include `BrowserRenderable` trait
- **Validation:** Skills load without errors

---

## Phase 4: Testing

### 4.1 Unit Tests: biscuit-terminal
- Test `HorizontalRule` rendering for all styles × all tiers (Image, Unicode, ASCII)
- Test attribute parsing correctness
- **Validation:** `cargo test -p biscuit-terminal` passes

### 4.2 Unit Tests: darkmatter
- Test attribute parsing from markdown markers (`---`, `***`, `___`)
- Test `RuleProcessor` pattern matching edge cases
- **Validation:** `cargo test -p darkmatter` passes

### 4.3 Integration Tests
- End-to-end test: markdown → parse → terminal output
- End-to-end test: markdown → parse → HTML output
- **Validation:** Integration tests pass

### 4.4 Snapshot Tests
- Maintain SVG output snapshots for browser rendering
- Maintain ANSI output snapshots for terminal rendering
- **Validation:** Snapshots match expected output; no regressions

---

## Dependencies

```
Phase 1 (biscuit-terminal)
    └─ Phase 2 (darkmatter) ──► Phase 3 (Documentation) ──► Phase 4 (Testing)
         ↑                                                        │
         └────────────────────────────────────────────────────────┘
```

- Phase 2 depends on Phase 1 (HorizontalRule component must exist)
- Phase 3 can begin after Phase 2 code is stable
- Phase 4 testing depends on Phases 1-3 completion

## Parallelization Opportunities

- **Within Phase 1:** Steps 1.1–1.5 can proceed in parallel once trait is defined
- **Within Phase 2:** Steps 2.1 and 2.2 can proceed in parallel (event model and parser independent initially)
- **Within Phase 3:** Steps 3.1–3.3 are independent and can proceed in parallel
- **Within Phase 4:** Steps 4.1 and 4.2 are independent (different packages)
