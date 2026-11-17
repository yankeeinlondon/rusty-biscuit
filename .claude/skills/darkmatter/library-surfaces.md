# Darkmatter Library Surfaces

Use this reference when changing public exports or deciding which module owns a
behavior.

## Contents

- [Document and frontmatter](#document-and-frontmatter)
- [Composition and expressions](#composition-and-expressions)
- [Schemas and references](#schemas-and-references)
- [Rendering and style](#rendering-and-style)
- [Effects, cache, and remote access](#effects-cache-and-remote-access)
- [Launching the user's editor](#launching-the-users-editor)
- [Package boundaries](#package-boundaries)

## Document and frontmatter

- `Markdown` is the primary parsed Markdown document.
- `Frontmatter` retains structured values and raw source information used for
  span-aware diagnostics.
- Markdown-aware hashing treats the frontmatter and body as distinct segments.
  Use the Darkmatter library or `md hash`; do not substitute a generic file
  hash.
- YAML analysis and span support are shared through `biscuit-file` rather than
  reimplemented in Darkmatter.
- `yaml_edit` (`edit_yaml_paths`, `YamlEdit`, `YamlEditError`) edits plain
  YAML files in place with the frontmatter path editor's engine and never
  serializes the whole file; see [frontmatter.md](frontmatter.md#plain-yaml-files).
- `config::write` (`ConfigTransaction`, `TargetChange`, `ConfigWriteError`)
  is the one configuration writer: sibling `<name>.darkmatter-lock` (kept,
  `File::try_lock`, 2 s/20 ms), locks in portable-path order, re-read,
  `edit_yaml_paths`, caller's passive validation callback, expected-bytes
  check, then the shared `atomic_replace::persist` (also used by
  `effects::fs_write`). `config::migration_receipt` is the one receipt
  reader/writer (`<name>.darkmatter-migration.json`, parsed by
  `strict_json`); `complete_cleanup` acts only on caller-listed known
  sources. Test seams: `config::write::test_hooks` under
  `effects-instrumentation`. User docs: `docs/topics/config-writes.md`.
- `config::schema` holds the shipped configuration schema
  (`darkmatter/schemas/darkmatter-config.yaml`, in `embedded::SHIPPED`,
  served as `Name@darkmatter:config` through `SHIPPED_ALIASES`; the alias
  pairs are the artifact's preparation `configuration`). A host program
  prepares its own compiled-in config schema (Claudine's
  `claudine-config.yaml`, which imports `permissions@darkmatter:config`)
  through `config::schema::prepare_host_config_type(source, path, name)`,
  never by building a context itself (the context-construction guard
  refuses that outside Darkmatter). `config::load`'s
  `ConfigSnapshot::load(context, Option<&CommittedTreeSnapshot>, mode)`
  reads the user file (home from the context) and the repository file at
  HEAD and in the working copy, with the admitted-YAML loader, judging one
  setting at a time against the scope's selected type (`user-config` /
  `repository-config`): whole-file failures are syntax, non-mapping, and
  unknown keys; `permissions()` returns one merged error; `register`
  findings omit only their entries. Typed entries reuse
  `shell_expansion::store::parse_rule_entry` and biscuit-file's
  `GlobReference`/`FileReference`. Permission evaluation, reference
  following, and legacy migration are later work. User docs:
  `docs/topics/configuration.md`.

## Composition and expressions

- `ComposeRequest` is the request authority: it owns the required
  `FileResolutionContext` (built by `build_resolution_context` from a
  `RequestSnapshot`) and `Deref`s to `ComposeOptions`, which carries remote
  policy, cache, schemas, and rendering options but no context.
- `EffectiveContext` and the expression runtime expose `ctx.*`, `fm.*`, and
  `env.*` without ambient recapture.
- Expression descriptors are typed catalogs used by both library callers and
  CLI documentation. Add a function once in the registry and keep parsing,
  evaluation, descriptors, and completion aligned.
- Author functions in `schemas/partials/functions.yaml` (a tagged SimplifiedSchema
  file: `categories:` plus one `function(...)` type, or a list of them for
  overloads, per `types:` entry) and `ctx.*` in the `context` type of
  `schemas/partials/context-variables.yaml`. The loader in
  `expression/catalog/mod.rs` embeds the file and its
  `function-examples/*.yaml` files with `include_str!`; a new file-form
  example must be added to its `EXAMPLE_FILES` table. The function registry
  panics at load when a cataloged function has no runtime binding, so every
  catalog entry needs a `FunctionBinding` in the same change.
- A new catalog function also reaches these hand-kept or generated copies,
  and each fails a test until updated: `just regen-expr-doc` (the function
  table in `docs/topics/darkmatter-expressions.md`), `just
  regen-lifecycle-schema` (verb types in `schemas/partials/lifecycle.yaml`,
  plus the verb count in `lifecycle_schema_generation.rs`), the two verb
  pattern lists in `schemas/partials/md-lifecycle.yaml`, Claudine's mirrored
  `expression` category in `claudine/schemas/{action,lifecycle}.yaml`, a row
  in `tests/l1/function_binding_registry.rs`, an alias row in
  `function_catalog_parity.rs`, the recursive-schema goldens
  (`DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`), DMLS's
  `a3_completions.json` (`DMLS_UPDATE_GOLDEN=1`), and Claudine's insta
  snapshot `claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap`
  (only the Claudine suite fails; Darkmatter's `just test` stays green).
- A binding whose failed argument binding means a domain error registers
  `FunctionBinding::refining_binding_errors` (`load_data`: a scalar `as`
  is `dm.data.not_a_container`, not a type error); a passive per-function
  verdict a literal call can prove belongs on `CallCheck::data_problem`, so
  composition and DMLS publish its code from `CallCheck::diagnostics`.
- A runtime `ExpressionError::Data` is projected only in
  `compose/report_diagnostics.rs`: `DataError::code()`, `data_location` for
  `dm.data.malformed`/`dm.data.contract_mismatch` (positions from
  `expression/data/locate.rs`, records from simplified-schema's
  `ConversionError::located_failures`, never interpreted in darkmatter),
  capped at `CONTRACT_RECORD_CAP` plus one overflow record. Every
  `ExpressionError::FileReference` fills `reason`/`construct`/`reference`/
  `failure` without changing `message`; a host the fetch policy refuses is
  the typed `FileRefFailure::RemoteDenied`, not text. Any other failed
  fetch from an allowed host (`RemoteTextError::Failed`, through
  `functions/fs.rs::remote_text_error`) is `ExpressionError::RemoteRead`
  (displayed and graded like `Other`), projected with reason `unreachable`
  and failure `io` (`report_diagnostics::with_failed_fetch`).
  A `$schema` target or import `validate_schema` cannot read
  (`SchemaError::{Unresolved, NoMatch, Io}`, also inside
  `ReferencedSchema`/`Aggregate`) is `ExpressionError::SchemaFileRead`
  (displayed and graded like `Other`, carrying a `FileReferenceDiagnostic`
  from `FileReferenceDiagnostic::from_schema_read`, the constructor
  `load_data`'s contract imports share); any other schema error stays `Other`.
- Consumers read `expression_function_catalog()` (categories in list order,
  descriptors in documentation order: category, then canonical name, then
  authored overload) or `function_descriptors(name)`, which resolves aliases
  and case like dispatch. List canonicals only; never re-sort by a field or
  keep a second signature list.
- Descriptors own their data (`Box<str>`, boxed slices, owned `FunctionType`;
  `Clone`, not `Copy`); `load_catalog` leaks nothing, even on error. `&'static`
  views exist only by borrowing a `LazyLock` owner:
  `expression_function_descriptors()`, `function_descriptors(name)`,
  `effect_function_descriptors()`, lifecycle signatures.
  `FunctionCatalog::descriptors_for`, `HostFunctionCatalog::function_names`,
  and `Described::{key, description, category}` borrow from `&self`;
  `Described::example()` returns a `Copy` `Example<'_>` view. Never
  reintroduce a leak or `'static` field to satisfy a borrow; borrow or clone.
- The public `RegistrationBuilder::narrows` setter is gone: narrowing facts
  are declared with a `narrows(...)` clause in `function(...)` signatures
  (built-ins, host catalogs, `FunctionRegistration::builder`/
  `builder_from_types`). Do not reintroduce a Rust-side fact setter.
- Host function catalogs (`HostFunctionCatalog`, `HostFunctions`) share one
  `Arc` freed with the last holder. `host_function_catalogs` caches one result
  per discovery path, replaced on any catalog/artifact byte change and never
  evicted; see `expressions.md` and `docs/topics/host-functions.md`.
- Non-function `types:` entries are named helper types that signatures use:
  `ip-address` (IPv4, IPv6, or a scoped `fe80::1%en0` literal; no DNS),
  `agentic-cli` (an `enum(...)` that a test pins to
  `expression/functions/agentic_cli_generated.rs`), and `recent-commits` (the
  return of `recent_commits(count)` and, through
  `recent-commits@darkmatter:functions`, the type of `ctx.recent_commits`; the
  two descriptions name each other). `functions.yaml` is in `SHIPPED` for that
  import, so on-disk copies of the partials need no copy of it. Typed signatures
  show the helper's name, not its expansion.
- `agentic_cli_generated.rs` is written by `claudine-gen generate` from
  `claudine/docs/providers.yaml`. Never hand-edit it; `claudine-gen check` and
  its nextest drift test fail on a stale table. It stores `AiCli` variant
  names as strings, so a stale table cannot block building the generator
  (claudine-gen depends on this crate).
- `reserved_root_descriptors()` lists exactly `fm`, `ctx`, `env`, and the lazy
  mirror `current` (of `ctx`). The evaluator reserves all four:
  `context/current.rs` resolves the lazy root ahead of frontmatter,
  external state, and injected globals. Public surface:
  `CurrentProvider`/`CurrentRefresh` (the invocation's per-key capability),
  `CurrentAuthority`, `ComposeOptions::with_current_provider`,
  `EffectiveStateBuilder::with_current_authority`, and `DeferredCapabilities`
  with `ComposePreflightReport::deferred_context`.
- Every cataloged `ctx.*` key has exactly one owning `ContextGroup`.
- File arguments resolve through the shared document-backed
  `FileResolutionContext`. Shape/probe helpers must agree with actual
  resolution.

## Schemas and references

The SimplifiedSchema language is the `simplified-schema` crate
(`simplified-schema/lib/src/`, imported as `simplified_schema`); load the
`simplified-schema` skill for it. Import its types from `simplified_schema`
directly: `darkmatter::markdown::schemas` does not re-export them.

- Library-owned (edit in `simplified-schema/lib/src/`): the parsed `$schema`
  AST (`SchemaRoot`, `SchemaShape`, `PropertyDef`, `TypeExpr`), the grammar,
  `parse_standalone_schema_document` and its passive product
  `StandaloneSchemaDocument`, named-type resolution, JSON Schema lowering,
  subtyping, the conversion engine, `ValidatorCache`, `SchemaError`,
  `ValidationProblem`, `SchemaOriginMap`, `SchemaPhase`, the standalone
  facade `simplified_schema::SimplifiedSchema` (with `Prepared`,
  `validate_strict`/`validate_soft`, and layering), and the schema-kind
  infrastructure: `triggers` (grammar, discovery, matching, `TriggerOrigin`,
  `SCHEMA_INVALID_TRIGGER_CODE`), `roots` (`SchemaRoots`), `embedded`
  (`EmbeddedSchemaBundle`, `ShippedSources`), and `yaml_documents`
  (`LoadedYamlDocument`, `KindRegistry`, selection, kind checks). Every entry
  point there that resolves a payload or prepares a kind definition takes
  the host's `ShippedSources`. Darkmatter must not
  redefine any of these names
  (`lib/tests/l1/schema_language_ownership_guard.rs`).
- Darkmatter-owned (`darkmatter/lib/src/markdown/schemas/`):
  `DarkmatterSchemas` loads and merges document, trigger, and baseline schemas
  for a Markdown document; `EffectiveSchema`, `ValidationOptions`, and
  `PendingPolicy`; the shipped baseline and global catalog; the YAML adapter
  (`YamlContract`, `YamlValidation`, `YamlOutcome`, `Route`); the wrappers
  that pass `embedded::SHIPPED_SOURCES` to the library (`schemas::resolve`,
  `schemas::triggers::{scan, scan_with_embedded, scan_with_source,
  prepare_trigger_bundle}`, `shipped_sources()`, `kind_host()`,
  `shipped_kind_registry()`); `ProblemPositions`. `darkmatter::markdown::schemas`
  does not re-export the moved types; import them from `simplified_schema`.
- `ValidationProblem` carries legacy message/kind fields plus typed code,
  instance/schema paths, offending property, source location, and optional
  file-reference detail.
- `SchemaOriginMap` records the owner of each effective top-level property.
- `simplified_schema::{layered_property_shape, property_def_at_path,
  nested_property_shape, expression_atom, frontmatter_expression_values}` and
  Darkmatter's `schemas::effective_property_shape` (`schemas/frontmatter_shape.rs`,
  which layers Darkmatter's baseline) are the passive authority for which property
  definition governs a frontmatter key path and which values are
  Expression-typed. DMLS completion, hover, and `dm.expression.*` diagnostics
  and the corpus gate share them; do not re-derive arm selection or the
  Expression check elsewhere. Trigger payloads and pattern-dictionary keys
  contribute no definitions here.
- Reference graph nodes retain origin, dependency, identity, and freshness
  information; do not collapse these to plain paths.

## Rendering and style

- `Markdown::as_terminal`, `Markdown::as_html`, and `DarkmatterPage::render`
  route through the render-tree document fold.
- `DarkmatterPage` owns only viewport/page framing.
- Command-line style is a later `StyleLayer` (setters or a `--style` file)
  merged by `prepare_style_layers`; there is no separate CLI style model and
  no `apply_*_style` override argument.
- `CodeBlock` is the primary highlighted-code component. `YamlBlock` is a
  deprecated compatibility wrapper.
- Component layout and paint lower to `renderable` types before target folds.

## Effects, cache, and remote access

- The effect engine owns explicit writes, shell calls, and HTTP POST behavior.
- Remote reads and writes share `biscuit_file::FetchPolicy`.
- Only raw remote-URL bodies (transport artifacts) persist (R18, R36).
  Compose keys (source identity, state, context, options, overlay) are
  run-local only; no semantic-result persistence code exists until a
  `ContentPolicy` does.
- Passive schema, DMLS, and validation surfaces never invoke the effect engine.

## Launching the user's editor

`darkmatter::editor` is the one editor path for `md edit`, Claudine's wrapper
`--edit`, and the lifecycle `edit` action:

- `resolve_editor(SurfaceRequest)` is the only resolver (`resolve_editor_command`
  is its `Any` display string). Every editor fact (surface, wait flags,
  identifying `$EDITOR`'s first word) comes from Sniff's `Editor` catalog; never
  add a flag table here. A non-blocking GUI editor is never selected. The order
  and filtering are unit-tested over `resolve_editor_from` with fake lookups
  (`editor/resolve.rs`); the env reads are the guard's `editor/resolve.rs`
  allowance.
- `LaunchPlan::run(LaunchOptions)` is the only launcher. `LaunchOptions::PLAIN`
  (`md edit`, `--edit`) keeps the host's process group and an uninterruptible
  wait. `own_tree: true` makes the editor the root of a `sniff::process::ProcessTree`
  (process group / Job Object, the same owner the login-shell probe launcher uses;
  [process trees](../../../sniff/docs/topics/process-trees.md)), polls
  `cancelled` every 20 ms, terminates with `SIGTERM` then `SIGKILL` after
  `EDITOR_TERMINATE_GRACE`, hands a terminal editor that inherits stdin the
  foreground through `sniff::process::ForegroundHandoff` (prepared after
  `TreeConfig::configure`), and restores the host's foreground group and
  `termios` explicitly after the editor is reaped: a prepare failure (including
  `WouldBlock` while another handoff is active) is `EditorError::Io`
  `prepare the terminal`; a restore failure fails only an otherwise successful
  edit (`restore the terminal`) and is logged otherwise. A successful exit `release`s the tree
  (Windows clears kill-on-close) so a GUI client's main window survives.
  `LaunchPlan::run_reporting` returns a `LaunchReport` whose `warning` is set when
  that Windows release fails; `ProcessEditorService` copies it to
  `EditOutcome::warning`, and the edit still succeeds.
- `ProcessEditorService` (trait `EditorService`) is the `edit` action's service:
  cancellation check, resolution, eligibility (`EditorHostFacts`,
  `EnvironmentProbe`, the thread-local `deny_terminal_in_this_thread` scope for
  parallel members, one process-wide terminal claim), then
  `effects::fs_write::create_new_guarded` (one `O_EXCL` create, never
  `ensure_file`), launch on the `canonicalize_simplified` path, and a post-exit
  containment and regular-file check. `EditOutcome::attempted` is false only for
  refusals before creation.
- Tests launch the fake editor installed under a real editor's name
  (`cli/tests/l1/editor_support.rs::install_fake_editor`; reinstalling a name
  reuses the existing hard link, because copying through it would truncate the
  shared fixture binary).

## Package boundaries

| Package | Responsibility |
|---|---|
| `simplified-schema` | The SimplifiedSchema language: grammar, resolution, conversion, validation, JSON Schema lowering, passive expression syntax (no Markdown dependency) |
| `darkmatter` | Parsing, composition, schema policy (baseline, roots, triggers), references, hashing, rendering |
| `darkmatter-cli` | The `md` CLI and presentation/orchestration policy |
| `dmls` | Language Server Protocol implementation over passive Darkmatter APIs |
| `zed-dmls` | Zed extension integration for DMLS |

Keep CLI-only parsing and flags out of the library. Keep editor transport and
workspace state in DMLS, while document, composition, and schema-policy
authority remains in `darkmatter`. The schema language and passive expression
syntax belong to `simplified-schema`, which never depends on Darkmatter, DMLS,
Claudine, or a CLI (`simplified-schema/lib/tests/l1/dependency_direction_guard.rs`).
