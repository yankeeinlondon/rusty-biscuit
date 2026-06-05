1. The terminal renderer never actually applied layout before this.
The plan framed Task 11 as "migrate active_layout_hints to Layout." But active_layout_hints turned out to
be write-only dead code — with_layout() set it, for_child() propagated it, and render.rs never once read
it. So "apply Layout in the terminal tree renderer" wasn't a migration of existing behavior; it was the
first time the terminal tree path applied margins/alignment at all. That reframes how much of this was
refactor vs. net-new feature.

2. A latent regression hidden by a 5-task compile gap.
Task 5's decision to #[derive(Default)] on Layout made word_wrap fall through to WordWrap::default() =
WrapProse(Some(8), None) instead of None. That silently flipped every Prose::new() in biscuit-terminal to
wrapping. But biscuit-terminal didn't compile from Task 11 through Task 14, so the broken test
(narrow_terminal_stacks_columns) was invisible for ~5 tasks. The bug was committed in Task 5 and only
surfaced when the crate recompiled. The first bisect even pointed at the wrong commits (Task 11's
render.rs) — the real culprit was a one-word derive three tasks earlier.

3. The plan's task order contradicted the real dependency graph.
biscuit-terminal cannot compile until Tasks 11 + 12 + 14 all land, and darkmatter depends on
biscuit-terminal compiling. The plan ordered Task 13 (darkmatter) before Task 14 — which would have left
darkmatter unbuildable. I had to resequence to 11→12→14→13. TDD verification ("run the test, watch it
fail") was also impossible for Tasks 11–12 because the crate didn't link; verification had to be deferred
to Task 14.

4. The validators were built, tested, and then never called.
Layout::validate(), Margin::validate(), TargetValue::validate() all exist with thorough unit tests — but
nothing in the render pipeline invokes them. Combined with Length::Percent(f32) being a public variant
alongside the checked Length::percent() -> Result, the validation is trivially bypassed. The plan specified
building validators but never specified wiring them in. A whole correctness layer is decorative.

5. The drift ledger is self-policing — which made blind regeneration safe.
I expected RECORD_DRIFT=1 snapshot regeneration to be risky (blindly accepting whatever output appears).
But render_matches_bespoke fails in both directions — unrecorded live drift AND ledger entries that no
longer drift. So a regenerated ledger that makes the test pass is provably the exact live drift set; you
can't hide a regression by deleting an entry.

6. "Drift" sometimes meant the new path was more correct.
Several divergences the tree renderer introduced are cases where it correctly applies vertical margins /
block alignment that the legacy bespoke renderer never did. "Burn down drift so tree matches bespoke" would
have meant regressing the new code to match a wrong baseline. And BlockQuote::render_tree() was outright
dropping its own Layout — with a doc comment that proudly said so.

  The durable, non-obvious ones — the cross-crate compile dependency (biscuit-terminal needs 11+12+14;
  darkmatter needs biscuit-terminal) and the unwired validators — are worth saving as project memory. Want me
   to record those?
