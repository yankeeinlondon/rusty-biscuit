---
area: darkmatter
status: implemented
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-30
review_iterations: 2
completed: true
implemented: true
implemented_by: "claude/opus"
created: 2026-10-30
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
    - simplified-schema
related:
    - 2026-10-16-unified-logical-or
next_spec: 2026-10-30-yaml-document-schemas
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

## Goals and boundaries

A reference must select the same file in validation, document links, and
go-to-definition. Value hovers must explain that selection without running
composition, while key hovers continue to explain the property's schema.
The root-precedence change and format restriction below are deliberate
behavior changes, decided by the author on 2026-10-30.

## Decisions

### Schema root order: `SCHEMAS_DIR` moves first

A bare-name `$schema` (and every other bare schema reference: `Name@file.yaml`,
`example(file.yaml)`, trigger payloads) searches in precedence order:

1. the folder `SCHEMAS_DIR` names
2. `{package-root}/schemas`
3. `{package-area-root}/schemas`
4. `{base_dir()}/schemas` (the file tree root; the repository root inside a repository)
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

### One resolver for validation and the editor

Expose the file-selection portion of the
[simplified-schema resolver](../../../simplified-schema/lib/src/resolve.rs)
so validation and DMLS choose paths through one implementation. It returns
the selected path or typed failure, the ordered attempted locations, and any
safe prospective creation target. Selection must remain usable when the
selected file's contents are malformed.

DMLS's [`schema_target`](../../dmls/src/providers/frontmatter.rs), which
supplies frontmatter navigation destinations, uses that selection with the
roots from Darkmatter's
[`SchemaRoots::for_document`](../../lib/src/markdown/schemas/roots.rs).
The shared function delegates non-bare references to their existing
biscuit-file rules. Darkmatter continues to own root discovery; the extracted
`simplified-schema` package must not acquire a dependency on Darkmatter.

Rejected alternative: deriving navigation exclusively from the last successful
validation result. It cannot describe missing or malformed schema files and
may reflect an earlier edit. Prepared editor data may cache selection and
source together, provided failed resolutions and all dependencies participate
in invalidation; a successful validation is not a prerequisite for navigation.

### Link target when a `$schema` reference does not resolve

A missing `$schema` target is an error (the validator's diagnostic is
unchanged). Clicking the link anyway opens the path where the file would be
created, chosen by the schema-root precedence above, using roots already
classified as searchable:

- **Bare name:** `{first searchable schema root}/{name}`; when no root
  folder exists, `{document folder}/{name}`.
- **Explicit relative:** the path beside the document, as today.

DMLS returns a file URI without creating a file or directory. Zed may open
that URI as an empty buffer; other clients decide how to handle a nonexistent
target. Opening an empty buffer is not a server-side acceptance requirement.

### What a `$schema` reference may point at

| Target | Valid? |
|---|---|
| Recognized standalone YAML schema with an explicit `$schema` payload (mapping, root-union sequence, or existing whole-file delegation reference) | yes, if the payload and its dependencies are valid |
| JSON object declaring `"$schema": "https://json-schema.org/draft/2020-12/schema"` | yes, if the schema is valid (see JSON recognition below) |
| YAML file with no top-level `$schema` — including a `kind: schema` file that exports only `types` | **no** under the new restriction: use a named-type import instead |
| JSON file that is not a JSON Schema document | **no** |

This is an **intentional compatibility change**, not a navigation requirement.
Today unrecognized YAML objects fall back to raw JSON Schema, and JSON objects
are loaded as raw schemas without requiring a dialect declaration. Applying
a `types`-only document as a document schema is newly rejected; importing a
named type from that same document remains supported.

#### YAML schema documents

Decided by the author on 2026-10-30. There are two formal document types and
one informal one:

| Type | Identifier | Properties |
|---|---|---|
| `schema` | `kind: schema` | `$schema`, `types`, `description` (string), `categories`; each optional, but at least one of `$schema` or `types` |
| `schema-trigger` | `kind: schema-trigger` | match rules plus `$schema` (unchanged by this fix) |
| informal `schema` | no `kind` | as `schema`, with `$schema` required |

Any other top-level key is a malformed-envelope error. A Markdown `$schema:`
may target only a document that has a `$schema` property; a `types`-only
`schema` serves named-type imports from other schema files.

