---
total_phases: 6
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Implementation Plan: Schema Enhancements

## Scope and Successful Completion

Implement `2026-10-28-schema-enhancements` in Darkmatter, DMLS, and Claudine's
catalog consumers. Extend SimplifiedSchema with tuples, function descriptions,
compact unions, categories, and executable examples. Build one conversion core
for frontmatter and function arguments, migrate every registered function to
binding before computation, standardize numbers on `f64`, and use one passive
compatibility API for composition rejection and editor diagnostics.

Success means all specification acceptance criteria 1–59, including 33a and
33b, have named evidence. The replacement catalog is complete, all 117 existing
examples are accounted for, at least 72 executable examples pass, and every
registered function has dispatch-level accepted, rejected, and converted-input
coverage under the fixture-applicability ruling recorded in Phase 1. Invalid
arguments never enter handlers. Lazy calls remain lazy; returned values are
checked without conversion; failed conversion never mutates input. Composition
and DMLS agree on proven errors before evaluation, including dead branches.
The behavior ledger records every function and its null contract, current docs
and skills agree with behavior, and scoped validation passes. Implementation
ends ready for review; the author reviews the ledger before merge and closes
the feature after the review cycle.

Waves are globally numbered execution steps. Each wave waits for the preceding
checkpoint. Tasks marked **parallel** can run in separate implementation
subagents with the stated file ownership; shared types, dispatch wiring, Cargo
feature changes, and consolidated test registration have one integrator. Each
task includes relevant tests and a review of changed symbols' comments. The
whole agent tree is non-interactive: no credential prompts, terminal focus,
commits, or feature closure commands. This plan creates no implementation changes.

## Phase 1: Establish Contracts and Reduce Risk

### Necessary Rules

1. **Authority and prerequisites.** `spec.md` governs; the supporting design,
   charter annexes, and spikes contain superseded policies. Confirm that
   `2026-10-16-core-schema-types` supplies presence and core types, that
   `2026-10-14-union-type-constraints` supplies the list-of-union representation,
   and that `2026-10-18-fm-namespace` supplies static `fm` access. Reuse their
   implementations. Their names and dates are not evidence of completion.
   The inspected tree contains ongoing prerequisite edits; do not overwrite
   them or treat a partially implemented dependency as an approved contract.
   Missing prerequisite behavior blocks the dependent implementation task,
   while independent inventory and fixtures may proceed.
2. **One owner and one release.** Darkmatter owns descriptors, conversion,
   binding, variable projection, and compatibility rules. DMLS and Claudine
   consume public APIs. All built-ins switch together: parallel handler edits
   are a development technique, never a pilot release, compatibility mode, or
   mixed-dispatch milestone.
3. **Presence and null.** Document properties reuse effective presence from
   Phase A. Function arguments and tuple slots are required by default;
   optional means omission, not explicit null. At a function parameter only
   an explicit null arm admits null, including parameters spelled `any` or
   `unknown`. The family table's shorthand “or any” does not override this
   normative rule: predicates that inspect null declare `unknown | null`.
   Recursive unconstrained values still keep their contents, including null
   elements in `any[]`; do not impose the argument-boundary rule on descendants.
4. **Registration facts.** Rust registration holds evaluation mode, aliases,
   predicate facts, source exclusions, and conversion recovery. Share these
   facts between binding and whole-call classification. No `behavior:` block,
   `propagates-null` flag, function-specific DMLS setting, or second signature
   vocabulary is introduced. Higher-order descriptions are legal, but an
   executable registration requiring callable runtime values is rejected.
5. **Existing spikes are sufficient.** Do not repeat the baseline or engine
   spike. Reuse 3,575 recorded baseline outcomes, the ledger draft, and settled
   translator/format findings. Existing warm samples are 0.41–2.84 µs for the
   core and 3–19 µs for one-time translation; they answer the initial engine
   cost question. There is no performance target or scheduled performance
   spike. Only a newly unexplained cost warrants one quick before/after sample
   on one host and one representative fixture, separated from I/O. Wider
   measurements require an author ruling and are not in this plan.
6. **Numeric ingress spike.** Run the bounded spelling-preservation spike
   below once, before numeric implementation. It selects a lossless loading
   seam, not a different number model. Runtime numbers remain finite `f64`;
   whole-number admission checks mathematical input before rounding, decimals
   may round, and nonzero underflow errors. Numeric literals, native YAML/JSON,
   defaults, enums, and constraints all use that seam. Already supplied finite
   `f64` values receive only a finiteness check.
7. **Raw schemas remain authoritative.** Translation proposes conversions;
   complete Draft 2020-12 validation decides admissibility. Preserve `oneOf`,
   `$ref` siblings, overlapping properties/patterns, typed additional keys,
   local reference identity/dialect, unmodeled assertions, and bounded cycles.
   Never fetch a remote reference or accept an unresolved reference to enable
   conversion. Candidates start independently from the original value.
8. **Configuration failure policy.** Add `expressions.strict` to the existing
   configuration layers, default false, editor overlay over workspace file.
   Inspection found comment/code drift: `ConfigState` claims an invalid merge
   retains the previous effective config, but `recompute` uses the file layer.
   Preserve the code's file-layer fallback and fix that comment in the config
   task; pin it through diagnostic output. Explicit null at `expressions` or
   `strict` is invalid, whereas whole-settings null clears the overlay under
   the existing transport contract. Reload without restart.
