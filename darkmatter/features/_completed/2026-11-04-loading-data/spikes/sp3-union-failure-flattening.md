# SP3: Union Failure Flattening for `load_data` Contract Diagnostics

## Verdict

**Feasible, with one spec correction.** The coercion engine
(`simplified_schema::coercion::bind`) already reports *every* failing
location, in document order, with a path, the expected-type text, and the
received value. A small, deterministic flattening rule turns its
`NoOptionAccepts` trees into one record per location with a JSON Pointer,
`expected_type`, and `actual_type`, and it gave the right answer for every
case below.

The correction: R5's "the same coercion and validation path that frontmatter
uses" must **not** mean `validate_soft`'s `ValidationProblem` list. That list
comes from the JSON Schema validator run over the *unconverted* value whenever
the partial copy cannot be built, and it reports **false positives**. For
example, a CSV row with an extra column makes every numeric cell in the file
(`"8080"`, `"1"`) show up as "is not of type number". `dm.data.contract_mismatch`
records must be built from `bind`'s `ConversionError`.

## Method

- A throwaway example binary (`simplified-schema/lib/examples/`, deleted
  afterwards) lowered each `as` through
  `SimplifiedSchema::try_from("$schema:\n  v: \"<as>\"")`,
  took `properties.v` of the JSON Schema, built the `Shape` with
  `shape_from_json_schema`, and called `coercion::bind`. The same input was
  also run through `validate_soft` for comparison.
- Wrapping `as` in a property adds an outer `null |` arm (an optional
  property admits null). Darkmatter's bare translator
  (`call_binding/translate.rs::shape_from_property_def`) does not add it, and
  the rule below strips null arms anyway, so the results carry over. Object
  properties inside `{ name: string }` *do* stay `null | string` in either
  translator, because they are optional properties.
- Values imitated CSV (all strings) and YAML (mixed natives).

## Failure-tree dumps (trimmed)

`null` options are omitted from the dumps unless they matter.
`P` = `{ name?: null | string, port?: null | number, no other keys }`, which is
the engine's `describe()` text for `{ name: string, port: number }`.

**`number[]`, `["macOS", 2, "3", true, null]`** (YAML control)

```text
"" NoOptionAccepts          (only the wrapper's null arm + number[])
  number[]:
    /0 expected=number received="macOS" MalformedText
    /3 expected=number received=true    NoConversion
    /4 expected=number received=null    NoConversion
```

**`{ name: string, port: number }[]`, seven rows, five bad.** Every bad row
is reported, not only the first:

```text
  P[]:
    /1/port  "null | number" received="eighty" NoOptionAccepts
      null:   NoConversion
      number: MalformedText "not a number"
    /2/port  "null | number" received=""  NoOptionAccepts → number: MalformedText
    /5/extra expected=P received="z" UnexpectedProperty
    /6/port  "null | number" received="x" NoOptionAccepts → number: MalformedText
```

Rows `/0` (`"8080"`) and `/3` (`" 42 "`) converted and are absent, as they
should be. Row `/4` (no `port`) is fine because `port` is optional.

**`tuple([string, number])[]`, `[["a","1"],["b","x"],["c"],["d","4","extra"]]`**

```text
/1/1 expected=number received="x" MalformedText
/2/1 expected=number received=<missing> MissingRequired
/3/2 expected=tuple([string, number]) received="extra" UnexpectedElement
```

**`{a: string}[] | {b: number}[]`, `[{"b":"1"},{"b":"x"},{"b":"y"}]`.** Both
arms are reachable and fail differently:

```text
"" NoOptionAccepts
  {a?…}[]: /0/b, /1/b, /2/b UnexpectedProperty
  {b?…}[]: /1/b, /2/b → number: MalformedText
```

**`({a: string} | {b: number})[]`, `[{"b":"1"},{"b":"x"},{"c":"z"},"scalar"]`.**
The union sits per element:

```text
/1 NoOptionAccepts  {a?}: /1/b UnexpectedProperty ; {b?}: /1/b → number MalformedText
/2 NoOptionAccepts  {a?}: /2/c UnexpectedProperty ; {b?}: /2/c UnexpectedProperty
/3 NoOptionAccepts  {a?}: /3 NoConversion        ; {b?}: /3 NoConversion
```

**`string[] | {a: string}`, mismatched top-level values**

```text
{"b":1}  → string[]: "" NoConversion ; {a?}: /b UnexpectedProperty
{"a":[1]}→ string[]: "" NoConversion ; {a?}: /a → string: NoConversion
42       → string[]: "" NoConversion ; {a?}: "" NoConversion
```

**`number[] | boolean[]`, `["x","true"]`**

```text
number[]:  /0 MalformedText, /1 MalformedText
boolean[]: /0 MalformedText
```

### CSV cell conversions (text to number or boolean)

