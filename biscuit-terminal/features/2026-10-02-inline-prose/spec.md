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
human_review: false
message_to_agent: |-
    Phase 5 (darkmatter `code_link()`) is done; see "## Phase 5" in
    implementation-log.md. Facts for Phase 6+:
    - `code_link(file)` / `code_link(target, desc)` (alias `codelink`) exist
      in darkmatter, the catalog (order 78), DMLS completion, and `claudine
      context --expressions`. The four templates are migrated. The manual
      step for Ken is still open: `~/.claudine/prompts/plan.md` and
      `~/.claudine/prompts/_reviews/review-spec-inline.md` still use
      `` `{{link(x)}}` ``; replace with `{{code_link(x)}}`. Phase 8 must list
      this in the final report.
    - Phase 6 doc work: `biscuit-terminal/docs/components/prose.md:93` still
      describes the removed "code span holding exactly one link is a link"
      rewrite and recommends `` `{{link(plan)}}` ``; rewrite it to say a code
      span is opaque and point at `code_link()`.
    - Phase 7 snapshot ledger: `error_snapshots::link::missing_href` and
      `error_snapshots::image_ref::*` moved for the same reason as
      `unrecognized_format` (backticks around a code span are gone under ANSI
      strip). They were left unaccepted because they are Phase 7's. Their
      `.snap.new` files are untracked and regenerate on any run.
    - Known-red after Phase 5: darkmatter area 49 fail + 2 timeouts. The
      second timeout is `entry_point_parity` under load; it passes alone.
      claudine `nested_span_error_renders_property_literal_rewrite_and_escape_hint`
      is an AC 17 `escape_text` site (line 15, `success.say`).
    Phase 4 facts (still current):
    biscuit-terminal `just test`/`just lint` are green and the whole workspace
    compiles (`cargo check --workspace --all-targets`).
    Facts for Phase 5+:
    - Containers: table cells, `TableColumn::header_prose`, and InlineContent
      take `InlineProse` only (no `From<Prose>`; variant
      `TableCellContent::StyledInlineProse`, hint token `styled_inline_prose`;
      `InlineContent::add_inline_prose`). Lists, BlockQuote, TwoColumn,
      StatusBlock body, Section and Compose embed `Prose` as its own blocks via
      `Prose::embedded_nodes()` (prose/tree.rs), special-cased in
      `render_tree/projection.rs`; no nested Root; layout moves onto the Prose
      blocks (never the container node). `Prose::is_block_level()` is true.
    - StatusBlock body is now STRUCTURAL (each item's blocks, inline styling,
      and its LineBreaks mode), so Phase 7's `LineBreaks::Hard` on single-`\n`
      body sites will take effect; header and hint are still plain text.
    - Compose inserts a "\n\n" Text between blocks of one Prose part.
      Trailing-`\n` Prose parts no longer produce a newline: move the newline
      to `.add_text("\n")` (done for three lib tests).
    - Four downstream cell sites were migrated type-only so the workspace
      compiles: darkmatter-cli schema/about.rs `prose_cell`, biscuit-icon-cli
      (commands.rs x2, sets_table.rs), claudine-cli steer/render.rs `cell()`.
      biscuit-icon-cli is a consumer missing from this spec's packages list.
    - Known-red unchanged from Phase 3: darkmatter 50 fail + 1 timeout (2
      pre-existing `current_root_*`), claudine 33 fail. NOTE: `just test` in
      claudine ran only 7359 tests in this worktree; measure claudine with
      `cargo nextest run -p claudine -p claudine-cli -p claudine-gen
      -p claudine-contract -p claudine-catalog-types --no-fail-fast` (8045).
    - Phase 5 can rely on `renderable::markdown::code_span` (Phase 2) and on
      darkmatter compiling; darkmatter snapshot reds remain Phase 7's.
clarified: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-02
review_iterations: 0
needs_rulings: false
---

# `Prose` Is a Block, `InlineProse` Is Inline, and Code Spans Are Code

## Problem

This feature fixes two defects in Prose that change the same code, the same
documentation, and the same snapshots, so they land together: Prose's
shape (block or inline) depends on the target, and Prose has no notion of
inline code.

### Block or inline depends on the target

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

### Code spans are not code

Prose has no notion of inline code. A code span is made opaque during
Markdown pre-processing (`escape_code_spans` in
`lib/src/components/prose/markdown.rs`) and then reaches the render tree as
ordinary `Text`, backticks included. Every target therefore shows the
backticks literally and nothing else:

| Input | Terminal | Markdown | HTML |
|---|---|---|---|
| `` See [`inline-block`](u) `` | OSC 8 link, label is plain `` `inline-block` `` | unchanged | ``<a href="u">`inline-block`</a>`` |
| `` See `plain_code` `` | `` `plain_code` `` as plain text | unchanged | `` `plain_code` `` as plain text |

The render tree already has `NodeKind::InlineCode`, and every shared renderer
handles it: the terminal renderer (`render_tree/render.rs`) styles it dim, or
with the prose theme background when the caller supplies one; the browser
renderer (`renderable/src/tree/render/browser.rs`) emits `<code>`; the
Markdown renderer (`renderable/src/tree/render/markdown.rs`) emits a
backtick span. Darkmatter documents already use it. Prose is the only
producer that never creates the node.

#### Incident: the inside-out rewrite