9. **Duplicate policy and reader scope.** Reject duplicate keys in changed
   YAML schema/frontmatter and TOML configuration readers. For JSON readers
   changed for numeric admission or raw schema loading, use rejection of
   duplicate keys as the planned policy; JSON itself does not forbid them,
   so if an established consumer requires otherwise, record an author ruling
   before changing that contract. LSP already-decoded objects cannot reveal
   duplicate wire keys; transport validation owns that cell. Do not claim a
   `serde_json::Value` test proves duplicate detection. Inventory supported
   formats first; do not add new formats just to fill a matrix.
10. **Scope limits and closure.** No flow-sensitive narrowing, operator or
    return-type inference, recursive type-system expansion, strict-root
    validation, `is_file_type`, callable runtime values, or new CLI strictness
    policy. Rows f–i of the worked example retain this phase's conservative
    types. Human ledger review remains a pre-merge requirement; non-interactive
    agents prepare the concrete ledger and report review outstanding, without
    inventing approval or moving this directory to `_completed`.

11. **Fixture applicability needs a ruling.** Criterion 10 literally requires
    accepted, rejected, and converted-input fixtures for every function. A void
    function has no convertible argument, and preservation-only `unknown`
    parameters intentionally keep their input. Before declaring that criterion
    complete, obtain an author ruling on those impossible converted-success
    cells. Recommend a registry-wide fixture row that explicitly marks the
    impossible success, still runs candidate conversion inputs through normal
    dispatch where arguments exist, and covers actual conversion with registered
    probes. This is an acceptance clarification, not permission to skip a
    function, narrow the catalog, or weaken its signature. Record unresolved
    applicability as blocking criterion 10. Also settle duplicate/empty category
    names here; recommend nonempty unique names while preserving list order.

### Wave 1 — Inventory and one-time investigation

- [ ] **Contract inventory** — integrator; prerequisite for every later wave.
  - Map each acceptance criterion to a task, test identity, and public result
    in `implementation-log.md`. Inventory actual canonical names, aliases,
    evaluation modes, handler families, descriptors, examples, and current
    public catalog consumers; use registry enumeration, not “about 110.”
  - Start the per-function behavior ledger from `current-state-coercion.md`,
    `spikes/coercion-baseline/baseline.json`, and `ledger-draft.md`; record null
    family/analogy and every intentional change. Reserve one fixture row per
    canonical function, with alias checks separate from documentation entries.
  - Inspect `markdown/schemas/{coerce,reference,rewrite,example}.rs`,
    `compose/schema_validation.rs`, `expression/{catalog,functions,prepared}`,
    DMLS configuration/overlays, and Claudine's context renderer. Record API
    and file ownership before assigning parallel edits.
  - Complete the reader field/format inventory and matrix expansion below.
    Capture existing loader/error/fallback behavior; identify any policy
    conflict as an explicit Phase 1 ruling, not an implementation guess.

- [ ] **Numeric ingress** — bounded spike, **parallel** with inventory; own
  throwaway spike artifacts only.
  - Trace production YAML/JSON loading and expression numeric tokenization for
    `9007199254740993`, `.0`, exponent spelling, negative boundaries, `0.1`,
    overflow, and underflow; include numeric schema defaults/enum/constraints.
  - Prove one seam can retain spelling or equivalent lossless parsed input
    until admission, including JSON/YAML text parsed by content-format types.
    Preserve nested map order and source spans. Do not implement arbitrary
    precision runtime arithmetic or copy the obsolete spike's integer model.
  - Record the selected seam, required parser features, affected public APIs,
    and a small proof using actual loaders. This spike runs once and is not
    scheduled as a later re-check; production regression tests follow normally.

**Checkpoint:** Prerequisite status, shared interfaces, all reader fields,
fixture sources, numeric admission seam, and every unresolved author ruling are
recorded. No dependent implementation proceeds on an unresolved contract.

### Input Robustness Matrix

This applies to changed YAML/JSON schema and numeric/frontmatter loaders and
TOML/JSON DMLS configuration. It excludes the tuple/function/expression text
grammars, documentation generation, and rendering. Those receive grammar and
output tests separately. The tables below establish outcomes; Wave 1 expands
parameterized columns to every concrete field/path the changed reader handles,
for each format it actually supports. No matrix cell may be left implicit at
implementation time.

`E` means an observable load/validation error, never a usable partial catalog
or converted document. `K` means successful unchanged data; `C` means successful
conversion through the common engine. `V` means complete schema validation,
with the assertion still in force. `N/A` needs an explicit reason and is not
an omitted test. All errors preserve caller inputs. A row that supplies wrong
collection elements tests one bad element and then all bad elements.

YAML standalone/catalog envelope (repeat the `$schema` column for JSON schema
envelopes only if the inventory confirms that reader supports them):

