---
ready: true
agent: ""
model: ""
reviewed: 2026-04-30
feature: dim-markdown
verdict: ready-with-followups
---

# Review: Dim Markdown Syntax

Reviewed against:
- `darkmatter/features/_unscheduled/dim-markdown/spec.md`
- `darkmatter/features/_unscheduled/dim-markdown/tech-design.md`
- `darkmatter/features/_unscheduled/dim-markdown/plan.md`

## Summary

The feature is implemented end-to-end and all 44 dim-related tests pass under `cargo test -p darkmatter --lib`. `cargo clippy -p darkmatter --lib` is clean. The pipeline covers detection (`biscuit_terminal::discovery::detection::dim_support`), config (`DimMode`), inline parsing (`InlineStyleProcessor` with `InlineTag::Dim`), terminal SGR 2 emission, table-cell `<dim>` Prose serialization, HTML literal preservation, and scope-cache mapping. Skills, READMEs, and topic docs were updated.

The implementation is production-quality for the common in-line cases the spec emphasizes (`⌄word⌄`, `*⌄word⌄*`, `**⌄word⌄**`, `==⌄word⌄==`), but there are several real gaps in test coverage, two semantic questions, and a handful of cleanups that would improve maintainability and parser fidelity.

## Functional Gaps

### 1. Cross-text-event dim spans degrade silently

`darkmatter/lib/src/markdown/inline/mod.rs:235`

