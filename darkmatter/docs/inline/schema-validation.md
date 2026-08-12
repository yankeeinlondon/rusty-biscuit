---
related_specs:
    - "@darkmatter/features/_completed/2026-05-11-schemas/spec.md"
    - "@darkmatter/features/_completed/2026-05-23-compose-schema/spec.md"
    - "@darkmatter/features/_completed/2026-05-28-schema-coercion/spec.md"
---

# Schema Validation

Schema validation lets a Markdown document declare the **shape of its frontmatter** and have Darkmatter enforce it. You describe the expected properties once — using a friendly single-line grammar called **SimplifiedSchema** — and Darkmatter checks every document against it, reporting a styled error that names the offending property instead of letting a downstream tool fail cryptically.

The same machinery powers three surfaces:

- the `md schema validate` and `md schema detect` CLI subcommands,
- the `DarkmatterSchemas` library API, and
- an always-on stage inside the **compose pipeline** that validates (and lightly **coerces**) frontmatter before any shell expansion runs.

This guide covers all three from a user's perspective. For the complete grammar reference — every type, every constraint, and the full JSON Schema mapping — see [`docs/topics/schema-definition.md`](../topics/schema-definition.md).

## Why It Exists

A document that needs a `spec` path or a `total_phases` count has no way, on its own, to insist that those values are present and well-typed. Without a schema, a missing or malformed value silently flows downstream and breaks something far from the cause:

```yaml
---
spec: ""                       # author forgot to fill this in
dir:  "$(dirname '{{ spec }}')"
---
```

Here `{{ spec }}` interpolates to an empty string, `dirname ''` exits 1, and the compose run dies with a shell error that says nothing about `spec`. Declaring a schema turns that into a fail-fast, human-readable error at the right layer:

```yaml
---
$schema:
    spec: "file(required)"
---
```

Now compose aborts *before* the shell command ever runs, with a message that points straight at `spec`.

## Declaring a Schema

The `$schema` property is reserved by Darkmatter inside frontmatter. Its value can take three forms.

### 1. Inline SimplifiedSchema

A mapping from property names to a `{type}({constraints}) -> {description}` string:

```yaml
---
$schema:
    title:  "string(required; not-empty)"
    tags:   "string[]"
    rating: "number(min(0); max(5); default(3))"
    status: "enum(draft, published, archived; default(draft))"
    spec:   "file(match('*.md'); required) -> The spec this prompt implements"
---
```

