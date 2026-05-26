# Comment Quality — Rubric and Cleanup

**Date:** 2026-05-25
**Status:** Draft
**Scope:** Rubric is repo-wide policy. Active cleanup targets `claudine/`
only in this spec; other areas adopt the rubric as they are touched.

## Goal

Establish a content-quality rubric for code comments and doc comments that
complements (does not replace) the existing structural conventions in
`CLAUDE.md` ("Rustdoc Convention"). Apply the rubric to `claudine/` in a
single cleanup pass. Ship a low-friction heuristic check that flags
regressions without blocking CI.

## Motivation

The existing `CLAUDE.md` "Rustdoc Convention" governs *structure*: no H1,
which H2 sections to use, what order they appear in. It is silent on
*content* — what to write inside those sections, when to omit them, and
which patterns to avoid. The result is that the structural rules are
followed but the resulting docs often:

- restate the implementation in prose ("Format: `<bg-orange-500>…`")
- attach examples that assert what the signature already guarantees
- carry `## Arguments` / `## Returns` blocks that duplicate field docs
- narrate exact format strings or color names that subsequently drift
  (`prompt_reporting/system_prompt.rs::render_system_prompt_header`'s doc
  still describes a `📔` glyph and a white-on-orange band that the code
  now emits as `■` and `<orange-500>`)
- pad field-accessor methods with one-line docs that say what the signature
  already says (`dispatch/loader.rs::RuntimeEventBinding` has four in a row)
- mark code "sections" with `// Protocol`, `// Model`, `// Cost` style
  comments where the next line obviously says the same thing
  (`stream/reporting.rs` has ten in a single function)

These patterns inflate file size, increase drift risk, and obscure the
comments that *do* carry load-bearing information.

## Non-goals

- **No changes to the existing structural conventions** in CLAUDE.md or in
  the user's global Rust documentation rules. H1/H2/section-order rules
  remain authoritative.
- **No automated rewriting.** The cleanup pass is manual and judgement-led.
- **No blocking CI.** The heuristic check emits warnings only and is run
  on demand via `just`.
- **No monorepo-wide cleanup in this spec.** Other areas adopt the rubric
  as a side effect of normal work.
- **No clippy lints.** Custom lints are expensive to maintain for this kind
  of content-level rule.

## The rubric

Eight anti-patterns to remove, five positive criteria to preserve.

### Anti-patterns — comments that fail the test

1. **HOW-narration in doc comments.** Prose that restates the
   implementation step-by-step. If a sentence describes *how* the code
   does something, delete it — the code already says that, and the prose
   will drift when the implementation changes.

2. **Tautological examples.** Docblock `## Examples` blocks whose
   assertion is guaranteed by the function signature.
   `assert!(header.contains("System Prompt"))` against a function whose
   body is `format!("System Prompt …")` adds no information.

3. **`## Arguments` / `## Returns` blocks that restate field docs.** If
   the parameter's type already documents the parameter (via its own
   doc comment), don't duplicate that prose in every consumer's doc.
   Use these sections only when the parameter has a constraint or
   semantic *not* expressed by its type.

4. **Format-string, color, or glyph narrations.** Do not quote literal
   format strings, ANSI colors, or emoji codepoints in prose. They
   drift. The narration sets up a contradiction the next time the code
   changes.

5. **Redundant docs on field-accessor methods.** A one-line doc that
   restates the method name (`/// Whether enabled.` on
   `fn enabled() -> bool`) adds no information. If the method's name and
   return type are self-evident, no doc is needed. Use
   `#[allow(missing_docs)]` at the impl block if `clippy::missing_docs`
   is the only reason the comment exists.

6. **Section-marker `//` comments inside functions.** Comments like
   `// Protocol` immediately followed by `extra.insert("protocol", …)`
   restate what the next line says. They obscure the comments that *do*
   add information.