| Shape | `kind` | `types` | `types.<canonical>` / named type | `categories` | root `$schema` |
|---|---|---|---|---|---|
| Positive control | `schema`: catalog usable | real catalog resolves canonicals | real signature registered | actual category order rendered | real standalone schema validates |
| Absent | not a tagged envelope; catalog load E | tagged catalog E | missing registered canonical E | unrestricted category names | not a pure envelope; requested schema load E |
| Explicit null | not a tagged envelope; catalog load E | E | E | E; differs from no category list | E |
| Wrong whole type | catalog load E | E | E | E | E |
| One wrong element | N/A scalar | invalid mapped type E | invalid union arm E | E; no filtering | invalid declaration E |
| Every element wrong | N/A scalar | E | E | E; not empty | E |
| Empty | empty name: catalog load E | empty schema valid; registered catalog incomplete E | empty union/type E | declared empty list; category uses E | empty mapping schema valid; reference spelling empty E |
| Duplicate key | E at YAML reader | E | E | duplicate YAML key E | E |
| Trailing/invalid content | E | E | E | E | E |


`kind` classifier results must be asserted through explicit catalog/schema
loading or document behavior, not the standalone parser's optional return.
`categories: []` is a declared list, distinct from no list. Duplicate names
inside that list are a separate semantic case to settle against the existing
schema-definition contract in Wave 1; they are not duplicate mapping keys.

DMLS readers (file and editor sides are separate columns):

| Shape | TOML `expressions` | TOML `expressions.strict` | JSON editor `expressions` | JSON editor `expressions.strict` | JSON settings root / `dmls` section |
|---|---|---|---|---|---|
| Positive control | table with strict true: categories 2/4 warn | true: categories 2/4 warn | strict true overrides false file | true: categories 2/4 warn | wrapped/bare settings yield strict diagnostics |
| Absent | false default | false default | file setting retained | file setting retained | existing no-overlay contract |
| Explicit null | N/A TOML; invalid null spelling E | N/A TOML; invalid null spelling E | invalid overlay: log and use file layer | invalid overlay: log and use file layer | clear overlay; use file layer |
| Wrong whole type | file error reported; defaults/discovery fallback | same | invalid overlay: file layer | invalid overlay: file layer | non-object logged; existing overlay retained |
| One wrong element | collection supplied instead of table: E | collection supplied instead of bool: E | malformed collection: file layer | malformed collection: file layer | malformed transport/settings rejected or logged per existing boundary |
| Every element wrong | E | E | file layer | file layer | same transport policy |
| Empty | empty table: false default | empty string/collection: E | empty object: retain file fields | empty string/collection: file layer | empty settings: clear overlay under existing contract |
| Duplicate key | E | E | transport-owned; not detectable after decode | transport-owned; not detectable after decode | transport-owned; preserve established protocol reader policy |
| Trailing/invalid content | E | E | invalid wire JSON never applied | invalid wire JSON never applied | invalid wire JSON never applied |


Test malformed files through file discovery/reload plus diagnostics/logging.
Test malformed editor objects through `ConfigState` and published diagnostics;
exercise raw transport syntax only through the actual protocol reader. Explicit
`false` must suppress the strict warnings even when the file says true.

Raw JSON Schema loader/translator fields (JSON; repeat for supported YAML raw
schema documents). Each keyword has its own column. Wave 1 assigns its concrete fixture
path and producer provenance before implementation.

| Shape | `$id` | `$anchor` | `$ref` | `$defs` | `definitions` |
|---|---|---|---|---|---|
| Positive control | resolved identity/reference constrains converted value | resolved identity/reference constrains converted value | resolved identity/reference constrains converted value | referenced/property/pattern number converts | referenced/property/pattern number converts |
| Absent | no identity/reference contributed | no identity/reference contributed | no identity/reference contributed | no definitions/declarations contributed | no definitions/declarations contributed |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E |
| One wrong element | N/A scalar | N/A scalar | N/A scalar | bad schema value E | bad schema value E |
| Every element wrong | N/A scalar | N/A scalar | N/A scalar | E | E |
| Empty | resolve/validate empty string according to keyword; unresolved E | resolve/validate empty string according to keyword; unresolved E | resolve/validate empty string according to keyword; unresolved E | empty map valid; assertion absent by format | empty map valid; assertion absent by format |
| Duplicate key | E under planned reader policy | E under planned reader policy | E under planned reader policy | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | `properties` | `patternProperties` | `type` | `anyOf` | `oneOf` |
|---|---|---|---|---|---|
| Positive control | referenced/property/pattern number converts | referenced/property/pattern number converts | declared number converts | actual alternatives constrain result | actual alternatives constrain result |
| Absent | no definitions/declarations contributed | no definitions/declarations contributed | no type assertion; never invented conversion | no alternatives contributed | no alternatives contributed |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E |
| One wrong element | bad schema value E | bad schema value E | invalid member of type array E | bad alternative E | bad alternative E |
| Every element wrong | E | E | E | E | E |
| Empty | empty map valid; assertion absent by format | empty map valid; assertion absent by format | empty/unknown type or empty array E | empty alternatives E | empty alternatives E |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | `items` | `additionalProperties` | `prefixItems` | `required` |
|---|---|---|---|---|
| Positive control | typed item/additional key converts | typed item/additional key converts | tuple/required property constrains result | tuple/required property constrains result |
| Absent | dialect default applies; not an empty schema list | dialect default applies; not an empty schema list | no tuple prefixes/required names | no tuple prefixes/required names |
| Explicit null | E | E | E | E |
| Wrong whole type | E | E | E | E |
| One wrong element | bad schema structure E | bad schema structure E | bad prefix/name E | bad prefix/name E |
| Every element wrong | E | E | E | E |
| Empty | empty schema object accepts; no conversion invented | empty schema object accepts; no conversion invented | empty list valid; tuple tail/required semantics retained | empty list valid; tuple tail/required semantics retained |
| Duplicate key | E | E | E | E |
| Trailing/invalid content | E | E | E | E |