Today the
[standalone parser](../../../simplified-schema/lib/src/simplified/standalone.rs)
departs from this in two ways, both corrected here:

- the informal form rejects `description` (it falls back to raw JSON Schema);
- `kind: schema` with a `$schema` mapping and no `types` is refused as a
  "global catalog". That shape check exists for
  `darkmatter/schemas/darkmatter.yaml` (whose `fm` entry is the frontmatter
  base schema) and Claudine's `claudine/schemas/globals.yaml`, but it also
  refuses every formal `schema` without `types`. Remove it with no
  replacement. Both files are loaded only by explicit registration in code
  (`GlobalCatalog`), whose own strict loader is unchanged. A Markdown document
  that points `$schema:` at one of them now gets its entries as optional
  properties; this is accepted.

These rules remain hand-written in the parser for this fix. Expressing
`schema` and `schema-trigger` as YAML definitions in `darkmatter/schemas/` and
applying them to YAML documents by `kind` is `2026-10-30-yaml-document-schemas`.

#### JSON recognition

Decided by the author on 2026-10-30. A referenced `.json` file is a schema
only when its root is an object whose `$schema` is exactly the Draft 2020-12
URI the existing compiler supports
(`https://json-schema.org/draft/2020-12/schema`); it is then validated with the
existing offline validator. A declaration alone does not make it valid.

| JSON target | Outcome |
|---|---|
| Object declaring the 2020-12 URI | accepted, then validated as a schema |
| Object with no `$schema`, or a non-object root (array, boolean, scalar) | not a schema: the "not a valid schema definition" hover; the diagnostic adds the hint to declare `"$schema": "https://json-schema.org/draft/2020-12/schema"` |
| Object declaring another dialect (for example Draft 7) | a distinct "unsupported JSON Schema dialect" error naming the declared URI, not "not a schema" |

JSON objects without the declaration have no migration path other than adding
it. A repository audit on 2026-10-30 found no `$schema:` reference to a
`.json` file outside test fixtures, and the raw-schema fixture
(`darkmatter/dmls/tests/fixtures/suggest_constraint/raw-schema.json`) already
declares the URI; the implementation audit still re-checks this.

Do not fork classification into DMLS. Extend the existing standalone parser
and referenced-schema loader in `simplified-schema`, preserving typed errors,
source spans, imports, whole-file delegation, root-union handling, cycle/depth
guards, and embedded-source resolution. A file found at the winning path that
has invalid contents stays the winning path: report its error rather than
silently trying a lower-precedence root. Navigation still opens that file so
the author can repair it.

Before landing the restriction, audit shipped schemas, trigger payloads,
embedded sources, direct references, and root-union file arms. Record newly
rejected authored files and their migration in the implementation log; update
those files and contract fixtures in the same implementation change. Raw YAML
JSON Schema users must migrate to a `.json` file declaring the 2020-12 URI.
If the audit exposes a further public-format decision, bring it back to Open
Questions rather than silently widening or narrowing the parser.

### `$schema` value hover

Hovering the **value** of `$schema` (a file reference) shows a value hover. The
key hover is unchanged.

**Resolved, valid target:**

1. If the target file has a nonempty top-level string `description`, its full
   text, then a blank line. (Only the property counts; a leading YAML comment block
   is not used.)
2. `Schema Definition:`, then a blank line.
3. The directly referenced target's `$schema` value only (no other top-level
   properties), as a fenced, highlighted block: `yaml` for a YAML target, `json` for a JSON
   Schema target (for JSON, the whole document is the definition).
4. At most **10** lines inside the block. When truncated, after the block: a
   blank line, then `*Showing **10** of **{total}** lines*`, where `{total}`
   counts the lines of the full definition.

After the preview (and its truncation line, when present), a blank line and
`Resolves to [{display path}]({uri})`; the description stays first, as the
author specified. The label
is repository-relative when inside the document's repository, otherwise an
absolute portable path; an outside-repository document uses an absolute label.
The URI comes from the existing file-path-to-URI conversion, not string
concatenation. Escape labels and authored error text for Markdown. The client
controls link color and highlighting.

