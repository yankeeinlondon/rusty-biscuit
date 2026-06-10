---
ready: false
agent: codex
model: ""
---

# Review 5

## Findings

### High: A matched node policy still changes renderer-wide terminal capabilities

The unmatched-policy regression from review 4 is fixed, but the replacement
still violates the Option A boundary. `capability_signals` reports `applies`
when any node has policy-baked layout or text-layout attrs
(`darkmatter/lib/src/layout/page.rs:1024-1031`, `:1347-1359`), and
`DarkmatterPage::render` uses that bit to select the optimistic terminal
capability profile for the entire document (`page.rs:875-888`). The adapter
documents that this profile includes TrueColor and OSC8
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:399-408`).

Consequently, a matched layout-only policy can change unrelated content. For
example, centering a table sets `applies = true`; on an ambient no-color or
non-OSC8 terminal, an unrelated fenced code block can gain TrueColor output and
an unrelated link can gain OSC8 solely because the table policy matched. That
is renderer-wide policy behavior, not an attr resolved on the table node. It
contradicts the selected page-frame contract that the frame carries no
per-component policy and the documented architecture that component policy is
baked onto nodes and resolved by the target fold.

The new tests do not cover this case. Both parity pages explicitly pin
`ColorDepth::TrueColor`, and the Level 2 assertion compares only foreground
color sets (`darkmatter/lib/tests/level2_render_tree_terminal.rs:1376-1426`,
`:1462-1516`). They therefore cannot detect an ambient-versus-optimistic
capability switch or an OSC8 difference. Keep capability selection independent
of component-policy matches; only explicit renderer/page capability settings
should alter the global profile. Add a discriminating Level 1 parity test with
a matched layout-only policy plus unrelated code/link content under differing
ambient and captured capabilities, and Level 2 real-terminal parity covering
both rendered color and hyperlink behavior.

### Medium: The performance record has not been re-baselined after the topology change

The implementation removed a complete construction fold, but
`performance-record.md` still publishes the review-3 double-fold timing and
explicitly says the styled terminal median “should be re-anchored”
(`renderable/features/2026-06-06-tree-closeout/performance-record.md:55-75`).
The spec requires Criterion results for the final production corpus, so the
current record is not evidence for the implementation under review. Re-run the
documented styled-production benchmark and replace the stale terminal baseline
before closeout.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Unmatched policy does not alter unrelated terminal color | Level 1 byte parity; Level 2 foreground-color parity | Appropriate for the covered color behavior |
| Unmatched policy does not alter the full capability profile, including OSC8 | Tests pin TrueColor and do not inspect hyperlink escapes | Gap; Level 2 terminal verification is required |
| Matched node policy remains local to that node | None; implementation selects global optimistic capabilities | Gap and implementation defect |
| Page-frame terminal width independence from unmatched policy | Level 1 discriminating parity | Appropriate |
| Terminal HR appearance and layout | Level 2 real-terminal capture | Appropriate |
| Browser HR and component layout/style | Browser computed style/geometry | Appropriate |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `git diff --check` and `git diff --cached --check`: clean.
- Focused darkmatter tests were started but not completed because three
  concurrent Cargo invocations contended on the shared build lock and exceeded
  the non-interactive session limit; they were terminated.

The single-document fix resolves review 4's duplicate construction fold, but
renderer-wide capabilities remain coupled to matched component policy. The
feature is not ready for production.