| Shape | `enum` | `const` | `default` | `minimum` | `maximum` |
|---|---|---|---|---|---|
| Positive control | allowed value/default changes public validation/composition | allowed value/default changes public validation/composition | allowed value/default changes public validation/composition | boundary/divisibility changes validation | boundary/divisibility changes validation |
| Absent | no enum/const/default supplied | no enum/const/default supplied | no enum/const/default supplied | no numeric assertion supplied | no numeric assertion supplied |
| Explicit null | E | null is an asserted value; V | present null annotation; existing default stage decides | E | E |
| Wrong whole type | non-array E | all JSON values legal; V | all JSON values legal; existing default stage decides | E; numeric tokens admitted losslessly | E; numeric tokens admitted losslessly |
| One wrong element | heterogeneous JSON values valid; V against surrounding assertions | arbitrary nested data legal; V | arbitrary nested data legal; existing stage validates if applied | collection in place of number E | collection in place of number E |
| Every element wrong | same heterogeneous-value rule; never filter enum | arbitrary nested data legal; V | same; no filtering | E | E |
| Empty | [] E | empty value remains an assertion; V | empty value is present; existing default stage decides | empty string/collection E | empty string/collection E |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | `exclusiveMinimum` | `exclusiveMaximum` | `multipleOf` | `minLength` | `maxLength` |
|---|---|---|---|---|---|
| Positive control | boundary/divisibility changes validation | boundary/divisibility changes validation | boundary/divisibility changes validation | count bound changes validation | count bound changes validation |
| Absent | no numeric assertion supplied | no numeric assertion supplied | no numeric assertion supplied | no count assertion supplied | no count assertion supplied |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E; numeric tokens admitted losslessly | E; numeric tokens admitted losslessly | E; numeric tokens admitted losslessly | E | E |
| One wrong element | collection in place of number E | collection in place of number E | collection in place of number E | collection in place of count E | collection in place of count E |
| Every element wrong | E | E | E | E | E |
| Empty | empty string/collection E | empty string/collection E | empty string/collection E | empty string/collection E | empty string/collection E |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | `minItems` | `maxItems` | `minProperties` | `maxProperties` | `pattern` |
|---|---|---|---|---|---|
| Positive control | count bound changes validation | count bound changes validation | count bound changes validation | count bound changes validation | matching text changes validation |
| Absent | no count assertion supplied | no count assertion supplied | no count assertion supplied | no count assertion supplied | no text/format assertion |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E |
| One wrong element | collection in place of count E | collection in place of count E | collection in place of count E | collection in place of count E | collection in place of string E |
| Every element wrong | E | E | E | E | E |
| Empty | empty string/collection E | empty string/collection E | empty string/collection E | empty string/collection E | empty regex valid; empty format uses dialect's unknown-format behavior |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | `format` | `uniqueItems` |
|---|---|---|
| Positive control | matching text changes validation | duplicates rejected when true |
| Absent | no text/format assertion | duplicates allowed by format |
| Explicit null | E | E |
| Wrong whole type | E | E |
| One wrong element | collection in place of string E | collection in place of bool E |
| Every element wrong | E | E |
| Empty | empty regex valid; empty format uses dialect's unknown-format behavior | empty string/collection E |
| Duplicate key | E | E |
| Trailing/invalid content | E | E |


Raw `default` remains an annotation unless an existing surrounding stage applies
it; tests observe that stage and do not invent raw-schema default insertion.
`const`/`enum` values are arbitrary JSON data, so “wrong element type” is only
wrong where the authored schema says so. Validation-only fields reached by the changed loader also retain their own
columns below. They assert on final candidates and gain no conversion rules.
For these controls, removing the field must change validation, reference
resolution, or the effective dialect. A conditional `then`/`else` control
includes the matching `if` needed to exercise that branch.

| Shape | `allOf` | `not` | `if` | `then` |
|---|---|---|---|---|
| Positive control | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result |
| Absent | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default |
| Explicit null | E | E | E | E |
| Wrong whole type | E | E | E | E |
| One wrong element | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E |
| Every element wrong | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E |
| Empty | empty array E | {} rejects every instance; false accepts every instance | {} selects then for every instance | {} leaves the selected branch unconstrained |
| Duplicate key | E under reader policy | E under reader policy | E under reader policy | E under reader policy |
| Trailing/invalid content | E under reader policy | E under reader policy | E under reader policy | E under reader policy |

| Shape | `else` | `dependentRequired` | `dependentSchemas` | `propertyNames` |
|---|---|---|---|---|
| Positive control | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result |
| Absent | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default |
| Explicit null | E | E | E | E |
| Wrong whole type | E | E | E | E |
| One wrong element | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E |
| Every element wrong | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E | invalid child/name/value E |
| Empty | {} leaves the selected branch unconstrained | {} adds no dependency; [] entry requires no names | {} adds no dependency; {} entry is an unconstrained schema | {} accepts all names |
| Duplicate key | E under reader policy | E under reader policy | E under reader policy | E under reader policy |
| Trailing/invalid content | E under reader policy | E under reader policy | E under reader policy | E under reader policy |

