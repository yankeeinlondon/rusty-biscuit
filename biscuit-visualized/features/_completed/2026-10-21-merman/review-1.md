---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-23T15:01:31-07:00"
spec: 2026-10-21-merman/spec.md
feature: 2026-10-21-merman/review-1.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/log.md
description: "A **feature** review of `2026-10-21-merman/spec.md`"
ready: false
readiness_assessed: false
findings: []
recurrence: false
human_review: true
human_resolved: false
has_blocked_findings: false
blocked: false
human_in_the_loop:
    - |-
        **Review the current diagram appearance.** The specification explicitly
        requires your visual approval in this first review. Open the regenerated
        macOS and Linux galleries linked in the body and compare the diagrams
        with Zed screenshots. Each row shows one diagram type under four fixed
        themes, a terminal with a complete color palette, and a terminal whose
        background is the only known color.

        The implementation generated the expected-output snapshots before your
        approval, as authorized. Later readability fixes changed eleven diagram
        types, including C4 boundaries, kanban columns, and timeline sections.
        Review these current galleries rather than an earlier version. Sequence
        participants drawn as database or boundary symbols still share one
        accent; agentflow, usecase, and wardley have limited automatic theming.

        - Approve the current appearance: keep the existing snapshots.
        - Request changes to named diagram types: change their styling and
          regenerate their snapshots.
        - Reject the overall direction: requires broader styling work.

        Recommendation: review the current galleries and request specific
        changes where needed. This review does not substitute an agent's visual
        judgment for your approval.
    - |-
        **Choose whether to retain four departures from the styling rules.**
        The implementation reports that following the original rules left
        several diagram types unreadable on dark backgrounds. It shipped these
        changes without amending those rules:

        1. Pass palette colors through ordinary renderer configuration for
           diagram types whose dedicated theme interface cannot reach enough
           of their colors.
        2. Give class, block, and architecture boxes a background fill from the
           palette. These types have no rotating node-color palette for that
           fill to override.
        3. When the author's diagram supplies a theme color, disable the entire
           compiled theme and apply the author's colors over palette defaults.
           For example, changing one author color can also change other
           automatic styling, rather than replacing only that color.
        4. Identify sequence participant boxes by their name attribute when
           assigning accents, because their paint comes from a stylesheet
           rather than an inline palette color.

        - Accept all four and record the revised contract: keeps the current
          appearance and author-color behavior.
        - Accept selected changes: specify which to retain; reverting others
          restores the documented rendering limitations.
        - Keep the original rules: revert these changes or implement another
          approach that satisfies the original contract.

        Recommendation: retain the dark-background fixes and the participant
        matching approach. Explicitly confirm the whole-theme response to an
        author's single color override, since that changes visible behavior
        beyond the overridden key.
    - |-
        **Decide whether unreadable sequence message text must be fixed now.**
        The log reports that a dark-themed sequence diagram with an
        author-specified light participant box can have message text with
        contrast of only 1.07–1.50 against that box, below the specified 4.5:1.
        The box color is the author's, but the unreadable text color is supplied
        by this library. This differs from preserving an author's deliberately
        chosen text color. The implementation exempts this case from its
        contrast check and leaves the repair for later.

        - Fix it in this feature: preserve the readability requirement. Start
          with a repair limited to this sequence-box case; a general system
          for matching every text label to a drawn region may be unnecessary.
        - Accept a documented exception: explicitly narrow the contract and
          retain this limitation until a separate repair lands.

        Recommendation: fix the demonstrated case now, using the smallest
        repair that preserves the author's box color and makes library-chosen
        text readable. Accepting a deferral is a scope decision, not evidence
        that the current contrast requirement was met.
    - |-
        **Decide what to do about the exceeded render-time limit.** The log
        reports warm-render medians of about 56 ms for sequence diagrams and
        84 ms for timeline diagrams on the designated Linux reference host.
        The specification sets a 50 ms median limit for every diagram type.
        The local benchmark therefore currently exits with failure on that
        host. Recording these timings as a baseline does not make them meet
        the absolute limit.

        - Keep the limit and improve these two render paths: preserves the
          agreed performance contract and requires more implementation work.
        - Accept explicit exceptions for these two types: preserves the limit
          for other types but permits the reported overruns.
        - Raise the general limit, for example to 100 ms: simple, but permits
          slower rendering for every type.

        Recommendation: preserve the general limit. Either require targeted
        improvements now or explicitly approve narrow exceptions; do not
        silently treat the baseline as an exception to the specification.
    - |-
        **Confirm how the remaining verification and dependency gates close.**
        The final phase ran the complete biscuit-visualized Level-1 suite and
        selected affected tests in dependent packages, instead of every full
        suite listed in the plan. The log still reports 45 Darkmatter test
        failures caused by the renderer dependency changing JSON key order,
        and an unresolved worktree Kitty terminal test. Restoring Darkmatter's
        key ordering was already assigned to the separate
        `2026-10-23-frontmatter-key-order` repair; that decision needs no new
        approval. Its completion still matters to this feature's requirement
        that dependent packages pass.

        - Retain the scoped local verification and require the existing
          dependent-package and terminal checks to pass before declaring the
          implementation ready.
        - Require the plan's full local verification sweep as well: adds local
          work but provides the originally planned evidence before CI.

        Recommendation: retain scoped local verification, with the existing
        passing-test gates intact. Neither this record nor the earlier scope
        decision establishes that the outstanding failures are repaired.
        Cross-OS evidence itself is left to CI, as requested.
    - |-
        **Keep the newly discovered graph-cache defect in the repair inventory.**
        The implementation log reports that non-Mermaid graph images use the
        shared font chain, but their cache key does not identify those fonts.
        Changing MERMAID_FONT or installed fonts can therefore return an old
        cached graph with the previous font. This was discovered during the
        implementation and left unfixed; it is not an already approved
        exclusion from scope.

        - Repair it in the subsequent review/fix work: add the font identity
          to the graph cache key and invalidate older graph cache entries.
        - Explicitly accept it as a separate repair: stale images remain
          possible until that repair lands.

        Recommendation: repair it in the next fix cycle rather than drop it
        from the inventory. This first-stage review records the reported
        defect; it has not independently reproduced or classified it as a
        numbered code-review finding.