7. **Heavy-setup doc examples.** A docblock example that needs ten or
   more lines of fixture construction to make one assertion becomes its
   own maintenance burden. Either link to a real test
   (`See [`tests/foo.rs::test_bar`])` or omit. Reserve doc examples for
   APIs where the example *teaches the type*.

8. **Stale comments past their code.** When a function's body changes,
   comments that describe its behavior must be re-evaluated in the same
   change. A comment that was accurate when written and is now wrong is
   worse than no comment — it actively misleads. The drifted
   `render_system_prompt_header` doc (still describing a `📔` glyph
   the code no longer emits) is the failure mode.

### Authoring discipline

Anti-pattern #8 is not just a pattern to spot in existing code — it is
also the discipline that keeps the rubric applied after this cleanup
lands. Any edit that changes a symbol's behavior must include a pass
over:

- the symbol's doc comment (`///` or `//!`),
- the surrounding module's doc, and
- any inline `//` comments inside the symbol.

If a comment still applies verbatim, leave it. If it has drifted, fix
it or delete it. Behavior-changing PRs that do not touch the relevant
comments are an invitation for drift and should be flagged in review.

### Positive criteria — comments worth their length

A comment earns its length when it carries information the code itself
does not express:

A. **Contract or invariant** not derivable from types — e.g.
   `atomic_write` documenting `last-rename-wins` semantics under
   concurrent writers.
B. **WHY a counter-intuitive choice was made** — e.g. an inline note
   that a particular workaround exists because an upstream API emits
   doubled newlines. These belong *at the line that does the surprising
   thing*, not in the surrounding function's doc.
C. **Semantics of complex return shapes** — e.g.
   `extract_frontmatter_text` documenting what `base_line` means in its
   `Some((yaml_text, base_line))` return.
D. **Hidden coupling or external surprise** — e.g. "this struct must
   serialize compatibly with `X`," or "this enum's discriminants are
   persisted to disk."

E. **Link to authoritative design.** Module-level (`//!`) and
   module-defining-type (`///`) docs should link to the design or topic
   doc that is the source of truth for that area, when one exists. Use
   intra-doc links or relative markdown paths so the link is reachable
   from both rustdoc and the source. **Per-function linking is not
   required** — module/type level is the right granularity. Per-function
   links become a maintenance treadmill of their own.

When in doubt, ask: *would deleting this comment lose information that a
future reader needs?* If no, delete.

## Where the rubric lives

Two places. Both ship as part of this spec.

1. **Repo-root `CLAUDE.md`** gains a new section, "Comment Quality,"
   placed immediately after the existing "Rustdoc Convention." Contents:
   the eight anti-patterns and five positive criteria as a tight bullet
   list (one line each), plus the authoring-discipline rule, with a
   one-line pointer to the detail doc. Target length: 30 lines or fewer.

2. **`docs/comment-quality.md`** is created at repo root. Contents: each
   anti-pattern and positive criterion expanded with a real before/after
   pair drawn from the codebase, plus the rationale for keeping the rule.
   Real examples (cited by file path) calibrate the rubric better than
   abstract advice — they show *exactly* what kind of comment is being
   targeted, so a reader does not over-correct and strip load-bearing
   docs.

## Cleanup pass

Scope: all `.rs` files under `claudine/lib/src/` and `claudine/cli/src/`.
Out of scope: `claudine/lib/tests/`, `claudine/cli/tests/`, and any
generated code.

Order of cleanup:

1. **High-density first** — files surfaced by the heuristic as having the
   largest concentration of long docblocks. The sweep that informed this
   spec identified `prompt_reporting/*` as the worst offender in
   `claudine/lib/`, followed by `dispatch/template.rs`,
   `dispatch/loader.rs`, `stream/reporting.rs`, `stream/badges.rs`, and
   `config/claudine_config.rs`.
2. **Then the remainder of `claudine/lib/src/`** in module order.
3. **Then `claudine/cli/src/`**, starting with the largest files
   (`main.rs`, `argv/*`, `completion/*`, `commands/*`).

Each cleanup commit:

- Touches one module or one cohesive area.
- Has a commit message that names the anti-patterns removed
  (e.g. `chore(claudine): strip HOW-narration and tautological examples from prompt_reporting`).
- Leaves the test suite passing without changes.
- Does not modify behavior — only comments and (where allowed)
  `#[allow(missing_docs)]` placement to compensate for #5.

`prompt_reporting/*` is included in the cleanup despite an imminent
encapsulation rewrite (see
`features/2026-05-25-prompt-reporting-encapsulation/spec.md`). Cleaning
the comments first makes the structural rewrite reviewable on its own
merits and means the functions that survive the rewrite carry the better
comments forward.

## Heuristic check

A `just check-comments` recipe runs a small script that flags suspicious
patterns. **Warn-only** — the script exits 0 even when it finds matches.
Not wired into the CI gate; intended for use during development and
review.

What it flags:

- `///` blocks longer than 15 lines attached to functions whose body is
  fewer than 10 lines (likely over-documentation).
- The literal string `## Arguments` inside `///` blocks (flag for review:
  is this restating field docs?).
- Doc-comment examples (` ```` ` blocks inside `///`) longer than 20
  lines (likely heavy-setup).
- Sequences of three or more `///` lines whose content is shorter than
  60 characters and attached to a `pub fn .*->\s*\w+\s*{\s*self\.\w+\s*}`
  pattern (likely redundant accessor docs).

What it does **not** try to flag:

- HOW-narration (requires semantic understanding).
- Format-string drift (would require parsing the doc and comparing to
  literals).
- Section-marker `//` comments (too many false positives).
- **Stale comments past their code (#8)** — fundamentally requires
  understanding what the code does vs what the comment claims. The
  authoring discipline is the only practical safeguard.
- **Missing links to design docs (Criterion E)** — too many
  reasonable cases where no topic doc yet exists; a heuristic would
  produce more noise than signal. Reviewer judgement applies.

These remain judgement calls for human review.

Implementation: a single Bash + `ripgrep` script at
`scripts/check-comments.sh`, invoked by a `just check-comments` recipe
in the root `justfile`. Output format: one line per finding, with file
path, line number, and the anti-pattern category. Output is parseable so
it can be filtered (`just check-comments | grep prompt_reporting`).

The script is intentionally minimal — false positives are acceptable
because the output is warn-only and reviewed by a human. False negatives
are also acceptable because the rubric, not the script, is the
authoritative policy.

## Risks

- **Over-correction.** A team member reads "delete HOW-narration" and
  strips a 22-line doc that captures atomicity contracts. Mitigation:
  the positive criteria in the rubric (and the worked good examples in
  `docs/comment-quality.md`) explicitly call out what to *keep*.
- **Wasted work on prompt_reporting.** The encapsulation rewrite will
  rewrite or delete many of the same functions. Accepted: the cleanup
  is small and the two-PR sequence is easier to review than one combined
  rewrite-and-comment-cleanup PR.
- **Heuristic false positives.** The accessor-doc regex in particular
  will catch legitimate cases (e.g. trait method docs that genuinely
  add semantic information beyond the return type). Mitigation: the
  script is warn-only and not in CI; humans triage.
- **`#[allow(missing_docs)]` proliferation.** Removing redundant
  accessor docs requires an `#[allow]` if `missing_docs` is enabled on
  the module. Mitigation: prefer placing `#[allow]` at the impl-block
  level rather than per-method.

## Acceptance criteria

- `CLAUDE.md` contains a new "Comment Quality" section (≤ 30 lines)
  immediately after "Rustdoc Convention," listing all eight anti-patterns
  and five positive criteria, plus the authoring-discipline rule, with a
  one-line link to `docs/comment-quality.md`.
- `docs/comment-quality.md` exists at repo root with one expanded
  before/after pair per anti-pattern and per positive criterion, each
  citing a real file path in the codebase. Criterion E's examples cite
  existing `docs/topics/*` documents where the link pattern already
  applies.
- High-traffic module-level (`//!`) docs in `claudine/lib/src/` link to
  their authoritative topic doc under `claudine/docs/topics/` where one
  exists (e.g. `composition`, `mcp`, `protect`, `system_prompt`,
  `stream`). No claim of completeness — this seeds the convention.
- All `.rs` files under `claudine/lib/src/` and `claudine/cli/src/` have
  been reviewed against the rubric and updated where applicable. No
  behavioral changes.
- `scripts/check-comments.sh` exists and is invokable as
  `just check-comments`. Running it produces no findings inside
  `claudine/` (or each finding is documented as an accepted exception).
- `cargo test -p claudine` and `cargo test -p claudine-cli` pass without
  modification.
- `cargo doc -p claudine` and `cargo doc -p claudine-cli` produce no
  warnings about broken intra-doc links.

## Out of scope (deferred)

- Applying the rubric to other workspace areas (`darkmatter`,
  `biscuit-terminal`, `homelab`, etc.). Those teams adopt the rubric as
  they touch their own code.
- Promoting `just check-comments` to a CI gate.
- Custom clippy lints for any of the eight anti-patterns.
- Cleanup of `tests/` directories. Test code follows different
  conventions and is not addressed here.
- **Spec-to-design-doc lifecycle convention.** A separate spec is needed
  to define how `features/YYYY-MM-DD-<name>/spec.md` transitions to a
  permanent `{area}/docs/topics/<name>.md` source-of-truth document
  after implementation completes, what happens to the historical spec,
  and how fixes (which usually do not change design) are distinguished
  from features (which often do). Criterion E above already names
  `docs/topics/` as the link target because those documents largely
  exist today; the lifecycle spec will formalize the convention.
- **Drift-detection tooling.** Cross-referencing Criterion E links
  against design-doc mtimes to surface candidate-drift sites is a
  useful future extension, but it is tooling — not policy — and
  needs the lifecycle convention to land first to be reliably useful.