| Shape | `contains` | `minContains` | `maxContains` | `unevaluatedItems` |
|---|---|---|---|---|
| Positive control | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result |
| Absent | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default |
| Explicit null | E | E | E | E |
| Wrong whole type | E | E | E | E |
| One wrong element | invalid child/name/value E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E | invalid child/name/value E |
| Every element wrong | invalid child/name/value E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E | invalid child/name/value E |
| Empty | {} matches any item; empty instance still subject to minContains | empty string/collection E | empty string/collection E | {} permits unevaluated items |
| Duplicate key | E under reader policy | E under reader policy | E under reader policy | E under reader policy |
| Trailing/invalid content | E under reader policy | E under reader policy | E under reader policy | E under reader policy |

| Shape | `unevaluatedProperties` | `$schema` | `$dynamicRef` | `$dynamicAnchor` |
|---|---|---|---|---|
| Positive control | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result | authored assertion/reference changes public result |
| Absent | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default | no keyword contributed; retain dialect default |
| Explicit null | E | E | E | E |
| Wrong whole type | E | E | E | E |
| One wrong element | invalid child/name/value E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E |
| Every element wrong | invalid child/name/value E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E | N/A scalar; collection supplied instead is E |
| Empty | {} permits unevaluated properties | empty/unknown dialect follows validator error contract; no default guessed | resolve empty reference by dialect; unresolved E | empty anchor E |
| Duplicate key | E under reader policy | E under reader policy | E under reader policy | E under reader policy |
| Trailing/invalid content | E under reader policy | E under reader policy | E under reader policy | E under reader policy |

| Shape | `$vocabulary` |
|---|---|
| Positive control | authored assertion/reference changes public result |
| Absent | no keyword contributed; retain dialect default |
| Explicit null | E |
| Wrong whole type | E |
| One wrong element | invalid child/name/value E |
| Every element wrong | invalid child/name/value E |
| Empty | {} declares no vocabularies |
| Duplicate key | E under reader policy |
| Trailing/invalid content | E under reader policy |


Frontmatter/numeric input fixture, YAML and every supported JSON value-loading
surface; expand each archetype to actual changed load-bearing property paths:

| Shape | required numeric `n` | optional numeric `maybe` | numeric preservation `zip` | boolean preservation `flag` | typed list `values` |
|---|---|---|---|---|---|
| Positive control | exact number K | number K | `"02134"` K | `"On"` K | numeric text elements C |
| Absent | validation E | existing optional materialization; no invented number | existing optional presence | existing optional presence | optional property presence; never [] |
| Explicit null | validation E | valid property null K; distinct source presence | property null K by optional presence | property null K by optional presence | property null K by optional presence |
| Wrong whole type | boolean/object/list E; numeric text C | same | invalid text/bool/container E | native number/invalid word/container E | scalar E |
| One wrong element | container E | container E | container E | container E | whole value E at index |
| Every element wrong | E | E | E | E | E; not [] |
| Empty | empty text/container E | same | empty text/container E | same | [] valid K unless constrained |
| Duplicate key | YAML/JSON reader E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

| Shape | typed object `meta` | content target `config` |
|---|---|---|
| Positive control | nested numeric text C | native mapping C to JSON text |
| Absent | optional property presence; never {} | optional property presence; never serialized null |
| Explicit null | property null K by optional presence | property null K; not JSON text |
| Wrong whole type | scalar/list E | valid native non-null value serializes; invalid content text E |
| One wrong element | whole value E at property | native data serializes; malformed text E |
| Every element wrong | E; not {} | native data serializes; malformed text E |
| Empty | {} valid unless required child missing | empty native collection serializes; content text follows JSON/YAML syntax |
| Duplicate key | E recursively | outer reader E; textual content follows its documented content-format policy |
| Trailing/invalid content | E | E at outer reader; full consumption of textual content required |


Add required variants, refined string/enum/literal, tuple slots, pattern-key
values, whole-value unions, and numeric schema metadata to the concrete matrix
when inventory identifies their loader paths. Optional tuple null is E, unlike
optional document properties. This distinction must appear in public results.

- [ ] **Matrix fixtures** — follows inventory, before reader implementation.
  - Obtain a fixture from each real writer/tool: shipped SimplifiedSchema
    catalog and `md schema` output, current `md frontmatter` output, DMLS's
    real client configuration exchange, and documented `.dmls.toml` settings.
    Record provenance; use existing raw-schema fixtures with their source
    tool identified or capture one from the actual schema producer.
  - One table-driven test per supported format walks all applicable cells,
    making exactly one edit from an unedited positive control. Maintain separate
    reader-side cases inside the test. Assert catalog usability, composition
    output/error and immutability, or published DMLS diagnostics/config fallback,
    never merely parser return values. Preserve raw text for duplicates and
    trailing content; decoded maps cannot represent those cases.
  - Scan changed readers for load-bearing `#[serde(default)]`, conflating
    `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()`.
    Defaults are allowed only for documented absence. Review findings report
    every cell in the affected row and column, not a lone malformed example.

## Phase 2: Build Grammar and Value Foundations

### Wave 2 — Independent foundations

