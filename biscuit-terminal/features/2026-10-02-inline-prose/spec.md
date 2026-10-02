---
area: biscuit-terminal
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
created: 2026-10-02
owner: Ken Snyder <ken@ken.net>
packages:
    - biscuit-terminal
    - biscuit-terminal-cli
    - renderable
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
    - claudine-gen
    - sniff-cli
    - messenger-cli
    - worktree-cli
    - model-citizen-cli
    - playa-cli
    - homelab-cli
depends-on:
    - 2026-10-02-prose-inline-code
human_review: false
clarified: false
reviewed: false
needs_rulings: true
---

# `Prose` Is a Block, `InlineProse` Is Inline

## Problem

A component is either block content or inline content, and it must be the
same one on every target. Prose is not: its shape depends on the target
and on the caller.

| Path | Used by | Shape |
|---|---|---|
| `Prose::render_tree()` | Markdown, terminal | block: `Root` → `Paragraph` + `Code` blocks |
| `Prose::render_html_fragment()` | browser | inline: nodes joined inside `<span class="prose">` |
| `Prose::to_render_nodes()` | `Table`, lists, block quotes, `TwoColumn`, render-tree projection | inline nodes, folded or degraded by each container |

Consequences today:

- **Paragraphs differ by target.** Prose never interprets newlines. A `\n`
  stays inside a text node, and each renderer treats it differently:

  | Input | Terminal | Markdown | HTML |
  |---|---|---|---|
  | `a\nb` | two lines | soft break, one paragraph | `a b` |
  | `a\n\nb` | blank line between | two paragraphs | `a b` |

- **Invalid HTML.** A fenced code block renders as `<pre><code>…` inside the
  `<span class="prose">`, and `<pre>` is not allowed inside `<span>`.
- **Layout is split.** `render_html_fragment` drops Prose's `Layout`; the
  `bt prose` CLI wraps the fragment in its own `<div style>` to get it back.
  This was recorded as an interim contract on 2026-06-04.
- **Each container decides.** Lists and block quotes fold Prose into
  paragraphs with `fold_prose_nodes_into_blocks`; `Table` degrades fenced
  code with `degrade_code_nodes`; `TwoColumn` groups text into paragraphs.
  The caller cannot say which shape it means.

About 980 non-test call sites construct `Prose`. An audit estimates
(±15%) that about 60% use it inline: table headers and cells, list items,
labels, badges, and strings pre-rendered with `.render(term)` and placed in
a container or `format!`. The rest are block output: `println!` messages,
darkmatter error bodies (`StatusBlock::body`), compose sections.

## Expected Behavior

Two components share one grammar.

| | `InlineProse` | `Prose` |
|---|---|---|
| Content model | phrasing only | one or more blocks |
| Grammar | tags, `**`/`_`, links, code spans, escapes, soft and hard breaks | `InlineProse` paragraphs, separated by blank lines, plus fenced code blocks |
| Render tree | phrasing nodes | `Paragraph` and `Code` blocks under a `Root` |
| Markdown | inline text, no trailing paragraph break | paragraphs and fenced blocks |
| Terminal | a styled run, no block layout | blocks, with `Layout` (margins, alignment, wrap) |
| Browser | phrasing HTML: `<strong>`, `<em>`, `<a>`, `<code>`, … | one element per paragraph (default `<p>`), `<pre><code>` for code, `Layout` as CSS |
| Typical use | table cells, list labels, badges, a value inside a line | messages, error bodies, document sections |

```rust
let label = InlineProse::new("Run `md hash` on [the plan](plan.md)");
let body = Prose::new("First paragraph.\n\nSecond paragraph,\nsame block.");
```

```html
<!-- label -->
Run <code>md hash</code> on <a href="plan.md">the plan</a>
<!-- body -->
<p>First paragraph.</p>
<p>Second paragraph, same block.</p>
```

### Newlines follow Markdown

