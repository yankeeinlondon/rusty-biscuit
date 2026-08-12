---
created: 2026-06-23
status: ready for planning and implementation
reviewed: true
review_iterations: 2
related_specs:
    - "@darkmatter/features/_completed/2026-05-11-schemas/spec.md"
    - "@darkmatter/features/_completed/2026-05-23-compose-schema/spec.md"
    - "@darkmatter/features/_completed/2026-06-10-schema-improvement/spec.md"
---

# Surface Property Descriptions in Schema Validation Errors

SimplifiedSchema lets authors attach a human-readable description to a property
with the `-> {description}` arrow syntax (and inside inline objects). That text
is parsed into `PropertyAtom.description`, compiled into the standard JSON
Schema `description` keyword, and exposed on `EffectiveSchema.json_schema`.

It is, however, **never shown when validation fails**. A
`ValidationProblem` carries `path`, `message`, `kind`, `property`, `line`,
`column`, and `arm_index` — but not the property's description. So an author
who took the trouble to document `title: "string(required) -> The headline
shown in listing pages"` is told only `type "title": expected string` when it
goes wrong. The most useful moment to remind the author what a property is
*for* — the moment they got it wrong — is exactly when the description is
dropped.

This feature threads the declared description onto every validation problem and
renders it across the three surfaces that present validation failures.

> **Context.** This feature was scoped after considering, then rejecting, a
> new `desc(...)` universal constraint. The `-> {description}` arrow syntax
> already populates `description` end-to-end; a second authoring syntax for the
> same field would be redundant. The real gap is that the description is
> *captured but not leveraged*. This spec closes that gap rather than adding a
> parallel way to write the same value.

## Goals & Non-Goals

**Goals**

- Attach the failing property's description to each `ValidationProblem`.
- Render the description in `md schema validate` pretty output as a dimmed
  sub-line beneath the problem.
- Add a `description` field to each problem in `md schema validate --format
  json`.
- Render the description in the `MarkdownError::SchemaValidationFailed` status
  block (compose / claudine), reusing the existing dimmed-italic line treatment
  already used for the document-level `description:`.
- Resolve descriptions from the **compiled JSON Schema**, so descriptions
  authored in referenced raw JSON Schema files — not only SimplifiedSchema
  `->` text — surface too.
- Cover the common failure shapes: missing required, wrong type, and other
  constraint failures, at the top level and at nested inline-object / array
  paths.
- Keep schema-preparation failures unchanged. Those failures have no
  `ValidationProblem` list and therefore no property description to resolve.

**Non-Goals (this feature)**

- No new authoring syntax. The `-> {description}` arrow (and inline-object
  per-property descriptions) remain the only way to write a description. The
  rejected `desc(...)` constraint is **not** added.
- No change to validation outcomes. JSON Schema `description` is an annotation,
  not an assertion; surfacing it never changes whether a document is valid.
- No description rendering on **success**. This feature only enriches *failed*
  problems. Showing descriptions for valid properties (e.g. a `--describe`
  listing) is out of scope.
- No change to `md schema detect`. Detection still infers base types only and
  never synthesizes descriptions.
- No new completion behavior. `CompletionSuggestion.description` already exists;
  wiring it into a description-bearing completion protocol is a separate effort.
- No localization or templating of description text. It is rendered verbatim
  (escaped for the target surface).
- No attempt to explain unknown-property / `additionalProperties` failures by
  guessing intent from sibling properties. If the offending key is not declared
  in the schema, there is no declared property description to show.

## Foundational Decisions

- **Decision #1 — Source of truth is the compiled JSON Schema, not the
  SimplifiedSchema projection.** Descriptions are resolved by walking
  `EffectiveSchema.json_schema` along the problem's instance path to the
  property node and reading its `description` keyword. This is strictly more
  general than reading `EffectiveSchema.simplified`: the `simplified` field is
  `None` for raw JSON Schema input and for mixed root unions, whereas the
  compiled `json_schema` always exists and already carries every description
  (SimplifiedSchema-authored or file-authored). One resolver covers every
  schema origin.

- **Decision #2 — Resolution happens at report assembly, in
  `EffectiveSchema::validate_with_positions`.** After `collect_problems` /
  `collect_root_union_problems` produce the problem list, each problem is
  enriched with its description. `self.json_schema` is in scope there, and for
  root unions the winning arm is `self.json_schema["anyOf"][arm_index]`. The
  low-level collectors in `validate.rs` are not given new responsibilities
  beyond exposing a resolver helper.

