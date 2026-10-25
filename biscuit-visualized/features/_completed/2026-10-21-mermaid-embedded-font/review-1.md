---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-24T10:09:30-07:00"
spec: 2026-10-21-mermaid-embedded-font/spec.md
feature: 2026-10-21-mermaid-embedded-font/review-1.md
next: 2026-10-21-mermaid-embedded-font/review-2.md
implemented: true
description: "A **feature** review of `2026-10-21-mermaid-embedded-font/spec.md`"
ready: false
readiness_assessed: false
findings: []
recurrence: false
human_resolved: false
human_in_the_loop:
    - |-
      Decide whether small embedded fonts justify simpler typography. The implementation log reports that allsorts removes the font tables for kerning, ligatures, and contextual alternates. The agent changed diagram measurement to match that simpler browser output, after finding approximately 1.3% width disagreement with normal measurement. This changes spacing and character combinations in opted-in diagrams; it is more than an internal implementation detail. The spec was amended during implementation and explicitly flags the choice for review.

      Options:
      - Retain the current subsets and matching measurement. This keeps files small but loses those typography features. Recommended for the current scope if that tradeoff is acceptable.
      - Embed whole fonts and retain normal shaping. Bundled Inter is about 410 KB per weight before base64, according to the log; the spec permits exceeding the size budget when logged.
      - Investigate a subsetter that preserves shaping tables. This adds implementation work and needs an explicit scope decision.

      Expected ruling: accept the simpler typography, or select a different font-embedding approach.
    - |-
      Decide whether an unused bold face may force regular labels to use Inter. The spec bases substitution on weights the page uses. The agent instead checks both regular and bold before layout. For example, a page using only a permitted regular face still switches to Inter if that family's unused bold face prohibits embedding.

      Options:
      - Accept the conservative two-weight check. Recommended to avoid a second layout pass, provided this visible font substitution is acceptable.
      - Require checking only weights the page actually uses, accepting additional layout or discovery work.

      Expected ruling: confirm which font-selection behavior the feature should promise.
    - |-
      Confirm the additional font-format restrictions. The agent substitutes Inter for variable fonts and fonts using CFF outlines, even when their embedding permission allows redistribution. Only static TrueType outlines qualify; a face in a collection qualifies when extracted as an individual TrueType font. This limits which installed fonts survive the opt-in.

      Options:
      - Accept the restriction and explain the substitution. Recommended because the spec currently requires TrueType data URLs.
      - Expand support to other outline formats or variable-font conversion, with corresponding changes to output and sanitizer rules.

      Expected ruling: accept the supported-font boundary or authorize broader support.
    - |-
      Decide how to handle the unfinished command-line option. The log says `md --feature embed-mermaid-font`, its command-line wiring test, and the test that a frontmatter key has no effect were not implemented. The prerequisite work that introduces `--feature` had not landed. The agent continued with library and page support despite the plan's prerequisite gate. The docs correctly mark the command-line option as planned.

      Options:
      - Keep the original scope and finish the option and tests after the shared flag lands. Recommended; do not declare the feature complete meanwhile.
      - Authorize introducing the shared flag in this work, coordinating ownership with the prerequisite feature.
      - Explicitly narrow this feature to library/page APIs and assign the command-line behavior to separate active work.

      Expected ruling: choose the completion path. This review does not treat the deferred option as an approved scope reduction.
    - |-
      Decide what the cross-host width check must measure. The spec requires every SVG label's bounding-box width to agree within 0.5 px. The log reports four of 56 comparisons outside that tolerance, with maximum differences of 0.673 px and 0.969 px. Text advance widths—the horizontal distance used to place the text—matched exactly. The agent accepted the advance widths instead, attributing the bounding-box differences to browser/platform behavior observed in controls.

      Options:
      - Amend the check to compare text advance widths, retaining bounding boxes as diagnostic evidence. Recommended if consistent text layout is the intended guarantee.
      - Keep bounding boxes but increase the tolerance to 1 px, accepting that visible extent may differ slightly.
      - Retain the current bounding-box requirement and investigate until it passes.

      Expected ruling: choose the measurement and tolerance. The log's explanation is evidence for that choice, not approval to change the acceptance criterion.
human_review: true
human_review_items:
    - "Choose small font subsets with simpler typography, whole fonts with normal shaping, or further subsetter work; the tradeoff is download size versus typography."
    - "Choose conservative regular-and-bold permission checks or checks limited to weights actually used; the tradeoff is simpler rendering versus preserving the requested regular face."
    - "Accept static TrueType-only embedding or authorize broader format support; unsupported installed faces currently become Inter."
    - "Choose whether to wait for the shared CLI flag, implement it here, or explicitly reduce scope; the specified command-line option and associated tests remain unfinished."
    - "Choose advance-width comparison, a 1 px bounding-box tolerance, or further investigation under the existing 0.5 px requirement."
has_blocked_findings: false
blocked: false
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/log.md
implemented_by: claude/opus
---

# Review 1: design decisions need human review

This review stops at the requested first-review decision checkpoint. Two separate agents reviewed the implementation log and specification. Both identified implementation-time choices requiring human consideration. No implementation-to-specification comparison, code review, or test execution was performed.