All properties are **optional** unless marked `required`. Any type gains an array form with a `[]` suffix (`string[]`), and item constraints versus array-level constraints are written in separate parenthesised lists (`"string(min(1))[](max(5); unique)"`). The full type vocabulary — `string`, `date`, `datetime`, `time`, `number`, `numberlike`, `boolean`, `boolish`, `object`, `file`, `enum`, `url`, `email`, `any` — and every constraint is tabulated in [the topic reference](../topics/schema-definition.md#types).

### 2. File reference

Point `$schema` at a local file. **Relative paths resolve from the document's parent directory.**

```yaml
---
$schema: ./schemas/post.yaml
---
```

The referenced file may be:

- a YAML file whose root carries its own `$schema:` SimplifiedSchema property, or
- a `.json` / `.yaml` file that is already a valid Draft 2020-12 JSON Schema.

Darkmatter disambiguates by inspecting the file: a root `$schema` key whose value is a *mapping* means SimplifiedSchema; anything else (including a `$schema` string URI) is treated as raw JSON Schema. Remote (`http(s)://`) references are **not** supported — download the schema locally first.

### 3. Root-level union

Make `$schema` a YAML sequence to accept *any one of* several shapes. Each arm may be an inline schema or a file reference:

```yaml
---
$schema:
  - ./schemas/post.yaml      # arm 0
  - ./schemas/page.yaml      # arm 1
  - title: string            # arm 2: inline
    body:  string
---
```

The document is valid if its frontmatter matches at least one arm. When it matches none, the CLI reports the *closest* arm (fewest problems) and prefixes each problem with its arm index, e.g. `schema arm 1: title is required`.

## How Validation Works

Every SimplifiedSchema is compiled to a single Draft 2020-12 JSON Schema and validated with the `jsonschema` crate — SimplifiedSchema is purely an authoring surface, never a parallel validator. Compiled validators are cached (keyed by schema content), so validating a whole corpus does not rebuild the same validator hundreds of times.

A few behaviours worth knowing:

- `additionalProperties` is `true` — documents may carry extra tooling-specific frontmatter without tripping the schema.
- Unrecognized constraints are a **hard error at compile time**, so typos surface immediately rather than being silently ignored.
- `file` is **lazy by default**: a bare `file` value is only checked for syntactic validity (it must parse as a biscuit-file reference) and is never resolved against the filesystem. Add `eager` (`file(eager)`) to require the referenced file to exist. For document-backed validation, an implicit reference such as `spec.md` searches the containing repository before the prompt document's directory; an explicit `./spec.md` or `../spec.md` is source-relative only. The captured launch area is retained for diagnostics, not searched. Only legacy callers that configure no document anchor resolve from the ambient current working directory. `match(...)` shapes path *suggestions* only and never rejects a value.

## Baseline Schemas

A **baseline** is a schema that every validated document inherits — useful for a workspace-wide convention without editing each file. The effective schema is `baseline ⊕ document`, a deep merge keyed by property name in which the **document side wins** on any conflict.

```bash
md schema validate post.md --schema ./schemas/baseline.yaml
md schema validate post.md          # falls back to $BASELINE_SCHEMA
```

`md compose` has its own baseline default: it injects the Darkmatter base
frontmatter schema unless you pass `--no-baseline-schema` or set
`DARKMATTER_NO_BASELINE_SCHEMA=1`. Pass `--baseline-schema <path>` to replace
that default with a custom SimplifiedSchema YAML baseline. `md schema validate`
does not inject the Darkmatter base schema by default; it keeps the explicit
`--schema` / `BASELINE_SCHEMA` behavior shown above.

```rust
let api = DarkmatterSchemas::new()
    .with_baseline_from_file("./schemas/baseline.yaml")?;
```

Because a whole property is replaced when both sides declare it, replacing a baseline property also drops any inherited `required` — restate `required` in the document if you need to keep it. When the baseline is a *JSON Schema* (rather than SimplifiedSchema) it must be a simple object schema (top-level `properties` / `required` only); `$ref`, `allOf`, `if/then/else`, and similar constructs are rejected, because the merge operates on property names.

## CLI: `md schema validate`

```bash
md schema validate <file>... [--schema <path>] [--format pretty|json] [--quiet]
```

Pretty output (the default) renders one styled block per file:

```
post.md  ✓ valid (schema: ./schemas/post.yaml)

draft.md  ✗ 2 problems
  • title is required
      at line 2, column 1 of frontmatter
  • tags[2] does not match pattern ^[a-z0-9-]+$
      at line 6, column 5 of frontmatter
```

`--format json` emits newline-delimited JSON (one object per file) for piping into other tools, and `--quiet` suppresses the success lines.

| Exit code | Meaning |
|-----------|---------|
| 0 | All files valid. |
| 1 | One or more files failed validation. |
| 2 | Schema or baseline could not be loaded. |
| 3 | A file's frontmatter could not be parsed at all. |
| 64 | Usage error. |

When a document has **no** `$schema` and no baseline is configured, validation succeeds vacuously (with a warning line in pretty mode).

## CLI: `md schema detect`

`detect` is a productivity aid for adopting schemas on an existing corpus — it infers a starting SimplifiedSchema from real documents.

```bash
md schema detect <file>... [--format yaml|json] [--merge]
```

It infers only **base types** from each value (e.g. an integer → `number(integer)`, an ISO date string → `date`, a resolvable path → `file`). It never synthesises constraints, and every detected property is marked optional — a single sample cannot prove requiredness. With `--merge` across several files, a property is required only if present in *every* file, disagreeing types widen to a common ancestor (or become a union when genuinely disjoint), and enum members accumulate. Treat the output as a draft to refine, not a finished schema.

## Compose Pipeline Integration

Schema validation also runs as an **always-on stage inside the compose pipeline**. It sits between Frontmatter Interpolation and Frontmatter Shell Expansion:

```
Apply --set / --state overrides
  └─ Frontmatter Interpolation     ({{ var }})
      └─ Schema Validation          ◄── here (bind + coerce + validate)
          └─ Frontmatter Shell Expansion ($(cmd))
              └─ … rest of pipeline
```

This placement is deliberate:

- It runs **after** `--set` / `--state` and interpolation, so a schema-required field can be satisfied by an override or a template. A document with `spec: ""` plus `--set spec=design.md` validates fine — validation sees the *effective* frontmatter.
- It runs **before** shell expansion, preserving fail-fast: an invalid schema aborts the run before any side-effecting `$(...)` command executes.

By default, `md compose` validates documents with no `$schema` against the
Darkmatter base frontmatter schema. Use `--no-baseline-schema` or
`DARKMATTER_NO_BASELINE_SCHEMA=1` for raw compose behavior; with no document
`$schema` and no baseline, the stage is a complete no-op. Use
`--baseline-schema <path>` to replace the default baseline for that invocation.
Library callers can inject a workspace-wide baseline directly:

```rust
let opts = ComposeOptions::default()
    .with_baseline_schema(my_baseline);   // SimplifiedSchema
```

### Optional parameter bindings

During composition, an absent top-level property becomes an explicit `null`
binding when the document's inline or referenced SimplifiedSchema declares it
as optional and gives it no `default(...)`. This happens after the first
frontmatter-interpolation pass and before coercion and validation. Later stages
can therefore distinguish a declared-but-unset parameter from an undeclared
root, while authored values and caller overrides remain unchanged. Explicit
`null`, an empty string, `false`, zero, and empty collections are all preserved.

This binding behavior is deliberately narrow:

- baseline and trigger-schema properties remain validation policy and do not
  create bindings;
- raw JSON Schema, root-union declarations, nested properties, required
  properties, and properties with `default(...)` do not create bindings; and
- a compose run with no effective schema does not create bindings.

The standalone validation APIs remain passive and never mutate the document.
Because the first frontmatter-interpolation pass precedes schema validation, it
cannot reference a binding that will only be materialized by this stage. Body
interpolation and later compose stages can use the binding. Repeating the schema
stage is idempotent, and serialized composed frontmatter retains the `null` key.

This means `md compose` and `md schema validate` can intentionally diverge when a
document has no `$schema`: compose sees the default Darkmatter base schema, while
schema validation is vacuously valid unless `--schema` or `BASELINE_SCHEMA` is
provided.

### The shell-deferral contract

A property whose value still contains an unresolved `$(...)` shell expression has not been computed yet at validation time. Such a problem is **deferred** — the downstream consumer (e.g. claudine's `prepare_*_with_schema`) re-validates the post-shell frontmatter. If *every* problem is deferred, compose proceeds; any schema problem that does not depend on shell expansion still fails fast. When shell expansion is disabled, nothing can be deferred and those problems fail immediately.

### Error rendering

A compose schema failure renders as a styled `BlockError` with an OSC8 link to the source file, the document's `description:` for context, and one bullet per problem:

- `missing {property}: required but not provided`
- `type {property}: {message}`
- `invalid {property}: {message}` (format/constraint failures such as `darkmatter-file`)

Each bullet carries the YAML `line:col` when available, and root-union failures include the matched arm index. Crucially, the error names the *property* — not the downstream tool that would otherwise have blown up.

## Type Coercion

Strict JSON Schema rejects a string where a boolean is declared — but many value sources can only ever produce strings. Shell expansion always yields a string; a ternary like `"{{ spec ? true : false }}"` renders the literal into a **quoted** scalar, so the stored value is the string `"true"`, not a real boolean. Coercion fixes this class of mismatch.

When a frontmatter value is *trivially* the wrong JSON type but **unambiguously** convertible to the declared type, Darkmatter coerces the stored value and accepts it. Coercion is **default-on** with no opt-out.

| Declared type | Incoming value | Coerced to |
|---------------|----------------|------------|
| `boolean` / `boolish` | `true`/`false`/`True`/`False`/`TRUE`/`FALSE` (as a string) | real boolean |
| `number` / `integer` / `numberlike` | numeric string (`"42"`, `"-13"`, `"3.14"`) | real number |
| `string` (incl. `date`/`url`/`email`/`file`…) | a number or boolean scalar | its canonical string (`42` → `"42"`) |
| `array` of `T` | an array | each element coerced by `T`'s rule |
| `object`, `any`, bare `enum`, `null` | — | untouched |

Two consequences matter:

- **Coercion mutates the stored value, not just the validator.** A `has_spec` field holding the string `"false"` becomes a real `false`. This is correctness, not just ergonomics: condition evaluators (`::block when="!has_spec"`, init-stack `when:`) treat any non-empty string as truthy, so leaving `"false"` as a string is a latent logic bug that coercion eliminates.
- **`boolish` and `numberlike` now normalize**, where previously they merely *accepted* the string form. After coercion their stored value is the canonical JSON type.

### What is never coerced

Anything ambiguous falls through to the existing strict error unchanged — coercion only ever *adds* acceptances for unambiguous cases, never masks a genuinely-wrong value:

- `"yes"` / `"no"` / `"on"` / `"off"` → boolean (not in the boolish set)
- `"1"` / `"0"` → boolean (equally valid as numbers)
- any non-numeric string → number
- arrays, objects, or `null` → a scalar type; a string → object, or vice versa

Coercion also never *parses into* a constrained string type: a number landing in a `date` field becomes its string form and then fails the `date` format check normally — it can never produce a false accept.

### Where coercion runs

- **Library validation** coerces a *working copy* of the instance, then validates it. It reports post-coercion validity but performs **no document mutation** — `md schema validate` does not rewrite files.
- **The compose stage** writes coerced top-level properties back into the frontmatter, so the real boolean/number/string flows to every later stage and into the composed output. Values still holding `$(...)` are skipped here and coerced at the post-shell re-validation point. Coercion is idempotent, so re-validating an already-coerced value is a no-op.

For root unions, Darkmatter coerces against each arm in order and commits the first arm that validates post-coercion.

### Eager-`file` value normalization

Coercion has a sibling write-back pass that fires only on the **eager** `file` type. When a property is declared `file(eager)` (the eager marker is the compiled-schema `format: darkmatter-file`) and its value validates, the stored value is **rewritten to its resolved, repo-relative path** — the same projection `relative(value)` / `dirname(value)` already produce. After the rewrite, the document state is uniformly resolved: `spec` and `dirname(spec)` agree by construction, so an author never needs to hand-prepend `{{ctx.area}}` to make a derived path match.

The rewrite runs at the same two surfaces as coercion (the explicit library API and the compose stage's write-back) and is **idempotent**: re-validating an already-rewritten value is a fixpoint, so compose → re-compose never drifts.

**Triggered on:**

- a present, non-null string value under `file(eager)` / `format: darkmatter-file` — including top-level properties, inline-object sub-properties, array-of-`file(eager)` elements, and the committed arm of a root or property union.

**Left verbatim (never rewritten):**

- `string`-typed properties, even when their value looks path-shaped — `string` is the literal-text contract.
- bare (lazy) `file` properties — `format: darkmatter-file-reference` is syntax-only and may legitimately name a file that does not exist yet (e.g. a `review_file` this run is about to produce).
- a value that resolves to a remote URL — there is no local path to project.
- an absent or `null` optional `file(eager)` property.
- a value still holding a `$(...)` shell expression or unresolved `{{ ... }}` template — the post-shell re-validation handles it once it expands.

**Read-only validation contract.** Library callers that use `validate` / `validate_with_positions` keep their current contract: validation coerces on a working copy and **does not mutate** the caller's `serde_json::Value`. The eager-`file` rewrite is opt-in via the explicit [`EffectiveSchema::normalize_frontmatter`](../../lib/src/markdown/schemas/mod.rs) API; compose calls it on its accepted effective schema so the normalized values are what downstream interpolation, lifecycle events, and `inline-compose` see.

Stored values use `/` path separators on every OS, so a committed eager-`file` reference is portable across macOS, Linux, and Windows.

Caller-originated eager-file overrides are the exception to the document-authored
rewrite. A `set` value resolves against the caller's captured launch-area context
and remains an absolute, native path in effective frontmatter. Path functions,
comparisons, lifecycle state, and other typed consumers therefore retain the
caller-owned filesystem identity. When that value is interpolated into Markdown
body text, Darkmatter uses a separate portable presentation value; direct
variables and static member/index selections share this presentation behavior
without changing effective frontmatter.

## Interaction With `--set` and `--state`

Because the compose stage validates the *effective* frontmatter, overrides participate fully:

```bash
md compose doc.md --set '{spec: "design.md"}'
```

- `--state` fills missing or null values before validation.
- `--set` overrides values before validation.
- An eager-file `--set` value keeps its resolved absolute native identity in
  effective frontmatter while body interpolation renders its separate portable
  presentation value.

So `spec: ""` + `--set spec=design.md` validates, while `spec: "design.md"` + `--set spec=""` fails. The same applies to transcluded children: a parent's `::file set=` overlay is applied before the child's schema stage, so the parent can satisfy a child's required property.

## Library API

```rust
use darkmatter::markdown::schemas::DarkmatterSchemas;

let api = DarkmatterSchemas::new()
    .with_baseline_from_file("./schemas/baseline.yaml")?;

let report = api.validate(&markdown)?;
if !report.valid {
    for problem in &report.problems {
        eprintln!("{}: {}", problem.path, problem.message);
    }
}
```

`DarkmatterSchemas` also exposes `effective_for(&Markdown)` (resolve + merge into an `EffectiveSchema`) and `detect(&[&Markdown], DetectOptions)`. See [the topic reference](../topics/schema-definition.md#library-api) for the full type surface.

## Limitations (v1)

- No remote `$schema` references.
- No nested object schemas — `object` accepts any shape; use a referenced sub-schema for stronger typing.
- Arrays whose items may be one of several types cannot be expressed in SimplifiedSchema; reference a JSON Schema file instead.
- No `additionalProperties: false` opt-in, no `--no-coerce` strict mode, and no `md schema validate --write` to persist coerced values to disk.

## See Also

- [`docs/topics/schema-definition.md`](../topics/schema-definition.md) — the complete grammar, type, and constraint reference.
- [`docs/inline/fm-interpolation.md`](./fm-interpolation.md) — the stage that runs immediately before schema validation.
- [`docs/inline/fm-shell-expansion.md`](./fm-shell-expansion.md) — the stage that runs immediately after.