- **Decision #3 — `Missing` problems resolve through the parent.** For a
  `Required` failure the instance `path` points at the *parent* object and the
  missing property name lives in `problem.property`. The resolver walks `path`
  to the parent node, then reads `node["properties"][property]["description"]`.
  This is the headline case: "missing `title`: required" plus *what `title` is*.

- **Decision #4 — Array index segments descend through `items`.** A numeric
  JSON-pointer segment (e.g. the `0` in `/authors/0/name`) descends into the
  current node's `items` schema rather than indexing by position, because the
  SimplifiedSchema array form gives every element the same `items` schema.

- **Decision #5 — Nullable `anyOf` wrappers are unwrapped while descending.**
  `convert.rs` encodes an optional typed property as `anyOf: [{ "type":
  "null" }, <typed-fragment>]` and places the property's `description` on the
  **outer** wrapper. So the resolver reads `description` at the wrapper node
  it lands on, but when it must descend *further* (into a nested property), it
  first steps into the non-null arm to find `properties` / `items`.

- **Decision #6 — A property-level union prefers a single shared description,
  and only articulates a union when arms genuinely differ.** Single-atom and
  hoisted descriptions land on the property (wrapper) node and are read
  directly. When the resolved node is a property-level union (`anyOf` with two
  or more non-null arms), let `D` be the set of non-null arms that declare a
  `description`. The rationale: in the common case a union just expresses
  alternate shapes of *the same* property, so one author description applies to
  all arms; divergent descriptions are the uncommon case worth spelling out.
  - **`|D| == 1`** — use that lone description verbatim as the property's
    description. It speaks for the whole union; no `a union type of:` wrapper.
  - **`|D| >= 2`** — synthesize `a union type of: {A} | {B}`, where each `{X}`
    is that arm's own `description` if it declares one, otherwise a
    human-readable type label (`string`, `number`, `object`, …) derived from
    the arm schema.
  - **`|D| == 0`** — articulate from type labels alone (`a union type of:
    number | string`) so the author at least sees the accepted shapes; `None`
    when no type label is derivable.

  The nullable sentinel arm (`{ "type": "null" }`) is always excluded from both
  the `D` count and the articulation.

- **Decision #7 — Presentation is a dimmed sub-line, never inline.** In both
  the pretty CLI output and the error block the description renders on its own
  line beneath the problem message, styled dimmed/italic. Existing location
  text remains on the message line for renderers that already include it. The
  description is never appended to the message line, so long descriptions do
  not crowd the diagnostic.

- **Decision #8 — Absent or empty descriptions render nothing.** When a
  problem's `description` is `None` (or whitespace-only), no sub-line is emitted
  and the JSON `description` field is `null`. The feature is purely additive to
  the existing output.

- **Decision #9 — De-duplicate against the message.** If the resolved
  description is byte-for-byte equal to the rendered problem message (an
  unusual but possible authoring choice), it is suppressed to avoid printing the
  same sentence twice.

- **Decision #10 — Unknown-property failures intentionally have no
  description.** Inline objects compile with `additionalProperties: false`, so
  an undeclared nested key can fail validation. The resolver must not attach the
  parent object's description or any "nearest" sibling description to that
  problem, because that would misidentify what the author needs to fix. These
  problems keep `description: None`.

- **Decision #11 — Existing path display behavior is preserved.** This feature
  enriches each problem; it does not redesign target labels. In particular,
  `md schema validate` pretty output keeps using the full stripped JSON Pointer
  (`authors/0/name`), while the compose status block keeps its current
  top-level target label behavior (`authors`) unless another spec changes that
  diagnostic surface.

## Data Model Changes

### `ValidationProblem`

```rust
// darkmatter/lib/src/markdown/schemas/mod.rs

pub struct ValidationProblem {
    pub path: String,
    pub message: String,
    pub kind: ValidationProblemKind,
    pub property: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub arm_index: Option<usize>,
    /// Declared description of the failing property, resolved from the
    /// compiled JSON Schema (the SimplifiedSchema `-> {description}` text, or a
    /// `description` keyword authored in a referenced JSON Schema file).
    /// `None` when the property declares no description.
    pub description: Option<String>,
}
```

