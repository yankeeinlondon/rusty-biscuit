# Single Coercion Engine: Findings from the Spike

Throwaway spike, 2026-09-26. Code: `src/`. Per-case results: `frontmatter-diff.json`.

## Recommendation

Build one **rule core** over a small neutral `Shape` type (`core.rs`) and feed it from two **translators**:

- **Rule core.** `bind(shape, value)` keeps the value, converts it, or returns a type error with a path. The scalar rules, the union rule, and the container walk each exist once.
- **Translator 1**: compiled or raw JSON Schema → `Shape`. Frontmatter always uses this one, because the effective schema may be baseline-merged, raw, or a mixed root union.
- **Translator 2**: grammar descriptors (`PropertyDef`/`TypeExpr`) → `Shape`. Function calls and design-time checks use this one, and it keeps the authored type text for errors.

Put the engine in a new `markdown/coercion/` module. `coerce.rs` becomes a thin adapter. These callers also choose union options themselves and must call the engine: `compose/schema_validation.rs::root_schema_arm_applies`, `rewrite.rs`, and `example.rs`.

**Does this satisfy "one engine, one rule list"?** Yes. Translators only map type syntax to shapes and never decide keep/convert/error. Their risk is drifting apart, so add a conformance test that binds each grammar case through both translators. They agreed on **94 of 94** grammar cases here.

**Alternatives:**

- **Compile function signatures to JSON Schema and use translator 1 only.** Viable, but numberlike and boolish must be sniffed from exact emitted shapes, and error messages lose the authored types.
- **Use the grammar only.** Raw `.json` schemas would get no conversion, which is a per-surface exception.
- **Walk JSON Schema directly, as today.** This needs a `jsonschema` validator per union option, and DMLS's compatibility API cannot reason over it cheaply.

## JSON Schema Keywords

| Keyword | Grammar emits it? | Handling |
|---|---|---|
| `type` (`integer`, `null`, arrays) | yes | translate; a type array becomes a union |
| `properties`, `required`, `additionalProperties`, `patternProperties`, `min`/`maxProperties` | yes | translate |
| `items`, `min`/`maxItems`, `uniqueItems` | yes | translate |
| `prefixItems`, `items: false` | planned (tuples) | translate as a tuple |
| `minLength`, `maxLength`, `pattern`, `minimum`, `maximum` | yes | translate |
| `enum`, `const` | yes | refined text, number literal, or boolean literal |
| `format` | yes | the existing format checks |
| `anyOf` | yes | union |
| `x-darkmatter-url-scheme` | yes | translate |
| `x-darkmatter-type-definition`, `x-darkmatter-schema` | yes | validate only |
| `oneOf` | no | union, then an exclusivity check |
| local `$ref`, `$defs`, `exclusiveMinimum`, `exclusiveMaximum` | no | translate |
| `allOf`, `not`, `if`/`then`/`else`, `dependent*`, `propertyNames`, `contains`, `unevaluated*`, `multipleOf`, remote `$ref` | no | validate only: no conversion, checked on the final value |

The repository uses almost no raw JSON Schema: two fixtures (`lib/tests/fixtures/validate/json_schema_ref`, `dmls/tests/fixtures/suggest_constraint/raw-schema.json`). Claudine schemas all use the grammar.

## Unions and Timing

The shape carries every constraint the grammar emits, so union selection needs **no per-option `jsonschema` validators**. Formats are delegated, one check per format name.

Release build, warm, µs per call:

| Scenario | Today (coerce only) | Core (convert + check) |
|---|---|---|
| `number \| boolean`, `"1"` | 0.45 | 0.41 |
| `{key,count} \| string` | 0.83 | 0.61 |
| `json \| yaml`, native mapping | 3.05 | 2.84 |
| Root union, 3 options | 1.14 | 1.78 |
| 8 scalars, no union | 0.87 | 1.47 |

One-time translation costs 3–19 µs, and a cold per-option validator build about 7 µs. The root union is slower because order independence means evaluating every option instead of stopping at the first. The spike's core also allocates a path string at every node. All figures are in microseconds.

## numberlike and boolish

Custom formats (`darkmatter-numberlike`, `darkmatter-boolish`) registered through `with_format` and calling the rule core work with `jsonschema` 0.55 and agree with the core on every probe.

A regex cannot express numberlike's range rule: `"99999999999999999999"` and `"1e-400"` pass the pattern but fail the core. The boolish regex does agree, but use formats for both.

What the format change affects:

- `md schema detect --format json` output changes, and so does the `snapshot_atom_numberlike` snapshot.
- External schema consumers become looser, because they ignore unknown formats.
- DMLS validates through the library, so its results are unchanged. Only the message wording differs.

This change also fixes a live gap: today's compiled schemas reject `"+4"`, `"yes"`, and `"On"`, which the core accepts.

## Frontmatter Diff

The diff covers 241 cases: `coerce.rs` tests turned into value cases, the L1 validate fixtures, the L1 literal/expression and CLI compose cases, AC28–34, and the spec's Unions and Containers tables.

- **180 unchanged.**
- **45 intentionally changed**, each matching a Changes-table row or the new tuple type. The AC34 `"02134"` row already holds today.
- **15 unexpectedly changed:**
  - **Rule-consistent but not in the table:** a number sent to `enum(...)` or a string literal converts, and values under grammar pattern keys (`<string>: number`) convert.
  - **Raw JSON Schema:** `$ref`, type arrays, `oneOf`, mixed-type `enum`, `patternProperties`, and `additionalProperties` schemas now convert.
  - **AC33:** `serde_json` sorts keys, so the output is `{"debug":true,"port":8080}`, not the spec's `{"port":8080,"debug":true}`.

The spec's "nested objects" row is also inaccurate. Nested inline objects already convert today (AC32 passes). The real gaps are unions and numberlike/boolish inside nested objects, and dictionary values.

## Rulings Needed

1. Should raw JSON Schema get conversions through every keyword the translator models (recommended), or only through what today reaches?
2. Should the Changes table add enum/literal refined text and pattern-key values?
3. AC33: enable `preserve_order`, or reword the example?
4. Restate the "nested objects" row?

## Limits

- Formats are checked through cached single-keyword validators.
- No source positions and no `ValidationProblem` mapping.
- `$(...)` handling only defers pending keys.
- No overloads or call binding.
- Tuples were tested only in their compiled form.
- Timings come from simple loops, not criterion.
- Signed zero was not tested.

## Reproduce

Copy `src/*.rs` into `darkmatter/lib/src/markdown/schemas/spike_engine/` and apply `src/schemas-mod.rs.patch`, then:

```sh
SPIKE_OUT=<dir> cargo nextest run -p darkmatter --lib spike_engine --no-capture
cargo nextest run -p darkmatter --lib --cargo-profile release spike_union_timing --no-capture
```
