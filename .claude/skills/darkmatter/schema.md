# Darkmatter Schemas

Use this reference for SimplifiedSchema parsing, validation, triggers, imports,
and DMLS schema behavior.

## Contents

- [Document kinds](#document-kinds)
- [Composition and validation](#composition-and-validation)
- [Imports and dictionaries](#imports-and-dictionaries)
- [Named union references](#named-union-references)
- [Special types](#special-types)
- [Suggestions and DMLS](#suggestions-and-dmls)
- [Testing](#testing)

## Document kinds

Darkmatter recognizes two standalone SimplifiedSchema shapes:

```yaml
$schema:
  title: string(required)
```

```yaml
kind: schema
types:
  Person:
    name: string(required)
```

`parse_standalone_schema_document` is the passive authority. A pure schema must
have `$schema` as its only root key. A kinded schema must declare `kind: schema`
and provide a `types` mapping. Referenced YAML without either envelope remains
raw JSON Schema. When a non-empty bare map consists entirely of valid
SimplifiedSchema property strings and has no recognized or reserved JSON
Schema/custom-vocabulary keys, resolution emits the conservative
`dm.schema.missing_simplified_envelope` advisory without changing validity or
interpretation. Remedy it by wrapping the properties under the sole root
`$schema:` key or under `kind: schema` plus `types:`. Classification is passive:
it uses the already-parsed YAML value and performs no additional I/O.

Schema-trigger documents use `kind: schema-trigger`. Preserve the kinded root
declaration so Darkmatter and Claudine can select the right schema formally.

## Composition and validation

SimplifiedSchema compiles to Draft 2020-12 JSON Schema. Composition validates
after initial frontmatter interpolation and before shell expansion, then
revalidates values deferred because they contained pending shell syntax.
Only authored syntax is pending. Data is judged immediately, including a data
override, an expression result, and a decoded literal token. The shared
lexical test is `literal_token::holds_pending_syntax`, which never treats a
whole valid token as pending. Validation checks the token-decoded instance. The
`expression` format validator parses a token's decoded text. Being
string-only, it cannot see origin, so decoded data holding `{{` in an
expression-typed field is still accepted lexically. See
[compose.md](compose.md#inserted-text-is-data).

Validation-only APIs are passive and read-only. Composition may coerce declared
scalar types and normalize a successful eager `file(eager)` value to its
repo-relative path. At the same schema seam, after first-pass frontmatter
interpolation and before coercion, composition materializes an absent optional,
no-default top-level property as `null` only when the winning declaration comes
from the document's inline or referenced SimplifiedSchema. Baseline and trigger
properties, raw JSON Schema, root unions, nested properties, required
properties, and defaulted properties do not materialize. A compose run with no
effective schema and all validation-only APIs remain non-mutating. Repeated
schema passes are idempotent, and present values are preserved exactly.

Runtime consumers use `SchemaPhase::Launch` and `SchemaPhase::Completion` on
an already-resolved `EffectiveSchema`. `required` and `eager` are independent
axes, derived recursively:

| Declaration | Launch | Completion |
|---|---|---|
| neither | absence allowed; a present value is type-checked | absence allowed; a present value is type-checked |
| `eager` | absence allowed; a present value is validated eagerly | absence allowed; a present value is type-checked |
| `required` | absence may be deferred; a present value is type-checked | must be present and valid |
| `required; eager` | must be present and valid | must be present and valid |

Explicit null counts as absence, so an eager-only null is allowed at both
phases. Completion observes the final working instance without coercion, while
the existing unphased `validate*` authoring behavior remains unchanged. Raw JSON
Schema retains its authored `required` behavior at both phases, and trigger
match conditions reject `eager` because matching has no runtime phase.

Caller records retain an immutable raw value and file-resolution origin per
property. Before frontmatter interpolation pass 1, an exactly selected eager or
non-recursive lazy file arm materializes that value from its caller origin.
Eager local files must exist; lazy local files bind their first ordered
candidate without a probe, lazy HTTP(S) values remain remote, and recursive
lazy values fail because they have no single unprobed identity. This prelude
does not validate or mutate document-authored values. Markdown body
interpolation reads a separate portable presentation value, including through
static member and index selection; path operations, comparisons, frontmatter
expressions, and lifecycle state keep the native semantic identity. The raw
record remains unchanged for fresh preparation against another active schema.

`ValidationProblem` retains the public message plus typed code, JSON-pointer
instance path, optional schema path, offending property, source position, and
file-reference diagnostics. `ValidationOptions` controls pending values and
excluded keys without executing anything.

## Imports and dictionaries

- `Name@file` and `Name@this` import named types eagerly with dependency and
  cycle tracking. A bare non-keyword `Name` parses to the same
  `TypeExpr::Imported { reference: "this" }` as `Name@this` (no stored
  spelling; `SimplifiedType::is_keyword` decides built-in vs reference, so
  `string` is built-in and `string@this` is local). Local names are the
  schema's own top-level entries (`types` of `kind: schema`, else `$schema`);
  each inline root-union arm uses only its own entries. Root-union,
  whole-file-reference, and raw JSON Schema files export no names. A bare name
  inside an imported definition resolves in its defining file. Serialization
  writes local references bare unless the name is a keyword.
- Every constraint survives a reference, including `required` and `generated`
  (nothing is stripped; no use-site opt-out). Presence declared inside an
  object type does not make the holding property required. An optional variant
  is declared directly; aliasing the required type re-inherits presence.
  Nullable unions are planned.
- `apply_import_postfix` (`resolve.rs`) merges use-site constraints: stricter
  bound wins per kind (string length, number, item count, keys), lower > upper
  is `SchemaError::ConstraintConflict` (`Bounds`/`Scheme`/`Default`, each
  origin a `SchemaConstraintOrigin` with source, path, keyword, occurrence,
  span), flags dedupe, every `pattern` and `match` group is kept, schemes
  intersect, defaults compare after number normalization, one `suggest` per
  property (checked after union flattening), examples keep their defining
  file. Bounds are read against the resolved target (`label(max(80))` is a
  length on a string; on an already-array target a value-level `min`/`max` is
  an item count). `$constraints` key bounds and postfix key bounds are one
  scope; the parser moves block `required`/`generated` onto the property atom;
  other block keys are `Convert` errors, and the root accepts only
  `min-keys`/`max-keys`. One-atom contradictions are `Convert` errors. Several
  patterns lower to `allOf`; one keeps the single `pattern` keyword.
- A missing name is `SchemaError::UnknownNamedType { name, namespace, site }`;
  cycles stay `ImportCycle`. Both new errors are wrapped by `ReferencedSchema`
  when raised inside a referenced schema.
- `resolve::expand_named_types` expands an in-memory schema with it as
  `this` (DMLS unsaved buffers); `resolve::named_type_reference_path` returns
  the file an explicit `Name@target` selects. Neither adds a second resolver.
- Root unions can compose schema arms without erasing each arm's origin.
- `file(match(...))` only suggests files in a single schema. In a root union,
  each simplified arm's declared glob emits `x-darkmatter-match`, including
  beside a raw JSON Schema arm. An existing file outside that glob rules the
  arm out. Attachment happens per arm in `convert.rs` and both `resolve.rs`
  union sites. `FileMatchGlobs` (a `GlobReference` with the file-name view)
  is the one judgment, shared with Claudine's candidate walk (which walks
  `roots(ctx)` and keeps a file only when `lists_file(path, ctx)` holds:
  `matches` plus `list_files`' out-of-tree file-symlink skip); a caller
  property is judged from its origin (`DarkmatterSchemas::with_caller_input_records`,
  which every host that validates caller overrides itself must call, as
  Claudine's pre-validation and launch schema do).
- Each authored `match(...)` is one group. `FileMatchGroups` holds every
  group of a property and accepts a path only when each group does (positives
  and `!` exclusions stay inside their own group; groups are never flattened).
  `PreparedFileMatch` walks the first group's `roots()` and keeps a file when
  every group `lists_file`s it. `root_union_match_groups`/`file_match_admits`
  take the groups. `x-darkmatter-match` is a flat array for one group and an
  array of arrays for several; its rejection still comes from the keyword
  itself (never wrapped in `allOf`), which `validate.rs` relies on. An empty
  intersection is accepted (no file search) and completes nothing.
- `CompletionKind::File { alternatives: Vec<Vec<Vec<String>>> }` is an OR of
  AND-ed groups: one alternative of every group for a property, one per
  `file` arm for a root union; empty means a bare `file`.
  `file_match::file_match_groups` is public for consumers such as Claudine.
- Pattern dictionary keys lower to `additionalProperties` or
  `patternProperties`; literal keys take precedence.
- `min-keys` and `max-keys` constrain dictionaries.
- Examples are documentation artifacts validated at schema-load time and
  emitted through `x-darkmatter-example`.

## Named union references

User docs: `darkmatter/docs/topics/schemas/definition.md` (Named Union
References; Choosing an Arm: Reporting versus Changing Data).

- A reference to a union-typed named type (bare, `@this`, `@file`, all
  identical) accepts: `U` (inlined, unchanged), `U(required|generated|eager)`,
  `U -> text`, `U[]`, and `U[](min; max; unique; required; generated; eager;
  default; example)`. `apply_import_postfix` hands a union base to
  `apply_union_postfix` (`resolve.rs`).
- Everything else is `SchemaError::UnionReference { name, rejection, site,
  definition }` with `SchemaUnionRejection::{Constraint { keyword,
  array_level }, ArrayArm { arm }, ScalarDefault { arm, value }}`. Rejected:
  value constraints without `[]` (`U(max(10))`), non-list constraints after
  `[]`, `[]` when a flattened arm's outer value is an array (looks through
  imports and nested postfixed unions; arrays inside object arms are fine),
  and any arm `default` under `[]`. A list before `[]` parses into the atom's
  `constraints` (the list after `[]` into `array_constraints`, as on a
  primitive); on a union only the markers are accepted there and join the
  container (`U(eager)[]` == `U[](eager)`), a value constraint is a
  `Constraint { array_level: false }` rejection, and on a single named type
  any list before `[]` is a `SchemaError::Convert`. Match on the
  fields, never the message. DMLS projects it as
  `dm.schema.invalid_type_definition` with the definition as related info.
- Scope: arm constraints stay on items; post-`[]` constraints go on the
  container (merged via `merge_constraints`, so contradictions are
  `ConstraintConflict`). Arm `required`/`generated`/`eager` hoist to the
  property, on `U` and `U[]` alike (`phase.rs` counts arms under `[]` as
  `is_required` does). `eager` there is property timing and never makes a
  lazy `file` arm eager; only an arm declared `file(eager)` probes files.
- Defaults: `default([...])` is a string in the grammar, so every arm default
  under `[]` fails as `ScalarDefault`; do not document list defaults as
  working. No default is ever inserted.
- Representation: a bare `U` stays `PropertyDef::Union`. A postfixed reference
  is `PropertyDef::Single(atom)` with `atom.ty == TypeExpr::Union(arms)`
  (resolver-only; never parsed) and `atom.is_array` for `U[]`. Helpers:
  `PropertyAtom::union_arms()`, `PropertyDef::alternatives()` (`None` for a
  list of a union); the crate-private `is_required`/`is_generated`/
  `has_default` read through arms. There is no anonymous `(A | B)[]` syntax and no second encoding.
- Lowering (`convert.rs`): `type: array` + `items.anyOf` of the flattened
  arms; items get no null arm; the optional `anyOf: [null, …]` wrapper (and a
  use-site description) goes on the property only. A nested postfixed arm
  flattens into the one `anyOf`, its markers hoisted to the property.
- Consumers must not read `properties` off an `anyOf` wrapper. Use
  `schemas::flattened_alternatives(def)` (`Alternative { atom, arm,
  is_array }`) for typed-tree walks and `completion::alternative_accepts(atom,
  value)` to judge a present value (so `null` matches only a null-accepting
  arm). Editors use `completion::{for_list_item, for_property_value,
  alternatives_for_def}`; never add a DMLS-local union interpretation.
- Safety rule per consumer: validation accepts any arm; literal-tag narrowing
  (and root-union closest arm) only shapes reports; coercion commits only
  when exactly one candidate validates; eager-file rewriting needs exactly one
  validating arm containing an eager `file`; cleanup never repairs inside a
  union.
- Known gaps: item problems carry the top-level key's line/column.

## Special types

- `literal(value)` lowers to JSON Schema `const`. Bare YAML bool/number values
  are typed; quoted values are strings; bare null is rejected. Only `required`
  and an equal default are allowed.
- `expression` is a parse-only string format. Native bool/number values coerce
  to strings, but no expression is evaluated.
- `yaml` and `json` accept string or native structured values and validate the
  encoded content format.
- `type-definition` validates one property definition.
- `schema` validates one complete `$schema` declaration.
- `eager` is universal timing metadata and never controls presence. Ordinary
  schema preparation retains the existing `file` existence check, while phase
  validation stays passive; `file(eager)[]` owns item validity and
  `file[](eager)` owns the array property's validation timing. Declare
  `required` independently when the property must exist.

The meta-types delegate to the same passive parser used by authoring and DMLS.
They do not perform imports, I/O, matching side effects, or rewrites.

## Suggestions and DMLS

`suggest(...)` attaches advisory completion candidates without changing
validation. `lint_suggestions()` reports malformed or misplaced suggestions;
`suggestions_for_path()` supplies structured completion items.

Literal discriminants use one presentation-neutral union-arm selector shared
by library validation and DMLS. Expression-typed values enable expression
completion, hover, and `dm.expression.*` diagnostics.

Source spans come from one locator in `simplified/source.rs` with two
grammars. Strict (schema documents) and tolerant (frontmatter) share
indentless sequences and plain scalars continued onto more-indented lines;
only tolerant follows tags, block scalars, quoted continuations, and empty
values. Strict fails unless every non-comment line belongs to the value, and
checks each decoded scalar against `serde_yaml_ng` before projecting. A
source-aware slice must start at a line boundary (indentation included) or be
dedented, because indentation decides where a continuation ends; DMLS widens
mid-line value spans with `overlay::frontmatter::line_aligned_span`. Never add
a third locator. The authored boundary is documented in
`darkmatter/docs/topics/schemas/definition.md` (Source-Aware Presentation
Boundary).

## Testing

For grammar or schema changes, cover:

- Native and quoted YAML representations.
- Missing, explicit null, valid, malformed, and boundary values.
- Passive parsing across every shipped schema and trigger artifact.
- An end-to-end `md schema` or normal compose invocation using the real shipped
  artifact.
- Imported dependency/cycle errors and source spans.
- Read/write/read repetition when composition persists normalized values.

## Schema roots and `$path` triggers

- `SchemaRoots::for_document(&ctx)` (`schemas/roots.rs`) is the one root list:
  package, package area, `base_dir()`, `SCHEMAS_DIR`, `~/schemas`, each with a
  `SchemaRootState` (`Searched`, `Absent`, `Duplicate { of }`,
  `NotApplicable`, `Invalid`). `md schema triggers` prints exactly these
  states; DMLS and `md` share them through `triggers::scan(&ctx)`.
- `for_document` is fallible: only `NotFound`/`NotADirectory` (and the
  package/area/tree symlink policy) make a root `Absent`. Any other metadata
  failure (e.g. `PermissionDenied` on an ancestor) is `SchemaError::Io`
  naming the folder, so `scan` never installs a registry missing that root
  and bare-name lookup never falls through to a less local root. DMLS turns
  that `Io` scan error into `SchemaOutcome::Failed` instead of keeping its
  last-good registry (last-good is only for trigger-file load errors).
- `ctx` must be the checked document's context (compose's
  `source_file_resolution_context`, the CLI's `request.document_context`,
  DMLS's per-document derivation). The registry keeps that context and
  `evaluate_registry(registry, fm, Some(path))` judges `$path` in it.
- Discovery still runs only inside a repository (`md`) or a workspace folder
  (DMLS); the roots do not decide that gate.
- `PathGlobs::new` compiles one single-pattern `GlobReference` per `$path`
  pattern (negation kept beside it). Bare and `./` patterns are judged from
  the trigger's `LoadedTrigger::pattern_cwd` (the folder holding its
  `schemas/`, or the document's `base_dir()` for `SCHEMAS_DIR`/home
  triggers); `&`, `^`, `~`, absolute from the document's own context. `@`,
  `vault:`, `%`, and `{{` anywhere are `TriggerMatch` definition errors naming
  the pattern.
- A test fixture for roots builds its context with `build_resolution_context`
  from a `RequestSnapshot` carrying a fixture home and env, never
  `FileResolutionContext::new` (ambient `HOME` would add a real `~/schemas`).
  `md` child processes scrub `SCHEMAS_DIR`; declare it with
  `application_input("SCHEMAS_DIR", ..)`. A DMLS fixture whose triggers live
  in the workspace `schemas/` must be a repository: outside one, a
  document's tree root is its own folder.

## Authored global catalog

The authored replacement schema entry point is `darkmatter/schemas/darkmatter.yaml`.
It declares all globals, including `doc`, and imports types from `partials/`.
Runtime migration is pending: do not confuse the existing embedded document
baseline with this global catalog. The planned document baseline is its resolved
`doc` definition; `ctx` and `current` share a context type, and there is no
`current_env`. Register the global catalog explicitly rather than auto-applying
its root as frontmatter properties.