The pairing stack (`dim_opener_stack`, `dim_role`) is local to a single `process_text()` invocation — i.e., a single `Event::Text` from pulldown-cmark. When a dim span crosses another inline tag boundary (e.g., the spec's "Dim spans may be nested within other inline elements … and **may contain other inline elements**"), the two `⌄` delimiters land in different `Event::Text` fragments and never see each other. Both are then classified as unpaired and rewritten as literal `⌄` characters.

Concrete example: `⌄dim and **strong**⌄`
- Terminal: no `\x1b[2m` is emitted at all; the delimiters are literal.
- HTML: the existing test `test_html_dim_with_nested_strong` (`html.rs:1034`) passes only because it asserts the literal delimiters that result from this degradation, not because `Start/End(InlineTag::Dim)` events are emitted.

This is at least worth documenting in the spec/test-suite as a known limitation; ideally, the pairing state should be hoisted from a per-text-event stack into the iterator adapter so dim delimiters can pair across intervening events. Mark already has the same limitation, but Mark's pair almost always sits inside one text event because `==…==` is unlikely to wrap `**bold**`. For Dim — which the spec encourages composing with other inline styles — this is more visible.

**Recommendation:** Add an explicit terminal-renderer test asserting today's behavior (literal `⌄`), then file a follow-up to lift `dim_opener_stack` / `dim_role` onto the processor struct so it survives across `Event::Text` events while being reset by block-level boundaries.

### 2. `DimMode::Auto` under `TERM=dumb` is untested

The plan's Step 5.2 explicitly listed this case: "`DimMode::Auto` with `TERM=dumb` produces no `\x1b[2m`." The actual test that landed (`terminal.rs:7928 test_terminal_dim_mode_auto_with_support`) only asserts `Always.should_emit_dim()` and `Never.should_emit_dim()` — it never exercises the `Auto` arm. The end-to-end behavior for the *most common runtime mode* therefore has zero coverage.

**Recommendation:** Add a `#[serial]` test (using the existing `ScopedEnv` pattern from `biscuit-terminal/lib/src/discovery/detection.rs`) that sets `TERM=dumb` and asserts `for_terminal(...)` with `DimMode::Auto` does not emit `\x1b[2m`.

### 3. Asymmetric escape handling for `==` vs `⌄`

`darkmatter/lib/src/markdown/inline/mod.rs:162-198`

```rust
let is_escaped = i > 0 && bytes[i - 1] == b'\\';
if /* is == */ {
    if is_escaped {
        // Escaped: skip the backslash, treat == as literal
        // The backslash will be consumed when we emit text before it
    }
    delimiters.push(...);  // pushed unconditionally
    ...
}
if /* is ⌄ */ {
    if is_escaped {
        i += 3;
        continue;  // skipped
    }
    ...
}
```

For `==`, `is_escaped` is computed but the branch is empty — the delimiter is always pushed. For `⌄`, `is_escaped` short-circuits the push. The asymmetry happens to be invisible in tests because pulldown-cmark pre-strips `\==` into `==` text before our processor sees it, but a future refactor could trip on this. Either delete the dead `is_escaped` branch for `==` (relying on pulldown-cmark) or apply the same skip behavior to both.

### 4. Weak escape assertion

`test_escaped_dim` (`mod.rs:740`) asserts `content.contains("⌄") || content.contains(r"\⌄")` — i.e., it accepts the dim character or the literal escape sequence. This makes the test green even if the escape were entirely broken. Tighten it to assert exactly one literal `⌄` and zero `Dim` events (the `Dim` half is already there).

## Test Coverage Gaps

| Case | Plan called for | Implemented |
|------|-----------------|-------------|
| `DimMode::Auto` + `TERM=dumb` → no `\x1b[2m` | yes (5.2) | **no** |
| Cross-event dim across `**strong**` (terminal) | implicit (spec nesting examples) | **no** |
| Multiple adjacent dim spans (`⌄a⌄⌄b⌄`) | no | no — worth adding |
| SGR-22 reset preserves outer bold (renderer-level) | implied (risk #2 in tech-design) | **no** — only `emit_prose_text` unit-level test exists; nested render at the document level isn't asserted |
| `Auto` mode resolves through `supports_dim()` (not just Always/Never) | yes | **no** — current test name is misleading |

The terminal tests are otherwise solid (basic, unclosed, inline code, fenced code, list, blockquote, table cell, multiple, empty, nested-bold, never). The inline tests are thorough (10 dim cases plus mixed). HTML coverage is good (literal, nested-strong-degraded, inline code, fenced, escaping).

## Code Quality / Ergonomics

### 5. `dim_opener_stack.iter().rposition(|_| true)` is a stack pop in disguise

`mod.rs:264`

```rust
if let Some(stack_pos) = dim_opener_stack.iter().rposition(|_| true) {
    let opener_idx = dim_opener_stack.remove(stack_pos);
    ...
}
```

This is equivalent to `dim_opener_stack.pop()`. Replace with:

```rust
if let Some(opener_idx) = dim_opener_stack.pop() {
    dim_role.insert(opener_idx, true);
    dim_role.insert(idx, false);
    paired = true;
}
```

### 6. `HashMap<usize, bool>` for delimiter roles is overkill

`mod.rs:257`

Roles are keyed by the dense index `0..delimiters.len()`. A `Vec<Option<bool>>` (or `Vec<DimRole>` with three variants `{ Opener, Closer, Literal }`) initialized with the delimiter count is simpler, faster, and avoids the `std::collections::HashMap` import inside a hot path.

### 7. Manual UTF-8 byte detection for U+2304

`mod.rs:182`

```rust
if bytes[i] == 0xE2 && i + 2 < bytes.len() && bytes[i + 1] == 0x8C && bytes[i + 2] == 0x84 { ... }
```

Equivalent and cleaner:

```rust
for (byte_pos, _) in s.match_indices('\u{2304}') { ... }
```

…and the `==` scan can use `s.match_indices("==")`. The two passes can be merged with a tagged enum, or `find_delimiters` can run two `match_indices` and merge by byte offset. This removes the `bytes[i + 1]`/`bytes[i + 2]` bounds-check footguns and lets the compiler do its job.

### 8. Per-segment `String` allocations

`mod.rs:284, 322, 329, 310`

Every text segment is built via `CowStr::from(slice.to_string())`, which always heap-allocates. The owning `CowStr<'a>` produced by pulldown-cmark already owns or borrows the underlying bytes; we can't propagate that borrow into pending events without lifetime gymnastics, but at minimum the literal `CowStr::from("\u{2304}")` and `CowStr::from("==".to_string())` paths can use `'static` `CowStr::Borrowed("⌄")` and `CowStr::Borrowed("==")` respectively, avoiding two allocations per unpaired delimiter / unclosed mark.

### 9. `process_text` does two delimiter walks plus a HashMap

The current shape is: walk delimiters once to build pairings, walk again to emit segments, with a HashMap carrying state between. A single-pass algorithm that emits Start/End directly when pairing succeeds (and back-patches an unpaired opener at the end, exactly like the `last_mark_start_idx` mechanism already used for Mark) would be ~half the code and ~half the work.

### 10. `test_terminal_dim_mode_auto_with_support` is misnamed

The test never touches `DimMode::Auto`; it tests `Always` and `Never`. Rename to `test_dim_mode_always_and_never_resolve` or replace with a real `Auto` test (see gap #2 above).

## Documentation

### 11. Plan doc references `.opencode/skill/...` paths

`plan.md:24, 47-48, 382, 390, 452-453` reference `.opencode/skill/...`, but the actual canonical skills directory in this monorepo is `.claude/skills/...`. The skill updates landed in the right place (`.claude/skills/darkmatter/SKILL.md`, `.claude/skills/biscuit-terminal/discovery.md`), so this is purely a plan-document inaccuracy — but worth correcting if anyone uses the plan as a reference later.

### 12. Skill / docs coverage is good

- `darkmatter/lib/README.md:10` — lists `⌄dimmed⌄` ✓
- `.claude/skills/darkmatter/SKILL.md:42` — table entry for dim ✓
- `.claude/skills/biscuit-terminal/discovery.md:97-99` — `dim_support()` example ✓
- `darkmatter/docs/topics/output-formats.md:13` — terminal note ✓
- `darkmatter/docs/topics/html.md:7-23` — explicit literal-HTML behavior ✓

The biscuit-terminal SKILL.md frontmatter / top-level capability list does not appear to mention `dim_support` (only the discovery sub-doc does). Minor — worth adding a one-liner alongside `italics_support` so the surface is discoverable from the SKILL entry point.

## Suggested Follow-ups (priority order)

1. **(must-fix)** Add a `DimMode::Auto` runtime test using `ScopedEnv` to set `TERM=dumb` and assert no `\x1b[2m` is emitted.
2. **(must-fix)** Tighten `test_escaped_dim` to require exactly the literal `⌄` (drop the `||` fallback).
3. **(should-fix)** Document or fix cross-event dim pairing — at minimum, add a regression test asserting current behavior for `⌄dim and **strong**⌄`.
4. **(should-fix)** Replace the `dim_opener_stack` / `HashMap` two-pass with a single-pass pop-based pairing (`dim_opener_stack.pop()`); collapse `dim_role` into local back-patching.
5. **(nice-to-have)** Switch to `s.match_indices('\u{2304}')` and `s.match_indices("==")` for delimiter scanning; remove manual byte-pattern matching.
6. **(nice-to-have)** Use `CowStr::Borrowed("⌄")` / `CowStr::Borrowed("==")` for literal segments to avoid per-emit allocations.
7. **(nice-to-have)** Rename `test_terminal_dim_mode_auto_with_support`.
8. **(nice-to-have)** Resolve the asymmetric `is_escaped` handling between `==` and `⌄` (delete the dead Mark branch or unify the behavior).
9. **(nice-to-have)** Add `dim_support` to the top-level biscuit-terminal SKILL.md detection list.
10. **(nice-to-have)** Update `plan.md` skills paths from `.opencode/skill/...` to `.claude/skills/...`.

## Verdict

`ready: true` — the feature meets the spec for the documented common cases, has end-to-end tests, clippy clean, and updated docs/skills. The follow-ups above are real but none are user-blocking for the documented examples. Items 1–3 should be addressed before declaring full spec parity for nested cases.
