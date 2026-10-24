# Schema Validation

Schema validation lets a Markdown document declare the **shape of its frontmatter** and have Darkmatter enforce it. You describe the expected properties once — using a friendly single-line grammar called **SimplifiedSchema** — and Darkmatter checks every document against it, reporting a styled error that names the offending property instead of letting a downstream tool fail cryptically.

The same machinery powers three surfaces:

- the `md schema validate` and `md schema detect` CLI subcommands,
- the `DarkmatterSchemas` library API, and
- an always-on stage inside the **compose pipeline** that validates (and lightly **coerces**) frontmatter before any shell expansion runs.

This guide covers all three from a user's perspective. For the complete grammar reference — every type, every constraint, and the full JSON Schema mapping — see [`docs/topics/schemas/definition.md`](../topics/schemas/definition.md).

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

All properties are **optional** unless marked `required`, and a property whose type admits `null` (`null`, `unknown`, or a union with such an arm) stays optional even then. Any type gains an array form with a `[]` suffix (`string[]`), and item constraints versus array-level constraints are written in separate parenthesised lists (`"string(min(1))[](max(5); unique)"`). The full type vocabulary — `string`, `date`, `datetime`, `time`, `number`, `numberlike`, `boolean`, `boolish`, `object`, `file`, `enum`, `literal`, `url`, `email`, `yaml`, `json`, `expression`, `shell`, `alphanumeric`, `unknown` (alias `any`), `null` — and every constraint is tabulated in [the topic reference](../topics/schemas/definition.md#types).

### 2. File reference

Point `$schema` at a local file. **Relative paths resolve from the document's parent directory.**

```yaml
---
$schema: ./schemas/post.yaml
---
```

The referenced file may be:

- a YAML file whose root uses one of the two SimplifiedSchema envelopes, or
- a `.json` / `.yaml` file that is already a valid Draft 2020-12 JSON Schema.

The SimplifiedSchema envelopes are either `$schema:` as the sole root key, or
`kind: schema` together with a `types:` mapping:

```yaml
$schema:
    title: string(required)
```

```yaml
kind: schema
types:
    title: string(required)
```

Anything else is treated as raw JSON Schema, including a `$schema` string URI.
A non-empty bare map whose string values all parse as SimplifiedSchema property
definitions still remains raw JSON Schema and therefore constrains nothing. If
its keys do not identify JSON Schema or a custom vocabulary, Darkmatter emits
the `dm.schema.missing_simplified_envelope` warning without changing validity
or exit status. Add either envelope above to make the file a SimplifiedSchema.
Remote (`http(s)://`) references are **not** supported — download the schema
locally first.

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
- Conversion collects **every** independent per-property failure into one aggregate rather than stopping at the first, so a schema with two bad definitions reports two problems (and DMLS anchors one diagnostic per property). A structural failure that makes further traversal meaningless is still a single error.
- `file` is **lazy by default**: it builds the reference's candidate plan and materializes the first absolute candidate without probing the filesystem. Add `eager` (`file(eager)`) to require an existing regular file and materialize the first matching candidate. For document-backed validation, an implicit reference such as `spec.md` checks the prompt document's directory before the repository root; an explicit `./spec.md` or `../spec.md` is source-relative only. Caller-supplied values use the captured launch context, while the captured launch area is retained only as diagnostics for document-authored values. `match(...)` shapes path *suggestions* in a single schema; in a root union an existing file outside an arm's declared globs rules that arm out (see [Root unions](#root-unions-and-file-match)).

## Validation Phases

The APIs above answer "is this document well-formed as authored?". A *runtime*
consumer — a tool that starts a process, lets other actors write, and then
judges the result — needs two different answers at two different moments, so
`EffectiveSchema` also exposes a phase-aware entry point:

```rust
use darkmatter::markdown::schemas::SchemaPhase;

let launch = effective.validate_for_phase(&frontmatter, SchemaPhase::Launch)?;
let done   = effective.validate_for_phase(&frontmatter, SchemaPhase::Completion)?;
```

| Constraint | `Launch` | `Completion` |
|------------|----------|--------------|
| `eager` | valid when present; absence is allowed | valid when present; absence is allowed |
| `required; eager` | must be present and valid | must be present and valid |
| `required` | may be absent; a present value is still type-checked | must be present and valid |
| `generated; required` | may be absent while the host can still supply it | must be present and valid |
| none | may be absent; a present value is still type-checked | may be absent; a present value is still type-checked |

- **`Launch` is the stabilized pre-provider seam**, not the instant the process
  started: caller overrides, interpolation, coercion, and deferred frontmatter
  shell expressions have all run.
- Missing and explicit `null` are both **absence**, and `required` alone decides
  whether absence is an error. An eager-only `null` is therefore allowed at both
  phases; a `required; eager` `null` fails at launch; a required-but-not-eager
  `null` is tolerated at launch and fails at completion. A null-admitting
  property (`null`, `unknown`, or a union with such an arm) is the exception:
  it is optional at both phases, whatever `required`, `eager`, or `generated`
  says (see [Null-Admitting Properties](../topics/schemas/definition.md#null-admitting-properties)).
- Both phases are **passive**: no mutation, no coercion, no expression or shell
  execution, no schema resolution, and no filesystem access. The eager-`file`
  existence probe belongs to ordinary schema preparation, not to phase
  validation.
- Phase constraints are projected recursively from the resolved
  SimplifiedSchema. Array placement decides ownership: `file(eager)[]`
  constrains each present item, while `file[](eager)` validates the array
  property at launch when present. `required` independently controls whether
  the property must exist.
- A named union's arms are not items in this sense, even under `[]`: an arm's
  `required`, `generated`, and `eager` belong to the property holding the
  reference. With `choice: ["string(required; eager)", number]`, both
  `p: choice` and `p: choice[]` must be supplied at launch. An arm's `eager`
  never adds an existence check to a lazy `file` arm.
- Raw JSON Schema has no phase vocabulary and keeps its authored `required`
  behavior at both phases. Trigger match conditions reject `eager` outright,
  because matching is passive and has no launch.
- The unphased `validate` / `validate_with_positions` methods are unchanged,
  including their `generated` exemption — phases are an addition, not a
  redefinition.

`claudine compose` and `claudine inline-compose` are the first consumers: they
validate at `Launch` before starting a provider and at `Completion` before
choosing between the `success` and `failure` lifecycle events.

## Baseline Schemas

A **baseline** is a schema that every validated document inherits — useful for a workspace-wide convention without editing each file. The effective schema is `baseline ⊕ document`, a deep merge keyed by property name in which the **document side wins** on any conflict.

Darkmatter ships a **built-in baseline** that types its own frontmatter
properties (`title`, `last_updated`, `style`, `ctx`, …) and, through Content
Policy's editor schema, `content_policy`. It is compiled into the library, so
an installed `md` or `dmls` uses it outside a checkout with no schema files on
disk. `md compose`, `md schema validate`, and DMLS all apply it by default.

`md schema validate` picks its baseline from the first rule that applies:

| # | Input | Baseline | Example |
|---|-------|----------|---------|
| 1 | `--no-baseline-schema` | none | `md schema validate post.md --no-baseline-schema` |
| 2 | `--schema <path>` | that file, even when the opt-out variable is set | `md schema validate post.md --schema ./schemas/baseline.yaml` |
| 3 | `DARKMATTER_NO_BASELINE_SCHEMA` is `1`, `true`, `TRUE`, `yes`, or `YES` | none; `BASELINE_SCHEMA` is ignored | `DARKMATTER_NO_BASELINE_SCHEMA=1 md schema validate post.md` |
| 4 | `BASELINE_SCHEMA=<path>` | that file | `BASELINE_SCHEMA=./schemas/baseline.yaml md schema validate post.md` |
| 5 | nothing | the built-in baseline | `md schema validate post.md` |

`--no-baseline-schema` and `--schema` cannot be combined (a usage error, exit
`2`). A custom path resolves from the directory you ran the command in, and a
custom baseline that cannot be loaded exits `2`; it never falls back. Opting
out removes only the baseline: the document's own `$schema` and matching
trigger schemas still apply, and `--no-schema-triggers` controls triggers
separately. See
[Selecting the Baseline](../topics/schemas/definition.md#selecting-the-baseline)
for the precedence flowchart.

Because the built-in baseline is the default, a document with no `$schema` is
checked the same way by `md schema validate` and `md compose`:

```bash
printf -- '---\nlast_updated: not-a-date\n---\n' > post.md
md schema validate post.md                       # exit 1: last_updated is a date
md schema validate post.md --no-baseline-schema  # exit 0
```

`md compose` keeps its own flags: `--baseline-schema <path>` replaces the
built-in baseline, and `--no-baseline-schema` or
`DARKMATTER_NO_BASELINE_SCHEMA=1` removes it.

A library caller starts with no baseline and attaches one explicitly, either a
file or the built-in baseline (`with_darkmatter_baseline_json_schema()`):

```rust
let api = DarkmatterSchemas::new(context)
    .with_baseline_from_file("./schemas/baseline.yaml")?;
```

Because a whole property is replaced when both sides declare it, replacing a baseline property also drops any inherited `required` — restate `required` in the document if you need to keep it. When the baseline is a *JSON Schema* (rather than SimplifiedSchema) it must be a simple object schema (top-level `properties` / `required` only); `$ref`, `allOf`, `if/then/else`, and similar constructs are rejected, because the merge operates on property names.

## CLI: `md schema validate`

```bash
md schema validate <file>... [--schema <path> | --no-baseline-schema] [--no-schema-triggers] [--format pretty|json] [--quiet] [-vv]
```

Pretty output (the default) renders one styled block per file:

```
- ✔ the document post.md has a schema definition and is valid.
- ✗ the document draft.md failed schema validation:
    - title is a required property (at line 2, column 1)
    - tags/1 42 is not of type "string" (at line 6, column 5)
```

Each location points at the offending YAML node — the value, unknown key, or
mapping that is wrong — counted in lines of the complete file (the opening
`---` is line 1) and 1-based UTF-8 byte columns. Above, `tags/1` is the second
item of a block list on line 6, so its value starts at column 5, after the
`- `. See [Problem Locations](../topics/schemas/definition.md#problem-locations)
for the full rules and fallbacks.

`--format json` emits newline-delimited JSON (one object per file) for piping into other tools, and `--quiet` suppresses the success lines.

| Exit code | Meaning |
|-----------|---------|
| 0 | All files valid. |
| 1 | One or more files failed validation. |
| 2 | Schema or baseline could not be loaded. |
| 3 | A file's frontmatter could not be parsed at all. |
| 64 | Usage error. |

A document with no `$schema` is validated against the baseline and reported as having a schema definition; in JSON output its `schema` field is still `null`, because the field names the document's own `$schema`. Only with the baseline turned off does such a document succeed vacuously (with a warning line in pretty mode).

### Stage timings (`-vv`)

To see where a slow validation spends its time, pass `-v` twice. After each
document's normal output, `md` prints a stage breakdown to **stderr**; stdout
(pretty or JSON) and the exit code are exactly what they are without it. A
single `-v` changes nothing, and neither `--debug` nor `RUST_LOG` is needed.

```console
$ md schema validate docs/prompt.md -vv
- ✔ the document docs/prompt.md has a schema definition and is valid.
Validation timings for docs/prompt.md
matched triggers: prompt.trigger.yaml
compile cache: miss
┌────────────┬───────────┐
│ stage      │ time      │
├────────────┼───────────┤
│ discover   │ 2.13 ms   │
│ match      │ 0.02 ms   │
│ resolve    │ 17.14 ms  │
│ compile    │ 84.65 ms  │
│ validate   │ 0.07 ms   │
│ unassigned │ 0.09 ms   │
│ total      │ 104.09 ms │
└────────────┴───────────┘
```

The stages never overlap; each one's time is work done in it and nowhere else:

| Stage | Work it covers |
|-------|----------------|
| `discover` | Scanning the schema roots for trigger files and checking each trigger's envelope and `match:` definition. Zero when trigger discovery does not run (outside a repository, or with `--no-schema-triggers`). |
| `match` | Judging each trigger's `match:` conditions against the document. |
| `resolve` | Resolving `$schema` and trigger payloads, lowering SimplifiedSchema to JSON Schema, and merging the baseline, trigger, and document layers. |
| `compile` | Building validators. A validator reused from the cache costs nothing here, so this time is taken out of the stage that asked for it. |
| `validate` | Checking the frontmatter against the compiled schema, including the conversion of its values. |
| `unassigned` | The rest of `total`: reading the frontmatter and other bookkeeping no stage claims. |

`compile cache` says whether the validators were already built:

| State | Meaning |
|-------|---------|
| `hit` | Validators were looked up and every one was cached. |
| `miss` | At least one validator was built for this document. |
| `not requested` | No validator was needed: the document has no schema at all, or validation stopped earlier. |

When a document fails before validating, for example because its `$schema`
does not resolve, the error is printed as usual and the breakdown that follows
covers the work done before the failure. Documents that cannot be opened or
parsed get no breakdown.

The breakdown covers all of the document's schema work, not just the final
check. With `key=value` assignments, `md` first resolves the schema to coerce
the assigned values, then resolves it again after assigning them, because an
assignment can change which triggers match. Both resolutions are counted, so a
validator first built while preparing the assignments is still a `miss`:

```console
$ md schema validate --no-baseline-schema review_counter=2 doc.md -vv
- ✔ the document doc.md has a schema definition and is valid.
Validation timings for doc.md
matched triggers: none
compile cache: miss
...
```

If that first resolution fails, its error is printed as usual and the breakdown
covers the work done up to it.

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
  └─ Materialize schema-selected caller files (captured per-property origin)
      └─ Call Check (declared types only; nothing evaluated)
          └─ Frontmatter Interpolation ({{ var }})
              └─ Schema Validation      ◄── here (bind + coerce + validate)
                  └─ Frontmatter Shell Expansion ($(cmd))
                      └─ … rest of pipeline
```

The [call check](./call-check.md) rejects a document whose calls its declared
types prove wrong before any of these stages computes a value.

This placement is deliberate:

- Before interpolation, the narrow projection prelude materializes
  caller-owned values selected as eager or non-recursive lazy files from each
  property's captured origin. It does not bind, coerce, validate, or normalize
  document-authored values. This gives frontmatter expressions one stable
  semantic value from their first lookup. Selection reads through named and
  recursive types the way validation does, so with
  `node: "file(eager) | { children: node[] }"`, both `spec: node` and
  `spec: node[]` select a caller's `--set spec=spec.md` (or `["spec.md"]`) as
  an eager file and materialize the launch directory's `spec.md`, exactly as
  `spec: file(eager)` would. A value that more than one alternative accepts
  (`file(eager) | string`) is not selected and stays raw, and only a string
  or a list of strings is a caller file: an object value such as
  `{ children: [spec.md] }` is kept as supplied.
- Full validation runs **after** `--set` / `--state` and interpolation, so a schema-required field can be satisfied by an override or a template. A document with `spec: ""` plus `--set spec=design.md` validates fine — validation sees the *effective* frontmatter.
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

- baseline and schema-trigger properties remain validation policy and do not
  create bindings;
- raw JSON Schema, root-union declarations, nested properties, required
  properties, and properties with `default(...)` do not create bindings; and
- a compose run with no effective schema does not create bindings.

The standalone validation APIs remain passive and never mutate the document.
Because the first frontmatter-interpolation pass precedes schema validation, it
cannot reference a binding that will only be materialized by this stage. Body
interpolation and later compose stages can use the binding. Repeating the schema
stage is idempotent, and serialized composed frontmatter retains the `null` key.

For example, the built-in baseline types `content_policy`, but a document that
omits it gains no `content_policy` key. A document whose own `$schema` declares
`content_policy` as an optional property, with no default, gets a `null`
binding as described above.

### The shell-deferral contract

A property whose value a later stage will compute has not been computed yet at validation time. Such a problem is **deferred**: once shell expansion and the second interpolation pass have run, compose validates the resolved frontmatter against the same effective schema and reports what remains. Deferral follows what a later stage will actually resolve, not what the text looks like:

| Value before shell expansion | First pass |
| --- | --- |
| `rule: "$(echo TimeSensitive)"` — a whole-value command, run in the shell stage | deferred |
| `content_policy: ["{{ rule }}"]` or `"{{ [rule] }}"`, reading that command's value | deferred, then judged as `[TimeSensitive]` |
| `content_policy: ["$(echo X)", TimeSensitive]` — `$(...)` inside a list is never run | judged as written, so rejected |
| `title: "see $(date)"` — `$(...)` after other text is never run | judged as written |

Deferral is decided per problem, at the problem's own path. A wholly later-resolved key — a whole-value command, or a value read from one — defers every problem beneath it. Otherwise a problem waits only when the value at its path (or, for a missing node, its nearest existing parent) still holds a `{{ … }}` template: rendering a template changes only its own value, so it can fix a problem there or on a parent that contains it, never one on a literal sibling. Two problems never wait: an unknown or missing key on a mapping (rendering changes values, not keys), and a problem beside a template the schema already accepted at a place marked `deferred` (that template's value was already allowed for, so what remains is a real mistake).

```yaml
$schema:
  cfg: "{ counts: number[] }"
cfg:
  counts: [three, "{{ x }}"]   # fails fast at /cfg/counts/0; /cfg/counts/1 waits
```

Any schema problem that does not depend on a later stage still fails fast, before a command runs. The post-shell check defers nothing except schema locations marked `deferred`: no later stage of compose resolves frontmatter, so a value that still holds `$(...)` or `{{ … }}` text there is judged as that text. The one exception is a `{{ … }}` template nested below a top-level key at a place the schema marks [`deferred`](../topics/schemas/definition.md#universal-constraints); its consumer renders it later (a lifecycle event, for example), so every validation pass accepts it without judging it, while its literal siblings are still checked. When shell expansion is disabled, nothing else can be deferred and those problems fail immediately.

The post-shell check applies whichever schemas are in effect — the document's `$schema`, the baseline, and any matched trigger schema. `--no-schema-triggers` only removes trigger schemas from that set; it does not skip the check. Shell output is text, so with the built-in baseline:

```yaml
content_policy: "$(cat policy.txt)"   # policy.txt holds ["Duration(3mo)"]
```

fails `md compose` and `md compose --no-schema-triggers` alike: the resolved value is a string, not a list. Only with no effective schema at all (`--no-baseline-schema` and no `$schema` or matching trigger) is the expanded value left unchecked.

A library caller that validates the composed frontmatter itself can take this verdict over with `ComposeOptions::with_caller_owned_post_shell_verdict(true)` (Claudine does, so it can drop an optional value that shell output made invalid). Compose then still judges what that caller does not: values a result suffix (`::ok`, `::exit-code`, `::result`) typed, and, when trigger schemas are enabled, the full post-shell pass. A `{{ … }}` template that does not depend on a shell value has resolved by the time the first pass runs, so its value is checked like any authored value.

### Error rendering

A compose schema failure renders as a styled `BlockError` with an OSC8 link to the source file, the document's `description:` for context, and one bullet per problem:

- `missing {property}: required but not provided`
- `type {property}: {message}`
- `invalid {property}: {message}` (format/constraint failures such as `darkmatter-file`)

Each bullet carries the YAML `line:col` when available, and root-union failures include the matched arm index. Crucially, the error names the *property* — not the downstream tool that would otherwise have blown up.

## Type Coercion

Strict JSON Schema rejects a string where a boolean is declared — but many value sources can only ever produce strings. Shell expansion always yields a string; a ternary like `"{{ spec ? true : false }}"` renders the literal into a **quoted** scalar, so the stored value is the string `"true"`, not a real boolean. Coercion fixes this class of mismatch.

When a frontmatter value is not the declared type but converts to it without
changing its meaning, Darkmatter converts the stored value and accepts it:
`"yes"` against `boolean` becomes `true`, `"4"` against `number` becomes `4`,
and `"pear"` against `number` is an error. Conversion is always on, and
function arguments use the same engine. The rules (what converts, what never
converts, unions, nested values, raw JSON Schema, and the changes from
earlier releases) are described once, in
[Type Coercion](../topics/schemas/definition.md#type-coercion).

Two consequences matter:

- **Conversion changes the stored value, not just the verdict.** A `has_spec`
  field holding the text `"false"` becomes a real `false`. Condition
  evaluators (`::block when="!has_spec"`, init-stack `when:`) treat any
  non-empty string as truthy, so leaving `"false"` as text would be a latent
  logic bug.
- **`numberlike` and `boolish` do not normalize.** They exist for values whose
  written form matters, such as a ZIP code.

### Where conversion runs

- **Library validation** converts a *working copy* of the instance, then
  validates it. It reports validity after conversion but performs **no
  document mutation**: `md schema validate` does not rewrite files.
- **The compose stage** writes converted top-level properties back into the
  frontmatter, so the real boolean, number, or text flows to every later
  stage and into the composed output. Each top-level property is converted on
  its own: one failing property does not stop its siblings. Values deferred
  to the post-shell check are skipped here and converted there. Conversion is
  idempotent.

A union keeps a value one option already accepts; otherwise the one
reachable option wins, a number beats a boolean, and JSON beats YAML. Any
other tie is an error naming each reading, and option order never matters,
including for root unions.

### Schema-selected `file` value materialization

File materialization is a sibling of coercion. It runs only when the effective
schema selects a `file` or `file(eager)` arm; an ordinary string is never parsed
or probed merely because it looks path-shaped. The stored effective value is an
**absolute native path**, while Markdown presentation keeps a separate portable
spelling. Expressions, comparisons, lifecycle state, proxy handoffs, retries,
and sequence tasks therefore retain one stable filesystem identity.

The materialization pass is idempotent: re-validating an already-materialized
absolute value is a fixpoint, so compose → re-compose never drifts.

**Triggered on:**

- a present, non-null string selected by `file` or `file(eager)`, including
  top-level properties, inline-object sub-properties, arrays, and the committed
  arm of a root or property union;
- each item of an array of a union (`assets: asset[]`), judged on its own by
  the property-union rule: the value is rewritten through the one validating
  arm that carries an eager `file`, either as the item itself or as a field of
  an object arm. With `asset: [file(eager), number]`, the item list
  `["./a.md", 3]` normalizes to `["a.md", 3]`;
- eager values probe the ordered plan and require the first existing regular
  file; lazy values materialize the first lexically normalized candidate
  without probing;
- caller-originated values use the captured launch context, document defaults
  use that document's source context, proxy overlays use the proxying source,
  and sequence parameters use the sequence document.

**Left verbatim (never rewritten):**

- `string`-typed properties, even when their value looks path-shaped — `string` is the literal-text contract.
- document-owned bare (lazy) `file` properties — `format: darkmatter-file-reference` is syntax-only and may legitimately name a file that does not exist yet (e.g. a `review_file` this run is about to produce). Caller records selected by a lazy file arm are materialized in the pre-interpolation input prelude without adding an existence check.
- recursive lazy `file` values — they fail with a typed binding error because a
  recursive plan has no single path shape without I/O; use `file(eager)`;
- a value that resolves to a remote URL — there is no local path to project.
- an absent or `null` optional `file(eager)` property.
- a property or union item that two eager-`file` arms both accept, such as
  `{ path: ./a.md }` against overlapping object arms that each declare
  `path: file(eager)`; the rewrite never picks the closest arm.
- a lazy `file` arm under `eager` written on the union or its array
  (`lazy_asset[](eager)`): that `eager` is property timing and does not make
  the arm eager.
- a value still holding a `$(...)` shell expression or unresolved `{{ ... }}` template — the post-shell re-validation handles it once it expands.

**Read-only validation contract.** Library callers that use `validate` / `validate_with_positions` keep their current contract: validation coerces on a working copy and **does not mutate** the caller's `serde_json::Value`. The eager-`file` rewrite is opt-in via the explicit [`EffectiveSchema::normalize_frontmatter`](../../lib/src/markdown/schemas/mod.rs) API; compose calls it on its accepted effective schema so the normalized values are what downstream interpolation, lifecycle events, and `inline-compose` see.

Stored values use `/` path separators on every OS, so a committed eager-`file` reference is portable across macOS, Linux, and Windows.

Caller records are the exception to the document-authored rewrite. Each record
retains its raw value and captured file-resolution origin. Before frontmatter
interpolation pass 1, an exactly selected eager or non-recursive lazy file arm
materializes the semantic identity from that origin. Eager local files must
exist; lazy local files bind the first ordered candidate without probing,
HTTP(S) values remain typed remote identities, and recursive lazy references
fail because no single unprobed identity exists. Path functions, comparisons,
lifecycle state, and frontmatter expressions consume the native semantic
identity. Markdown body text uses a separate portable presentation value;
direct variables and static member/index selections share this presentation
behavior without changing effective frontmatter. The raw record remains
unchanged so fresh preparation can apply another active document's schema
without recapturing ambient state.

A root union selects a file arm without committing to one. When several arms
accept the caller's values, or none does, a caller property is still
materialized when every contending arm declares it as the same kind of file,
because the value resolves from the caller's origin whichever arm applies:

```yaml
$schema:
  - kind: 'literal(feature)'
    spec: 'file(required;eager;match(**/features/**/spec.md))'
  - kind: 'literal(fix)'
    spec: 'file(required;eager;match(**/fixes/**/spec.md))'
```

`--set spec=fixes/x/spec.md` from the repository root resolves from the root,
and its glob then selects the `fix` arm (below). When the arms disagree on
the file mode (`spec: string` in one, `spec: file` in the other) or one omits
the property, the caller value stays raw, and the eager-`file` rewrite leaves
it raw too rather than re-anchor it as though the document had authored it.

### Root unions and file `match`

Each simplified arm's declared glob is compiled into that arm as
`x-darkmatter-match`, and an existing file outside it rules the arm out. An
arm with several `match(...)` groups, as when a reused named type adds its
own, keeps every group, and a file must satisfy each one. The glob is
enforced on a property whose type is `file` or `file[]` (directly or through
a named type that is just that file) and on every file inside a recursive
named type the arm uses, at every depth of the value. Any other file in the
arm (a union alternative such as `file(match(…)) | number`, a file in an
inline object, a dictionary value, a tuple slot, a list of a union) only
suggests, as it does in a single schema:

| Value | Result |
|---|---|
| `spec=fixes/x/spec.md` | the `feature` arm is ruled out; the `fix` arm applies |
| `spec=features/x/spec.md` | the `fix` arm is ruled out; the `feature` arm applies |
| `kind=fix spec=features/x/spec.md` | invalid: `` `…/features/x/spec.md` is outside this schema arm's `match(**/fixes/**/spec.md)` `` |

A recursive type is not changed for its other uses: the arm compiles its own
copy of each definition that reaches a glob (`$defs` key
`<name>@match:<property>`), and the shared definition keeps the
suggestion-only body a single schema or a merged trigger payload uses. A
glob reached through a definition makes the schema depend on where the
document lives, so its validator is never shared between documents in
different folders.

```yaml
$schema:
  - node: "file(match('./notes/*.md')) | { children: node[] }"
    spec: node(required)
  - spec: number
```

| Document | `spec` | Result |
|---|---|---|
| `a/doc.md` | `./notes/x.md` (exists) | valid |
| `b/doc.md` | `../a/notes/x.md` (the same file) | invalid: outside `b/notes` |
| `b/doc.md` | `{ children: [../a/notes/x.md] }` | invalid: the nested file is judged too |

With `spec: "file(match('./notes/*.md')) | number"` in place of `node`, the
same `b/doc.md` with `spec: ../a/notes/x.md` is valid: that glob only
suggests.

The same judgment runs wherever an arm is chosen: caller-file projection,
coercion, and the final verdict. When every arm fails, an arm ruled out by its
glob is reported only if all of them were, so the reader sees the requirement
of the arm the path selected. Identical globs and globs declared in just one
arm are enforced too, including beside a raw JSON Schema arm. A missing file
and template or shell syntax are never judged; the comparison uses `/`
spelling relative to the launch directory, where completion walks the glob.
The full rule is in [Defining Schemas — Files](../topics/schemas/definition.md#files).

## Interaction With `--set` and `--state`

Because the compose stage validates the *effective* frontmatter, overrides participate fully:

```bash
md compose doc.md --set '{spec: "design.md"}'
```

- `--state` fills missing or null values before validation.
- `--set` overrides values before validation.
- A caller `--set` value selected as an eager or non-recursive lazy file keeps
  its materialized native identity in effective frontmatter while body
  interpolation renders its separate portable presentation value.

So `spec: ""` + `--set spec=design.md` validates, while `spec: "design.md"` + `--set spec=""` fails. The same applies to transcluded children: a parent's `::file set=` overlay is applied before the child's schema stage, so the parent can satisfy a child's required property.

## Library API

```rust
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;

// The context resolves `$schema` references and `file` values; build it for
// the directory the documents belong to.
let context = build_resolution_context(&RequestSnapshot::new("/work/project"))?;
let api = DarkmatterSchemas::new(context)
    .with_baseline_from_file("./schemas/baseline.yaml")?;

let report = api.validate(&markdown)?;
if !report.valid {
    for problem in &report.problems {
        eprintln!("{}: {}", problem.path, problem.message);
    }
}
```

`DarkmatterSchemas` also exposes `effective_for(&Markdown)` (resolve + merge into an `EffectiveSchema`) and `detect(&[&Markdown], DetectOptions)`.

`validate_timed(&Markdown)` returns the same result as `validate` together
with a `ValidationTimings` (the stages above, `total()`, `unassigned()`,
`compile_cache()`, and `matched_triggers()`), on failure too. Trigger
discovery happens when the instance is built, so record it separately and fold
it in:

```rust
use darkmatter::markdown::schemas::{ValidationStage, ValidationTimings};

let (api, discovery) = ValidationTimings::record(|| api.with_trigger_discovery());
let (report, mut timings) = api?.validate_timed(&markdown);
timings.absorb(discovery);
for (stage, elapsed) in timings.stages() {
    eprintln!("{}: {elapsed:?}", stage.name());
}
```

`ValidationTimings::record` times any span of work on the calling thread, and
`ValidationStage::time` lets a host attribute its own work (such as checking
whether a cached trigger registry is still fresh) to a stage. Timings carry
durations, trigger file names, and the cache state, never document values or
schema bodies. See [the topic reference](../topics/schemas/definition.md#library-api) for the full type surface.

## Limitations (v1)

- No remote `$schema` references.
- No nested object schemas — `object` accepts any shape; use a referenced sub-schema for stronger typing.
- Arrays whose items may be one of several types cannot be expressed in SimplifiedSchema; reference a JSON Schema file instead.
- No `additionalProperties: false` opt-in, no `--no-coerce` strict mode, and no `md schema validate --write` to persist coerced values to disk.

## See Also

- [`docs/topics/schemas/definition.md`](../topics/schemas/definition.md) — the complete grammar, type, and constraint reference.
- [`docs/inline/fm-interpolation.md`](./fm-interpolation.md) — the stage that runs immediately before schema validation.
- [`docs/inline/fm-shell-expansion.md`](./fm-shell-expansion.md) — the stage that runs immediately after.
