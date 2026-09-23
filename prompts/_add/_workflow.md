---
description: |-
    Shared process partial for the `_add/*` prompts. The parent document supplies `kind`
    (singular noun), `kinds` (plural), `catalog_command` (the `claudine context` report that
    lists the catalog being extended), `spec_dir` (where the feature spec is written), and
    `requirements` (the caller's raw ask). Do not set those keys here: a child's hardcoded
    frontmatter overrides the parent's during transclusion.
---
## How to Work

- You are in an **interactive** session. The caller is present and expects to be asked
  questions when their requirements are unclear, but they also expect you to make routine
  calls yourself. Ask when different readings would produce materially different work;
  otherwise decide, state the assumption, and continue.
- Act as an **orchestrator**. Do the intake, clarification, and specification yourself so the
  contract lives in your context; delegate implementation tracks, test writing, and
  documentation sweeps to subagents with the spec as their brief.
- Before editing any existing symbol, run GitNexus upstream impact analysis and record the
  callers and risk. `HIGH`/`CRITICAL` means stop and tell the caller before editing;
  `UNKNOWN` means confirm with a text search, never treat it as low.
- Never run `cargo fmt`. Never commit. Your terminal state is "implementation complete, ready
  for review" with the spec's frontmatter updated as described in the Closure section.

## Phase 1 — Intake

1. Read the caller's requirements above and restate them in your own words as a numbered list
   of **candidate {{kinds}}**, one item per name. Give each item a working name that follows
   the catalog's naming style (snake_case, no abbreviations, US English).
2. Run `{{catalog_command}}` and check every candidate against the existing catalog:
    - an exact name collision is a **change** to an existing entry, not an addition, and needs
      the caller's explicit confirmation;
    - a near-duplicate (same fact under another spelling, or an existing entry that already
      answers the need with a fallback such as `{{{ ctx.x || 'y' }}}`) should be
      raised as an option: extend or reuse rather than add.
3. For each candidate, record what you still do **not** know using the checklist in the
   "Definition Checklist" section below. That list is your clarification agenda.

## Phase 2 — Clarify

Work the agenda one topic at a time, most consequential first (the topic that most changes the
shape of the implementation goes first). For every topic:

- explain the topic in plain language first; assume a technical caller who does not know this
  repo's symbols, so give context before the question;
- offer **2–4 concrete options**, each with pros and cons, and always leave room for "other";
- **recommend one** and say why: what you assumed, which pro/con weighed most, and how much
  weight you gave simplicity over the theoretically best answer;
- when the caller picks "other", check the answer is unambiguous and ask a follow-up if it is
  not.

Stop clarifying when every item on the Definition Checklist is answered for every candidate,
or when the caller says the remaining questions are yours to decide. Record decisions as
numbered rulings (`R1`, `R2`, …) with the date; they go into the spec verbatim.

Do not convert an open question into a decision on the caller's behalf. If a question is
genuinely a design choice with no obvious default and the caller declines to rule, write it
into the spec's "Open Questions" section with your recommendation and do **not** implement it.

## Phase 3 — Specification

Write the spec to `{{spec_dir}}/spec.md` before touching code. It follows the repo's feature
conventions (a dated directory under the package area's `features/`, `spec.md` inside) and
has these sections:

1. **Summary** — one paragraph: what is added and why.
2. **Scope** — in scope / out of scope, including which packages change
   (`darkmatter`, `sniff`, `claudine`, `claudine-gen`).
3. **Definitions** — one subsection per {{kind}}, filled from the Definition Checklist.
   Every value shape, failure value, and example must be stated, not implied.
4. **Rulings** — the numbered `R…` list from Phase 2.
5. **Acceptance Criteria** — numbered `AC…` items, each testable, each tagged with its test
   level (L1 unless it needs a real terminal or a real provider run). Include the parity
   tests by name, the catalog report check, and at least one end-to-end composition through
   the normal invocation path.
6. **Documentation** — the exact files that must agree with the spec at completion.
7. **Blast radius** — the GitNexus impact results for every existing symbol you will edit.
8. **Open Questions** — anything left unruled, with your recommendation.

Set the spec's frontmatter: `created: {{ctx.today}}`, `status: draft`,
`clarified_by: "{{ctx.agent}}/{{ctx.model}}"`, and `requirements` holding the caller's
original text verbatim (use a `|-` block scalar).

Show the caller the Definitions and Rulings sections and get a **go** before Phase 4.

## Phase 4 — Implement

- Work in the order the "Wiring Recipe" section above prescribes; that order exists because
  the parity tests are red between steps and the recipe tells you which red is expected.
- Land the shared contract first (descriptor entry, group or binding registration), then
  fan out independent tracks (capture or handler, Claudine evidence, docs) to subagents.
- Every {{kind}} gets: a positive test, a "not available" test proving the documented empty
  or null value, a wrong-input test where inputs exist, and a test through the normal
  composition path (`md compose` on a fixture, or the library `compose_with` API). Tests must
  be hermetic: fixture repositories, fixture environments, injected evidence, stub probes.
  Never read the developer's real dotfiles, HOME, or repository state.
- Consider macOS, Linux, native Windows, and WSL2 for every path, environment, or process
  decision; use the `os` skill before claiming a platform cannot be tested here.
- Keep the change surgical. Do not tidy adjacent code, comments, or formatting.

## Phase 5 — Documentation and Drift

Documentation is part of the deliverable, not a follow-up. Update, in the same change:

- every file listed in the spec's Documentation section;
- the transcluded docs you read at the top of this prompt, wherever they disagree with the
  code you just wrote or found. **When a doc and the code disagree, the code is right**: fix
  the doc and say so in your report;
- `docs/dependencies.md` at the repo root and in each changed package area when a crate or a
  crate feature was added or removed;
- the `darkmatter` and `claudine` skills under `.claude/skills/` when the change alters how
  composition, capture, evaluation, or the `claudine context` reports behave;
- any modified Markdown file that carries a `hash:` frontmatter property: refresh it with
  `md hash <file>` and write the result back.

## Phase 6 — Verify

Run, from the package areas you changed:

```sh
cd darkmatter && just build && just test && just lint
cd claudine   && just test && just lint      # when Claudine changed
cd sniff      && just test && just lint      # when Sniff changed
```

Then prove the catalog surface end to end:

- `{{catalog_command}}` lists every new entry exactly once, in the intended category;
- `claudine context --values` shows the observed value when this host can observe it;
- `md compose <fixture.md>` composes a fixture that uses each new entry, and the output
  shows the expected value rather than a leaked `{{{ … }}}` span.

Do not run workspace-wide Cargo gates for a package-scoped change. Do not weaken or skip a
red test to get green; report it.

## Closure

Update `{{spec_dir}}/spec.md` frontmatter: `implemented: true`,
`implemented_by: "{{ctx.agent}}/{{ctx.model}}"`, `status: "implemented, ready for review"`,
and `human_review_items` when a design decision was taken without a ruling (write each item
as a `|-` block so it reads well; explain it without repo jargon).

Report to the caller, in this order: what was added (names and shapes), what was ruled, what
was left open, which docs were corrected for drift, the exact test and lint results, and
anything a reviewer should look at first. Do not move the spec to `_completed`; that is the
author's call after review.