| Cell | `number` | `boolean` | `string` |
|---|---|---|---|
| `"8080"` | 8080 | — | kept |
| `" 42 "` | 42 (surrounding whitespace is trimmed) | — | kept |
| `""` | **error** (`not a number`) | **error** (`not a boolean word`) | kept `""` |
| `"1e3"`, `"+5"`, `"-0"`, `"1_000"` | 1000, 5, 0, 1000 | — | |
| `"1,000"`, `"NaN"`, `"Infinity"`, `"0x10"` | error | — | |
| `"9007199254740993"` | error (`NumberNotAdmitted(Inexact)`) | — | |
| `"true"`, `"TRUE"`, `"yes"`, `"on"`, `"1"` | — | true | |
| `"False"`, `"no"`, `"0"` | — | false | |

The vocabulary is YAML-1.1-like (`yes`/`on`/`1` read as booleans, `1_000` as a
number). That is the engine's existing rule, and it applies to CSV cells
unchanged. The docs should say so, because a CSV author will not expect `"1"`
to read as `true`.

### `validate_soft` comparison (the false positives)

| Case | `bind` + rule | `validate_soft` problems |
|---|---|---|
| `{name,port}[]`, row 1 has an extra column | `/1/extra` | `/0/port "8080"`, `/1/port "1"`, `/1` unknown key |
| `tuple([string,number])[]`, row 1 too long | `/1/2` | `/0/1 "1"`, `/1/1 "4"`, `/1/2` |
| `{a}[] \| {b:number}[]` | one record at `""` | 6 records mixing both arms (`/0` unknown key **and** `/0/b` not a number) |
| `number[] \| boolean[]` | one record at `""` | 4 records, the same pointer listed twice |
| `tuple(...)[]`, short row | `/1/1` missing | `/1` "has less than 2 items" |

Cause: `coerce.rs::partial_copy` sets failing paths aside by deferring them.
`UnexpectedProperty` and `UnexpectedElement` are not reached through the
deferral predicate, so the partial copy fails (`more` is empty), the whole
top-level property stays as written, and the JSON Schema validator then
judges unconverted text. This is a pre-existing frontmatter defect for nested
containers with an extra key or element. For CSV it is the common case: a
header column that the schema does not declare.

## Failure ordering

The order is deterministic and follows the document. The engine walks arrays
by ascending index and objects in the **value's** key order (`serde_json`
`preserve_order`, so a CSV header order or YAML key order is kept; unexpected
keys appear in their own place). Union options appear in declared order.
Flattening walks in that order, so taking the first 20 records gives the first
20 violating locations in the file. The 25-row fixture produced `/0/port` …
`/24/port` in order.

## Proposed rule

`flatten(failures) → Vec<Record{pointer, expected_type, actual_type}>`, in
failure order:

1. **Not a union failure** → one record at `failure.path`:
   - `MissingRequired` → `actual_type: "missing"`;
   - `UnexpectedProperty` / `UnexpectedElement` → `expected_type: "absent"`
     (the engine's `expected` names the *container* here, which misleads);
   - otherwise `expected_type = failure.expected`.
   - Map `actual_type` from `ValueKind` to the schema vocabulary:
     `text→string`, `list→array`; `number`, `boolean`, `null`, and `object`
     are unchanged.
2. **`NoOptionAccepts { options }`:**
   1. Drop arms whose `expected` is `null`.
   2. An arm is **reachable** for the value's runtime kind unless it failed
      with `NoConversion` *at the union's own path*. That failure means no
      conversion leads from the value's kind to the arm's top-level kind.
      Every other failure (deeper paths, `MalformedText`, a constraint)
      means the value got into the arm.
   3. If **exactly one** arm is reachable → recurse into that arm's failures.
   4. Otherwise (zero, or two or more) → **one** record at the union's path,
      with `expected_type` set to the remaining arms joined with ` | ` and
      `actual_type` set to the value's kind.
3. **`AmbiguousUnion` / `ConflictingDeclarations` / others** → rule 1.

Steps 2.1 and 2.3 extend `coerce.rs::flatten_failures`, which already looks
through a union whose only non-null option failed. Step 2.2 is the new part.

### Results of the rule (pointer · expected · actual)

| `as` / value | Records |
|---|---|
| `number[]`, YAML control | `/0` number string · `/3` number boolean · `/4` number null |
| `number[] \| null`, `["macOS","2","x"]` | `/0` number string · `/2` number string |
| `number[] \| null`, `{"a":1}` | `""` number[] object |
| `tuple([string, number])[]` | `/1/1` number string · `/2/1` number missing · `/3/2` absent string |
| `{a: string}[] \| {b: number}[]`, all `b` rows | `""` `{a?…}[] \| {b?…}[]` array (**one record**) |
| `({a: string} \| {b: number})[]` | `/1`, `/2` both arms object · `/3` both arms string |
| `{name, port}[]`, 5 bad rows | `/1/port`, `/2/port`, `/6/port` number string · `/5/extra` absent string |
| `string[] \| {a: string}`, `{"b":1}` | `/b` absent number |
| `string[] \| {a: string}`, `{"a":[1]}` | `/a` string array |
| `string[] \| {a: string}`, `42` | `""` `string[] \| {a?…}` number |
| `number[] \| boolean[]`, `["x","true"]` | `""` `number[] \| boolean[]` array |
| 25 bad rows | 25 records `/0/port`…`/24/port`; cap → 20 + "5 more" |

**Known limit.** Two container arms of the same kind (`{a}[] | {b}[]`,
`number[] | boolean[]`) give one record at the union's location, so there are
no per-row pointers. That is the honest answer: nothing in the data says
which arm was meant. Picking the arm with the fewest failures was considered
and rejected, because it can blame rows for violating an arm the author never
intended. The record's `message` can carry the engine's `Display` text, which
already lists each arm's failures, cut to a few lines.

## `expected_type` / `actual_type`

- **`actual_type`** is meaningful everywhere once mapped to the schema
  vocabulary (`string`, `number`, `boolean`, `null`, `array`, `object`, plus
  `missing`).
- **`expected_type`** is meaningful for leaf records (`number`, `string`,
  `tuple(...)`). For union-location and object-element records it is the
  engine's `describe()` text (`{ a?: null | string, no other keys }[]`). That
  text is correct but verbose, and it spells optional properties as
  `null | T`. Recommendation: at pointer `""`, use the `as` argument **as
  written**, since there the union *is* `as`. Elsewhere, keep `describe()`.
  `absent` and `missing` are the two sentinels.

## Ownership

- **simplified-schema** owns the flattening:
  `ConversionError::located_failures()`, or a free function
  `coercion::flatten_located`, returning
  `{ path: Vec<PathSegment>, expected: String, actual: ActualKind, failure: &ValueFailure }`
  in engine order. Three reasons:
  1. It interprets `FailureReason` semantics: `NoConversion` at the union's
     path means unreachable, `null` arms, and the unexpected/missing
     sentinels. Those semantics change with the engine.
  2. `coerce.rs::flatten_failures` already does steps 2.1 and 2.3 privately.
     The two should become one function, so frontmatter annotation and
     `load_data` agree.
  3. The schema-ownership guard
     (`darkmatter/lib/tests/l1/schema_language_ownership_guard.rs`) exists to
     keep coercion interpretation out of Darkmatter.
- The pointer encoding should reuse `validation.rs::JsonPointer`, or the
  private `coerce.rs::pointer`, rather than add a third encoder.
- **Darkmatter** owns policy: the code `dm.data.contract_mismatch`, the cap of
  20 plus the overflow record, `data_location` (file, pointer, and line/column
  through `PositionMap`), substituting the `as` text at the root, and message
  wording.

## Recommended spec changes

- **R5 requirement.** Replace "converted and validated against `as` by the
  same coercion and validation path that frontmatter uses" with: *converted by
  the coercion engine (`coercion::bind` over the bare shape of `as`). On
  failure, records come from the engine's `ConversionError` through the
  simplified-schema flattening helper, never from `validate_soft`'s JSON
  Schema problems.* Add a design note about the `partial_copy` false
  positives, either as a fix to schedule for frontmatter or at least as the
  reason for the choice.
- **R5 acceptance criteria, add:**
  - `{ name: string, port: number }[]` over a CSV with an undeclared column
    yields exactly one record per row at `/i/<column>` with `expected_type`
    `absent`, and **no** record for a convertible `port` cell.
  - Several bad rows yield one record each, in file order (`/1/port`,
    `/2/port`, …), and the 25-violation fixture lists rows 0–19 and then
    "5 more".
  - `"number[] | null"` over `["macOS"]` reports `/0`, `number`, `string`.
    The null arm never appears in `expected_type`.
  - `"{a: string}[] | {b: number}[]"` over data that fits neither arm yields
    **one** record at pointer `""` with `expected_type` equal to `as` as
    written and `actual_type` `array`.
  - `"string[] | {a: string}"` over the YAML object `{a: [1]}` reports `/a`,
    `string`, `array`, because only the object arm is reachable.
  - A missing tuple slot reports `actual_type` `missing`, and an extra
    element or key reports `expected_type` `absent`.
- **R4 acceptance criteria, add:**
  - An empty cell against `number` or `boolean` is a contract mismatch at that
    cell. Against `string`, it is accepted as `""`.
  - `" 42 "` converts to `42`.
  - The documented cell vocabulary follows the engine: `yes`, `on`, and `1`
    read as `true`, and `1_000` reads as `1000`. One fixture pins this, so a
    later engine change is a visible behavior change.
- **R2.** No change. The container predicate and the flattening rule share
  the notion "drop null arms", and the helper's doc comment should name that
  link.