Production readiness is **unassessed**. The `ready: false` value records that this halted review does not certify readiness; it is not a technical finding or a conclusion that the implementation failed a code review. There are no findings yet, and the spec is not marked completed. The detailed first-review stop instructions take precedence here over the normal completed-review relationship between `ready` and the findings list.

## Decisions that need a ruling

The five items in `human_in_the_loop` give the choices, consequences, and recommendations. Their sources are:

| Decision | Evidence and authority |
| --- | --- |
| Simpler typography to keep small subsets | [Spec: measurement amendment](spec.md#d6-the-opt-in-measures-with-the-face-it-embeds-amended-2026-10-22-by-d13-amended-2026-10-24-during-implementation-under-d17); implementation log, `decisions_for_review`, third entry. The spec's bounded autonomy permits evidence-based choices but explicitly requires this choice to reach review. |
| An unused bold face can trigger Inter substitution | [Plan: Necessary Rules](plan.md#necessary-rules), “which weights decide embeddability”; spec, “Choosing the face,” limits the rule to weights the page uses. |
| Variable and CFF fonts become Inter | [Plan: Necessary Rules](plan.md#necessary-rules), “what counts as an embeddable outline format”; implementation log, first and fourth decision entries. |
| Command-line option and tests deferred | [Implementation log: Phase 5](implementation-log.md#phase-5); [plan: Darkmatter](plan.md#phase-5-darkmatter), unchecked command-line task. The spec still includes this interface in its requirements and acceptance criteria. |
| Advance widths accepted instead of bounding boxes | [Implementation log: two-host comparison](implementation-log.md#72-ac-e1-two-host-width-comparison); spec, acceptance-criteria table, first row. The reported bounding-box measurements do not meet the written tolerance. |

The measurement decision concerns what the acceptance test means, rather than a request for additional operating-system proof. The reported results distinguish the two quantities clearly; choosing which quantity fulfills the product goal belongs to the author.

## Other recorded decisions

The log contains further decisions worth preserving for the next review. They do not independently justify stopping for human input. These are reports of implementation choices, not endorsements of code that this review has not inspected.

| Area | Recorded choice and effect |
| --- | --- |
| Font selection | Substitution is local to a render and kept in the text measurer; process-wide environment state is not changed. Only upright regular and bold are selected. Unreadable fonts share the unreadable-permission-table reason; Restricted takes precedence over bitmap-only in the reported reason. Direct bundled Inter resolution is described as “resolved,” while an installed Inter selected through the environment is checked like any installed face. |
| Label collection | XML and CSS libraries read final SVG text and effective font weight. Characters are grouped by family and weight so unused families and unrelated glyphs are omitted. The plan had proposed a handwritten SVG scanner and a shared character set. |
| Font output | Rules explicitly identify regular/bold and normal style, use the existing CSS string escaper, and treat 60 KB as 60,000 bytes. Generated base64 needs no additional URL escaper. Expected oversize and whole-font outcomes log at info; failures warn. Any subset failure disables embedding for the whole page. Inter attribution is emitted for bundled Inter or a family named Inter. |
| Shaping implementation | Measurement uses a view of the font without the tables omitted by the subset, rather than attempting to disable every OpenType feature by name. This implements the typography decision above. |
| Public interfaces | Separate opt-in builders and artifact/font-data methods were added instead of changing the target enum or the shared artifact structure. Page/component builders let Darkmatter retain the component's palette and background. |
| Cache | Each SVG has a companion character/weight record; page subsets are rebuilt from the union. Faces are resolved again on cache hits because process-local face IDs cannot be persisted. Missing or unreadable records force a rerender. A collection failure preserves the diagram but contributes no characters to the page font. |
| Sanitizer | The existing CSS tokenizer replaces the proposed handwritten scanner. Quoted font data URLs are accepted. Inert comments/strings no longer cause rejection merely for containing URL-like text; malformed tokens do. Generated page CSS uses the shared safety check. |
| Page placement | A generated placeholder is replaced after all diagrams are known, placing comments and font rules in the body before the first diagram. No shared rendering-trait hook was added. |
| Tests and maintenance | Two incorrectly tier-marked tests were renamed to run at Level 1. Three helpers adopted the shared manifest-directory macro; four benchmark-only paths were allow-listed. The macOS CoreText setup was extended to existing Darkmatter test modules. One JSON-order expectation was corrected. |
| Documentation | The unavailable CLI option remains marked planned. This follows the repository's documentation rule despite the plan checkpoint asking for no remaining planned markers. |

## Deferred work and reported verification limits

The command-line option and its two associated tests remain explicit unfinished scope, as described above. The author-approved exclusions—terminal changes, WOFF2, a frontmatter opt-in, and embedding fallback faces for uncovered characters—are not implementation deferrals.

The log also reports 42 Darkmatter JSON-order test failures reproduced on the unchanged commit: 40 schema-conversion snapshots, one constraint snapshot, and one CLI JSON-output test. The implementer left the ordering decision with the earlier work. This review has not independently verified those failures or classified them as findings; they should remain visible when the implementation review resumes.

After the author records the design rulings, the next review can compare the implementation against the resulting contract, examine verification at the appropriate levels, and produce the complete finding inventory. No source changes or implementation repairs were made in this review.