human_review_items:
    - |-
        Review and approve the current diagram galleries beside Zed screenshots,
        or list the diagram types and visual changes you want. The first entry
        in human_in_the_loop gives the gallery context and options. The other
        entries record contract and scope choices needed before code review
        resumes; they are not additional visual approval requests.
next: 2026-10-21-merman/review-2.md
---

# First review: human decisions required

This review stops at the decision stage requested for the first review cycle.
Two separate reviewers examined the specification and implementation log. Both
found meaningful choices that need the author's judgment, and the specification
expressly moves gallery approval to this review.

**Production readiness has not been assessed.** No comparison of the code with
the specification, defect sweep, or test execution was performed. No numbered
findings have been issued. The metadata records `ready: false` conservatively
to avoid declaring an unreviewed implementation production-ready;
`readiness_assessed: false` distinguishes this early stop from a completed
review with findings. Human review is not being classified as a code defect.
The spec's completion flag is not changed.

The review instructions give conflicting types for `human_in_the_loop`: the
detailed early-stop process and repository schema require a list, while a later
paragraph requests a boolean. This record uses the list required by the schema,
so the decisions are preserved and the review loop can recognize the stop.

## Decisions for the author

The six frontmatter entries contain the requested decisions, options, effects,
and recommendations. They cover the current appearance, four styling-rule
departures, the unreadable sequence-box exception, the performance overruns,
the remaining verification gates, and the newly deferred graph-cache repair.

Start the visual review with these artifacts:

- [Current macOS gallery](gallery/themed-gallery-macos.html).
- [Current Linux gallery](gallery/themed-gallery-linux.html).
- [Implementation decision list](implementation-log.md:291).
- [Specification's first-review approval rule](spec.md:1534).

The appearance approval must include the later readability changes. The log's
initial appearance request still discusses generating snapshots in the future,
but its later entries say the snapshots and galleries have already been
regenerated. This review uses the later state; no approval is inferred from
snapshot generation.

## Implementation choices disclosed for information

These choices do not independently justify another human decision. They remain
subject to the subsequent implementation review; the descriptions below report
the log rather than independently verified code behavior.

| Choice | Effect and review disposition |
| --- | --- |
| Layout before cache lookup for tagged gitGraph diagrams | Finding tag collisions requires Merman's layout. A tagged diagram pays for that layout even on a cache hit; the log reports about 13 ms in a 100-commit debug example. Recorded in the spec during implementation. |
| A separate time budget for each Merman request | A tagged gitGraph can perform a layout request and an SVG request, spending up to twice the configured budget in Merman. Disclose this operational limit; the code review should check the exact promised contract. |
| Math labels fall back to the complete source diagram | With math disabled, Merman rejects the whole diagram. The implementation uses the existing code-block fallback rather than rewriting math syntax. Recorded in the spec during implementation. |
| Explicit ZenUML fallback | Merman can render it, but the implementation preserves the specification's source-block behavior. |
| Size queries populate the SVG cache | Computing natural size renders and caches an artifact, which a later render can reuse. |
| Stable diagram identifiers derived from the cache key | Identical diagrams can share an identifier and stylesheet; embedding behavior remains for the later code review to assess. |
| Tag clearance uses the theme font size | The one-em spacing follows existing documentation and the geometry experiment. |
| Snapshot tests inject a bundled-only Inter font chain | Avoids process-global environment races and installed-font differences; PNG snapshots use 1× scale to reduce size and runtime. |
| Tag font sizes follow Merman's interpretation | The final phase repaired spacing for values such as `150%`, added comparison tests, and changed the cache version. This preserves the intended non-overlap behavior. |
| Readability adjustments | Mindmap fills, Neutral accents, and eleven diagram types changed to make library-selected text readable. Included in the current visual approval request. |
| Preserve author-selected text colors | The contrast pass can change library-selected text on author fills, while preserving author-selected text on author fills. The unresolved sequence-box case is separately disclosed above. |
| Benchmark details | Reference host identified by hostname or an explicit flag; cold limit judged on median; a missing family baseline flags on the reference host; an empty family map is accepted. These are implementation choices, not approval to exceed the speed rule. |
| Documentation and license-file placement | Keeping the separate gitGraph topic page was allowed by the spec. Existing fixture license filenames were retained. Neither choice changes the feature contract. |

See the log's [renderer choices](implementation-log.md:1403),
[styling choices](implementation-log.md:1912), and
[final decision inventory](implementation-log.md:291).

## Deferred work and previously approved boundaries

The distinction matters: scope already approved by the author does not need a
second ruling, while newly deferred failures cannot be treated as approved just
because the implementation reached its final phase.

| Work or limitation | Current disposition reported by the log/spec |
| --- | --- |
| HTML font embedding | Already separate work: `2026-10-21-mermaid-embedded-font`. Reader fonts may still differ from the fonts used to measure a static SVG. |
| Text-art fallback | Already separate work: `2026-10-22-merman-text-fallback`. |
| Darkmatter key ordering | Already assigned to `2026-10-23-frontmatter-key-order`; the dependent-suite acceptance gate remains open until repaired. |
| Return from the Merman fork to upstream | Already conditional on upstream accepting equivalent tag-spacing support. |
| Upstream issues and tag-spacing pull request | Six issue drafts and a pull-request draft are prepared. Filing/opening them belongs to the author; an agent must not publish them. |
| Upstream label defects | Wrapped swimlane labels, ER attribute comments containing line breaks, and the specified unsupported backslash-newline cases remain documented upstream limitations. These were already known to the spec. |
| Windows color detection | Already a release prerequisite, not a new design choice in this review. Its release condition must remain visible. |
| Sequence symbol accents and limited theming in three types | Included in the visual approval request rather than silently treated as approved appearance. |
| Sequence-box readability and sequence/timeline speed | New deferrals requiring the explicit scope choices above. |
| Non-Mermaid graph cache missing font identity | Newly reported, unfixed defect; retained in the decision inventory above. |
| Stale Darkmatter Mermaid comments | Reported unfixed references to old rendering services and icon behavior. Needs follow-through in the later review/fix inventory. |
| Root dependency-document links | Log reports 58 pre-existing broken workspace links, left unrepaired. No independent link audit was performed in this pass. |
| biscuit-terminal tests with all features | Log reports a compilation failure left unrepaired. No independent reproduction was performed in this pass. |
| worktree Kitty terminal test | Log reports an unresolved real-terminal test. Its cause and required repair remain to be established in the implementation review. |

See the [final open acceptance criteria and follow-up list](implementation-log.md:2720).
The log also contains earlier author rulings on font-cache identity, native
Windows palette queries, the renderer and fork, and moving human decisions to
this first review. Those are existing author decisions, not requests to approve
them again.

## Findings and continuation

No code-review findings were produced in this decision-only pass. There is no
previous review inventory to carry forward or mark repaired.

After the author records the decisions under `## Human rulings` and sets
`human_resolved: true`, the implementation review can assess the agreed
contract, reproduce the reported defects, sweep their sibling sites, and check
the required verification levels. Approval of a deferral must be explicit;
silence does not amend the specification.