- [ ] **Grammar descriptors** — **parallel**, own
  `schemas/simplified/{types,grammar,source,serialize,convert,standalone}.rs`
  and grammar tests; depends on Wave 1.
  - Add tuple positions, optional suffixes, final array spread, function
    parameters/returns, rest parameters, repeated inline/file examples,
    fallibility, category annotations, and the `categories:` envelope field.
    Reject every direct and indirect invalid form in criteria 1, 56, and 57.
  - Add `|` and grouping with array precedence; reuse list-of-union lowering.
    Preserve constraints, spans, descriptions, category order, and effective
    property presence. Normalize draft void/optional spellings on migration,
    rather than adding undocumented aliases to the new grammar.
  - Lower tuples and unions to Draft 2020-12; emit category annotations on data
    types. Reject functions anywhere reachable from a document schema with
    bounded reference traversal. Allow description-only higher-order types.
    Compare semantic resolved overload binding contracts, not signature text.

- [ ] **Numeric foundations** — **parallel**, own the admission module,
  ingress seam selected in Wave 1, numeric lexer/evaluator operations, and
  workspace `serde_json` feature change; coordinate shared schema files.
  - Implement canonical numeric/boolean text admission and numeric formatting;
    retain spelling until exactness checks. Use the same admission for native
    inputs, literals, schema metadata, `numberlike`, and conversion functions.
  - Remove integer/saturating paths from operators. Reject non-finite computed
    results and division by zero; preserve truthiness, string concatenation,
    nonnumeric comparisons, and short-circuiting. Test negative/decimal/exponent
    whole-number boundaries and finite externally supplied `f64`.
  - Enable workspace `preserve_order`; audit recursively nested/function-built
    maps and consumers of serialized maps. Inventory actual affected packages
    and preserve established sorted contracts with explicit consumer sorting.
    Do not mechanically replace snapshots or run workspace-wide tests.

**Checkpoint:** Grammar positives/negatives and tuple/union snapshots pass;
numeric admission cannot lose digits before its check; insertion-order effects
are classified. Freeze descriptor and numeric APIs for engine/adapter work.

## Phase 3: Implement the Shared Engine

### Wave 3 — Rule core

- [ ] **Conversion core** — sequential; own a neutral shape module under
  `markdown/coercion/` (or the equivalent agreed module) and core tests.
  - Implement keep/convert/error with one-step conversions, no null conversion,
    constraint checking after conversion, and passive refinements. Preserve
    original `numberlike`/`boolish` values and use canonical numeric admission.
  - Recursively handle lists, tuples, structured objects, patterns and unions;
    build converted copies atomically with paths and complete reasons. Exact
    matches win; apply only number-over-boolean and JSON-over-YAML tie-breakers.
    Inconsistent field-wise choices produce ambiguity naming each reading.
  - Keep omission distinct from null. Expose reusable error details for
    parameter/property source context and return-contract checking without
    conversion or I/O. Respect existing depth/resource bounds.

**Checkpoint:** All primitive/container/union rule tables pass through the core,
including alternative-order permutations, failed-input immutability, and both
preservation formats. Translators receive a stable rule-free shape API.

### Wave 4 — Translators and binding

- [ ] **Schema adapter** — **parallel**, own raw-schema translator,
  `schemas/{coerce,reference,rewrite,example,format}.rs` and
  `compose/schema_validation.rs`; depends on Wave 3.
  - Translate modeled compiled/raw JSON Schema syntax only. Keep resolved
    reference identity and dialect, assertion intersections, full-validator
    candidate filtering, and bounded validation-only fallbacks.
  - Replace existing frontmatter rules and independent root/property union
    selection with the core. Preserve default/presence stages, pending-shell
    deferral, validate-only nonmutation, and compose's existing write-back stage.
  - Register both custom formats using canonical admission; update detect
    snapshots intentionally. Cover raw `oneOf` overlap, `$ref` siblings,
    overlapping patterns/properties, additional keys, `multipleOf`, and cycles.

- [ ] **Call binder** — **parallel**, own grammar-to-shape translator and new
  bound-argument/call-binding APIs; no production dispatch switch yet.
  - Compile callable signatures once and cache descriptors. Bind arity, all
    supplied arguments, omission, optional/rest positions, constraints, and
    order-independent overloads; aggregate every failed parameter/candidate.
  - Add registered source exclusions and recovery for `number`/`round`;
    validate fallback even when the primary input succeeds. Accessors read
    already bound values and never parse or convert.
  - Provide an incremental lazy-binding path, and return validation that
    reports a distinct implementation-contract error without converting data.
    Reject executable registrations requiring function-valued inputs/results.

**Checkpoint:** Every grammar shape has equivalent outcomes through both
translators. Frontmatter acceptance criteria 28–35 and raw-schema assertion
checks pass; binder tests prove no compute on failure, recovery, laziness,
complete diagnostics, and order-independent overloads.

## Phase 4: Migrate Every Function and Catalog Consumer

### Wave 5 — Prepare the complete migration

- [ ] **Catalog replacement** — **parallel**, own catalog YAML,
  `expression/catalog/` and catalog/example tests; depends on Wave 4.
  - Move functions and mutations into `darkmatter/schemas/partials/`; normalize
    envelopes and syntax, remove retired metadata, fix descriptions/categories,
    add the 14 missing canonicals plus `keys`/`values`, and exclude `is_file_type`.
  - Preserve aliases/case-insensitive resolution and canonical-only listing;
    share the named recent-commit result with its context variable. Expose full
    descriptors instead of preserving the old reduced parameter vocabulary.
  - Account for all 117 old examples; migrate structural results/error outcomes,
    file examples, and display-only reasons. The executable runner uses the
    normal evaluator and hermetic fixtures; display-only cases never run.

