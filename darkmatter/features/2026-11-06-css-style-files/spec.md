---
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
kind: feature
created: 2026-11-06
status: draft-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
reviewed: false
clarified: false
implemented: false
review_iterations: 0
packages:
    - darkmatter-cli
    - darkmatter
related:
    - 2026-11-05-render-surface
depends-on:
    - 2026-11-05-render-surface
---

# CSS Style Files

Let `md render --style=<FILE>` read a CSS file as well as the data formats
`2026-11-05-render-surface` accepts (JSON, JSON5, YAML, TOML).

```sh
md render notes.md --style=notes.css
```

This is a **draft** that records the idea and what is known. It will get more attention
after the data formats have landed, and it is not part of the current implementation
pipeline.

## The idea

Many style keys share a name with a CSS property, so a stylesheet could be a familiar way
to write a style file:

```css
max-width: 50;

table { max-width: 40ch; }
```

## What is known

Measured 2026-11-06 against `descriptor::SCHEMA`, excluding `style.markdown`:

- **53% of style paths end in a name CSS also uses** (69 of 129). By distinct names that
  is 7 of 24: `background`, `border`, `color`, `margin`, `max-width`, `padding`, `width`.
  The rename of page margin and padding keys to CSS order, decided in
  `2026-11-05-render-surface`, adds the side-specific names.
- **Some names match without matching in meaning.** `background` takes
  `transparent`, `subtle`, or `pronounced` here.
- **Some keys have no CSS counterpart:** `kind`, `theme`, `meta`, and `stylesheet`. Others
  have one under another name: `bg-color` and `background-color`, `alignment` and
  `text-align`.
- **`page.stylesheet` already means real CSS.** It sends a stylesheet to the browser for
  HTML output, so a `.css` file could mean two different things to a reader.
- **`cssparser` 0.37 is already a Darkmatter dependency.**

## Open questions

1. **Real CSS, or a CSS-like format?** The first sketch of this idea used forms that are
   not CSS: declarations outside a rule, `//` comments, unitless lengths, and
   `table max-width: 40` or `table > max-width: 40`. A CSS parser rejects these, and
   editors would flag them. Real CSS needs a selector for page-level values and units on
   lengths. Which does this feature accept?
2. **How do selectors map?** Which selector means the page, and which component names and
   nesting are accepted?
3. **How do values map?** Terminal cells, `ch`, and `%`; CSS values a target cannot honor;
   and values with no CSS spelling.
4. **What about CSS features with no style equivalent,** such as the cascade, specificity,
   media queries, and custom properties: ignored with a warning, or errors?
5. **Should `--style=notes.css` also reach the browser,** or does it only ever set style
   values, leaving `page.stylesheet` as the way to ship CSS?
6. **Do the unknown-key rules carry over?** Data files warn on unknown keys and fail when
   nothing applied.