The production validator path constructs `ValidationProblem` in
`validate::build_problem`. That constructor sets `description: None`;
enrichment is a post-pass (Decision #2). Tests and downstream code may also
construct `ValidationProblem` directly because the struct is public; every such
literal in the repository must add the new field explicitly.

## Description Resolution

A new helper lives in `validate.rs`:

```rust
/// Resolves the declared description for a problem by walking `root` (the
/// compiled JSON Schema) along the problem's instance path to the property
/// node and reading its `description`.
pub(super) fn resolve_problem_description(
    root: &serde_json::Value,
    problem: &ValidationProblem,
) -> Option<String>;
```

Algorithm:

1. **Pick the base schema.** If `problem.arm_index` is `Some(i)`, start from
   `root["anyOf"][i]`; otherwise start from `root`.
2. **Split the instance path** into JSON-pointer segments (`/authors/0/name` →
   `["authors", "0", "name"]`, unescaping JSON Pointer escapes `~1` → `/` and
   `~0` → `~`).
3. **For a `Missing` problem,** append `problem.property` as a final segment so
   resolution targets the missing property's node rather than its parent.
4. **Walk the segments.** At each node, before descending:
   - If the node is a nullable `anyOf` wrapper (`{ "anyOf": [...] }`), step into
     the non-null arm.
   For each segment:
   - **Numeric** → descend into `node["items"]`; stop (return `None`) if absent.
   - **Named** → descend into `node["properties"][segment]`; stop if absent.
5. **Read the description.** At the final node:
   - If `node["description"]` is a string, return it.
   - Else if the node is a property-level union (`anyOf` with ≥2 non-null
     arms), apply Decision #6 over `D` (the non-null arms that declare a
     description), skipping the `{ "type": "null" }` sentinel:
     - `|D| == 1` → return that lone arm's `description` verbatim.
     - `|D| >= 2` → synthesize `a union type of: {A} | {B}`, each `{X}` being
       the arm's `description` if present, otherwise `type_label(arm)`.
     - `|D| == 0` → synthesize `a union type of: {labels}` from
       `type_label(arm)`; `None` if no label is derivable.
   - Else `None`.
6. **Suppress unsuitable results.** The enrichment pass trims for the emptiness
   check but preserves the original description text when returning it. It
   filters out whitespace-only descriptions and descriptions equal to the final
   rendered problem message.

`type_label(arm)` maps a compiled arm schema to a short keyword for the union
articulation: a scalar `type` (`string`, `number`, `integer`, `boolean`)
returns that keyword; an object shape returns `object`; an array returns
`array`; an enum (`enum`/`const`) returns `enum`. It is a display aid only and
never affects validation.

The walk is defensive: any missing/typeless node returns `None` rather than
panicking, so a problem whose path cannot be resolved (e.g. an exotic
JSON-Schema-only construct) simply carries no description.

### Wiring

```rust
// EffectiveSchema::validate_with_positions, after collecting problems
for problem in &mut problems {
    problem.description = validate::resolve_problem_description(&self.json_schema, problem)
        .filter(|d| !d.trim().is_empty())
        .filter(|d| *d != problem.message); // Decision #9
}
```

## Library Access

The description must be trivially reachable by library callers (e.g. Claudine),
not only by the CLI renderers. Because `description` is a **public field on
`ValidationProblem`**, every caller that already reads a `ValidationReport`
gets it for free — no new accessor, trait, or method:

```rust
let report = api.validate(&md)?;            // DarkmatterSchemas::validate
for problem in &report.problems {
    if let Some(desc) = &problem.description {
        // surface alongside problem.message / problem.path
    }
}
```

This holds across every path that yields problems:

- `DarkmatterSchemas::validate` / `EffectiveSchema::validate*` return the
  enriched `ValidationReport` directly.
- The compose pipeline carries the same problems into
  `MarkdownError::SchemaValidationFailed`, so a caller that renders that error
  (or inspects its `problems`) sees the descriptions too.

No serialization or accessor work beyond adding the field is required;
enrichment runs inside `validate_with_positions`, so the field is populated on
every problem a caller can observe.

## Render Surface Changes

### 1. `md schema validate` — pretty output

`emit_problem_bullet` (`darkmatter/cli/src/commands/schema/validate.rs`) gains a
trailing dimmed sub-line when `problem.description` is `Some`:

```
- ✗ the document draft.md failed schema validation:
    - title expected string (at line 2 of frontmatter)
        The headline shown in listing pages
```