Commit `0e02bb613` worked around the missing node for one case. A code span
whose whole content is one link, `` `[desc](ref)` ``, is rewritten to
`` [`desc`](ref) `` so the link survives. That rule:

- turns example syntax into a live link. Darkmatter's `LinkError::UnrecognizedFormat`
  body, ``Input did not look like an HTML `<a>` tag or a Markdown `[text](href)` link.``,
  now renders as "a Markdown `text` link" (snapshot
  `l1__error_snapshots__link__unrecognized_format.snap`);
- modifies Markdown output: `` `[a](u)` `` renders to Markdown as
  `` [`a`](u) ``;
- departs from CommonMark, where a code span is always literal.

It exists so templates can wrap a generated link in backticks, as in
`` The `{{ link(plan) }}` plan `` (`prompts/plan.md`,
`prompts/_reviews/review-spec-inline.md`, and fixture copies of the latter in
`claudine/cli/tests/fixtures/nested_span_regression/` and
`darkmatter/dmls/tests/fixtures/mapping_only_corpus/`).

## Expected Behavior

Two components share one grammar.

| | `InlineProse` | `Prose` |
|---|---|---|
| Content model | phrasing only | zero or more blocks |
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

### Parsing order and boundaries

**Reader's note:** this is a curated Markdown subset, not a full CommonMark
parser. Ignoring trailing-space hard breaks is an intentional departure, so
standard Markdown renderers may read such input differently. Code-span
contents follow CommonMark: they are literal, backslashes included.

The shared parser first recognizes opaque fenced blocks, then separates
`Prose` paragraphs, then parses each paragraph's inline content. A blank line
inside fenced code is code, not a paragraph boundary. Code-span matching must
not cross a `Prose` paragraph boundary: unmatched backticks in separate
paragraphs stay literal. Within a paragraph a span may contain a single
newline, which becomes a space regardless of `LineBreaks` mode.

Bracketed style tags may cross paragraph boundaries. Preserve their scope by
reopening the same inline wrappers inside each paragraph, never by putting
paragraphs inside a `Span` or `Link`. For example,
`<red>one\n\ntwo</red>` produces two paragraphs with red text. Explicit
`<a>` tags follow the same rule. Markdown emphasis and Markdown links are
recognized within a paragraph only. Fenced blocks inside bracketed styles
remain sibling blocks, as today; the surrounding style resumes after them.
Newlines inside a quoted tag attribute belong to that attribute, not to the
paragraph splitter. Do not split the raw input with a regular expression
that loses tag scope or code opacity.

Retain the existing fenced-block recognition rules for this feature, including
language hints and an unclosed fence consuming the rest of the input. Expanding
recognition to every CommonMark fence form is outside this feature. In
`InlineProse`, discard the language hint and replace each body line ending with
a space before constructing `InlineCode`; do not trim its other whitespace or
apply code-span space stripping. Thus a body `a\nb` becomes `a b` on every
target, including in `Hard` mode. An empty body contributes no inline node.

Empty input and whitespace-only input produce no `Prose` blocks and empty
output. `InlineProse` preserves ordinary spaces and tabs, including around a
label, but empty input produces empty output. Both return a valid single
render-tree node even when empty.

### Newlines follow Markdown

