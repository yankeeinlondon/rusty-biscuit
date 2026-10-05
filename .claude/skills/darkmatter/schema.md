# Darkmatter Schemas

Use this reference for SimplifiedSchema parsing, validation, triggers, imports,
and DMLS schema behavior.

## Contents

- [Document kinds](#document-kinds)
- [Composition and validation](#composition-and-validation)
- [Imports and dictionaries](#imports-and-dictionaries)
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
  cycle tracking.
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
- Pattern dictionary keys lower to `additionalProperties` or
  `patternProperties`; literal keys take precedence.
- `min-keys` and `max-keys` constrain dictionaries.
- Examples are documentation artifacts validated at schema-load time and
  emitted through `x-darkmatter-example`.

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