The description line is indented one level beneath the problem bullet and
styled dim. The existing location suffix stays on the message line. Preserve
the current CLI location wording (`(at line N of frontmatter)`) and the current
full stripped JSON Pointer prefix for nested paths.

### 2. `md schema validate` — JSON output

`emit_json` builds each problem object with `serde_json::json!`; add the field
to that per-problem object:

```json
{"path":"/title","message":"...","line":2,"column":1,"arm_index":null,
 "description":"The headline shown in listing pages"}
```

`description` is `null` when the property declares none.

### 3. `MarkdownError::SchemaValidationFailed` status block

`schema_validation_failed_block` (`darkmatter/lib/src/markdown/errors/blocks.rs`)
pushes a sub-line after each problem bullet, reusing the dimmed-italic treatment
already used for the document-level description line:

```
Schema validation failed
  draft.md
  type title: expected string at 2:1
      The headline shown in listing pages
```

The sub-line uses the same `<i><dim>{desc}</dim></i>` markup the block already
emits for the document `description:`.

Schema-preparation failures arrive at this block with an empty `problems` list.
Do not add a description line to those failures; continue rendering the
preparation summary exactly as today.

## Examples

### Missing required, with description

Schema: `title: "string(required) -> The headline shown in listing pages"`
Document: (no `title`)

```
- ✗ the document post.md failed schema validation:
    - title is a required property
        The headline shown in listing pages
```

### Wrong type at a nested inline-object path

Schema: `authors: "{ name: string(required) -> The author's display name }[]"`
Document: `authors: [ { name: 42 } ]`

```
- ✗ the document post.md failed schema validation:
    - authors/0/name expected string (at line 6 of frontmatter)
        The author's display name
```

### Property-level union — one shared description

The common case: a union expresses two shapes of the same property, and a
single arm carries the description meant for all of them.

Schema:

```yaml
width:
  - "number(min(0)) -> the element width, in pixels or as a CSS length"
  - "string(pattern(^\\d+(px|%)$))"
```

Document: `width: [1, 2, 3]` (an array — matches no arm)

```
- ✗ the document page.md failed schema validation:
    - width is not valid under any of the schemas (at line 4 of frontmatter)
        the element width, in pixels or as a CSS length
```