| Input | Meaning |
|---|---|
| a single `\n` | soft break inside the block (`LineBreaks::Soft`, the default) |
| two or more `\n` in a row, with only spaces or tabs between them | block boundary (`Prose`) |
| `\` immediately before `\n` | hard break |

`\n\n` and `\n\n\n\n` are the same boundary. A soft break renders as a
space on the terminal and in HTML, and as a newline in Markdown; this is
how the shared renderers already treat `NodeKind::SoftBreak`. A hard break
renders as a newline on the terminal, `<br>` in HTML, and a Markdown hard
break.

Leading and trailing blank lines produce no empty blocks. Normalize CRLF and
lone CR to LF before applying prose newline rules, so text from Windows files
has the same paragraph structure. Fenced `Prose` code bodies retain the existing
parser's body handling; do not apply paragraph whitespace rules to them.

Discard spaces and tabs immediately around prose soft breaks, so
`"first  \nsecond"` becomes `first second`, not three spaces on the terminal.
Preserve whitespace elsewhere. A backslash forms a hard break only when it is
not itself escaped and is immediately followed by an in-paragraph newline.
A literal pair of backslashes before a newline produces a literal backslash
and the mode's ordinary break. A paragraph boundary takes precedence over a
hard-break marker; a backslash before a blank-line boundary or at end of input
remains literal.

CommonMark's other hard-break form, two or more spaces at the end of a
line, is not supported. Trailing spaces are ordinary whitespace, so
`"first  \nsecond"` is a soft break.

The Markdown target emits a hard break as `\` followed by a newline.
`renderable`'s Markdown renderer emits two trailing spaces today
(`renderable/src/tree/render/markdown.rs`, `NodeKind::HardBreak`); it
changes to the backslash form for every producer, darkmatter included.
Inside a table cell it keeps emitting `<br>`.

In a Rust string literal the backslash form is `"first\\\nsecond"`
(`\\` for the backslash, `\n` for the newline). `"first\\nsecond"` is a
backslash followed by the letter `n` and is not a break.

#### Line-break mode

Both components take a `LineBreaks` mode, so a caller whose text uses
single newlines as line breaks does not have to rewrite it:

```rust
InlineProse::new(cell).with_line_breaks(LineBreaks::Hard)
Prose::new(msg).with_line_breaks(LineBreaks::Hard)
```

| Mode | A single `\n` | Two or more `\n` in `Prose` |
|---|---|---|
| `Soft` (default for both) | soft break | block boundary |
| `Hard` | hard break | block boundary |

The default is the same for both components, so a string means the same
thing in either one. Markdown output preserves the resulting meaning, but
may normalize the source spelling; it is not a byte-for-byte serializer. The mode is
part of the shared grammar: a `Prose` passes its mode to the inline parser
of each paragraph. This is the same switch as markdown-it's `breaks` option.

### `InlineProse`

- Holds phrasing content only. `to_render_nodes()` returns its inline node
  sequence. `render_tree()` returns a single neutral `Span` containing that
  sequence, including for empty content. This satisfies the existing
  `TreeRenderable` single-node contract without block separators. The shared
  browser renderer currently emits `<span>` for that node: the component's
  browser fragment entry point instead folds and structurally concatenates
  its children, without an outer element or string-based HTML rewriting.
  Do not change unrelated `Span` rendering to achieve wrapper-free fragments.
- Owns the inline projection that `Prose::to_render_nodes()` provides today.
- Implements `TerminalRenderable`, `MarkdownRenderable`,
  `BrowserRenderable`, and `TreeRenderable`.
- Has no `Layout`. Positioning inline content is its container's job.
- In `Soft` mode, two or more `\n` in `InlineProse` are one soft break,
  the same as a single `\n`. In `Hard` mode each `\n` is a hard break.
- A fenced code block in `InlineProse` degrades to inline code: its body
  becomes an `InlineCode` node. (`Table` cells today degrade fenced code
  to plain text with `degrade_code_nodes`; that goes away with the
  container changes below, so table cells show inline code instead.)

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

### Public API and shared rendering

Export `InlineProse`, `LineBreaks`, and `ProseTag` from the component module,
the crate's normal public exports, and its prelude. Both components provide
`new`, `content`, `with_line_breaks`, and the existing text/attribute escaping
helpers. `InlineProse` deliberately has no margin, width, alignment, or
word-wrap builders; its containing block controls wrapping. Existing inline
call sites using those builders must move positioning to their container,
rather than silently dropping it. Block callers retain the current `Prose`
layout helpers.

Share each component's projection between `TreeRenderable::render_tree` and
`TerminalRenderable::render_tree_node`. Rendering a component directly and
embedding it structurally must use the same parsed content and break mode.
Do not keep a separate legacy terminal parser. Remove `Prose::to_render_nodes`
and migrate callers to either `InlineProse::to_render_nodes` or the block
children of `Prose`'s root. Containers must transfer the root's layout onto a
valid enclosing block rather than inserting a nested `Root` or losing layout.

The new paragraph-element attribute is optional; absent means `<p>`. Use a
restricted typed enum for exactly the listed tags, with no arbitrary HTML tag
string. Keep it distinct from `NodeKind::Section`, which carries document
structure. Honor it in both the normal browser renderer and the streaming
browser writer; terminal and both Markdown dialects ignore it. Preserve
existing progress-widget paragraph rendering. Update attribute serialization,
default omission, and validation together; existing serialized trees without
the field retain their behavior.

### Code spans are inline code

This applies to both components through the shared grammar.

#### Every code span becomes an `InlineCode` node

The Prose projection emits `NodeKind::InlineCode` for each code span, in
every context where Prose grammar applies: top level, inside a bracketed
style tag, and inside a link's text.

```text
See `plain_code` here        →  Text("See ") InlineCode("plain_code") Text(" here")
See [`inline-block`](u) here →  Text("See ") Link(u)[InlineCode("inline-block")] Text(" here")
<red>run `md hash`</red>      →  Span(red)[Text("run ") InlineCode("md hash")]
```

The span recognition rules do not change: a backtick run opens a span only
when a later run of the same length closes it, and an unmatched run is
literal text.

The node's `value` is the span content with these adjustments, in order:

1. Backslashes are literal, as in CommonMark: `` `a\_b` `` has value
   `a\_b`. This changes today's behavior, where Prose resolved escapes
   inside a span and showed `a_b`. Text placed inside a code span must not
   be passed through `Prose::escape_text`; a code span is already opaque.
2. Line endings become spaces (CommonMark §6.1).
3. When the content both begins and ends with a space and is not all
   spaces, one space is stripped from each side (CommonMark §6.1). This is
   what lets `` `` `a` `` `` hold a backtick at its edge, and it keeps
   Markdown round trips stable.

The content stays opaque: no emphasis, link, or tag syntax inside a span is
interpreted.

**Mechanism.** Follow the fenced-code-block pattern already in
`markdown.rs`: lift each span into a lifted-content table and leave a
sentinel placeholder in the text, which `tokens::parse_render_nodes`
resolves straight into an `InlineCode` node. The placeholder must survive
the link, bold, and italics phases, including inside a link description, so
`` [`x`](u) `` arrives as `<a href="u">PLACEHOLDER</a>` and the recursive
parse of the link's inner text produces the `InlineCode` child. A
placeholder is never re-expanded into string markup. Input containing a
sentinel-like control sequence must remain user content, not impersonate a
lifted node; distinguish parser-generated references from literal input and
cover this alongside escaped tags and delimiters.

#### Per-target output

For `` See [`inline-block`](https://x.io/a) here ``:

| Target | Output |
|---|---|
| Markdown | `` See [`inline-block`](https://x.io/a) here `` (unchanged) |
| Terminal, OSC 8 supported | `See ` + `ESC]8;;https://x.io/a ESC\` + inline-code styling + `inline-block` + reset + `ESC]8;;ESC\` + ` here` |
| Terminal, OSC 8 unsupported | `` See [<styled inline-block>](https://x.io/a) here `` |
| Browser | `<p>See <a href="https://x.io/a"><code>inline-block</code></a> here</p>` (`Prose`); without the `<p>` for `InlineProse` |

"Inline-code styling" is whatever the shared terminal renderer applies to
`InlineCode`: the prose theme background and color when the render context
carries them, otherwise dim. Prose's standalone `render`/`render_optimistic`
path supplies no theme, so it renders dim. The terminal output drops the
backticks, except on a terminal that cannot style, where it keeps a
backtick fence (see Decisions).

The browser shape is the existing `<code>` element. No class is added;
`<code>` is the standard element and the convention every common Markdown
renderer (CommonMark, GFM, markdown-it, pulldown-cmark, Goldmark, Pandoc)
follows.

#### The inside-out rewrite is removed

`` `[desc](ref)` `` is an ordinary, opaque code span again. On every target
it shows the literal text `[desc](ref)` as inline code, and Markdown output
is `` `[desc](ref)` ``. `` [`desc`](ref) `` is the one way to write a link
whose text is code.

Remove `whole_span_link` and its branch in `escape_code_spans`, the
`code_span_wrapping_a_whole_link_becomes_a_link_with_code_text` and
`code_span_wrapping_a_link_emits_osc8_with_code_text` tests, and the
corresponding rows and paragraph in `docs/components/prose.md` and the
`markdown.rs` module docs. Restore the darkmatter snapshot
`l1__error_snapshots__link__unrecognized_format.snap` to show the literal
`[text](href)`; after this change it renders without backticks, as inline
code.

#### `code_link()` replaces `` `{{ link(x) }}` ``

The template call sites that relied on the rewrite move to a new darkmatter
expression function, `code_link`, which emits a link whose text is inline
code:

```yaml
# before
stderr: "The `{{link(plan)}}` _plan_ has been created"
# after
stderr: "The {{code_link(plan)}} _plan_ has been created"
```

`{{code_link(plan)}}` expands to `` [`plans/foo.md`](/abs/plans/foo.md) ``,
which renders as a clickable link with a code-styled label.

- **Signatures** mirror `link()` exactly: `code_link(file)` uses the
  file's portable relative path as the text; `code_link(target, desc)`
  takes a local file reference or an HTTP(S) URL plus a string. Argument
  validation, null handling, destination spelling, and errors are
  `link()`'s; factor out resolution of the raw label and destination from
  [`link_fn`](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs)
  in the `darkmatter` package rather than parsing the Markdown string returned
  by `link()` or escaping its label twice. Report errors under the function
  the caller invoked while retaining the same underlying resolution failure.
  Register `code_link` next to `link` in
  `functions/paths.rs`.
- **Catalog entry.** Add a `code_link` function to
  `darkmatter/docs/schemas/expression-functions.yaml`, beside `link`, with
  both overloads, a description, and a `display-only` example for each.
  This catalog is what every consumer reads: darkmatter's
  `expression_function_descriptors()`, DMLS, and `claudine context
  --expressions`, which lists functions from those descriptors with no
  list of its own. The parity test
  `catalog_and_runtime_bindings_have_bidirectional_canonical_parity` fails
  until the catalog entry and the runtime binding both exist, and the
  expected-signature list in `expression/catalog/mod.rs` gains
  `code_link(file)` and `code_link(target, desc)`.
- **Text** goes inside a backtick fence chosen by the Markdown fence rule
  (longer than any backtick run in the text, padded when it starts or ends
  with a backtick). Unlike `link()`, `[` and `]` in the text are **not**
  backslash-escaped: a code span takes precedence over link brackets in
  CommonMark, and a backslash inside a code span would show literally in
  other Markdown renderers. Use the complete padding rule below, including
  labels that begin and end with spaces. Backslashes in the label are
  emitted as is and render literally in Prose and in darkmatter's Markdown
  parser alike. Line endings in the label become spaces. An empty label
  produces an ordinary empty link with no `InlineCode` child
  (`[](dest)`), the same as `link(target, "")`.
- **Migrated call sites:** `prompts/plan.md:30`,
  `prompts/_reviews/review-spec-inline.md:18`, and the fixture copies
  `claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md`
  and `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md`.
  Ken's installed copies in `~/.claudine/prompts/` (`plan.md`,
  `_reviews/review-spec-inline.md`) are outside the repository and are
  updated by hand.

