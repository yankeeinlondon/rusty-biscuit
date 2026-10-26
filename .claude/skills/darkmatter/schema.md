# Darkmatter Schemas

Use this reference for SimplifiedSchema parsing, validation, triggers, imports,
and DMLS schema behavior.

## Contents

- [Document kinds](#document-kinds)
- [Composition and validation](#composition-and-validation)
- [Runtime baseline](#runtime-baseline)
- [Imports and dictionaries](#imports-and-dictionaries)
- [Named union references](#named-union-references)
- [Conversion engine](#conversion-engine)
- [Tuples, functions, `|`, and categories](#tuples-functions--and-categories)
- [Special types](#special-types)
- [Core types](#core-types-unknown-null-alphanumeric-shell)
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

Schema-trigger documents use `kind: schema-trigger` (the envelope kind the
parser accepts, `triggers/envelope.rs::ENVELOPE_KIND`). Preserve the kinded root
declaration so Darkmatter and Claudine can select the right schema formally.
`kind: trigger-schema` and `any:` were renamed with no alias
(`SchemaError::TriggerRenamed { field, found, replacement }`); the old trigger
flags are now `--disable schema-triggers`, and spelling one is a usage error
raised by `md`'s argv pre-check (`main.rs::reject_removed_flags`, so
completions never offer it). A null or
non-string `kind` is a `TriggerMatch` error, not ordinary data; no `$schema` is
a matcher-only trigger (`resolve_trigger_layers` skips it).

Match grammar (`triggers/grammar.rs`): `TriggerEnvelope::conditions` is a
`MatchConditions` list that ANDs (`MatchConditions::matches`; never `.any()`
over it); a mapping `match:` is one condition. OR is `MatchExpr::Group` from an
explicit `group:`; a list entry that is itself a list is rejected. The vacuity
lint runs once over `MatchConditions::to_expr()`. Every `TriggerMatch` message
starts with its field (`` `match[1].group[0]`: ``). The input matrix is
`lib/tests/l1/trigger_input_matrix.rs`; selection parity for migrated files is
`lib/tests/l1/trigger_selection_parity.rs` and Claudine's
`claudine_trigger_selection_parity.rs`.

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
phases. Exception: a null-admitting whole value (see
[Core types](#core-types-unknown-null-alphanumeric-shell)) is optional at both
phases whatever its markers. Completion observes the final working instance without coercion, while
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

Problem positions (`schemas/positions.rs`, one `PositionMap` per validation
source snapshot, shared by every problem and root-union arm):

- `line` counts the complete file (opening `---` is line 1); `column` is a
  1-based UTF-8 byte column; LF and CRLF agree.
- Targets: a value failure → the value's start (a block list item excludes
  `- `); an unknown key (`path` = parent mapping) → the key token; a missing
  property → the lacking mapping's start (its first key; root = first key
  after comments). Block and quoted scalars are one node: anything at or
  below them reports the scalar's start.
- Unauthored paths (assignments) use the nearest authored non-root ancestor;
  a new top-level path has no position. A replaced path keeps its authored
  node.
- The alignment witness: `PositionMap::project` accepts the locator's tree
  only when its mappings match the parsed `MappingOrders` and it holds no
  alias. Otherwise (aliases, `<<`, unquoted `{{ }}`, multi-line or
  commented flow collections) it falls back to top-level key line, column 1.
  Positions never change validity.
- `positions_for(&Markdown)` needs loaded text (parsed from text or a file);
  a constructed `Markdown` is located in its serialized YAML view.
  `PositionMap::new()` (and `EffectiveSchema::validate`) is source-free: no
  positions. `validate::build_position_map(yaml)` is slice-relative (line 1
  = first YAML line). `PositionMap` is opaque, not an `IndexMap`.
- `md schema validate` prints `(at line L, column C)`; the compose block
  prints ` at L:C`; DMLS ranges start at the same nodes.
- Every projection keeps the instance path beside the position; a position
  alone does not name the list item or field. `md schema validate` and the
  compose block print the pointer without its leading `/`
  (`content_policy/1/rule`; compose appends a missing property's name to its
  parent's path); DMLS puts it in `data.instance_path`; `md clean` suffixes
  report-only schema messages with `` (at `/pointer`) ``; Claudine keeps it in
  `SchemaValidation`, dropped-optional reasons, and sequence-state violations,
  and treats a field missing inside a value as that value's problem, never as
  a top-level property to collect. Never reduce a problem to its first
  pointer segment for display.

**Stage timings.** `DarkmatterSchemas::validate_timed` returns `validate`'s
result unchanged plus a `ValidationTimings` (`schemas/timings.rs`):
non-overlapping `ValidationStage`s `discover`/`match`/`resolve`/`compile`/
`validate`, `total()`, `unassigned()`, `compile_cache()` (`Hit`, `Miss`,
`NotRequested`), and `matched_triggers()` (file names). A thread-local stack
recorder attributes time to the innermost open stage, so a validator built
while resolving counts once, as `compile`; the markers live in
`build_validator_with` (compile, miss), `ValidatorCache::lookup` (requested),
`scan_with_source` (discover), trigger evaluation (match), payload resolution
and `effective_for`/phase projection (resolve), and
`validate_with_positions`/`validate_raw_with_positions` (validate). Discovery
runs in `with_trigger_discovery`, outside `validate`: wrap it in
`ValidationTimings::record` and `absorb` the result; a host attributes its own
work (a registry freshness check) with `ValidationStage::time`. Recordings
nest (the innermost wins) and never carry values or schema bodies. A new
validator build path must go through `build_validator_with` or open
`timings::compiling()` itself. `md schema validate -vv` renders it; a
host's request is every schema call it makes for the document, so record each
one (`validate.rs::validate_one` records discovery, the pre-assignment
`effective_for`, and validation plus its presence lookup) or the first build
lands outside the report and a cold request reads as a `hit`.

## Runtime baseline

The runtime baseline is the resolved `fm` declaration of the global catalog
`darkmatter/schemas/darkmatter.yaml` (`schemas::catalog::GlobalCatalog::darkmatter()`),
authored in `darkmatter/schemas/partials/fm.yaml`. That partial declares
`content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"`
(optional, no default) and imports `style@./style.yaml` and
`context@./context-variables.yaml`. `schemas/embedded.rs::SHIPPED` holds the
compiled-in bundle (catalog, its partials including the expression-function
catalog `functions.yaml`, `content_policy::EDITOR_SCHEMA`)
keyed by repo-relative virtual paths. Embedded imports resolve
lexically with `/` (no `~ & ^ $ % @`, URLs, globs, or climbing above the
root), never touch the disk or network, add no dependency edge (DMLS never
watches them), and identify as `SchemaSourceId::Embedded`. One
`BASELINE: OnceLock` lowers it once per process. Resolving the catalog from
disk gives the same typed schema and JSON. Installed binaries therefore need
no schema files. `with_baseline_from_file` (and `BASELINE_SCHEMA`) pointing
at a global catalog uses its `fm`.

`md schema validate` selection (`cli/src/commands/schema/validate.rs::load_api`),
first match wins:

1. `--disable baseline-schema` → none (usage conflict with `--schema`, exit 2);
2. `--schema <path>` → custom, even with the env opt-out;
3. `DARKMATTER_NO_BASELINE_SCHEMA` exactly `1|true|TRUE|yes|YES`
   (`commands::compose::disables_baseline_schema`, read from the captured
   request snapshot) → none, ignoring `BASELINE_SCHEMA`;
4. `BASELINE_SCHEMA` → custom;
5. built-in (`with_darkmatter_baseline_json_schema`).

Custom paths resolve in the launch context (`crate::io::resolve_file_path`);
a failed load is exit 2 with no fallback. Opting out keeps document `$schema`
and triggers; `--disable schema-triggers` is independent. JSON `schema` stays null
for a document without `$schema`. Compatibility: `last_updated: not-a-date`
with no `$schema` now fails validate as it fails compose. Compose keeps
`--baseline-schema`/`--disable baseline-schema`/the env (live
`env_disables_baseline_schema`); DMLS layers built-in → extensions →
triggers → document. Each document with a distinct schema/context compiles the
merged baseline (validator cache keys on the full resolution context), so
tests that judge only document schemas pass `--disable baseline-schema` to stay in
the L1 budget.

The baseline types Darkmatter's five lifecycle events (`initialize`,
`composed`, `failed`, `cancelled`, `finalize`) with the md profile
(`partials/md-lifecycle.yaml`): md verbs are typed and offered, and an open
pattern-key arm that excludes every md verb lets another host's verb (`say`,
`retry`) pass untyped and unoffered. md's profile is still a deliberately
tolerant three-level unroll (`md-stack`, `md-item-1..3`, `md-loose-body`)
whose side-effect and function arguments are untyped (one pattern key);
host profiles built from `partials/lifecycle.yaml` (Claudine's) are
recursive and typed. The neutral shape of every verb, Claudine's
included, lives in `partials/lifecycle.yaml`. Its expression-function and
side-effect verb types (`<verb>-short`/`<verb>-long`) and their category and
bundle keys are generated from the typed catalogs by
`lifecycle::schema_projection` between `# BEGIN GENERATED`/`# END GENERATED`
markers: regenerate with `just regen-lifecycle-schema`, never by hand;
`lib/tests/l1/lifecycle_schema_generation.rs` fails on drift. Side-effect
parameter types come from `partials/mutations.yaml` (`effects::effect_function_catalog`,
`effect_registration`; a coverage test fails when an `EFFECT_DESCRIPTORS`
signature has no typed entry). Generated operands carry the `deferred`
constraint (`x-darkmatter-deferred`, `Shape::Deferrable`): a `{{ }}` template
there is accepted unjudged while literal siblings are judged, and
`ValidationReport::deferred` lists what was accepted; an unmarked template is
judged as text. A template is recognized only by
`expression::is_deferred_template` (scanner + parser), never by a substring;
`expression::is_malformed_template` names a span that fails to parse, which
every caller setting template problems aside for later rendering judges as
written: `EffectiveSchema::validate_with_options` (DMLS, `PendingPolicy::Defer`)
keeps that problem, as `md schema validate` and Claudine's launch
pre-validation do. The resolver stores only cycle
members in `$defs` and inlines every other named type at each reference, so
a host must write its stack families recursively (Claudine's are), never
unrolled: Claudine's typed three-level unroll resolved to 11 MB.
The passive operand check is `lifecycle::validate_operands` (runtime
registration, `select_overload_with`), and schema validation runs it: every
generated arm but `null` and `template-operand` carries `operands(<verb>-short|-long)`
(`Constraint::Operands`, `x-darkmatter-operands`, kept on the typed node), and
`EffectiveSchema::validate_instance` (the body every `validate*` path reaches)
walks the authored instance beside the JSON Schema (`schemas/bound_operands.rs`),
reads each marked value as the parser does (`lifecycle::action_call`), and binds
it (`validate_action_operands_with`: effect registration, else the expression
registration with test functions first), deferring a `{{ }}` operand only when
its `stored` counterpart is not a literal token: a token's decoded text is
data and is type-checked (`abs` refuses a token holding `{{ fm.count }}`).
A caller that decodes before validating loses that origin, so hand Darkmatter
the stored instance: `validate_with_options`, or `EffectiveSchema::validate_stored`
(optional `SchemaPhase`), which Claudine's launch verdict uses. An accepted call's operands join
`NestedTemplates` as settled paths (excused like templates, not in
`ValidationReport::deferred`); a rejected one is a positioned problem. Every
operand string (nested ones too) holding a span that fails to parse is a
`malformed template` problem at that string (`BoundOperands::report_malformed`,
message from `lifecycle::action_value_to_expr`), never settled, replacing any
problem at or below it, whatever the type accepts; it scans the `stored`
instance (literal tokens still encoded) so a decoded token stays data. The
hand-written verb types in `partials/lifecycle.yaml` (communication, `shell`,
the directives but `stop`/`skip`, `set`) carry the same marker; for them
`lifecycle::template_operands` names exactly the strings the parser reads as
templates (every `with:` and `set` value nested, not a `$( … )` `set` value,
`no_error`, or `break.code`) and the walk only scans those, never binds. So
`number: [pear, 7]` passes and `length: [true]` fails; never widen a generated
type to fix a schema/binder disagreement. Bind from the authored value, not
the converted working copy. `lib/tests/l1/lifecycle_generated_operands.rs`
holds the parity table, whose `short`/`arm-<i>` probes strip the marker to
judge the projected structure alone. `lib/tests/l1/lifecycle_schema_parity.rs`
derives events, action types, the open arm's exclusions, and the bare-verb
`suggest` list from the md registry and parser. The inlined stacks cost about
70–90 ms per debug validation; the regex memo in `convert.rs` and the import
engine's `expanded_cache` keep it there.

User docs: `darkmatter/docs/topics/schemas/definition.md` (The Built-in
Baseline, Selecting the Baseline); Content Policy's typing limits are in the
`content-policy` skill.

## Imports and dictionaries

- `Name@darkmatter:<stem>` is reserved: it names type `Name` in the shipped
  partial `darkmatter/schemas/partials/<stem>.yaml` from `SHIPPED`, whatever
  the importer is (disk file, embedded document, host bundle), never reading
  the disk (`DefNamespace::Shipped`; relative imports inside it stay in
  `SHIPPED`). `named_type_reference_path` returns `None` for it, as for
  `this`. Claudine's `action.yaml` uses it to reference Darkmatter's neutral
  action types.
- `Name@file` and `Name@this` import named types eagerly with dependency and
  cycle tracking. Dependencies (`imports`, `examples`, `referenced_files`)
  are canonical, existing, content-hashed files; `ResolvedSchema::searched`
  is a separate lexical list of every candidate probed through the selected
  one (may be absent; recursive `%` adds nothing). Propagate both into
  `ResolvedPayload` and never hash `searched` as a file (see
  [dmls.md](dmls.md) for the editor side). A bare non-keyword `Name` parses to the same
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
  is declared directly or as `[T, null]`; aliasing the required type
  re-inherits presence.
- `alphanumeric` options are identity, not mergeable constraints:
  `check_use_site_options` (`resolve.rs`, before merging) drops an equal
  canonical set and raises `ConstraintConflict` with
  `SchemaConstraintConflictKind::Options` for a different one (origins: the
  definition's option group and the use site).
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
- A missing name is `SchemaError::UnknownNamedType { name, namespace, site }`.
  Both it and `NonDescendingCycle` are wrapped by `ReferencedSchema` when
  raised inside a referenced schema; `ImportCycle` now means only an acyclic
  reference chain deeper than 32.
- **Recursive named types.** `resolve/graph.rs` discovers the reachable
  definitions once, classifies each reference edge as descending (object
  property, dictionary value, list item, tuple slot; never inside a
  `function(...)` signature), and rejects every cycle left after dropping
  descending edges as `NonDescendingCycle { cycles }` (each `CycleReference`
  carries a span for a file source). Members of a cyclic group are
  `DefKey { namespace: DefNamespace, name }` (every independently
  authored source has its own namespace: the document's inline `$schema`
  is `InlineRoot`, a disk trigger's inline payload is
  `TriggerPayload(trigger path)` via `resolve::InlineOwner`, because
  `EffectiveSchema::definition` searches several tables by key); every use becomes
  `TypeExpr::Ref(DefKey)` carrying the use site's markers (the definition's
  `required`/`generated`/`eager`/`default`/`suggest`/`example`/`category` and
  description merged with the postfix), and the presence-free body goes into
  the `DefinitionTable` (`ResolvedSchema.definitions`,
  `EffectiveSchema::definitions()`, `NamedTypeExpansion.definitions`).
  A schema with a cycle expands twice (`resolve/recursive.rs`: pass 1 learns
  each definition's `UseSummary`); an acyclic one takes the old single pass,
  pinned byte-for-byte by `lib/tests/l1/recursive_schema_golden.rs`. Lower
  with `to_json_schema_with_definitions` (`$ref` with use-site siblings, one
  root `$defs` body per reachable definition, in allocation order); plain
  `to_json_schema` errors on a `Ref`. Every typed and JSON walker follows a
  `Ref` (see the reference-aware entry points under "Named union
  references" and "Conversion engine"); none treats it as opaque. User docs:
  `topics/schemas/definition.md` "Recursive Named Types" (child-value rule,
  definition vs use site, errors, reach) is the one description; other pages
  link there.
- **Root-union `match()` through references** (`simplified/convert.rs`).
  A root-union arm enforces `match` in exactly two places, and an acyclic
  schema lowers byte-identically to its pre-recursion output (spec
  requirement): (1) a property that is directly `file`/`file[]`
  (`attach_direct_file_match`, via `file_match_groups`); (2) inside an arm
  copy of a match-reaching recursive definition. Lowering carries
  `Lowering { defs, matching }`; in `Matching::Enforce` every `TypeExpr::Ref`
  anywhere in the arm points at the arm copy `<key>@match:<property>`
  (`MatchCopies`), never the shared body, and `file_fragment` emits
  `x-darkmatter-match` only while lowering a copy (`in_copy`). Any other
  arm-level file (union alternative, acyclic name bound to a union, inline
  object, dictionary value, tuple slot, list of a union) stays
  suggestion-only. The four `resolve.rs` arm sites call
  `attach_root_union_match(arm, shape, defs, root_defs)` (direct attach, then
  graft the ref redirects and join the copies); `union_to_root_schema` lowers
  in that mode and attaches the direct keyword. A copy names its property in
  `x-darkmatter-match-property` because its `$defs` location cannot (the
  keyword factory reads it for the caller origin). Compatibility tests:
  `schema_cache_validity::acyclic_match_*` (six shapes, five routes, phases).
- **`$defs` across merges** (`schemas/scope.rs`). `$defs` live only at a
  schema's root. A sub-schema compiled alone (a root-union arm, a
  discriminated arm, a property union arm, an example target) goes through
  `with_root_definitions` / `wrap_arm_as_root_schema(arm, root)`; never
  compile a fragment of a schema that may carry `$defs` without its root.
  `merge_baseline` moves the baseline's `$defs` into the document root with
  `fold_definitions` (the document's keys never move; a colliding different
  body is renamed `key-2` with only the joining side's references rewritten,
  schema keywords only, never `const`/`default`/`examples`/`x-*` data).
  `validate_simple_object_schema(schema, label)` admits a root `$defs` and runs
  `check_joined_references` (a `#/$defs/…` ref must resolve in its own table:
  `UnresolvedReference`; with `$defs`, `$id`/anchors/external/other local refs
  are `UnsupportedMergeReference`, and the schema is meta-validated). A
  SimplifiedSchema file arm of a root union joins through
  `DefinitionTable::absorb`, so the union's typed table and JSON keys agree
  (phase lowering depends on that); a raw arm is folded after every arm.
  Phase projection projects definition bodies with `phase::project_definitions`
  (same keys) and `merge_with_effective` replaces the document's own bodies
  while keeping layer-only ones. Look a `Ref` up through
  `EffectiveSchema::definition(&DefKey)`, which also searches trigger tables;
  a table's `json_key`s are those of its own lowering.
- `resolve::expand_named_types` expands an in-memory schema with it as
  `this` (DMLS unsaved buffers) and returns `NamedTypeExpansion { schema,
  imports, definitions }`; `resolve::named_type_reference_path` returns the
  file an explicit `Name@target` selects. Neither adds a second resolver.
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
  every group `lists_file`s it. `file_match_admits` takes one alternative's
  groups; `root_union_match_groups(arm, property, definitions)` returns the
  file alternatives a top-level string value (or array item) is judged by,
  agreeing with the lowering: a direct `file` property, or the files reached
  through `TypeExpr::Ref` alternatives (through property unions) in the arm's
  table (value admitted when some alternative's groups all admit it; `None`
  when an alternative accepts a string without an enforced glob, including an
  arm-level file alternative such as `file(match(..)) | number`). It projects
  `file_match::file_alternatives(definition, definitions)`, the one walker of
  a property's `file` alternatives (each with its atom, so
  `is_eager()`/`match_groups()`, and its list shape; a definition entered once
  per list depth; `None` when a string could be a non-file or the property
  offers an acyclic file alternative beside others, which pre-recursion code
  never read as a file). Claudine's caller
  arm admission (`supplied.rs::admits_arm_files`) reads the groups; its eager
  and late-verdict completion targets and its per-arm existence relaxation
  read `file_alternatives` directly. Compose's caller-file classifier and
  materializer (`schema_validation.rs::select_file_mode` and
  `resolve_caller_file_value`) follow local `$ref` in the lowered JSON the
  same way (`SchemaScope::resolve_text`, a reference cycle that consumes no
  value ends unclassified). `x-darkmatter-match` is a flat array for one group and an
  array of arrays for several; its rejection still comes from the keyword
  itself (never wrapped in `allOf`), which `validate.rs` relies on. An empty
  intersection is accepted (no file search) and completes nothing.
- `CompletionKind::File { alternatives: Vec<Vec<Vec<String>>> }` is an OR of
  AND-ed groups: one alternative of every group for a property, one per
  `file` arm for a root union; empty means a bare `file`.
  `file_match::file_match_groups` is public for consumers such as Claudine.
- Pattern dictionary keys lower to `additionalProperties` or
  `patternProperties`; literal keys take precedence.
- `min-keys` and `max-keys` constrain dictionaries; `uncounted-keys(a, b)`
  leaves named keys out of that count (lowered to one `allOf` `if`/`then`
  per combination present, plus `x-darkmatter-uncounted-keys`, which
  `json_shape` reads into `ObjectShape::uncounted` so the core models the
  count instead of asserting it). The lifecycle categories use it for
  `when`/`no_error` beside one verb. An `anyOf` arm that is itself a failed
  union at the same path (a `$ref` to a recursive union) is flattened by
  `validate::union_branch_errors`, so drill-down still reaches the verb.
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
  and a scalar arm `default` under `[]`. A list before `[]` parses into the atom's
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
- Defaults: `default([...])` is a string in the grammar, so every authored arm
  default under `[]` fails as `ScalarDefault`. The Rust API can supply an
  actual JSON-array arm default, which hoists to the list; inherited and
  use-site defaults must agree under shared numeric normalization. This adds
  no list-valued default spelling to the grammar. No default is ever inserted.
- Representation: a bare `U` stays `PropertyDef::Union`. A postfixed reference
  is `PropertyDef::Single(atom)` with `atom.ty == TypeExpr::Union(arms)`
  and `atom.is_array` for `U[]`; the parser produces the same atom for the
  grouped spelling `(A | B)[]`, so there is no second encoding. Helpers:
  `PropertyAtom::union_arms()`, `PropertyDef::alternatives()` (`None` for a
  list of a union); the crate-private `is_required`/`is_generated`/
  `has_default` read through arms.
- Lowering (`convert.rs`): `type: array` + `items.anyOf` of the flattened
  arms; items get no automatic null arm (an explicit `null`/`unknown` arm can
  admit null); the optional `anyOf: [null, …]` wrapper (and a
  use-site description) goes on the property only. A nested postfixed arm
  flattens into the one `anyOf`, its markers hoisted to the property.
- Consumers must not read `properties` off an `anyOf` wrapper. Use
  `schemas::flattened_alternatives(def)` (`Alternative { atom, arm,
  is_array }`) for typed-tree walks and `completion::alternative_accepts(atom,
  value)` to judge a present value (so `null` matches only a null-accepting
  arm). Editors use `completion::{for_list_item, for_property_value,
  alternatives_for_def}`, and below a recursive type `for_path`/`def_at_path`
  (one definition opened per path step) and the `*_in` variants with a
  `DefinitionLookup`; never add a DMLS-local union interpretation. A typed
  walker meeting `TypeExpr::Ref` resolves it through
  `query::resolved_alternatives(def, lookup)`, `is_subtype_with_definitions`,
  or `DeclaredTypes::with_definitions`; a `Ref` judged without a table is a
  defect (it fails closed).
- Safety rule per consumer: validation accepts any arm; literal-tag narrowing
  (and root-union closest arm) only shapes reports; conversion follows the
  engine's union rule (exact match, one reachable option, number over
  boolean, JSON over YAML, else an ambiguity error; order-independent);
  eager-file rewriting needs exactly one validating arm containing an eager
  `file`; cleanup never repairs inside a union.

## Conversion engine

- One rule core, `markdown::coercion` (`Shape`, `bind`, `bind_with`,
  `check_exact`, `accepts_exactly`, `content_error`), serves frontmatter and
  function calls. Translators only map syntax: `schemas::shape_from_json_schema`
  (compiled/raw JSON Schema; unmodeled keywords become `Assert`s) and
  `compose::expression::call_binding::shape_from_property_def` (grammar).
  `tests/l1/coercion_translator_conformance.rs` requires identical outcomes;
  a new construct needs both translators and a row there.
- Recursion is `Shape::Ref(ShapeRef)`: the first use of a recursive target
  owns it strongly, back-references are weak (dropping the root frees the
  cycle). Both translators build it (`json_shape.rs` for a raw or generated
  `$ref` cycle that descends, `translate.rs`'s `*_with_definitions` entry
  points for `TypeExpr::Ref`), so conversion continues at any depth; the
  value bound (`BindOptions.max_depth`, `MAX_DEPTH` 64) and
  `MAX_REFERENCE_HOPS` (64 hops at one value) end in `ResourceLimit`, never
  acceptance. A raw cycle that never descends stays validation-only; grammar
  `TooDeep` (32) remains only for over-deep acyclic chains and loops that
  never descend.
- `schemas/coerce.rs` decides only what is committed: an instance the complete
  schema accepts is kept; else the whole converted document if the complete
  schema accepts it; else each top-level property that converts cleanly
  (root unions keep the whole-document rule). Never reintroduce per-surface
  conversion rules there (`coercion_engine_parity::old_frontmatter_rule_code_is_gone`).
- Performance: `EffectiveSchema` keeps a `ConversionPlan` keyed by the digest
  `ValidatorCache::shared_schema_hash` already records, and checks
  `accepts_as_is` with its own compiled validators first. Never hash or
  compile a large schema per call (`schema_content_hash` serializes the whole
  schema); `Shape::describe` is capped at `DESCRIPTION_LIMIT`.
- `number(integer)` lowers with `x-darkmatter-integer: true` (magnitude below
  2^64, a custom keyword in `validate.rs`); raw `integer` has no ceiling.
  `numberlike`/`boolish` lower to the `darkmatter-numberlike`/`-boolish`
  formats and are kept as written.
- User docs: the one rule list is `topics/schemas/definition.md` "Type
  Coercion" (interfaces, actions, conversions, unions, containers, function
  binding, type errors, changes for existing documents); other pages link
  there. Change it there, not in a second copy.

## Tuples, functions, `|`, and categories

User docs: `darkmatter/docs/topics/schemas/definition.md` (Tuples; The `|`
Operator and Grouping; Function Signatures; Categories).

- `grammar::parse_type_definition` reads a `|` union into the same
  `PropertyDef::Union` the YAML list produces; `parse_type_expr` is the
  one-atom form and rejects a union. `[]` binds tighter than `|`.
- `TypeExpr::Tuple(TupleType)` lowers to `prefixItems`/`minItems`/`items`;
  positions get no optional-null wrapper. `TypeExpr::Function` is
  description-only: `to_json_schema` fails on it with the property path, which
  is the one choke point keeping functions out of document schemas (the
  resolver inlines references first). Never emit JSON for a function.
- `Constraint::Category` lowers to `x-darkmatter-category`; the tagged
  envelope's `categories:` list (`StandaloneSchemaDocument::categories`)
  limits names. Duplicate overloads are judged by
  `signature::duplicate_overload` over canonical binding contracts.
- Numbers in schema metadata go through `markdown::numeric` admission; an
  inexact whole number, infinity, or underflow is a grammar error.

## Special types

- `literal(value)` lowers to JSON Schema `const`. Bare YAML bool/number values
  are typed; quoted values are strings; bare null is rejected. Only `required`
  and an equal default are allowed.
- `expression` is a parse-only string format. Native bool/number values coerce
  to strings, but no expression is evaluated.
- `yaml` and `json` accept string or native structured values and validate the
  encoded content format.
- Core types (`unknown`, `null`, `alphanumeric`, `shell`): see
  [Core types](#core-types-unknown-null-alphanumeric-shell).
- `type-definition` validates one property definition.
- `schema` validates one complete `$schema` declaration.
- `eager` is universal timing metadata and never controls presence. Ordinary
  schema preparation retains the existing `file` existence check, while phase
  validation stays passive; `file(eager)[]` owns item validity and
  `file[](eager)` owns the array property's validation timing. Declare
  `required` independently when the property must exist.

The meta-types delegate to the same passive parser used by authoring and DMLS.
They do not perform imports, I/O, matching side effects, or rewrites.

## Core types (`unknown`, `null`, `alphanumeric`, `shell`)

User docs: `darkmatter/docs/topics/schemas/definition.md` (`unknown` and
`null`; Null-Admitting Properties; Alphanumeric Strings; Shell Commands;
Subtype Relation). All four are built-in keywords, so a same-named local type
needs `@this`.

- `SimplifiedType::Unknown` is canonical; `any` parses to it
  (`SimplifiedType::aliases()`), and serialization, `md schema about`
  (`alias: any`), and DMLS say `unknown`. It lowers to `{}`, never coerces,
  and keeps nested nulls. An undeclared expression variable is unknown-typed;
  call checking assigns an unknown argument category 4 (a warning only under
  DMLS `expressions.strict`), rather than rejecting it as proven incompatible.
  Runtime binding still validates its actual value.
- `null` lowers to `{"type":"null"}`. YAML null scalars (`null`/`Null`/`NULL`/
  `~`, empty value, empty item) and the string `"null"` spell it at property,
  union-arm, nested, and named positions; a quoted `""` is still "expected a
  type name". Root `$schema: null`, a null root-union arm, and `literal(null)`
  stay errors. A non-null `default` is a `Grammar` error ranged on the
  argument (the converter backstops a use-site default on a `null` named
  type).
- **Effective presence** is one calculation: `PropertyDef::is_required` /
  `PropertyAtom::is_required` = a `required` marker (written, inherited,
  hoisted, or after `[]`) and not `admits_null()`. `admits_null()` is true for
  a non-array `null`/`unknown` or a non-array union (property-level or
  `TypeExpr::Union`) with such an arm, on resolved input. Lowering, trigger
  matching and the vacuous-arm lint, phase projection, completion,
  composition materialization, DMLS, and Claudine all read it; never re-derive
  presence from the raw marker (`marks_required` is crate-private). `eager`
  and `generated` never restore presence. `unknown[](required)` /
  `null[](required)` still require the list; a nullable child leaves its
  parent's presence alone. Raw JSON Schema keeps its authored `required`.
- Root unions are inclusive (`anyOf`, one or more arms); XOR is written with
  `key: null` in each arm.
- `alphanumeric` (`simplified/alphanumeric.rs`): `AlphanumericOption` (nine,
  table order), `AlphanumericOptions` (`canonical()`, `keywords()`,
  `allowed_characters()`, `first_characters()`, `forbids_double_period()`,
  `accepts()`, `pattern()`), `AsciiCharSet` (with `Display` for hover).
  Options ride in `Constraint::Options` before `;`; read the canonical set via
  `PropertyAtom::alphanumeric_options()`, never from display strings. Lowering:
  `string` + canonical linear-time `pattern` (user patterns and `not-empty`
  as further `allOf` entries) + `x-darkmatter-type` /
  `x-darkmatter-options`. The JSON translator maps
  `x-darkmatter-type: alphanumeric` to `TextShape { scalar_text: false }`, so
  numbers and booleans stay mismatches (other union arms keep their
  conversion). `default`/`suggest` values must
  validate, without coercion, or the load fails; other types' suggestions
  stay lint-only. `md schema validate` assignments force a string for a
  top-level `alphanumeric` (`assignment.rs::atom_prefers_string`), so
  `slug=123` is the text `123`.
- `shell` lowers to `format: darkmatter-shell` (registered and asserted by
  every validator builder, including the expression catalog's): non-blank
  text, no syntax check, passive. Constraints, scalar coercion, defaults, and
  pending deferral mirror `expression`. Exported-schema consumers must
  enforce the format. Excluding `shell` from an action union is authoring
  validation only; Claudine's runtime `initialize` shell ban stays.
- `is_subtype(a, b)` (`simplified/subtype.rs`) is the one structural
  relation, passive over resolved atoms: identical identity (type, canonical
  options, enum member set, literal value, structural inline objects/unions)
  is a subtype; `alphanumeric` compares character sets and the `..` ban;
  always-text types are subtypes of `string`; a `literal` is a subtype of its
  own family and a string literal of an `enum` listing it (`literal(4)` is
  not one of `enum('4')`); `enum` subsets; a non-array union arm by arm on
  either side, order-free. Everything else, including unresolved references,
  `unknown`, `null`, and arrays against other arrays, is not. Constraints and
  presence are ignored, so an optional atom is never a presence check.
  `is_definition_subtype` lifts it to whole `PropertyDef`s.

## Suggestions and DMLS

`suggest(...)` attaches advisory completion candidates without changing
validation. `lint_suggestions()` reports malformed or misplaced suggestions;
`suggestions_for_path()` supplies structured completion items. A property has
at most one `suggest(...)` (across union atoms too); its candidates are linted
against the property's effective union at their insertion site (scalar or
element), so any accepting alternative keeps a candidate. `content_policy`
completion offers all seven rules stored on the `Evergreen` arm. Never isolate
the storing atom when linting (`suggestions_for_def` and `alternatives_for_def`
lint the whole definition).
DMLS validates with `PendingPolicy::Defer`, so it reports nothing for a
`{{ … }}` value, unless the span is malformed (`{{ 1 + }}`): that problem
stands, as it does for `md schema validate` and Claudine's launch verdict.

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
  states; DMLS and `md` share them through `triggers::scan(&ctx)` (DMLS
  calls `scan_with_source`, the same scan over its cached root contents;
  `enumerate_root_files` is the one root listing). DMLS prepares workspace
  registries at startup and serves them from memory until an input changes;
  see [dmls.md](dmls.md) and `darkmatter/docs/lsp/indexing.md`.
- `for_document` is fallible: only `NotFound`/`NotADirectory` (and the
  package/area/tree symlink policy) make a root `Absent`. Any other metadata
  failure (e.g. `PermissionDenied` on an ancestor) is `SchemaError::Io`
  naming the folder, so `scan` never installs a registry missing that root
  and bare-name lookup never falls through to a less local root. DMLS turns
  that `Io` scan error into `SchemaOutcome::Failed` instead of keeping its
  last-good registry (last-good is only for whole-scan failures such as a
  case-fold collision; a skipped trigger installs the valid subset).
- `parse_trigger_envelope_from_str` is the one envelope classifier (discovery,
  embedded bundles, DMLS authoring, payload-cycle checks); never add a
  caller-side pre-check. Unparseable YAML is `Ok(None)` unless a provable
  column-0 `kind: schema-trigger` (or renamed `trigger-schema`) line claims it (`claims_envelope` skips lines
  inside multi-line quoted or block scalars and open flow collections), then
  `SchemaError::TriggerEnvelopeYaml { message, line, column }` (column in
  characters, BOM excluded) with a YAML hint; `TriggerMatch` stays for
  grammar and shape errors. Discovery records either as the cause of an
  `InvalidTriggerFinding` (below), never an `Err`. A malformed flow mapping is therefore unclaimed. BOM and
  CRLF are normalized before parsing (libyaml rejects BOM + CRLF as two
  documents). Tests: `lib/tests/l1/trigger_envelope_claim.rs`.
- Discovery **recovers per trigger** (`triggers/finding.rs`,
  `discovery.rs::scan_with_source`): an invalid envelope, or a payload that
  fails to resolve, import, merge, or compile, is left out of
  `TriggerRegistry::triggers` and recorded as an `InvalidTriggerFinding` on
  `TriggerRegistry::findings` / `DarkmatterSchemas::discovery_findings()`
  (discovery order; the first cause; `identity()` = normalized path or
  `(bundle, name)`; code `SCHEMA_INVALID_TRIGGER_CODE` =
  `dm.schema.invalid_trigger`, warning). A skipped file keeps its name
  claimed, so shadowing is unchanged. Only infrastructure failures stay
  `Err`: an unenumerable root, an enumerated file that cannot be read, a
  case-fold collision, a bundle structure error. The library never prints a
  finding: `ValidationReport::trigger_findings`, `CleanSchemaContext::
  discovery_findings`/`SchemaCleanAnalysis::trigger_findings`, and compose
  (`ComposeWarning::from_invalid_trigger`, added in `prepare_document` before
  any effect, then `stop_if_blocked`, so `--deny` stops it) carry them.
  `enforce_compiles` (`assemble.rs`) compiles every payload's structural
  validator through `SHARED_VALIDATOR_CACHE` (`TriggerPayloadCompile`). A
  failed payload keeps its observations through `resolve::observed::collect`
  (files read, locations probed), which `dependencies()` /
  `searched_locations()` report so a repair or creation refreshes.
  Provenance: `finding.rs::related_path` classifies every `SchemaError`
  variant (no wildcard) into `cause_path`, compared with the trigger on
  canonical forms but kept in the cause's own spelling; a file payload's
  gate errors (`TriggerPayloadNotMergeable`/`TriggerPayloadCompile`
  `payload_path`) take `ResolvedSchema::origin.file()`, the last file of an
  alias chain. `from_invalid_trigger` fills `ComposeWarning::diagnostic`
  with the trigger file at `position()` (source-free otherwise and for
  embedded), which `report-json` prints. The
  passive parsers (`parse_trigger_envelope_from_str`, `resolve_trigger_payload`)
  stay strict. `file_claims_trigger_envelope` counts a malformed claimed
  envelope as a trigger file, so an authored `$schema` naming a skipped
  trigger still fails with `TriggerSchemaReferenced`. Tests:
  `tests/l1/trigger_discovery_recovery.rs`,
  `compose/tests/invalid_triggers.rs`. User docs:
  `darkmatter/docs/topics/schemas/schema-triggers.md` (Invalid discovered
  triggers; per-command delivery, `--deny`, repair refresh).
- The `md` commands other than compose deliver findings through
  `cli/src/commands/trigger_findings.rs`: `emit_warnings` (stderr `Status`
  warning rows, as compose prints them), `FindingRow` (`{source, code, path,
  message}`, `path` null for embedded), and `DeliveredFindings` (one warning
  per `identity()` per invocation; `schema validate` attaches a shared
  trigger to the first file). `schema validate --format json` puts rows
  first in a record's `warnings` on any outcome; `schema triggers` (no
  report mode) writes the warning rows to stderr and lists only each
  source and code in its stdout "Skipped invalid triggers" section; `md clean` reads
  `CleanResult::trigger_findings` (stderr, or `--json`'s top-level
  `warnings`), and a schema failure carries them on
  `FrontmatterRepairError::Schema { trigger_findings, .. }`, which
  `clean.rs::CleanFailure` hands back so the caller prints them on stderr
  or, under `--json`, in `json::schema_failure_report`'s v1 envelope. Ordinary
  `md compose` prints a failed run's trigger warnings before its error
  (`report_failure_trigger_warnings`), except the ones a `--deny` block
  already shows. Tests: `cli/tests/l1/invalid_trigger_warnings.rs`.
- Claudine carries findings in an invocation-scoped
  `composition::TriggerFindings` record (`InvocationContext::trigger_findings`;
  one entry per `identity()`, first cause, discovery order, a delivery
  cursor). `load_effective_schema` records discovery's findings before the
  authored `$schema` resolves, so every preliminary check (`pre_validate_*`,
  `drop_invalid_optionals`, status reports, the launch schema, retry legs)
  takes the record and never turns a finding into a `CompositionError`;
  preparation records compose-report warnings through
  `ComposeWarning::invalid_trigger_source()`. The CLI shows each once:
  `emit_compose_warnings` drains the record (loops and retries rediscover
  silently), and `TriggerWarningDrain` drains the rest when the invocation
  returns, ahead of the error `main` renders. A `prep` target is its own
  invocation. Tests: Claudine `lib/tests/l1/invalid_trigger_findings.rs`,
  `cli/tests/l1/invalid_trigger_warnings.rs`.
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
- Embedded trigger bundles (`EmbeddedSchemaBundle`, `schemas/embedded.rs`):
  a host registers `(file name, &'static str)` pairs with
  `DarkmatterSchemas::with_embedded_trigger_bundle` (or
  `triggers::scan_with_embedded`); they are appended after the five roots,
  and a bundle file is skipped when any root enumerated a file of that name
  (the registry's private `claimed_names`). Bundle documents are keyed
  `{bundle}/{name}`; `NamespaceSource::Embedded { documents, bundle }`
  confines imports with `embedded::resolve_bundle_path`. Identity is
  `TriggerSource::{File, Embedded { bundle, name }}`; an invalid bundle
  trigger is a finding whose source is `Embedded` (never a file path, link,
  or watch) and still claims its name from later bundles. `SchemaOrigin { kind, source: Option<SchemaSourceId> }`:
  read `file()` (real files only; DMLS links only `ReferencedFile` through
  it) or `trigger_source()`. Embedded triggers add no dependency edges.
  Tests: `tests/l1/schema_embedded_trigger_bundles.rs` (incl. the envelope
  input matrix for block YAML and JSON syntax).
- Large host payloads (Claudine's resolves to about 1.4 MB, its recursive
  families stored once in `$defs`) are kept affordable by
  process-wide memos, each holding its inputs alive and keyed by `Arc`
  identity (plus value equality for small authored schemas): embedded
  trigger loads (`discovery.rs::load_embedded_trigger`, successes only),
  trigger merges and document schemas with or without the baseline
  (`mod.rs::merged_trigger_schema`, so an unchanged schema keeps one `Arc`
  and its digest is computed once), phase schemas (`phase_json_schema`), and
  wrapped root-union arms (`wrapped_root_arms`). Import namespaces are
  parsed once per file revision (`resolve.rs::load_named_types`, keyed by
  path and a hash of the bytes read on every use) and once per shipped
  partial. Payload and layer JSON are `Arc<Value>`; a lone trigger layer is
  shared, never cloned. `ValidatorCache` compiles a schema that names no
  `darkmatter-file*` format and no `x-darkmatter-match` once for every
  context (structural slot), and conversion reuses that validator
  (`ConversionPlan::validator`); a root union compiles only its arms
  (`CompiledValidators::RootUnion`, so `EffectiveSchema::validator()` is
  `None` for it). Coercion skips validating union arms when no arm changes
  the value. Lowering builds nested fragments by moving them
  (`convert.rs::any_of_value`), never `json!` with an interpolated `Value`,
  which re-serializes the subtree. Every trigger payload is still resolved
  and lowered at discovery, matched or not, because its load errors are
  reported there (`tests/l1/trigger_payload_load_errors.rs`). Do not
  reintroduce a per-call clone, serialization, or uncached compile of an
  effective schema.
- A test fixture for roots builds its context with `build_resolution_context`
  from a `RequestSnapshot` carrying a fixture home and env, never
  `FileResolutionContext::new` (ambient `HOME` would add a real `~/schemas`).
  `md` child processes scrub `SCHEMAS_DIR`; declare it with
  `application_input("SCHEMAS_DIR", ..)`. A DMLS fixture whose triggers live
  in the workspace `schemas/` must be a repository: outside one, a
  document's tree root is its own folder.

## Authored global catalog

`darkmatter/schemas/darkmatter.yaml` is the one declaration of every global's
shape: `kind: schema`, `description`, and a `$schema` mapping of `fm`, `ctx`,
`current`, `env`, `err`, `timing`, mostly named imports from the types-only
`partials/`. `schemas/catalog.rs` loads it strictly (`GlobalCatalog`,
`CatalogError { field, reason }`; `DARKMATTER_GLOBALS`): every global
required, null/empty/wrong kinds rejected, `current` must equal `ctx`,
duplicate keys and trailing YAML are errors. The `ctx.*` descriptors project
its `ctx`; `err` and `timing` members equal the runtime
`LifecycleErrorInfo`/`LifecycleTiming` values. A catalog (`kind: schema` with
a `$schema` mapping and no `types`) is never a document schema: the
standalone loader refuses it by content, wherever a `$schema` reaches it.
Hosts declare their own globals in a catalog of the same format
(`claudine/schemas/globals.yaml`, loaded with `GlobalCatalog::from_documents`
and the host's requirements). DMLS hovers and completes host globals from the
catalogs in a document's schema roots (`catalog::host_catalogs`), treating
each as `ExecutionDependent` (DMLS cannot know loop/group membership); a
catalog that fails to load is a `dm.schema.host_catalog` warning. Inside an
md event's stack DMLS uses that event's registry `BindingView`
(`dmls/src/overlay/event_views.rs`), so `err` where the event has none is
`dm.expression.unavailable_global`.
