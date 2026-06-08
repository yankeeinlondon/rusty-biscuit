---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High: Browser Vector/Rich rendering ignores thematic-break alignment

`ThematicBreakAttrs::alignment` was promoted as first-class layout intent, but
the Browser Vector/Rich fold passes only `kind`, `weight`, `width`, and `color`
to `horizontal_rule_svg`
(`renderable/src/tree/render/browser.rs:555`). The SVG helper then hard-codes
`margin-left/right: auto` (`renderable/src/tree/graphics.rs:123`), so `left`,
`right`, and `full` all render centered whenever a non-full width is used.
Only GraphicsMode::Off preserves alignment, and then merely as a
`data-hr-alignment` attribute rather than rendered layout.

This contradicts the inventory's classification of alignment as first-class
horizontal placement and leaves the shared browser renderer semantically
incomplete. The current browser-tier HR test checks waves DOM parsing and
stroke width, but never alignment or geometry
(`darkmatter/lib/tests/browser_render.rs:88`).

Fix the rich SVG lowering to honor all alignment values, then add real-browser
geometry assertions comparing left, center, right, and full rules at a narrow
authored width. This requirement currently has no valid Browser-level
verification, so it is a production-readiness blocker.

### High: The retained page frame still depends on component-policy state

Option A explicitly requires the page frame to carry no per-component policy.
Production terminal rendering still branches on
`!self.component_policies.is_empty()` to set the global render width
(`darkmatter/lib/src/layout/page.rs:806`) and to decide whether to normalize and
decorate the entire rendered body (`darkmatter/lib/src/layout/page.rs:872`).
Browser rendering likewise adds the page wrapper solely because component
policies exist (`darkmatter/lib/src/layout/page.rs:963`).

The new test does not prove the specified boundary. It explicitly accepts that
frame output depends on policy presence and compares only two non-empty policy
maps (`darkmatter/lib/src/layout/page.rs:1808`). An unmatched policy can
therefore change global width, vertical rhythm, and browser DOM despite having
no applicable component.

Remove component-policy presence from page-frame decisions. Policies should
affect only attrs attached during tree construction. Add parity tests comparing
no policy with an unmatched policy for terminal output and browser HTML, plus
the appropriate L2/browser checks for any retained user-visible distinction.

### High: Several claimed Level 2 HR checks pass when the observable row is absent

The canonical HR color, background, alignment, and width tests return
successfully when `locate_hr_between_sentinels` cannot find a rule row
(`darkmatter/cli/tests/level2_layout.rs:2025`, `:2065`, `:2121`, `:2165`).
The color test even labels this a timing skip. These are not harness-availability
skips; once WezTerm is available and the command completed, failure to observe
the required row must fail the test.

Consequently, the verification record's “55 passed, no skips” result does not
prove these terminal-visible requirements at Level 2. Keep the existing
`evaluate_level` skip for an unavailable harness, but panic with the full
capture when an expected row is missing.

### Medium: The structural performance gate does not prove all claimed gates

The hint-access counter proves that `set_hint`/`get_hint`/`remove_hint` were not
called. It does not prove “zero typed-attr serde round-trips”: code could
serialize a typed attr directly without touching `NodeAttrs::data`. The module
calls this a type-level guarantee
(`darkmatter/lib/src/markdown/render_tree/structural_gate.rs:44`), but Rust's
types do not prevent such serialization.

Similarly, `performance-record.md` says no regression was observed, while the
new styled benchmark has no same-corpus pre-change baseline and is compared
with differently sized unstyled workloads. Either instrument serde conversion
on the production fold and record a real before/after Criterion baseline, or
narrow the acceptance claim to what the current counter and timings establish.

### Medium: Completion metadata points to a directory absent from this worktree

The active closeout artifacts are under
`renderable/features/2026-06-06-tree-closeout`, but their frontmatter and links
claim they live under `_completed/2026-06-06-tree-closeout`. That completed
directory is absent in the current filesystem, while `plan.md` states the move
is complete. Acceptance criterion 9 is therefore not met in the reviewed tree.
Finish the move consistently or restore active-path links before closeout.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Typed HR storage and placement validation | L1 | Appropriate for structural behavior |
| Terminal HR kind/weight | L2 real-terminal capture | Appropriate |
| Terminal HR color/background/alignment/width | Nominal L2, but fail-open on missing row | Gap; not reliable L2 evidence |
| Browser HR SVG parsing and weight | Browser (real Chrome) | Appropriate |
| Browser HR alignment | None for rendered geometry | Gap; Browser tier required |
| Page-frame terminal geometry | L2 for general frame behavior; new boundary tests are L1 | Boundary itself remains violated |
| Browser page geometry | Browser computed-style tests | Appropriate for covered properties |
| Markdown/MarkdownPlus degradation | L1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification Run

- `renderable`: 6 focused tests passed.
- `biscuit-terminal`: 10 focused HR/performance tests passed.
- `darkmatter`: 9 focused structural/page-frame tests passed.

These green L1 runs confirm the implemented paths but do not resolve the
behavioral and verification gaps above.

## Additional Recommendation

Consider replacing free-form `String` fields for HR `kind`, `alignment`, and
`weight` with shared enums. Terminal and Browser currently duplicate string
matching and can silently diverge, as the alignment defect demonstrates.
