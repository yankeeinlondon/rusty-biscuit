# `::toc-linking` Output Ignores Surrounding Indentation Context

## Symptom

When a `::toc-linking` directive appears inside an indented Markdown context (e.g., nested under a list item), the rendered output is emitted at column 1 instead of being indented to match its container. This produces invalid/malformed Markdown where the generated link list visually breaks out of its parent block.

### Input

```md
- `schematic-definitions` - this is where we define the API definitions

  For more details on **schematic-definitions** choose from any of the links below:

  ::toc-linking ../definitions/README.md level=h2

- `schematic-define` - this library provides the _primitives_ used to define an API surface

  For more details on the primitives defined in **schematic-define** choose from any of the links below:

  ::toc-linking ../define/README.md level=h2
```

### Actual Output

```md
- `schematic-definitions` - this is where we define the API definitions

  For more details on **schematic-definitions** choose from any of the links below:

- [Overview](../definitions/README.md#overview)
- [Available APIs](../definitions/README.md#available-apis)
- [Usage](../definitions/README.md#usage)
- [Bitbucket API](../definitions/README.md#bitbucket-api)
- [GitLab API](../definitions/README.md#gitlab-api)
- [Gitea API](../definitions/README.md#gitea-api)
- [OpenAI API](../definitions/README.md#openai-api)
- [Critical Configuration Requirements](../definitions/README.md#critical-configuration-requirements)
- [Adding New APIs](../definitions/README.md#adding-new-apis)
- [Dependencies](../definitions/README.md#dependencies)
- [Schema Registry](../definitions/README.md#schema-registry)
- [License](../definitions/README.md#license)
- `schematic-define` - this library provides the _primitives_ used to define an API surface
```

The generated bullets sit at column 1 and are absorbed into the **outer** list — collapsing the structure so the second top-level item (`schematic-define`) appears as a sibling of the inner TOC entries.

### Expected Output

```md
- `schematic-definitions` - this is where we define the API definitions

  For more details on **schematic-definitions** choose from any of the links below:

  - [Overview](../definitions/README.md#overview)
  - [Available APIs](../definitions/README.md#available-apis)
  - [Usage](../definitions/README.md#usage)
  - [Bitbucket API](../definitions/README.md#bitbucket-api)
  - ...

- `schematic-define` - this library provides the _primitives_ used to define an API surface

  For more details on the primitives defined in **schematic-define** choose from any of the links below:

  - [Overview](../define/README.md#overview)
  - ...
```

The generated list must be indented so that it nests inside the parent list item, producing valid CommonMark that preserves the document's structural hierarchy.

## Requirements

1. **Indent to container level** — When `::toc-linking` is rendered, every emitted list line (including blank separators if any) must be prefixed with the indentation of the container in which the directive appeared.
2. **Preserve directive's own indentation as a hint** — If the directive itself is written at a deeper indent than its container demands, that deeper indent is the floor; never emit at a shallower level.
3. **Apply at column 1 too** — Even when the directive is at column 1 inside an indented block context (e.g., a list item continuation that the user wrote without leading spaces), the output must be re-indented to match the container so the resulting Markdown is valid.
4. **Do not change link generation** — Only the leading whitespace of each emitted line should change. Anchors, link text, ordering, and `level=` filtering remain untouched.
5. **Other transclusion directives may share the issue** — Verify whether `::file` and `::code` exhibit the same bug; if so, fix consistently. (Out of scope for this spec unless trivially co-located in the same code path; flag in the implementation plan.)

## Acceptance Criteria

- A test fixture with `::toc-linking` nested two levels deep inside a list (4-space continuation) produces output where every generated bullet has 4 leading spaces.
- A test fixture with `::toc-linking` at column 1 inside a list item continuation (i.e., user wrote the directive flush-left but the surrounding context is indented) still produces correctly indented output.
- A test fixture with `::toc-linking` at the document root (no container indentation) produces output starting at column 1, unchanged from current behavior.
- Round-tripping the rendered output through a CommonMark parser yields the structurally-correct nested list — the inner TOC entries are children of the outer list item, not siblings.

## Likely Code Locations

- TOC-linking transclusion implementation under `darkmatter/lib/src/markdown/transclusion/` (search for `toc_linking` / `TocLinking`).
- The directive parsing layer that determines where the directive sits in the source — the column/indentation of the directive line and/or its enclosing block context — likely in the same module or in `darkmatter/lib/src/markdown/inline/`.
- Compare with how `::file` and `::code` emit their bodies; if they already preserve indentation, mirror their approach.

## Notes

- The fix concerns **rendered Markdown output**, not the AST. Downstream consumers (HTML, terminal) render correctly only if the intermediate Markdown they receive is structurally valid.
- This is a correctness bug, not cosmetic — the malformed output silently corrupts list structure and causes visible regressions in any document that uses TOC-linking inside list items.
