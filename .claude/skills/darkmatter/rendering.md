# Darkmatter Rendering

Use this reference for the render tree, style lowering, disclosure blocks,
code blocks, and browser safety.

## Contents

- [Render pipeline](#render-pipeline)
- [Style authority](#style-authority)
- [Code blocks](#code-blocks)
- [Disclosure blocks](#disclosure-blocks)
- [Browser contract](#browser-contract)

## Render pipeline

Public terminal and browser paths fold Markdown into one
`renderable::Document`, then fold that document to the target. The tree build
context applies component policy, inherited color, alpha paint, link/image
attributes, text layout, and horizontal-rule defaults during construction.

Do not reintroduce a post-fold decoration or HTML-rewrite pass. A target fold
must receive enough typed layout and style data to render correctly in one pass.

`DarkmatterPage` is a viewport assembler. It owns page width, margin, padding,
background, max-width centering, browser wrapper metadata, and stylesheet
assembly. It cannot inspect component kinds or rewrite their content.

## Style authority

`style:` frontmatter lowers directly to per-component `ComponentPolicy` using
`renderable::layout::Layout` and alpha-bearing `PaintColor`. Parsed
`StyleColor` must not survive beyond the parser/apply boundary.

`CliStyleClaims` records only flags explicitly supplied by the user. Merge
claims with frontmatter as presentation policy; do not let CLI defaults
silently override authored styles.

Kebab-case keys are canonical. Supported snake-case compatibility aliases emit
deprecation warnings and become errors in strict-style mode. Width and
max-width are mutually exclusive where the component contract says so.

## Code blocks

`CodeBlock` is the primary component; its implementation is in
`darkmatter/lib/src/markdown/code_block.rs`. `YamlBlock` is a deprecated
compatibility wrapper that retains validation constructors.

Code-block mode defaults to inverse page color for contrast and supports
`inverse`, `dark`, `light`, and `same`. Terminal and browser paths must use the
same resolved mode. A direct component theme or `md code-block --theme` wins
over page/context defaults.

The CLI command is:

```text
md code-block <file-or-content> --language LANG --output terminal|html|markdown
```

It constructs a `CodeBlock` directly rather than routing through a Markdown
document fold.

## Disclosure blocks

The syntax is:

```md
::disclosure Summary
::details
Body content.
::end-disclosure
```

The summary is phrasing content; the body accepts block Markdown and nested
disclosures. `::file` and `::code` may wrap transcluded content with a
`disclosure` option.

Targets preserve one semantic node:

| Target | Projection |
|---|---|
| Terminal | Summary plus a dim/italic block-quote body |
| Markdown dialect | Original directive form |
| MarkdownPlus | Native HTML `<details>`/`<summary>` |
| Browser | Native `<details>`/`<summary>`, no JavaScript |
| JSON | `NodeKind::Disclosure` |

Malformed directives produce a ranged `MarkdownError::MalformedDisclosure`.
Inline style tokens override `style.disclosure.*`; unknown tokens remain part
of the summary rather than disappearing.

## Browser contract

- Run browser tests headlessly and never activate a visible window.
- Dispatch keyboard/pointer/media changes through browser automation, not
  `osascript`, `cliclick`, `xdotool`, or Windows `SendInput`.
- Assert computed styles, DOM/accessibility state, or used geometry.
- Lower alpha directly to CSS `rgba(...)`; do not patch serialized HTML.
- Preserve structured link and image attributes and apply URL policy before
  rendering or fetching.
- Validate malformed code directives before HTML output so browser rendering
  cannot soften a fatal Markdown error.