#### Markdown output picks a safe backtick fence

`renderable`'s Markdown renderer emits `` `{value}` `` for `InlineCode`,
which breaks when `value` contains a backtick. It must follow CommonMark:

- fence with a backtick run one longer than the longest run inside `value`
  (one backtick when there is none);
- pad with one space on each side when `value` begins or ends with a
  backtick, or begins and ends with a space and is not all spaces.

| `value` | Markdown |
|---|---|
| `plain` | `` `plain` `` |
| ``a`b`` | ```` ``a`b`` ```` |
| `` `a` `` | ```` `` `a` `` ```` |

The existing table-cell pipe escaping is kept and applied to the value before
fencing; table hard breaks remain `<br>` and soft breaks remain spaces. Factor
fence selection and padding into one shared `renderable` helper used by the
Markdown renderer, the terminal fallback, and `code_link()`, rather than three
implementations that can disagree. Terminal fallback uses the raw value,
without table-specific pipe escaping.

The fence rules follow [CommonMark code spans](https://spec.commonmark.org/0.31.2/#code-spans).
They preserve spaces at both edges by padding and preserve embedded backticks
by using a longer delimiter. A newline in a code span becomes a space under
CommonMark; empty content has no faithful code-span spelling. The helper must
make these limits explicit: newlines become spaces before fencing, and an
empty value renders as empty text with no fence. This shared
renderer change affects every producer, including darkmatter, so test it with
directly constructed nodes as well as Prose inputs.

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

This table describes the containers' Prose-specific entry points; it does not
remove support for strings, numbers, or other components. Table header labels
also use `InlineProse`. Update `From` conversions, helper traits, styled-cell
variants, and downcasts alongside constructor signatures. Strings converted
by a table or `InlineContent` use `InlineProse`; strings converted by a block
container use `Prose`.

The per-container folding and degrading (`fold_prose_nodes_into_blocks`,
`degrade_code_nodes` on Prose content) is replaced by the components' own
shapes. `IntoProseVec` stays for `StatusBlock` if still needed.

### Migration

Each call site moves to the component that matches its use. Inline sites
change `Prose` to `InlineProse`; block sites keep `Prose`. About 35 sites
use a single `\n` as a line break, and their lines would join. Each adds
`.with_line_breaks(LineBreaks::Hard)` and keeps its strings unchanged.
Known clusters:

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (13
  `<dim>Key:</dim> value\n…` sites) and `shell_blocks/types.rs`;
- `darkmatter/lib/src/markdown/errors/blocks.rs:192` (YAML excerpt lines);
- `claudine/cli/src/output/error_report.rs`, `commands/help.rs`,
  `commands/logs/errors.rs`, `commands/hooks/list.rs:430` (a line break in
  a table cell, so `InlineProse` in `Hard` mode);
- leading or trailing `\n` used for spacing (`claudine/lib/src/render/prompt/`,
  `sniff/cli/src/commands/repo.rs:439`, `worktree/cli/src/commands/create.rs:168`),
  which becomes either nothing or explicit spacing outside the component.

Content that already uses `\n\n` as a paragraph break keeps its meaning.

Call sites that interpolate `Prose::escape_text(..)` inside backticks (about
a dozen at review time, such as
`claudine/lib/src/composition/error/render/lifecycle.rs:150` and
`darkmatter/cli/src/commands/schema/triggers.rs:116`) drop the
`escape_text` call inside the span; otherwise its backslashes now show. Its
rustdoc stops promising clean output inside a code span. Find them with a
workspace search for `escape_text` near a backtick, not only these
examples.

## Documentation

`biscuit-terminal/docs/components/prose.md` is the current record of how
Prose behaves, and this feature rewrites most of it. It is updated in the
same branch, section by section:

| Section | Change |
|---|---|
| Intro (lines 1–15) | "Styled inline text component" is no longer true. State that `Prose` is a block component and `InlineProse` its inline counterpart, sharing one grammar. Remove the link to the Prose+ feature spec (`docs/` pages never link to specs) and describe the grammar in the page's own words. |
| Rendering Model | Show both components: `Prose::render_tree()` gives `Paragraph`/`Code` blocks, `InlineProse::render_tree()` gives phrasing nodes. Move `to_render_nodes()` to `InlineProse`. Add a Mermaid diagram of grammar → block split → inline parse → render tree → three targets. |
| Programmatic Use | Add `InlineProse::new`, `.with_tag(ProseTag::Div)`, and `.with_line_breaks(LineBreaks::Hard)` examples, plus a one-line rule for choosing between the two components. |
| **New:** Paragraphs and Line Breaks | Single `\n` is a soft break; two or more `\n` start a new block; `\` before a newline is a hard break, with the Rust-literal spelling (`"a\\\nb"`, and why `"a\\nb"` is not one); trailing spaces are not a break; `LineBreaks::Soft`/`Hard`; blank lines and fenced code inside `InlineProse`. One input/output example per rule. |
| **New:** Block Tag | `ProseTag` values, per-paragraph meaning (`<div>one</div><div>two</div>`), and that the tag affects only HTML. |
| **New:** Layout | `Prose` applies `Layout` on every target; `InlineProse` has none. |
| Markdown Subset, intro sentence | "inline code spans are kept opaque" becomes: code spans are inline code on every target. |
| Markdown Subset table | The `` `code` `` row changes from "unchanged, backticks included" to "inline code". Remove the `` `[desc](ref)` `` row. Add a `` [`desc`](ref) `` row: a link whose text is inline code. |
| "Code spans are opaque" paragraph | Keep the opacity and matching-backtick-run rules. Replace "The backticks stay visible" with what each target shows: terminal dim (or the theme's inline-code colors) without backticks, HTML `<code>`, Markdown a backtick span with a safe fence. Add the space-stripping and newline rules, with one example each. Delete "Unlike CommonMark, a Prose backslash escape still applies inside a code span"; backslashes inside a span are now literal. |
| "A code span holding exactly one link…" paragraph | Delete. Replace with one sentence: write `` [`desc`](ref) `` for a link with code text; `` `[desc](ref)` `` is literal code. |
| Escape Mechanism | Backslash escapes do not apply inside code spans. Replace "including inside a code span" in the `Prose::escape_text` guidance with: do not escape text placed inside a code span; it is already opaque. Add an example: `` `a\_b` `` shows `a\_b`. |
| Graceful Degradation | Add an "Inline Code" subsection: on a terminal that cannot style, inline code keeps a backtick fence. |
| Supported Tags → Special | Fenced code blocks are a `Prose` (block) feature; in `InlineProse` they become inline code. |
| Cross-Target Rendering | Browser row: paragraphs use the block tag (default `<p>`), inline code `<code>`, code blocks `<pre><code>`, layout as CSS, no `<span class="prose">`. Rewrite the `TreeRenderable` paragraph for the two components. |
| Prose in Other Components / Prose in Table Cells | Table cells and `InlineContent` take `InlineProse`; lists, block quotes, `TwoColumn`, and `StatusBlock` take `Prose`. Replace "fenced-code child degrades to escaped literal text" with "becomes inline code". Rename `StyledProse` wording if the cell variant is renamed. |
| Key API | Add `InlineProse`, `with_tag`, `with_line_breaks`; move `to_render_nodes()`; describe `render_html_fragment()` as block HTML. |
| Darkmatter `docs/topics/darkmatter-expressions.md` | Add `code_link(file)` and `code_link(target, desc)` to the function table (next to `link`) and to "Link Helpers", with one example each and the note that the text is a code span, so brackets are not escaped. |
| CLI | The `--html` example becomes `<div style="margin-left: 4ch"><p><strong>bold</strong></p></div>` (or whatever the renderer emits for root layout), produced by Prose itself, with no `class="prose"`. Add an example with a code span and its `--html` / `--md` output. If `bt prose` gains an `--inline` or `--tag` flag, document it here. |

If `InlineProse` grows enough to need its own page, it gets
`docs/components/inline-prose.md`, linked from `prose.md`, and the shared
grammar stays documented once, in `prose.md`.

## Scope

**In scope**

- The two components, the shared grammar, newline handling, `ProseTag`,
  `Layout` on every target, and removal of `<span class="prose">`.
- Code spans as `InlineCode`: `markdown.rs` (lifting, space handling,
  removal of the inside-out rule), `tokens.rs` (placeholder resolution),
  `parity.rs` expectations.
- The `renderable` block-element attribute, and two Markdown renderer
  changes: the hard-break form (`\` + newline instead of trailing spaces)
  and inline-code backtick fence selection.
- Container API changes in the table above.
- Migrating every call site and updating moved snapshots. Snapshots that
  strip ANSI lose the backticks around code; that is expected, not a
  regression.
- Darkmatter's `code_link()` expression function (runtime binding and
  catalog entry), its appearance in `claudine context --expressions`, and
  migration of the template call sites listed under "`code_link()`
  replaces".
- The documentation changes listed under Documentation, the
  `biscuit-terminal` skill, and the `bt prose` CLI help text.

The call-site counts are planning estimates, not evidence that migration is
complete. Search the whole workspace, including examples, tests, benchmarks,
macros, generated-source templates, and packages outside the frontmatter list.
Compile any newly discovered consumers and update the implementation's scope
record. Generated code is changed at its source template and regenerated by
the existing workflow.

**Out of scope**

- Changing the browser element for inline code, adding a class, or adding
  CSS for `<code>`.
- Giving Prose's standalone render path a theme background.
- Word-wrap behavior of inline code in the terminal.
- Making the backtick character escapable in Prose (`` \` ``). Today
  `Prose::escape_text` does not escape backticks, so interpolated text that
  contains a pair of backticks becomes inline code. That was already true of
  opacity; this change only alters how the span looks. Revisit separately if
  it bites.
- Call sites that pre-render Prose to a `String` before handing it to a
  container. They migrate by type only; moving them to pass the component
  itself is worthwhile but separate.
- Other Markdown block syntax (headings, lists, block quotes) inside
  `Prose`. Those are components of their own.

## Decisions

- **Every code span projects to `InlineCode`**, not only code inside links,
  so Prose and darkmatter agree on what a code span is. (Ken, 2026-10-02)
- **The inside-out rewrite from `0e02bb613` is reverted**; CommonMark's
  `` [`desc`](ref) `` is the only link-with-code-text form, and Markdown
  output is never rewritten. (Ken, 2026-10-02)
- **Browser output stays `<code>`**, the shared renderer's existing shape,
  with no class. (Ken, 2026-10-02)
- **Code-span contents are literal, as in CommonMark.** A backslash inside a
  code span is a backslash on every target, so a code span and a
  `code_link()` label mean the same thing to Prose and to darkmatter's
  Markdown parser. Callers stop passing `Prose::escape_text` output into
  code spans. Rejected: keeping today's escape resolution inside spans and
  restricting `code_link()` labels, or documenting target-dependent labels.
  (Ken, 2026-10-02; replaces the drafting default that resolved escapes
  inside spans.)
- **One shape per component on every target.** `Prose` is block,
  `InlineProse` is inline. (Ken, 2026-10-02)
- **`Prose` has a configurable block tag**, default `<p>`, changeable to
  `<div>` and other block elements. (Ken, 2026-10-02)
- **`Prose` applies `Layout`** on every target. (Ken, 2026-10-02)
- **Markdown newline semantics.** A single `\n` does not break a block; a run
  of two or more `\n` does outside opaque code and quoted tag attributes.
  (Ken, 2026-10-02; parsing boundaries clarified during review.)
- **Line breaks are an opt-in mode with the same default everywhere.**
  `LineBreaks::Soft` is the default for `Prose` and `InlineProse`;
  `LineBreaks::Hard` makes each single `\n` a hard break. Rejected: making
  `InlineProse` treat a single `\n` as a hard break by default while `Prose`
  does not, because the same string would then render differently by
  component and Markdown output would preserve different break meanings.
  `\` before a newline is a hard break in either mode. (Ken, 2026-10-02)
- **Two or more `\n` in `InlineProse` in `Soft` mode are one soft break.**
  `InlineProse` has no blocks, and `Soft` mode never forces a line break.
  Text that needs visible separation uses `Prose` or `LineBreaks::Hard`.
  Rejected: a hard break, which would make `Soft` mode only partly soft
  and change Markdown output. (Ken, 2026-10-02)
- **Trailing spaces are not a hard break.** CommonMark's two-trailing-spaces
  form is removed: it cannot be seen in review, editors strip it on save,
  and it hides inside Rust string literals. `\` before a newline is the
  only hard-break syntax, and the Markdown target emits that form.
  (Ken, 2026-10-02)
- **A fenced code block in `InlineProse` becomes inline code**, so it still
  reads as code where a block cannot appear. Rejected: leaving the fence
  as literal text. (Ken, 2026-10-02; the draft wrongly said `Table` cells
  already did this — they degrade to plain text — corrected the same day.)
- **`ProseTag` is each paragraph's element.**
  `Prose::new("one\n\ntwo").with_tag(ProseTag::Div)` renders
  `<div>one</div><div>two</div>`. Rejected: the tag wrapping every block,
  which cannot work for the default `<p>` and would make `p` a special
  case. (Ken, 2026-10-02)
- **One branch, one merge.** The components, container changes, newline
  rules, and every call-site migration land together and merge once
  everything is green. Work may be phased by package area on the branch.
  Rejected: merging in stages with `Prose` keeping today's newline
  behavior until the last area migrates, which needs a temporary
  compatibility path built only to be deleted. (Ken, 2026-10-02)
- **Inline code falls back to backticks on a terminal that cannot style.**
  When the capability-aware inline-code appearance produces no styling
  escape sequence, the shared terminal renderer
  emits the value inside a backtick fence chosen by the Markdown fence
  rule, so code stays marked. Decide from the renderer's effective appearance,
  not from `NO_COLOR` or color depth alone: inherited emphasis or underline
  may still produce styling. OSC 8 link sequences alone are not code styling.
  Styled output restores its enclosing appearance after code instead of
  always clearing it, so a red sentence stays red after a code span. This is in the shared renderer, so
  darkmatter's terminal output gains it too. Rejected: leaving inline code
  unmarked. (Ken, 2026-10-02)
- **Templates use a new `code_link()` function** for a link with code
  text, replacing `` `{{ link(x) }}` ``. Rejected: dropping the backticks
  (loses the code styling), and an option on `link()` such as
  `link(plan, code=true)` (darkmatter expressions have no named
  arguments, and the syntax reads poorly). (Ken, 2026-10-02)
- **Inline code and the `Prose`/`InlineProse` split are one feature.** They
  were drafted as two specs and merged because they land together and
  touch the same files, docs, and snapshots. (Ken, 2026-10-02)

## Open Questions

None.

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
4. `Prose::new("a\\\nb")` renders a hard break: a terminal newline,
   `<br>` in HTML, and `a\` plus a newline in Markdown.
5. `Prose::new("a\nb").with_line_breaks(LineBreaks::Hard)` and the same
   on `InlineProse` render the hard break of criterion 4, and
   `Prose::new("a\n\nb").with_line_breaks(LineBreaks::Hard)` is still two
   paragraphs.
6. `Prose::new("a  \nb")` is a soft break, not a hard break, and the
   Markdown target never emits trailing spaces as a hard break.
7. `InlineProse::new("first\n\nsecond")` renders `first second` on the
   terminal and in HTML, and `first\nsecond` in Markdown.
8. `Prose::new("x").with_tag(ProseTag::Div)` renders `<div>x</div>` in HTML,
   `Prose::new("one\n\ntwo").with_tag(ProseTag::Div)` renders
   `<div>one</div><div>two</div>`, and both are unchanged in Markdown and
   on the terminal; the validator rejects the block-element attribute on
   any node but `Paragraph`.
9. A `Prose` with a fenced code block renders `<p>` and `<pre><code>` as
   siblings, never `<pre>` inside an inline element.
10. A `Prose` with a left margin renders that margin in terminal and HTML
   output; `bt prose --margin-left` output carries it exactly once.
11. Neither component nor the `bt prose` wrapper generates `class="prose"`;
    literal user content containing that string is unaffected.
12. Each container in the Containers table accepts its stated type, and
   `fold_prose_nodes_into_blocks` / `degrade_code_nodes` no longer run on
   Prose content.
13. Every listed single-`\n` call site keeps its line structure after
    migration, verified by its existing snapshot or a new one.
14. A Prose L1 test asserts the render tree for each input in the
    projection example under "Every code span becomes an `InlineCode`
    node", including a span inside a style tag and inside a link.
15. A Prose L1 test asserts, for `` See [`inline-block`](https://x.io/a) here ``:
    - `render_markdown()` returns the input unchanged;
    - terminal output with OSC 8 support contains the OSC 8 open for
      `https://x.io/a`, then the dim SGR, `inline-block`, a reset, and the
      OSC 8 close, and contains no backtick;
    - `render_html_fragment()` contains
      `<a href="https://x.io/a"><code>inline-block</code></a>` inside a `<p>`.
16. `` `[desc](ref)` `` renders as inline code with value `[desc](ref)` on
    all three targets, and its Markdown output is `` `[desc](ref)` ``.
17. `` `a\_b` `` renders `a\_b` on every target, and each migrated
    `escape_text`-in-backticks call site shows its value without stray
    backslashes.
18. Space handling: `` `` `a` `` `` produces value `` `a` ``; `` ` a ` ``
    produces `a`; `` `  ` `` produces two spaces.
19. `renderable` Markdown renderer tests cover the inline-code fence table,
    inside and outside a table cell.
20. The darkmatter `unrecognized_format` snapshot shows `[text](href)`
    literally.
21. On a `ColorDepth::None` terminal, `` See `md hash` `` renders as
    `` See `md hash` `` (backticks kept), and `` ``a`b`` `` keeps a
    double-backtick fence; when inline-code styling is emitted, no delimiter
    backtick is added (literal backticks in the code value remain).
22. `code_link("plans/foo.md")` returns a code-text link with label
    `plans/foo.md` and the same destination spelling and quoting `link()`
    gives; `code_link(url, "a]b")` keeps `]` unescaped inside the code span;
    `code_link(url, r"a\_b")` shows `a\_b` in Prose and in darkmatter's
    Markdown parser; `code_link(url, "")` returns `[](url)`; text containing a backtick gets a
    longer fence; the four migrated templates render a link with an
    inline-code label on every target.
23. `claudine context --expressions` lists `code_link(file)` and
    `code_link(target, desc)` in the Filesystem group beside `link`, and
    DMLS offers `code_link` as a completion.
24. `just test` and `just lint` pass in every affected package area, including
    consumers discovered during the migration audit. Use nextest through the
    repository recipes; compile changed examples and benchmarks separately
    where L1 does not cover them.
25. Empty and whitespace-only `Prose` produce no blocks; empty `InlineProse`
    produces no output. A multi-node `InlineProse` returns one valid neutral
    `Span` and Markdown concatenates its children without block separators.
26. CRLF and lone-CR prose inputs have the same break structure as LF inputs.
    Cover escaped backslashes, trailing backslashes, and paragraph boundaries
    next to a backslash; opaque code is unaffected by break mode.
27. A bracketed style spanning paragraphs produces valid styled paragraphs;
    a fenced block inside that style remains a sibling block. Blank lines
    inside fenced code remain code, and code spans do not match across
    paragraph boundaries.
28. A multiline fence in `InlineProse` produces one inline-code value with
    spaces in place of line endings, no language hint, and no embedded block
    or forced line break. Test it directly and in a table cell.
29. Nested code restores enclosing terminal styles and preserves a surrounding
    hyperlink. Test styled and unstyled capabilities independently of OSC 8
    support; the unstyled fallback also works in a darkmatter-produced tree.
30. Shared hard-break output uses backslash-newline outside tables and `<br>`
    inside tables for both Markdown dialects. Test safe code fences with
    backticks, edge spaces, all-space values, table pipes, empty values, and
    multiline values.
31. Browser fragments and streaming browser output agree on every `ProseTag`;
    legacy serialized nodes retain the default tag, and invalid attribute
    placements are rejected. Embedded `Prose` retains layout exactly once
    without a nested `Root`. Inline callers with layout builders migrate the
    positioning to their container.

## Definition of Done

- All acceptance criteria are met. This document remains active and ready for
  review; only the author moves the feature into the completed lifecycle.
- Every row of the Documentation table is done, and no sentence in
  `docs/components/prose.md` describes the old behavior (inline-only
  Prose, `<span class="prose">`, newlines left uninterpreted, fenced code
  degrading to plain text in table cells, visible code-span backticks,
  `` `[desc](ref)` `` as a link); the interim-contract doc comments on
  `Prose::render_html_fragment` and the CLI's `render_html_with_layout` are
  removed with the code they describe.
- The paragraph boundaries, fenced-to-inline normalization, empty inputs,
  and literal code-span contents are documented with compact examples.
- The `markdown.rs` and `tree.rs` module docs, and any `renderable` docs
  that describe inline-code or hard-break Markdown output, match the new
  behavior.
- The `biscuit-terminal` skill names `InlineProse` and when to use it, and
  its styling topic describes code spans as inline code.
- Every moved snapshot was reviewed for an unexpected change, not accepted
  in bulk.