Use the authored payload's source span for YAML, preserving comments and line
order, and the authored whole document for JSON. Remove only the YAML key and
common indentation; normalize line endings for display. Count logical lines
without counting a final newline as an extra line. Show all lines when there
are ten or fewer; otherwise show the first ten and the exact total. Choose a
fence longer than any matching fence run in the excerpt. A delegated YAML
schema previews its authored reference value and links to the directly named
file; it does not expand an import tree or display generated JSON Schema.

**Resolved file that is not an accepted schema format:**

> The file [`{display path}`]({uri}) exists but it is **not** a valid schema definition!

**Other failures:** malformed YAML/JSON, invalid schema grammar, unreadable
files, invalid dependencies, and cycles retain their own reason and resolved
path when available. Do not describe an unreadable or malformed file as a file
that does not exist. Format rejection and schema-validation failure are
distinct outcomes from the shared loader.

**Unresolved:** list searched roots in order and show the prospective creation
target separately, explicitly labeled as missing. For a sibling-only match,
show the existing `./name` correction without linking to that sibling as if
the bare name had resolved. With no searchable roots, explain that ordinary
file-reference lookup was used and list its attempted candidates.

Hover is Markdown only; no inline HTML (Zed does not render it reliably).

### `file`-typed value hover

Hovering the **value** of a property whose effective type includes `file`. Use DMLS's existing selected-arm
analysis: a rejected union arm must not make a value a file. When several
arms remain possible, keep the existing any-surviving-file-arm behavior.
This applies to nested mappings and scalar array items as well as root
properties.
Values that are not static — a Darkmatter `{{ … }}` expression or `$(…)`
command — retain the applicable expression/property hover and get no file
hover. Preserve biscuit-file's environment-reference syntax; do not reject it
with a blanket brace check. Inline schema mappings, nulls, non-string values,
collection markers, and comments do not become file references.

| Case | Hover | Error? |
|---|---|---|
| Value is not a valid file reference (as classified by `FileReference::new`, not OS-specific character rules) | `` `{value}` is not a valid file reference: {reason} `` | **yes** |
| File exists | Resolves to [`{display path}`]({uri}), using the path-label rules above | no |
| File does not exist, property is `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `{property}` is defined as being an **eager** file reference so that means the file reference must refer to a real file at runtime." | no |
| File does not exist, property is not `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. Because `{property}` is not defined as being **eager** then the file reference is not required to exist even at run time." | no |

The two missing-file messages are the author's wording; keep them verbatim.

`eager` is read with DMLS's existing
[`is_eager`](../../dmls/src/providers/frontmatter.rs) helper (item or array
level across surviving arms). For an ambiguous union, say that the file arm
is eager rather than claiming every allowed value must name an existing file.
Resolution follows the existing `file` rules (implicit paths from the document
folder then the repository root; explicit `./`/`../` from the document folder).

At design time DMLS reports **no diagnostic** for a well-formed `file` value
whose target does not exist, `eager` or not; `eager` existence is a run-time
check. If DMLS's validation pass currently reports a missing eager file, that
diagnostic is removed from the editor (the `md` runtime check is unchanged).
Suppress only typed missing-target (`NoMatch`) findings attributable to
`file` value existence. Do not match diagnostic prose or suppress missing
`$schema`, import, example, required-value, syntax, context, or I/O errors.
Keep diagnostic codes, source ranges, and `resolution_failure` data for all
retained findings. Do not change shared runtime validation to achieve this.

Missing context, permission/I/O failure, and unsupported remote resolution
get their own explanation, not a missing-file message or a creation link.
A well-formed URL remains syntactically valid, but this local preview feature
never fetches it or claims to have checked its existence. Keep the single
existing context-build diagnostic rather than adding a duplicate per value.

### Body-link hover to a Markdown document

When a body link resolves to a Markdown document, the hover adds the target's
summary text: the first of its frontmatter properties `excerpt`, `summary`,
`description`, `about` that is a nonempty string after trimming (in that order), rendered as Markdown
below the existing title and path, preserving any heading/fragment preview.
Skip nulls, blank strings, collections, and non-string scalars; do not stringify
them. Read only top-level authored frontmatter, never compose the target,
execute expressions, or follow summary links. Omit a pending expression or
shell-command value and try the next summary field. When none is present, the hover is
unchanged. Links to every other file type are unchanged.