The lone description speaks for the whole union — no `a union type of:` wrapper
(Decision #6, `|D| == 1`).

### Property-level union — divergent descriptions

When two or more arms describe genuinely different things, the union is spelled
out (Decision #6, `|D| >= 2`):

```yaml
payload:
  - "string -> a raw message body"
  - "{ template: string(required), vars: object }[] -> a list of templated messages"
```

```
- ✗ the document page.md failed schema validation:
    - payload is not valid under any of the schemas
        a union type of: a raw message body | a list of templated messages
```

When no arm declares a description, the articulation falls back to type labels
— `a union type of: number | string` (Decision #6, `|D| == 0`).

### Description authored in a referenced JSON Schema file

`$schema: ./post.schema.json` where the file declares
`{ "properties": { "slug": { "type": "string", "description": "URL slug" } } }`.
A non-string `slug` surfaces `URL slug` even though no SimplifiedSchema was
involved (Decision #1).

## Module Layout & Touchpoints

**Modified files:**

```
darkmatter/lib/src/markdown/schemas/
├── mod.rs           # Add ValidationProblem.description; enrich in validate_with_positions
└── validate.rs      # Add resolve_problem_description; set description: None in build_problem
darkmatter/lib/src/markdown/errors/
└── blocks.rs        # Per-problem dimmed-italic description sub-line
darkmatter/cli/src/commands/schema/
└── validate.rs      # Pretty sub-line (emit_problem_bullet) + JSON field (emit_json)
```

The compose pipeline (`compose/schema_validation.rs`) consumes the enriched
problems through the existing `MarkdownError::SchemaValidationFailed` path with
no logic change — it already routes problems into
`schema_validation_failed_block`.

## Documentation Updates

- `darkmatter/docs/topics/schema-definition.md`: note that the `->` description
  now appears in `md schema validate` output (pretty + JSON) and in compose
  schema-failure blocks; update the Pretty Output and JSON Output examples to
  show the `description` sub-line / field.

## Testing Strategy

### Resolution unit tests (`validate.rs`)

- Top-level `Type` problem resolves the property's `description`.
- Top-level `Missing` problem resolves through `properties[property]`
  (Decision #3).
- JSON Pointer escaping is decoded before walking, so schema keys containing
  literal `/` or `~` resolve through `~1` and `~0` path segments.
- Nested inline-object path `/config/name` resolves through
  `properties.config` → (unwrap nullable `anyOf`) → `properties.name`.
- Array path `/authors/0/name` descends through `items` (Decision #4).
- Nullable optional property (`anyOf: [null, typed]`) reads the wrapper-level
  description (Decision #5).
- Property-level union where exactly one arm has a description uses that lone
  description verbatim, with no `a union type of:` wrapper (Decision #6,
  `|D| == 1`).
- Property-level union where two or more arms have descriptions articulates
  `a union type of: {A} | {B}`, mixing descriptions and type labels for
  description-less arms (Decision #6, `|D| >= 2`).
- Property-level union with no arm descriptions articulates from type labels
  (`a union type of: number | string`) (Decision #6, `|D| == 0`).
- Union articulation and the `D` count both exclude the `{ "type": "null" }`
  sentinel arm of an optional union.
- Property with no description → `None`.
- Unknown-property / `additionalProperties` failures → `None`; the resolver
  must not reuse the parent object's description (Decision #10).
- Unresolvable / exotic path → `None`, no panic.
- Root union: description resolves against `anyOf[arm_index]` for the winning
  arm; a problem with `arm_index: Some(1)` reads arm 1's schema.
- Description equal to the message is suppressed (Decision #9).
- Whitespace-only description is suppressed (Decision #8).
- Schema-preparation failures have no `ValidationProblem` and therefore remain
  outside description resolution.

### Render tests

- Pretty output (`md schema validate`) emits the dimmed description sub-line
  beneath the bullet when present, and omits it when absent. Existing location
  wording and nested JSON Pointer prefixes are unchanged.
- JSON output carries `"description"` with the string value, and `null` when
  absent.
- `schema_validation_failed_block` includes the description sub-line per
  problem; the document-level `description:` line and the per-problem
  description coexist without collision.
- `schema_validation_failed_block` preparation-failure rendering remains
  unchanged when `problems` is empty.

### Backward-compatibility tests

- Existing repository `ValidationProblem` literals, including error snapshot
  tests, compile with the new field.
- A schema with no descriptions produces byte-for-byte identical output to
  pre-feature behavior (no stray blank lines).

## Risks

- **JSON-pointer/JSON-Schema walk drift.** The resolver re-implements a small
  schema-path walk that must stay aligned with how `convert.rs` shapes
  fragments (nullable `anyOf` wrappers, `items` for arrays). Mitigated by
  resolution unit tests that mirror the converter's output shapes, and by the
  defensive "return `None` on any missing node" rule so drift degrades to
  *no description* rather than a wrong one or a panic.
- **Root-union arm selection.** A problem's `description` is resolved against
  the winning arm (`arm_index`). If `arm_index` is ever `None` for a union
  problem, resolution falls back to the root and likely finds nothing — an
  acceptable no-op rather than a wrong description.
- **Output verbosity.** Long descriptions add a line per failing property.
  Acceptable: failures are already multi-line, the text is dimmed, and authors
  opted in by writing the description. No truncation is applied in v1.
- **Union articulation length.** Articulating every arm (Decision #6) can grow
  long for wide unions. Acceptable in v1: unions are rare in frontmatter and
  the line is dimmed. No arm-count cap or truncation is applied; revisit if a
  real schema produces an unwieldy line.
- **`type_label` coverage.** The display-label mapping is best-effort over the
  compiled JSON Schema and may emit a generic label for exotic arm shapes.
  Because it is display-only, an imperfect label never affects validation, and
  an underivable label simply drops that arm from the articulation.

## Related Work

- `darkmatter/features/_completed/2026-05-11-schemas/spec.md` — base schema
  subsystem; defines `-> {description}` and `PropertyAtom.description`.
- `darkmatter/features/_completed/2026-06-10-schema-improvement/spec.md` —
  inline object literals and per-property descriptions inside `{ ... }`.
- `darkmatter/features/_completed/2026-05-23-compose-schema/spec.md` — the
  compose stage that routes problems into the `SchemaValidationFailed` block.
- `darkmatter/docs/topics/schema-definition.md` — public documentation to
  update when this lands.