- [ ] **Pure handlers** — **parallel**, own pure math/string/predicate/
  collection/date handler bodies and their family fixture rows.
  - Replace arity/type/conversion helpers with bound arguments. Apply ledger
    null contracts, `length`'s boolean exclusion, `contains` rendered-value
    behavior, numeric results/formatting, and new insertion-ordered map helpers.
  - Implement `number`/`round` documented computation after binder recovery,
    rounding halfway away from zero without narrowing casts. Keep true
    conversion-function domain operations distinct from input coercion.

- [ ] **Context handlers** — **parallel**, own filesystem/repository/network/
  provider/composition handler bodies and corresponding fixture rows.
  - Migrate all context handlers to bound arguments, including refined count
    limits, content types, and null predicate behavior. `file_exists(42)` binds
    `"42"`; keep request-owned `FileReference` resolution and existing policy.
  - Implement `validate_schema` override merge and repeated nonmutating
    validation. Keep I/O/domain failures typed and separate from binding errors;
    use fake adapters and private filesystem fixtures, never real providers.

**Checkpoint:** All registry entries, handlers, aliases and examples are
accounted for. Parallel changes are assembled on the implementation branch;
there is no completed mixed-runtime release. Shared dispatch wiring is next.

### Wave 6 — Atomic dispatch and consumer switch

- [ ] **Registry integration** — sequential integrator.
  - Join all implementations to the new descriptors and switch pure, context,
    and lazy dispatch together. Preserve `and`/`or` lazy evaluation, unknown
    function suggestions, aliases, source provenance, and bounded execution.
  - Remove all handler-local coercion and unused shared helpers. Run accepted,
    rejected, and converted-input fixtures through normal dispatch for every
    canonical. Apply the Phase 1 author ruling for impossible converted-success cells
    in preservation-only/void contracts; test unchanged delivery rather than
    weakening signatures to manufacture conversions. Test-registered conversion probes cover the core.
  - Instrument at handler entry: rejected calls keep counters at zero; accepted
    calls enter once; skipped lazy effects never enter; wrong-return handlers
    produce implementation-contract errors. Complete ledger coverage against
    recorded baseline and update affected shipped documents in the same change.

- [ ] **Consumer migration** — follows registry integration in this wave.
  - Update DMLS completion/hover and Claudine's
    `cli/src/commands/context/{expressions,format}.rs` to consume Darkmatter's
    full catalog API. Audit other library exports/generated references for old
    signatures or paths; do not copy YAML or conversion rules to a consumer.
  - Render ordered categories and alphabetical canonical functions with both
    example forms using existing terminal components. Delete the old catalog
    only after all readers have switched; preserve fixed transcluded draft
    paths and their document names.

**Checkpoint:** Bidirectional catalog/registry parity holds; all functions bind
through one engine; at least 72 executable examples pass and all 117 are
accounted for. No old catalog reader, reduced descriptor consumer, or redundant
handler coercion remains. Catalog/context rendering snapshots pass.

## Phase 5: Add Passive Checking and Editor Agreement

### Wave 7 — Library checking authority

- [ ] **Variable projection** — sequential, depends on Wave 6.
  - Expose one passive top-level type projection from `EffectiveSchema` using
    shared effective presence. Preserve constraints; optional properties add
    null; raw schemas/mixed root unions and undeclared names yield unknown.
  - Resolve bare document properties and static `fm.x`/`fm["x"]` consistently;
    reserved roots retain registered meanings. Nested/dynamic paths, host
    globals, calls, and operator expressions remain unknown in this feature.
  - Test projection without access to filesystem, remote, or runtime values,
    including inherited/hoisted required markers and null-admitting unions.

- [ ] **Compatibility API** — follows projection; same library owner.
  - Implement five-category argument and whole-call queries from the shared
    rules, including arity, overloads, source exclusions and fallback recovery.
    Typed refinements defer to runtime; literal constraints use passive checks.
  - Test every specified category pair, content-format classification, nullable
    targets, literal ambiguity, impossible calls, and recoverable bad first
    arguments. Offer no speculative batch API without measured need.
  - Extend prepared validation to enumerate executable authored calls with
    argument/call spans through all AST branches, conditional blocks, and
    frontmatter. Share this walk with both consumers; no evaluator invocation.

**Checkpoint:** Criteria 36, 48, 50–53 have library evidence; checking cannot
perform effects and visits skipped branches while preserving runtime laziness.
Freeze the diagnostic result/span API before consumer integration.

### Wave 8 — Parallel consumer integration

- [ ] **Composition precheck** — **parallel**, own preparation/compose wiring
  and composition public-path tests; depends on Wave 7.
  - Run passive checking after schema resolution and before first frontmatter
    interpolation. Reject category 5 for every function before any sibling
    expression executes; preserve interpolation → conversion → shell ordering.
  - Check only the requested surface. Exclude fences, literal tokens, inserted
    data and excluded body content. Check each transcluded child against its
    own origin/schema before child evaluation, without fetching unseen children
    just for parent checking.
  - Test ternary/boolean/dead-block branches, frontmatter effects, nested
    transclusion, frontmatter-only entry points, and conservative rows f–i.