`hover.rs` reads nothing from disk; the summary is captured into the workspace
graph's document record when the target is parsed, the same way headings are. This is content-derived state and must not enter
the graph's reusable path-only document table. Use existing on-demand target
preparation, including Markdown dependencies outside the workspace listing;
do not parse every listed file just to obtain summaries. An unreadable target
keeps the existing hover without a summary. Open buffers take precedence over
disk; watched changes, save-based refresh, closing a buffer, and deletion
refresh or remove the summary through existing document invalidation. This
feature does not require adding summary text to the persistent reference
index; if extraction stored there changes, its existing versioning and golden
fixture rules still apply.

The "inferred MIME, image preview" link-hover claim is removed from
`darkmatter/docs/lsp/features.md`.

## Shared resolution and editor lifecycle

The shared lookup belongs in `simplified-schema`; root discovery remains in
Darkmatter. DMLS supplies its document's captured context and ordered
`SchemaRoots`, never a new process snapshot. Use biscuit-file's
[`FileReference`](../../../biscuit-file/lib/src/file_reference/mod.rs) for
syntax, candidate planning, and resolution; do not implement a second sigil
or path parser. The biscuit-file skill governs this boundary.

The result must distinguish an existing target, a clean miss with an optional
creation candidate, a sibling-only schema error, and a typed resolution
failure. Keep the ordered attempted locations even on failure. Existing
`ResolvedSchema.searched` is sorted and deduplicated for dependency tracking;
it cannot provide display order. Preserve that dependency representation and
return the ordered search trace separately. Reuse this lookup for validation,
named imports, examples, trigger payloads, document links, and definition;
content parsing remains a separate operation.

