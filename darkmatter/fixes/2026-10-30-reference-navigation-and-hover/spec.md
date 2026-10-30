---
area: darkmatter
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
reviewed: false
implemented: false
created: 2026-10-30
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
    - simplified-schema
related:
    - 2026-10-16-unified-logical-or
---

# Reference Navigation and Hover in DMLS

This fix sequences after the `simplified-schema` extraction on this branch.
Paths below name the post-extraction locations: `$schema` resolution lives in
`simplified-schema/lib/src/resolve.rs`, schema roots stay in
`darkmatter/lib/src/markdown/schemas/roots.rs`.

## Problem

Observed in Zed on
`messenger/features/2026-09-17-research-metadata-pipeline/review-3.md`, whose
frontmatter declares `$schema: feature-review.yaml`.

### 1. A bare-name `$schema` link opens the wrong file

The validator resolves `feature-review.yaml` correctly to the repository's
`schemas/feature-review.yaml`. Cmd-clicking the same value opens a new, empty
buffer at `messenger/features/2026-09-17-research-metadata-pipeline/feature-review.yaml`.

The two paths use different rules:

| Consumer | Code | Rule |
|---|---|---|
| Validator | `resolve_reference_in_context` → `is_bare_name` + `try_bare_name_in_roots` | bare name searches the schema roots only |
| DMLS document link and go-to-definition | `schema_target` → `navigation_path` → `ctx.resolve_reference` (`dmls/src/providers/frontmatter.rs`) | generic file reference; on a miss, `navigation_target()` falls back to the first planned candidate, the document's own folder |

Go-to-definition shares `schema_target`, so it has the same defect.

### 2. Hovering a `$schema` value shows the key's documentation

`hover()` in `dmls/src/providers/frontmatter.rs` computes whether the cursor is
on the key (`on_key`) but only uses it to detect synthetic sequence positions;
key and value both reach `schema_hover`, which describes the property. The
author learns nothing about where the value resolves or what it contains.

### 3. Hovering a `file`-typed value shows the key's documentation

Same mechanism as problem 2: a `file`-typed scalar value gets its property's
schema description. Whether the file exists, and where it resolves, is not
shown.

### 4. Body links to Markdown documents show only a title and path

`dmls/src/providers/hover.rs` shows the target's first heading and `→ path`.
The target's own summary frontmatter is not used. `darkmatter/docs/lsp/features.md`
additionally advertises "inferred MIME, image preview" for link hovers; neither
exists.

## Decisions

### D1. Schema root order: `SCHEMAS_DIR` moves first

A bare-name `$schema` (and every other bare schema reference: `Name@file.yaml`,
`example(file.yaml)`, trigger payloads) searches, nearest first:

1. the folder `SCHEMAS_DIR` names
2. `{package-root}/schemas`
3. `{package-area-root}/schemas`
4. `{repo-root}/schemas` (the file tree root)
5. `{home}/schemas`

Previously `SCHEMAS_DIR` was fourth. Because the same order decides trigger
shadowing, a `SCHEMAS_DIR` file now shadows the same file name in a package's
own `schemas/`. That is intended: `SCHEMAS_DIR` is an explicit override.

Unchanged:

- an explicit relative reference (`./x.yaml`, `../x.yaml`) resolves from the
  referencing document's folder only, never from the process working directory
  and never from a schema root
- a bare name searches the schema roots only; a same-named file beside the
  document is the existing `BareNameSiblingExists` error
- when no schema root exists at all, a bare name resolves like any other
  implicit path, starting at the document's folder

### D2. One resolver for validation and the editor (Option A)

The bare-name lookup becomes one public function in `simplified-schema` that
both the validator and DMLS call. It returns the resolved path when one exists
and, either way, the ordered roots it consulted. DMLS calls it from
`schema_target` with the roots it already computes
(`SchemaRoots::for_document`, as `overlay/registry.rs` does) before anything
else; non-bare references keep their current `resolve_reference` path.

Rejected alternative: have the overlay remember the path the validator
resolved and link to that. It has no answer when resolution fails (which D3
needs), can be stale between an edit and revalidation, and has no answer when
the schema file itself is malformed.

### D3. Link target when a `$schema` reference does not resolve

A missing `$schema` target is an error (the validator's diagnostic is
unchanged). Clicking the link anyway opens the path where the file would be
created, chosen by the same precedence as D1 but testing for the *folder*:

- **Bare name:** `{first schema root whose folder exists}/{name}`; when no root
  folder exists, `{document folder}/{name}`.
- **Explicit relative:** the path beside the document, as today.

The editor opens that path as a new, empty buffer (standard LSP behavior for a
link to a nonexistent file).

### D4. What a `$schema` reference may point at

| Target | Valid? |
|---|---|
| YAML file with a top-level `$schema` (mapping or root-union sequence); `kind: schema` is recommended but not required | yes |
| JSON file that is a JSON Schema document | yes |
| YAML file with no top-level `$schema` — including a `kind: schema` file that exports only `types` | **no**: `types` serves other schema files, but a Markdown document referencing it gets no schema |
| JSON file that is not a JSON Schema document | **no** |

This narrows the library: today a YAML file without an envelope, and any JSON
file, are accepted as raw JSON Schema. The validator, the link, and the hover
all use one classification so they agree.