- [ ] **Editor diagnostics** — **parallel**, own DMLS config/overlay/provider
  code and protocol tests.
  - Add reloadable `expressions.strict`, honoring file/editor precedence and
    matrix error/fallback cases. Repair the observed fallback comment drift.
  - Use the compatibility API exclusively. Map category 3 to warnings and 5 to
    errors; strict additionally warns for 2/4. Range argument errors at the
    argument and arity/ambiguity at the call; preserve existing diagnostic codes.
  - Test rows a–e with all variables actually present; exercise file settings,
    `workspace/configuration`, runtime setting changes, malformed overrides,
    recovery and skipped branches. Assert passive diagnostics with effect
    counters and no function-specific settings.

**Checkpoint:** A shared document corpus produces exactly the same proven call
errors in full composition and DMLS. Severity/ranges/settings match the spec;
frontmatter-only and non-executable syntax cases stay within their surface.

## Phase 6: Document, Validate, and Prepare Review

### Wave 9 — Documentation and coverage audit

- [ ] **Current documentation** — **parallel**, own Darkmatter schema/expression
  and numeric/coercion topic docs and associated skill pages.
  - Update the actual schema-definition topic and related type/constraint/
    validation/authoring pages for grammar, categories, both example forms,
    the shared conversion rules, preservation types, containers, unions,
    numbers, errors, and intentional frontmatter changes.
  - Explain composition prechecking and dead branches with a small lifecycle
    Mermaid diagram and runnable examples. State the limits of this phase's
    type projection. Update Darkmatter README/public API docs as needed;
    current docs never link to or name a feature/fix snapshot.
  - Document `preserve_order` in root and Darkmatter dependency docs, including
    intentional ordering and custom-format interoperability.

- [ ] **Consumer documentation** — **parallel**, own DMLS diagnostics/config
  docs, Claudine context docs/README and claudine skill references.
  - Document category severities, `expressions.strict`, reload/precedence,
    errors versus uncertain warnings, recovery, and the shared catalog owner.
    Regenerate function references from the catalog, including category order,
    canonical signatures, executable/display-only/file examples and reasons.
  - Keep examples approachable to developers unfamiliar with the repository;
    remove descriptions claiming old coercion, numeric or null behavior.

- [ ] **Coverage audit** — **parallel**, own evidence mapping and ledger only.
  - Resolve every acceptance ID 1–59 plus 33a/33b to a passing test/public
    result. Audit every canonical's fixture set, null analogy, aliases,
    fallibility, returns, constraints, and baseline disposition.
  - Review completed field-by-field matrices and suspicious deserialization
    patterns; report complete affected rows/columns. Check passive shipped
    schema corpus and normal CLI paths, not just internal unit tests.
  - Determine downstream validation from actual `preserve_order` consumers and
    public API references; use `sniff` for package/repository discovery and
    call-graph tools only when useful. Record required package/target gates.

**Checkpoint:** Docs and generated references describe the implemented behavior;
ledger has no undecided functions, and evidence has no missing criteria or
implicit matrix fields. Remaining human ledger review is explicitly recorded.

### Wave 10 — Final scoped verification

- [ ] **Scoped verification** — sequential integrator.
  - Run `just test` and `just lint` in `darkmatter/` and `claudine/`, covering
    library, CLI, DMLS and their consumers via canonical nextest recipes. Run
    additional affected packages from the ordering audit with root
    `just test {pkg}` and their lint/check recipes as appropriate. Compile
    declared examples/bench targets when their APIs changed.
  - Use `CliProcessFixture`, Test Toolkit and runtime-resolved fixture paths;
    pin fake providers, private home/CWD and silent audio for Claudine subprocess
    tests. No real terminal/browser is needed for catalog or LSP behavior. If
    a changed real-rendering contract requires existing L2 tests, use only
    `just test-l2` and background/headless harnesses that never gain focus.
  - Check macOS, Linux, native Windows and WSL2 compatibility through required
    scoped evidence; load the `os` skill before platform/path work or deciding
    a host is unavailable. Distinguish evidence obtained from code review and
    planned CI coverage; do not claim an unexecuted environment passed.
    Reuse qualifying evidence and follow event-based CI scheduling without
    adding speculative jobs or demanding all-OS PR runs.
  - Do not run `cargo fmt`, `cargo test`, workspace-wide suites, commits, pushes,
    or closure commands. An existing lint recipe's check-only formatting gate
    may identify edits to fix manually. Re-run only gates affected by a fix.

- [ ] **Review package** — follows passing gates.
  - Finish `implementation-log.md` with decisions/departures, actual commands,
    outcomes, fixture/matrix provenance, baseline ledger, changed ordering
    consumers, and outstanding platform evidence or human review.
  - Confirm the old catalog is deleted, production conversions have one owner,
    descriptors are compiled once, docs/skills are current, and unrelated
    pre-existing changes remain untouched. Present the concrete behavior ledger
    for the required human pre-merge review.
  - Report “implementation complete, ready for review” only when technical
    checks pass; separately state that human ledger review is pending if it is.
    Leave the feature directory active for its author to close.

**Checkpoint:** All technical acceptance evidence passes, known limits are
recorded, and the complete review package is ready. Merge remains contingent on
the specification's human ledger review, not on an agent's inferred approval.