For a clean miss with searchable schema roots, the prospective target uses
the first searchable root. With zero searchable roots, preserve ordinary
implicit-reference resolution (document folder, then the context's fallback),
then use the first planned candidate only if no file matched. An existing
repository-root match must not be replaced by a document-folder creation
link. An explicit reference uses its existing candidate plan, including
boundary checks. Other file-reference forms keep their existing semantics.
Malformed references, missing anchors/context, and I/O failures never get a
guessed creation path.

Moving `SCHEMAS_DIR` first must preserve existing absolute-value validation,
request-snapshot environment capture, duplicate-root identity rules, symlink
policy, and trigger-pattern base directories. Only precedence changes.
Invalid/unset roots do not become creation targets; unreadable roots remain
errors rather than permitting a lower-priority file to win.

Schema hover reads prepared source and resolution facts from DMLS's existing
schema preparation/dependency machinery. The hover formatting function does
no disk or network I/O and never runs composition. Prepare only what the
requested preview needs; reuse dependency watching for valid, malformed, and
missing targets, including roots outside the workspace and JSON targets.
Invalidate when a selected file changes or disappears, or a higher-priority
candidate/root appears. A repair must refresh a prior failure too. Open schema
buffers, where supported by the existing source layer, take precedence over
disk; do not introduce a second independent cache for hover.

Value hovers use the value's authored source range and negotiated position
encoding. Keys keep their existing schema documentation; expression and named
type-definition hovers keep their existing priority. `$schema` root-union
string items receive the same reference behavior as a scalar declaration;
inline arms and inline `$schema` mappings retain schema-authoring hovers.

## Out of scope

- Editor-specific link underline behavior (Zed underlines document links only
  while the modifier is held).
- `DocumentLink.tooltip`.
- Previews for body links to non-Markdown files.
- YAML definitions of `schema` and `schema-trigger`, and validating YAML
  documents by `kind` (`2026-10-30-yaml-document-schemas`).

## Acceptance

- In `review-3.md`, Cmd-click and go-to-definition on `feature-review.yaml` open
  `schemas/feature-review.yaml`.
- A bare-name `$schema` that resolves nowhere links to the first existing
  schema-root folder; with no root folder, to the document's folder.
- `SCHEMAS_DIR` wins over every other root for both `$schema` and trigger
  discovery, without changing trigger-pattern bases or root eligibility.
- The `$schema` value hover in `review-3.md` shows `Schema Definition:`, a
  10-line `yaml` block of `feature-review.yaml`'s `$schema`, and
  `*Showing **10** of **{n}** lines*`; the key hover text is unchanged.
- A `$schema` naming a `types`-only YAML file, or a JSON file without the
  2020-12 declaration, is an error in validation and shows the "not a valid
  schema definition" hover; another declared dialect reports "unsupported JSON
  Schema dialect".
- `kind: schema` + `description` + `$schema` (no `types`), and an informal
  file with `description` + `$schema`, are accepted document schemas, and
  their `description` leads the value hover.
- `file`-typed values show each file-value outcome. A clean miss produces no
  editor existence diagnostic; malformed references, context failures, and
  I/O failures retain their existing typed diagnostics.
- A body link to a Markdown document with `summary:` shows that summary.

### Tests

All automated cases below are L1: protocol requests and temporary files need
no real editor or focused window. Use existing test-toolkit fixtures and
nextest through the package-area `just test` recipes for affected areas.

- Resolver tests: root order with `SCHEMAS_DIR` first; creation targets with
  zero, one, and several searchable roots; zero roots with an existing fallback
  match; sibling-only correction; invalid/duplicate roots; typed failures with
  no invented target. Preserve existing symlink and boundary fixtures.
- Consumer parity: validation, document link, and definition select the same
  existing file or failure; named imports, examples, and trigger shadowing
  obey the new precedence. Include a malformed winning file and a valid file
  in a later root to prove there is no fallback after a content error.
- Response-contract fixtures (`dmls/tests/fixtures/response_contracts/`): a
  bare-name `$schema` resolving to a repo-root `schemas/` file, for both
  document link and definition (regression for problem 1).
- Hover snapshots: schema success, short/exactly-ten/over-ten line previews,
  description policy, malformed and unreadable targets, clean misses, and
  sibling-only matches; every file-value outcome, nested/list values,
  ambiguous and selected unions, expressions, and unchanged key hovers.
- Body summaries: each field, precedence, blank/non-string/pending values,
  no summary, heading targets, external dependency targets, and buffer edits.
- Session tests: change/delete a schema, create a nearer candidate, repair an
  invalid target, and edit/close a summary target buffer. Assert that hover,
  links, definition, and diagnostics refresh together. Cover JSON dependency
  watching and watched roots outside the workspace using existing seams.
- Diagnostic tests: only missing `file` values lose existence errors; missing
  schemas/imports/examples, required values, malformed references, and context
  or I/O failures still report their typed errors. Confirm runtime eager-file
  checks remain unchanged through the ordinary CLI invocation fixture.
- Classifier tests: every row of the YAML and JSON tables (including
  `description` in both YAML forms, `kind: schema` without `types`, a
  `kind: schema` file with neither `$schema` nor `types`, and an unknown
  top-level key), rejected document-schema use of type libraries, preserved
  named-type imports, delegation/root-union handling, and the `GlobalCatalog`
  loaders still loading both shipped catalogs. Include the passive
  shipped-schema corpus and a normal validation invocation so `description`
  support cannot work only in hover.
- Assert URI and range behavior with spaces, Markdown punctuation, non-ASCII
  text, and existing portable/native path fixtures. Do not add a new OS
  character blacklist.

No performance spike is required: the design reuses prepared schema data and
on-demand document parsing. Verify those properties with existing work
counters or preparation seams; do not add host matrices or timing thresholds
for this fix.

## Documentation

- `darkmatter/docs/topics/schemas/definition.md`: "Schema roots" table and
  prose (new order); what a `$schema` reference may point at (the YAML and
  JSON tables above).
- `roots.rs` module/constant docs and the Darkmatter skill's schema-root
  reference (root order); CLI schema-root reporting and its snapshots.
- Standalone-parser and referenced-loader docs in `simplified-schema` for the
  envelope and JSON recognition rules, and the `catalog.rs` module docs, which
  currently state that the standalone loader refuses a catalog's envelope.
- `darkmatter/dmls/docs/file-references.md` and `darkmatter/dmls/docs/hover.md`:
  schema-specific lookup, creation candidates, value/key hover distinction,
  typed failures, and the editor-only missing-file diagnostic policy.
- `darkmatter/docs/lsp/features.md` and `darkmatter/docs/topics/dmls.md`: `$schema` and
  `file` value hovers, link fallback, Markdown link summary; remove the
  MIME/image-preview claim.