**Open for the implementer to propose before coding:** the test for "is a JSON
Schema document". Candidates: the root is an object whose `$schema` names a
JSON Schema meta-schema URI; or the root compiles under the Draft 2020-12
meta-schema. The first is cheap and explicit, the second accepts schemas that
omit `$schema`. Also audit the repository for `$schema` references that the
narrowing would newly reject, and list them in the implementation log.

### D5. `$schema` value hover

Hovering the **value** of `$schema` (a file reference) shows a value hover. The
key hover is unchanged.

**Resolved, valid target:**

1. If the target file has a top-level `description` property, its full text,
   then a blank line. (Only the property counts; a leading YAML comment block
   is not used.)
2. `Schema Definition:`, then a blank line.
3. The target's `$schema` value only (no other top-level properties), as a
   fenced, highlighted block: `yaml` for a YAML target, `json` for a JSON
   Schema target (for JSON, the whole document is the definition).
4. At most **10** lines inside the block. When truncated, after the block: a
   blank line, then `*Showing **10** of **{total}** lines*`, where `{total}`
   counts the lines of the full definition.

The resolved path is shown as a link, repo-relative, so it is clickable and
renders in the link color.

**Resolved file that is not a valid target (D4):**

> The file [`{repo-relative path}`]({uri}) exists but it is **not** a valid schema definition!

**Unresolved:** the bare-name message lists the roots searched in order, so the
author can see where to put the file.

Hover is Markdown only; no inline HTML (Zed does not render it reliably).

### D6. `file`-typed value hover

Hovering the **value** of a property whose type (any union arm) is `file`.
Values that are not static — a `{{ … }}` template or `$(…)` command — keep the
existing expression hover and get no file hover.

| Case | Hover | Error? |
|---|---|---|
| Value is not a valid file reference (invalid path characters, malformed reference) | `` `{value}` is not a valid file reference: {reason} `` | **yes** |
| File exists | resolves to [`{repo-relative path}`]({uri}) | no |
| File does not exist, property is `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `{property}` is defined as being an **eager** file reference so that means the file reference must refer to a real file at runtime." | no |
| File does not exist, property is not `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. Because `{property}` is not defined as being **eager** then the file reference is not required to exist even at run time." | no |

`eager` is read with the existing `is_eager` (item or array level, any arm).
Resolution follows the existing `file` rules (implicit paths from the document
folder then the repository root; explicit `./`/`../` from the document folder).

At design time DMLS reports **no diagnostic** for a well-formed `file` value
whose target does not exist, `eager` or not; `eager` existence is a run-time
check. If DMLS's validation pass currently reports a missing eager file, that
diagnostic is removed from the editor (the `md` run-time check is unchanged).

### D7. Body-link hover to a Markdown document

When a body link resolves to a Markdown document, the hover adds the target's
summary text: the first of its frontmatter properties `excerpt`, `summary`,
`description`, `about` that is present (in that order), rendered as Markdown
below the existing title and path. When none is present, the hover is
unchanged. Links to every other file type are unchanged.

`hover.rs` reads nothing from disk; the summary is captured into the workspace
graph's document record when the target is parsed, the same way headings are.

The "inferred MIME, image preview" link-hover claim is removed from
`darkmatter/docs/lsp/features.md`.

## Out of scope

- Editor-specific link underline behavior (Zed underlines document links only
  while the modifier is held).
- `DocumentLink.tooltip`.
- Previews for body links to non-Markdown files.

## Acceptance

- In `review-3.md`, Cmd-click and go-to-definition on `feature-review.yaml` open
  `schemas/feature-review.yaml`.
- A bare-name `$schema` that resolves nowhere links to the first existing
  schema-root folder; with no root folder, to the document's folder.
- `SCHEMAS_DIR` wins over every other root for both `$schema` and trigger
  discovery.
- The `$schema` value hover in `review-3.md` shows `Schema Definition:`, a
  10-line `yaml` block of `feature-review.yaml`'s `$schema`, and
  `*Showing **10** of **{n}** lines*`; the key hover text is unchanged.
- A `$schema` naming a `types`-only YAML file, or a non-schema JSON file, is an
  error in validation and shows the "not a valid schema definition" hover.
- `file`-typed values show each D6 message; only a malformed reference is an
  error, and no missing-file diagnostic appears in the editor.
- A body link to a Markdown document with `summary:` shows that summary.

### Tests

- L1 resolver tests: root order with `SCHEMAS_DIR` first; the D3 fallback
  target with zero, one, and several existing root folders.
- Response-contract fixtures (`dmls/tests/fixtures/response_contracts/`): a
  bare-name `$schema` resolving to a repo-root `schemas/` file, for both
  document link and definition (regression for problem 1).
- Hover snapshots for every D5 and D6 branch, and D7 with each summary property
  and with none.
- Validator tests for the D4 rejections.

## Documentation

- `darkmatter/docs/topics/schemas/definition.md`: "Schema roots" table and
  prose (new order); what a `$schema` reference may point at (D4).
- `roots.rs` module docs (root order).
- `darkmatter/docs/lsp/features.md` and `darkmatter/docs/topics/dmls.md`: `$schema` and
  `file` value hovers, link fallback, Markdown link summary; remove the
  MIME/image-preview claim.