| Input | Meaning |
|---|---|
| a single `\n` | soft break inside the block |
| two or more `\n` in a row, with only spaces or tabs between them | block boundary (`Prose`) |
| `\` immediately before `\n`, or two or more spaces before `\n` | hard break (Open Question 1) |

`\n\n` and `\n\n\n\n` are the same boundary. A soft break renders as a
space on the terminal and in HTML, and as a newline in Markdown; this is
how the shared renderers already treat `NodeKind::SoftBreak`. A hard break
renders as a newline on the terminal, `<br>` in HTML, and a Markdown hard
break.

Leading and trailing blank lines produce no empty blocks.

### `InlineProse`

- Holds phrasing content only. Its `render_tree()` returns phrasing nodes,
  which the render-tree validator already permits at the top level.
- Owns the inline projection that `Prose::to_render_nodes()` provides today.
- Implements `TerminalRenderable`, `MarkdownRenderable`,
  `BrowserRenderable`, and `TreeRenderable`.
- Has no `Layout`. Positioning inline content is its container's job.
- A blank line in `InlineProse` is a soft break (Open Question 2).
- A fenced code block in `InlineProse` degrades to inline code, as `Table`
  cells already do with `degrade_code_nodes` (Open Question 3).

### `Prose`

- Is a sequence of blocks: paragraphs (each an `InlineProse`) and fenced
  code blocks.
- Carries `Layout` and applies it on every target: terminal margins,
  alignment, and wrap as today; CSS on the browser. The browser fragment
  is rendered from `render_tree()` like the other targets.
  `<span class="prose">` is removed, and the `bt prose` CLI stops adding its
  own layout wrapper.
- Has a configurable block tag, `ProseTag`, used for each paragraph's HTML
  element:

  ```rust
  Prose::new("Note text").with_tag(ProseTag::Div)
  ```

  | `ProseTag` | HTML |
  |---|---|
  | `P` (default) | `<p>` |
  | `Div` | `<div>` |
  | `Section`, `Article`, `Aside`, `Header`, `Footer` | the matching element |

  The tag is browser presentation. Markdown and terminal output are the
  same block shape whatever the tag. Elements that already have a node
  kind and component (`blockquote`, `pre`, `li`, `h1`–`h6`) are not tags;
  use their components. Code blocks are always `<pre><code>`.

  The render tree has no way to choose a block's element today.
  `renderable` gains a typed block-element field in `BrowserAttrs`, valid
  only on `Paragraph`, which the browser renderer honors and the
  validator rejects elsewhere.

### Containers

Each container states which shape it accepts:

| Container | Accepts | Why |
|---|---|---|
| `Table` cells | `InlineProse` | GFM table cells are phrasing content |
| `InlineContent` | `InlineProse` | inline by definition |
| `UnorderedList`/`OrderedList` items | `Prose` | a Markdown list item holds blocks |
| `BlockQuote` | `Prose` | holds blocks |
| `TwoColumn` columns | `Prose` | each column is a block region |
| `StatusBlock::body` / `body_line` | `Prose` | already joins items as paragraphs |

The per-container folding and degrading (`fold_prose_nodes_into_blocks`,
`degrade_code_nodes` on Prose content) is replaced by the components' own
shapes. `IntoProseVec` stays for `StatusBlock` if still needed.

### Migration

Each call site moves to the component that matches its use. Inline sites
change `Prose` to `InlineProse`; block sites keep `Prose`. About 35 sites
use a single `\n` as a line break and must change to a hard break or a
blank line, otherwise their lines join. Known clusters:

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (13
  `<dim>Key:</dim> value\n…` sites) and `shell_blocks/types.rs`;
- `darkmatter/lib/src/markdown/errors/blocks.rs:192` (YAML excerpt lines);
- `claudine/cli/src/output/error_report.rs`, `commands/help.rs`,
  `commands/logs/errors.rs`, `commands/hooks/list.rs:430` (a line break in
  a table cell, so a hard break in `InlineProse`);
- leading or trailing `\n` used for spacing (`claudine/lib/src/render/prompt/`,
  `sniff/cli/src/commands/repo.rs:439`, `worktree/cli/src/commands/create.rs:168`),
  which becomes either nothing or explicit spacing outside the component.

Content that already uses `\n\n` as a paragraph break keeps its meaning.

## Scope

**In scope**

- The two components, the shared grammar, newline handling, `ProseTag`,
  `Layout` on every target, and removal of `<span class="prose">`.
- The `renderable` block-element attribute.
- Container API changes in the table above.
- Migrating every call site and updating moved snapshots.
- `biscuit-terminal/docs/components/prose.md` (and a new page or section
  for `InlineProse`), the `biscuit-terminal` skill, and the `bt prose` CLI
  docs.

**Out of scope**

- The inline-code projection; that is `2026-10-02-prose-inline-code`, which
  lands first and is inherited by both components.
- Call sites that pre-render Prose to a `String` before handing it to a
  container. They migrate by type only; moving them to pass the component
  itself is worthwhile but separate.
- Other Markdown block syntax (headings, lists, block quotes) inside
  `Prose`. Those are components of their own.

## Decisions

- **One shape per component on every target.** `Prose` is block,
  `InlineProse` is inline. (Ken, 2026-10-02)
- **`Prose` has a configurable block tag**, default `<p>`, changeable to
  `<div>` and other block elements. (Ken, 2026-10-02)
- **`Prose` applies `Layout`** on every target. (Ken, 2026-10-02)
- **Markdown newline semantics.** A single `\n` does not break a block; a run
  of two or more `\n` does. (Ken, 2026-10-02)

## Open Questions

1. **Hard-break syntax.** **Recommendation:** support CommonMark's two
   forms, `\` before the newline and two or more trailing spaces. The
   backslash form is visible and survives editors that strip trailing
   whitespace, so migrated call sites use it.
2. **A blank line inside `InlineProse`.** It cannot start a block.
   **Recommendation:** treat it as a soft break, so the content stays valid
   and nothing is lost. Alternative: a hard break.
3. **A fenced code block inside `InlineProse`.** **Recommendation:** degrade
   to inline code, matching what `Table` cells do today. Alternative: leave
   the fence as literal text.
4. **What `ProseTag` means with several paragraphs.** **Recommendation:**
   the tag is each paragraph's element (`<div>…</div><div>…</div>`). This
   is the only reading that works for the default, since `<p>` cannot
   contain another `<p>` or a `<pre>`, and it keeps every tag behaving the
   same way. Alternative: the tag wraps all blocks, with paragraphs inside
   always `<p>`; this suits `section`/`article` better but makes `p` a
   special case.
5. **Rollout.** **Recommendation:** land the components and container
   changes first, with `Prose` keeping today's newline behavior; then
   migrate call sites one package area at a time; then switch on Markdown
   newline semantics in the same change as the last migration.
   Alternative: change semantics at once and fix every area in one pass.

## Acceptance Criteria

1. `InlineProse::new("Run `md hash` on [the plan](plan.md)")` renders:
   - Markdown: the input unchanged, with no trailing blank line;
   - browser: ``Run <code>md hash</code> on <a href="plan.md">the plan</a>``,
     with no wrapping element;
   - terminal: one styled line with no margin.
2. `Prose::new("a\nb")` is one paragraph with a soft break on every
   target: Markdown `a\nb`, HTML `<p>a b</p>`, terminal `a b`.
3. `Prose::new("a\n\nb")`, `"a\n\n\nb"`, and `"a\n  \nb"` each produce two
   paragraphs on every target.
4. A hard break, in the syntax ruled for Open Question 1, renders as a
   terminal newline, `<br>`, and a Markdown hard break.
5. `Prose::new("x").with_tag(ProseTag::Div)` renders `<div>x</div>` in HTML
   and is unchanged in Markdown and on the terminal; the validator rejects
   the block-element attribute on any node but `Paragraph`.
6. A `Prose` with a fenced code block renders `<p>` and `<pre><code>` as
   siblings, never `<pre>` inside an inline element.
7. A `Prose` with a left margin renders that margin in terminal and HTML
   output; `bt prose --margin-left` output carries it exactly once.
8. No output anywhere contains `class="prose"`.
9. Each container in the Containers table accepts its stated type, and
   `fold_prose_nodes_into_blocks` / `degrade_code_nodes` no longer run on
   Prose content.
10. Every listed single-`\n` call site keeps its line structure after
    migration, verified by its existing snapshot or a new one.
11. `just test` and `just lint` pass in every package area listed in
    `packages`.

## Definition of Done

- All acceptance criteria are met and every Open Question has a ruling
  recorded under Decisions.
- `docs/components/prose.md` describes both components, the newline rules,
  `ProseTag`, and `Layout`; the interim-contract doc comments on
  `Prose::render_html_fragment` and the CLI's `render_html_with_layout` are
  removed with the code they describe.
- The `biscuit-terminal` skill names `InlineProse` and when to use it.
- Every moved snapshot was reviewed for an unexpected change, not accepted
  in bulk.
