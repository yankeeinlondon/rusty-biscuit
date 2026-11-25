---
name: darkmatter
description: Expert guidance for the Darkmatter Rust library, `md` CLI, and DMLS language server. Use when composing or rendering Markdown, working with frontmatter, expressions, schemas, file references, remote content, hashing, browser or terminal output, or extending Darkmatter's CLI and editor integrations.
---

# Darkmatter

Darkmatter is the monorepo's Markdown composition, schema, expression, and
rendering system. The package area contains the `darkmatter` library,
`darkmatter-cli` binary (`md`), `dmls` language server, and `zed-dmls`
extension.

## Non-negotiable boundaries

- Capture one request-scoped composition context. Pass the same
  `FileResolutionContext`, repository observation, remote policy, cache state,
  identity, and meta-schema controls through `ComposeOptions`; never recapture
  CWD or repository state in a downstream resolver. The one exception is
  source admission: a source whose nearest Git checkout (its closest `.git`
  directory or file, never containment under a known root) is a repository
  the request's context does not name is discovered once per request
  (`SourceRepositories`) and carried as `SourceDerivation::OwnRepository`;
  see [requests.md](requests.md).
- Resolve file-like values through `biscuit_file::FileReference`. Preserve its
  source context, explicit-vs-implicit syntax, and typed errors.
- Read the home only through `biscuit_file::home_dir()` (`RequestSnapshot::from_process`
  is the one production site; tests build expectations with it too). A canonical
  path that leaves a private comparison (link targets, cache and preflight keys,
  `referenced_files`, diagnostics) comes from `biscuit_file::canonicalize_simplified`;
  `path_lookup_guard.rs` in lib and cli rejects any unlisted raw `canonicalize`.
- Treat composition as effectful and validation as passive. Schema parsing,
  trigger matching, completion, hover, and validation must not perform I/O,
  execute expressions, fetch remotes, or mutate documents.
- Keep network access deny-all by default. Route every allowed remote read or
  write through `biscuit_file::FetchPolicy`: exact-host consent, or the
  configured `network: allow` projected by `config::network::NetworkPermission`
  (`FetchPolicy::allow_all()`, never a wildcard host). An invalid
  `permissions` block (`NetworkPermission::Invalid`) refuses every URL with
  its configuration error before any per-run host or approval is consulted.
- Keep schema `{{ … }}` slots passive by default. Only an explicit resolution
  through a `SlotSession` (`DarkmatterSchemas::resolve_template`) evaluates
  them, after every slot is admitted; nothing ever validates against a
  template or a placeholder. Every slot read passes the grant gate before the
  file is opened or the shared memo consulted, and grants come only from the
  captured `permissions.file-read` and this run's approvals; repository or
  workspace content never grants. See [schema.md](schema.md), "Schema templates".
- Render terminal output with `TerminalRenderable` components. The browser tier
  is headless and must use browser-protocol input, never host focus or OS input.
- Preserve source spans and typed diagnostic provenance through parsing,
  composition, schema validation, and DMLS projection.

## Choose the owning surface

Documentation under `docs/` becomes the authority when implementation lands;
it never links to a specification or feature/fix design. Preserve documented
file locations; an unimplemented grammar is not a reason to move schema
definitions away from their references or replace them with placeholders.

| Work | Start with |
|---|---|
| Compose APIs, stages, expressions, file resolution, cache | [compose.md](compose.md) |
| SimplifiedSchema, triggers, validation, meta-types | [schema.md](schema.md) |
| Composition requests, contexts, glob consumers | [requests.md](requests.md) |
| Remote reads and the transport cache | [remote-cache.md](remote-cache.md) |
| `md` file arguments and CLI conventions | [cli.md](cli.md) |
| Expression grammar, inserted text, warnings | [expressions.md](expressions.md) |
| `load_data`, data contracts, `dm.data.*`, read set | [expressions.md](expressions.md#load_data-data-files), [requests.md](requests.md#data-runtime-and-read-set) |
| Lifecycle grammar, executor, run accounting, request/reply `Run` | [lifecycle.md](lifecycle.md) |
| Test fixtures, source guards, parity matrix | [testing.md](testing.md) |
| Public modules and extracted library surfaces | [library-surfaces.md](library-surfaces.md) |
| DMLS architecture, protocol behavior, and rollout history | [dmls.md](dmls.md) |
| Render tree, style lowering, disclosure blocks, code blocks | [rendering.md](rendering.md) |
| Terminal rendering options | [terminal.md](terminal.md) |
| Frontmatter model, shared views, literal tokens, in-place leaf edits, plain-YAML edits (`yaml_edit`) | [frontmatter.md](frontmatter.md) |
| `.darkmatter.yaml` schema, `ConfigSnapshot` loading, locked config writes | [library-surfaces.md](library-surfaces.md), `docs/topics/configuration.md` |
| Permission policy: `config::permissions::PermissionPolicy` (built once per `ConfigSnapshot` from the user layer plus the repository's `HEAD` and working-copy layers; an exclusion in any layer wins, otherwise user allows or both repository layers allow; `network: allow` only when every layer says so), saving an "always allow" entry (`config::permissions::persist::save_user_approval`, user scope only, through `ConfigTransaction`) | `docs/topics/permissions.md` |
| Host registration: `darkmatter-host` manifests (format 3, seven closed fields `kind`/`format_version`/`host`/`version`/`files`/`lifecycle`/`digests`, no `functions`; `config::registration::manifest::read` checks the integer `format_version` before the contract and refuses 1, 2, or a later version as `ManifestRefusal::Unsupported` → `SnapshotRefusal::unsupported_format`, `unsupported host registration format version <n>; update Darkmatter and the host tool, then republish the registration`; an expression catalog's facts companion is derived, never listed, by `manifest::companion_spelling` over `host::facts_artifact_path` (last `.yaml`/`.yml` → `.facts.json`) and must be in `digests`, which must equal members ∪ companions; the manifest is verified into a digest-checked `bundle::VerifiedSnapshot` by one `attempt` shared by `bundle::load_snapshot`/`load_snapshot_with` (whose reader seam lets tests swap bytes between reads), `from_embedded`, and `verify_read`, in the order explicit members' digests → lenient `kind` classification → derive companions → exact digest set → companion bytes, every possible companion probed first so it is a watch candidate; `VerifiedSnapshot::companions()` (`BundleCompanion { catalog, facts }`) and `members()`, companions in the revision; `bundle::admit_manifest` applies the same envelope, digest-map, and lexical member-path admission without reading members, for a publisher deciding what its previous manifest lets it retire (it alone also admits a well-formed format 2 manifest through `manifest::read_for_cleanup`; `AdmittedManifest::format_version()`); `bundle::admit_installed_manifest` is the same admission with the installed reader's confined member resolution (a `../<bundle-dir>/…` re-entry is the in-bundle member, an escape is refused), for a diagnostic that reads the members itself (`claudine doctor`); `bundle::verify_read` (the same admission and digest checks over bytes a caller already read, nothing probed, shared `attempt` with `load_snapshot_with`) plus `registration::load_verified` (the `register` loader's `profile_of`) let a diagnostic learn whether a bundle loads without a `register` entry; `config::registration::load_manifest` classifies each member by `kind` (trigger, `yaml-schema`, `namespace-schema`, types-only library, `expression-schema`; `kind: mutation-schema` refused at `kind` with `member_envelope::MUTATION_REFUSAL`, `host-defined actions are not supported`) through the shared closed-envelope check `simplified_schema::member_envelope` (also Claudine's build check; a refused namespace, or a member refused before its kind is known, skips catalog and lifecycle preparation) and prepares it from verified bytes inside `simplified_schema::resolve::bundle::with`, then checks every `types` declaration, used or not, with `simplified_schema::resolve::check_declared_types` under the same scope (`registration::declared_types`; a namespace's before the catalog is used); `registration::namespaces::prepare_catalog` builds the `GlobalCatalog` and the lifecycle `catalog_hash`; each expression catalog loads once per installed revision through `registration::load_functions` (`HostFunctionCatalog::load` + `HostFunctions::analysis_only_at` over its companion, any incomplete function refusing the host) into `HostFunctionSet { bundle_path, catalog, facts, functions }`, and `name_collisions` refuses a later case-folded canonical name or alias across one host's catalogs (bundle-path order; another host may reuse it) with the earlier claim in `RegistrationDiagnostic::related` (`RegistrationRelated`, also carrying a catalog failure's `HostCatalogError::related()`); there is no process-wide host function cache or schema-root catalog discovery, and the selected profile's sets are entered (`HostFunctions::enter`) only for its documents (CLI `compose.rs`, DMLS `providers/dsl.rs`), an evaluated call failing `host function <name>() requires the host runtime`; every trigger identifies (no `identify` field) and must pass `admit_identifying_trigger`; registered YAML kinds are indexed by `RegisteredHosts::yaml_kinds()` and are the lowest tier of `prepare_yaml_contract` (`select_with_registered`; `YamlContract::registered()`), used by `md schema validate` and DMLS, whose contract key folds in the installed registrations' version), `config::registration` profiles, host selection (`RegisteredHosts`, `select` → `Selected{selection, matched}`, `HostSelection::{Registered, Ambiguous, Md}`; no schema-root host discovery remains), the registered trigger layer (`config::registration::merge::check` compares every pair of matched payloads (a narrowing's descriptions and rules at the union alternative and element/list level they govern, `merge::arms`, never pooled across alternatives; direction and rule pairing share `merge::covers` over resolved alternatives; a list stays one property alternative and compares item alternatives by coverage, with list and item rules kept separate, so a mixed list never narrows separate homogeneous lists) into one checked layer or a boxed `SchemaError::HostTriggerConflict`, code `dm.host.trigger_conflict`, whose `first_name`/`second_name` `TriggerConflict::into_error` fills from the profile's matched triggers so its message names each `name (path)`; `RegisteredHostProfile::checked_layer` memoizes static results per sorted matched set, a template payload resolves whole and bypasses it; the passive `DarkmatterSchemas::registered_trace` (`md schema triggers`) runs it over the matched concrete payloads only and reports `RegisteredTrace::conflict`, which fails that command; registered payloads may be one root union, `PayloadForm::ObjectOrUnion`, merged by `merged_trigger_schema`'s `merge_around_union`: lower layers go under each arm and every higher layer (root triggers, the document) over each arm through `merge_baseline`, so higher keys and root rules (`closed`) win in every arm, and a document root union yields the cross product, host arms major; the editor sees the union through `CheckedPayload::union_arms` (pruned like `shape`): `EffectiveSchema::trigger_shapes` holds every arm's merged view (`simplified_schema::root_union_property_shape`, shared declarations as written) and `property_shape`/`declared_property_shape` narrow it to the arms the frontmatter admits (`trigger_unions`, matched by `Arc` identity); a phase projection (`simplified_schema::phase::merge_with_effective`) merges an object document into every arm of such a union and a union document's arm `k % m` into effective arm `k`; a host can also register a bundle compiled into it (`bundle::VerifiedSnapshot::from_embedded`, `registration::load_embedded`, virtual root `<host>`, `TriggerSource::Embedded` provenance, duplicate check by embedded identity plus payload digest), which Claudine's in-process selection uses; `report_diagnostics::schema_error_diagnostics` is the record), every refusal located before it becomes a warning (`registration/locate.rs`: the most specific file and field of a typed `SchemaError`, a reference site through `simplified_schema::yaml_documents::declaration_segments` (union arms and the writing section kept), including the declaration a file-reference failure (`Unresolved`, `NoMatch`, `RemoteUnsupported`, `BundleBoundary`, `BareNameSiblingExists`, `InvalidExample`, `TriggerSchemaReferenced`) carries as `SchemaError::reference_site()`, recorded by the resolver where it first read the reference, and the declaration that named a listed file refused for its role (`NotASchemaDocument` with `NotSchemaReason::is_role()`, `UnsupportedJsonDialect`, `ReferenceCycle`/`ReferenceDepthExceeded`, and `TriggerPayloadCycle` after `refused_trigger_file`), attached by `SchemaError::refused_at` at the two load boundaries (`resolve_reference_in_context`, `ImportEngine::resolve_namespace`); a content refusal of the target stays in that file; the locator never searches text for a reference, and one with no site is file-level, a grammar failure at `locate_carrier_definition_failure`'s token, an importer's refusal moved to the imported member with `imported by` in the message, the member and each importer shown as `name (bundle path)` when its verified text declares a valid `name` (`registration::member_label` over `simplified_schema::member_envelope::declared_name`, the one lenient label rule ordinary discovery and Claudine's build share; never for a YAML kind, namespace, or a refusal before the bundle verifies; the embedded loader carries it as `SnapshotRefusal::label`) (a reference an imported file wrote arrives wrapped in `ReferencedSchema` naming it), a non-UTF-8 member at its manifest entry; lifecycle artifact reasons through `host::artifact::reason_site`, the inverse of the strict reader's field phrases, so a new lifecycle reader message keeps that form, while the facts reader records typed sites as it judges (`analysis_only_sited`: JSON pointer segments, or the strict parser's own line and column); `md` prints a positioned run notice as `path:line:column:` via `output::notice_text`), `md compose` of a foreign document (`ComposeOptions::with_selected_host`, `RegisteredHostProfile::analysis_globals`, `md_host::{shared_event_keys, execution_projection, foreign_deferred_keys}`), `dm.host.*` codes, a host's setup adding its own `register` entry (`config::init::register_manifest`, deduplicated by `config::registration::manifest_identity`; `replace_manifest_registration` for a moved manifest, matching a missing old file through `registration::manifest_location_identity`, the old-manifest match public as `registration::entry_names_location` so a diagnostic (`claudine doctor`) reports exactly the entries the writer removes; `replace_manifest_registration_existing` is the same edit for a repair that never creates the file and refuses a changed one; `replace_manifest_refusal` dry-runs that planned edit with `TargetChange::edit_refusal` before the manifest is published (a missing manifest identified by location), so a host can report an inline `register` list or root it cannot edit in place before changing anything; `init::check_referenced_permissions` judges another program's would-be text as a `permissions` reference target; Claudine publishes with `claudine::composition::registration` first), DMLS's per-document host view (`dmls/src/overlay/hosts.rs`, see [dmls.md](dmls.md)) | `docs/topics/host-registration.md` |
| Permission-widening comparison for CI (`config::widening`: `compare_revisions`, `WideningInputs` from `.github/ci/permission-inputs.json`, the `permission_widening` example the trusted CI job builds from the base commit, whose `--render` mode composes `.github/ci/permission-report.md` into the pull-request comment; commits read through `sniff`'s `committed_tree_at`, never checked out; resolution reuses `load::committed_layer`/`typed_block`, so a loader change is a checker-input change) | `docs/topics/configuration.md` ("The Cloned-Repository Risk"), repo `docs/topics/ci-cd.md` ("Permission-change report") |
| Error/status block conventions | [errors.md](errors.md) |
| `md clean`/DMLS formatting rules and frontmatter repair | [cleanup.md](cleanup.md) |
| Document comparison | [comparison.md](comparison.md) |
| Module layout | [structure.md](structure.md) |
| Parser details | [pulldown-cmark.md](pulldown-cmark.md) |
| Frontmatter ecosystem comparison | [frontmatter-crates.md](frontmatter-crates.md) |

Load only the topic needed for the task. For the SimplifiedSchema language
itself (grammar, named types, conversion, `SimplifiedSchema`/`Prepared`
validation, layering), load the `simplified-schema` skill
([SKILL.md](../simplified-schema/SKILL.md)). For render-tree implementation work,
also load the `renderable` skill; for terminal components, load
`biscuit-terminal`; for file-reference or JSON/YAML/TOML conversion work, load
`biscuit-file`.

## Composition authority

`ComposeRequest` is the request authority: `ComposeOptions` (remote
configuration, cache root and policy, compose identity, baseline/meta-schema
controls, rendering options) plus the one required `FileResolutionContext`
built from a `RequestSnapshot`. A source-derived file reference must resolve
against a context derived from that one. User-facing docs:
`darkmatter/docs/topics/compose-requests.md` (API, builder, Mermaid),
`topics/file-referencing.md` (forms and tree rule), and
`errors/file-reference-failures.md` (the `failure:` row and data field), and
`errors/composition-problems.md` (stopping rules, authored positions).

The root compose pipeline is ordered: the passive call check (a proven-bad
call in any branch rejects the document before anything runs); frontmatter
interpolation pass 1;
schema validation and coercion; frontmatter shell expansion; frontmatter
interpolation pass 2; literal replacement; conditional page blocks and
`::map`/`::filter` regions (one structural traversal, callbacks finish before
body stages); body interpolation; shell directives and blocks; link resolution; concurrent
transclusion; inline cleanup and optional reflow; root-only link
normalization. Keep this order stable. One interleaving: a body read-side call
(`remote::calls_read_side_function`) after the first command site is left in
`BodyProvenance::pending` and evaluated by the shell stages once no unrun
command precedes it (see compose.md "Reads after a command"). Whole-value `{{ ... }}` and `$(...)`
values are executable state: they must resolve or fail, never leak as literal
syntax. Demand-driven context capture must observe only referenced `ctx.*`
groups.

Every entry point that resolves file references takes a `ComposeRequest`
(`ComposeRequest::prepare(options, &RequestSnapshot)` or
`ComposeRequest::with_context`). The context is required, never `Option`;
only binaries call `RequestSnapshot::from_process()`, and magic roots enter
only through `RequestSnapshot::with_magic_root*`. `match()`, `find_files()`,
and `::file-links <glob>` are `GlobReference`s judged or listed in the
document's (or the caller's origin) context, never the process directory.
Read [requests.md](requests.md) before touching a request, a context view,
a validator's context, or a glob consumer.

`::map` and `::filter` share one executor (`compose/collection_blocks/`); read
the "Map and filter" section of [compose.md](compose.md) before touching it.

Read [compose.md](compose.md) before changing a stage, expression function,
context property, cache key, transclusion directive, or file-resolution path.

## Schema authority

Darkmatter uses the SimplifiedSchema language (the `simplified-schema` crate),
compiled to Draft 2020-12 JSON Schema.
`DarkmatterSchemas` owns baseline merging and validator caching. The standalone
classifier is `parse_standalone_schema_document`; it recognizes an informal
`$schema` document (optional `types`, `description`, `categories`), a `kind: schema`
document with `$schema` and/or `types`, or a `kind: yaml-schema` (a YAML
document kind, never a `$schema` target or import source: `SelectedByKind`),
and admits each against the library's compiled envelope definitions. The two
callable catalogs (`kind: expression-schema`, `kind: mutation-schema`;
`StandaloneSchemaEnvelope::is_callable_catalog`) are admitted strictly, are
never a `$schema` target, import source, or `schema_files.rs` resolution
(`NotSchemaReason::CallableCatalog`), and report callable failures as
`SchemaError::CallableCatalog { path, message, span, related }` (code
`dm.schema.callable_catalog`) from the one Darkmatter catalog reader that
registration, `md schema validate`, and DMLS share (`library-surfaces.md`). A `$schema` target must export
`$schema` (YAML) or be a JSON object declaring Draft 2020-12; anything else is
`NotASchemaDocument`/`UnsupportedJsonDialect`/`InvalidJsonSchema`, while a
`types`-only file still serves `Name@file` imports. A loader refusal of a
trigger file (`NotSchemaReason::TriggerEnvelope`) is reported as
`TriggerSchemaReferenced`/`TriggerPayloadCycle`
(`triggers::assemble::refused_trigger_file` returns the path and the
declaration site; pass the site on, `trigger_schema_referenced(path, site)`).
A whole `.yaml`/`.yml` document is validated by
`DarkmatterSchemas::validate_yaml_document` (claimed `kind` > root `$schema` >
none, no baseline or triggers) over a per-context `KindRegistry`; read
[schema.md](schema.md#whole-yaml-documents) first.
DMLS routes the four configuration paths (`.darkmatter.yaml`,
`.claudine/config.yaml` under the captured home or repo root, by
`PathIdentity`) before that selection: `dmls/src/config_contracts.rs`
resolves the scope type (`config::schema::resolve_host_config_type`; Claudine's
schema is `include_str!`'d, never the claudine crate) and
`DarkmatterSchemas::prepare_configuration_contract` makes a
`Route::Configuration` contract whose `kind`/`$schema` are data.
Ownership: the `simplified-schema` crate owns all six shipped definitions
(`simplified-schema/schemas/{schema,yaml-schema,schema-trigger,namespace-schema,expression-schema,mutation-schema}.yaml`),
envelope admission, triggers (`simplified_schema::triggers`), schema roots
(`roots`), `EmbeddedSchemaBundle` (`embedded`), and the YAML loader,
`KindRegistry`, selection, and kind checks (`yaml_documents`); edit those
there (load the `simplified-schema` skill). Darkmatter supplies its
`ShippedSources` and prepared shipped definitions (`schemas::kind_host()`,
`shipped_sources()`), adapts the selected schema into an `EffectiveSchema`
and `ValidationReport` (`YamlContract`, `YamlValidation`, `Route`), and
re-exports nothing it moved. A registry is built per call (or once per folder/worker and
passed with `with_kind_registry`); it never fails as a whole, and a
duplicate, reserved, or broken claim is a `KindSelection` error only for a
document whose `kind` it could affect. A YAML file is never Markdown: a
fenced `---`…`---` file is two documents and fails to load.

Important contracts:

- A missing key and `null` are the same value. `required` means only "the
  key is written" (or a default supplies it); null admission never cancels
  it, and a written `null` is then judged by the type. A `default(...)` fills
  a missing or `null` value, required or not, in the composed copy only:
  `data_copy` withdraws applied defaults so `md set` never writes one, and
  composition never synthesizes `null` keys for declared but unwritten
  properties. One shared presence calculation decides this for every
  consumer; see [schema.md](schema.md#composition-and-validation).
- `alphanumeric` options are type identity (canonical, never merged or
  widened); `shell` is a passive `darkmatter-shell` format, never parsed or
  run. See [schema.md](schema.md#core-types-unknown-null-alphanumeric-shell).
- Validation keeps source positions, origin information, typed problem codes,
  pending values, and file-reference diagnostics.
  `FileReferenceDiagnostic::resolution_failure()` gives the biscuit-file
  class; the diagnostic is re-derived in the validator's own request context.
- Eager `file(eager)` values may normalize only on successful composition;
  validation-only APIs remain read-only.
- `literal(value)` preserves YAML scalar typing and lowers to JSON Schema
  `const`; quoted and native YAML forms are observably different.
- `expression`, `yaml`, and `json` are passive content formats. They parse or
  coerce but never execute.
- One coercion engine (`simplified_schema::coercion`) converts frontmatter and binds function arguments; numbers enter only through `simplified_schema::numeric` as exact
  `f64`s. See [schema.md](schema.md#conversion-engine) and
  [expressions.md](expressions.md).
- The SimplifiedSchema language lives in the `simplified-schema` crate (no
  Markdown dependency): the AST and grammar (`simplified_schema::simplified`,
  `SchemaRoot` is the parsed `$schema` enum), resolution, conversion,
  validators, formats, `SchemaError`, `ValidationProblem`, the numeric model,
  `coercion`, `MappingOrders`, and the expression syntax. Import those from
  `simplified_schema` (every consumer crate depends on it directly);
  `darkmatter::markdown::schemas` no longer re-exports them. Darkmatter keeps
  only `markdown::{span, literal_token}` and the expression items its
  composition API names (`compose::expression::{Expr, Lexer, parse, …}`).
  Edit the language in `simplified-schema/lib/src/`. What stays in Darkmatter:
  `DarkmatterSchemas`/`EffectiveSchema`, the baseline, trigger layering into
  the effective schema, the YAML adapter (`schemas::yaml_documents`:
  `YamlContract`, `YamlValidation`, `kind_host`, `shipped_kind_registry`), the
  shipped catalog (`embedded::SHIPPED_SOURCES`, which `schemas::resolve` and
  the `schemas::triggers::{scan, scan_with_embedded, scan_with_source,
  prepare_trigger_bundle}` wrappers pass to the library so
  `Name@darkmatter:<stem>` resolves), pending classification
  (`coerce::nested_templates`), `ProblemPositions`, and completion over an
  `EffectiveSchema`. Work counters for moved code (including
  `TriggerDiscovery`, `PayloadMergeCheck`, `PayloadCompileCheck`,
  `YamlKindCheck`) are recorded by the library and folded into
  `darkmatter::work_counters::snapshot()`/`threads()`.
- The library's standalone object is `simplified_schema::SimplifiedSchema`
  (content and file factories, strict/soft validation, atomic layering, type
  queries, export; `simplified-schema/docs/topics/schema-object.md`).
  Darkmatter's own validation entry points do not route through it: the
  effective schema merges the already lowered baseline, trigger, and
  document layers with the same `merge_baseline` the object's `layer_under`
  uses (memoized by `Arc` identity), and
  `darkmatter/lib/tests/l1/effective_schema_layering.rs` pins the two
  equal.
  `compose::repository_scope_catalog` re-exports the library's projection,
  the same one its raw-path factories capture with: edit it there.
- `type-definition` and `schema` meta-types delegate to the same passive
  parsers used by authoring and DMLS.
- Trigger matching is schema-based and passive. A schema-trigger document is a
  kinded document and must retain its root `kind` declaration.
- Triggers and bare-name `$schema` lookup use the five schema roots of the
  **document's** context (`simplified_schema::roots::SchemaRoots::for_document`):
  `SCHEMAS_DIR` (the folder itself, from `ctx.env()`), package, package area,
  `base_dir()`, `~/schemas`. Never walk ancestors or read the process
  environment for them. Every bare-name consumer (`$schema`, `Name@file`,
  `example(file)`, trigger payloads) selects through
  `simplified_schema::resolve::select_schema_file`; never add another copy.
  Root lookup never depends on triggers: `DarkmatterSchemas` takes its roots
  from the installed trigger registry or, without one,
  `with_schema_root_lookup()`; `prepare_schemas`, `md clean`, and
  `md schema validate` install it for a repository document whenever
  triggers are off (the reference/graph commands, `--disable schema-triggers`).
  Whether to search the roots at all (trigger discovery and bare lookup) is
  decided from the **source document's** context
  (`options.source_file_resolution_context().repository_root()`, the context
  `DarkmatterSchemas` is built with), never from the enclosing request: a
  `~`-opened child or link target outside Git refuses bare names and gets no
  root triggers even under a repository parent, while one whose nearest
  `.git` (directory or file, so a checkout nested in the launch tree too) is
  a second Git checkout searches that checkout's roots (its context was
  derived there at admission, `SourceDerivation::OwnRepository`) and never
  the launch checkout's.
  DMLS keeps its own workspace-boundary rule. `$path` patterns are `GlobReference`s; see
  [schema.md](schema.md#schema-roots-and-path-triggers).
- The runtime baseline is compiled in with its imports (Content Policy's
  `EDITOR_SCHEMA` types `content_policy`) and is the default for compose,
  `md schema validate`, and DMLS; see [schema.md](schema.md#runtime-baseline).
  Its prepared form (catalog, baseline, `darkmatter:` named types, kind
  definitions, function and effects catalogs) loads from the checked-in
  `lib/generated/shipped.json` (`markdown::schemas::shipped`; each payload
  member is a section decoded on first read, so `md compose` decodes only
  the catalog and baseline; a refused artifact or section falls back per item
  to text; `Registry::check_all` forces every section). A host's trigger
  bundle can carry prepared payloads (`EmbeddedSchemaBundle::with_prepared`,
  `simplified_schema::triggers::prepared`): served after filename selection, only when the
  bundle's name and every file's text match, skipping the payload
  merge/compile checks (`Work::PayloadMergeCheck`/`PayloadCompileCheck`). After editing `darkmatter/schemas/`
  or the code that prepares it, run `just shipped-schemas` from the root
  (bump `shipped::PREPARATION_REVISION` for code-only changes). That runs
  `scripts/shipped_schemas.py`, which regenerates Simplified Schema's,
  Darkmatter's, and Claudine's artifacts together (Claudine's is prepared
  over this one, so it goes stale too) and replaces none unless all three
  generate and each generator accepts its candidate over the lower
  candidates published with it. Another package's file whose behavior changes a higher
  artifact's bytes (a source text, or code that parses, lowers, encodes, or
  digests it, such as `simplified-schema`'s `validate.rs` or biscuit-hash's
  xxHash) must be spelled in that owner's `shipped_artifact_is_fresh` and
  listed in its `source-inputs`, for Darkmatter and Claudine alike.

- Validators are cached by typed identity: an effective schema is keyed by
  the `LayeredIdentity` of its ordered layers (baseline, triggers, document,
  then any phase projection), built from digests the shipped artifacts store
  (`BaselineRecord`/`PayloadRecord::digest`), so the merged schema is never
  serialized for its key; untracked transforms take the canonical route. A
  new post-merge transformation must extend the identity or fall back to it,
  and a change to what a merge or projection produces bumps
  `MERGE_ALGORITHM_REVISION`/`PROJECTION_REVISION` in `simplified-schema`;
  see [schema.md](schema.md) ("Validator identities").

Read [schema.md](schema.md) for imports, recursive named types (`$defs`,
`TypeExpr::Ref`), unions, pattern dictionaries, suggestions, triggers, and
DMLS schema behavior.

The global catalog `darkmatter/schemas/darkmatter.yaml` declares every
global's shape; the built-in baseline is its resolved `fm`. A catalog acts
as globals only when registered explicitly; a document `$schema` naming one
just gets its entries as optional properties; see
[schema.md](schema.md#authored-global-catalog).

## Remote and cache safety

HTTP(S) composition covers `::file`, `::code`, and read-side expression
arguments where a remote runtime is present; rendered links are never fetched,
and frontmatter interpolation and `$()` fail loudly on a URL. Reads are
deny-all until `--allow host=HOST` or a configured `network: allow`; only transport artifacts (raw HTTP bodies) may
persist under a cache root; `Cache-Control` outranks every freshness flag;
host policy is checked before the cache is read. Read
[remote-cache.md](remote-cache.md) before changing any of this.

## Rendering authority

All public Markdown rendering folds through the render tree. Darkmatter builds
one context-aware `renderable::Document`, then performs one target fold.
Component policy is lowered during construction; do not add a post-fold HTML or
terminal decoration pass.

`DarkmatterPage` is only a viewport/page-frame assembler and never inspects or
mutates component content. Browser tests are headless (no window activation or
host input) and assert DOM, computed style, accessibility state, or geometry;
browser HTML and CSS come from the same lowered values, never string repair.

Read [rendering.md](rendering.md) before changing style precedence, code-block
themes, disclosure blocks, browser safety, or the render-tree fold.

## CLI orientation

The binary is `md`: `render`, `compose`, `clean`, `toc`, `delta`, `schema
validate|detect|about|triggers`, `hash`, `get|set|rm` (frontmatter), `edit`,
`code-block`, `graph`, `validate refs`, and `init` (configuration setup; see
[cli.md](cli.md#md-init)). Users' contract:
`docs/cli/output-and-width.md` and `docs/cli/data-contracts.md`.

Checks are relaxed or tightened only through three verbs: `--allow <id|all>`
(a recovery, or `host=HOST`), `--deny <id|all>` (make diagnostics blocking),
and `--disable <id>` (stop a feature; no `all`). `cli/src/args/policy.rs` is
the one registry of identifiers, commands, and help; on `md compose`,
`--deny` lowers to the library's `compose::policy::BlockingPolicy`. The legacy per-flag spellings are
usage errors (`main.rs::REMOVED_FLAGS`); see `docs/cli/policy.md`. The
pipeline's call precheck consults the policy per document, before its
expressions run (`MarkdownError::BlockedDiagnostics`; a child's is fatal,
never a tolerated transclusion failure); the pre-flight walk's warnings
(a skipped child's invalid trigger) are judged before any command is
approved: by `compose_preflight_approvals*` (`pipeline::stop_if_warnings_blocked`,
before policy files or the handler) and by `validate_pre_approved` before
membership, at the root and `as_markdown` gates; a runtime warning is checked by
`pipeline::stop_if_blocked` before each body stage, after transclusion
`when=` preparation, between `::shell` commands, and before frontmatter
shell expansion, and within a stage before each further expression
(`pipeline::blocking_guard` through the rewriters' `BeforeEval`, for body
and frontmatter interpolation and directive targets) or condition
(`pipeline::ReportWatch`, for `::block` and `when=`), judging only what is
new since the last check (undeclared reads only once state and schema are
final, through `unknown_identifiers::pending_warnings`; never in discovery
passes); a nested `as_markdown` child's stop is restored on the caller
(`NestedComposeSlot::take_blocked`), even when `||` absorbed the call;
the last stages' warnings are checked by the CLI before writing. A new coded warning adds its
code to `COMPOSE_DIAGNOSTIC_CODES`; a new recovery adds a registry entry
naming the codes it governs. The reference check (`compose.rs::check_references`, shared by the
direct and lifecycle routes) lets a `missing-*` allowance recover an issue
only when `ReferenceIssue::is_missing_target` (failure class `NoMatch`) holds;
an invalid, unreadable, or malformed link, image, or inclusion target always
refuses, whatever the kind's allowance.

Every other command leaf goes through `cli/src/output/command.rs::CommandRun`:
`CommandRun::begin` (target, `--width`, default width, `DataShape`), build a
`CommandValue` (`markdown::output::command`: the data struct, a `Document` for
the generic folds, optional terminal, Markdown, and HTML overrides), then
`run.finish(result)`; a leaf working on one document calls
`run.set_document` so a failure with no file of its own still names it. A handler returns `Completed::success` or
`Completed::failure` (a report that is the deliverable), or `Err` (no result),
which `finish` turns into a payload for a data output and leaves to `main` for a
formatted one. `DataShape` adds the run status: `Typed` always, `Object` only
when a run needs it, `Bare` as `{ outcome, diagnostics, value }` for a value whose
fields are the user's (`md get`, `md schema detect`). `md clean`'s own
`diagnostics` keep their meaning, so its run-level ones are `run_diagnostics`. A
document command calls `.exact_markdown()` and `commands/render.rs::document_value`,
keeping its text's bytes and page-rendering everything else. An override reads
only the `OutputOptions` it is given (`Terminal::from_output_options`), never the
host: it also serves `plain`. A component that paints regardless of its terminal
(`DeltaReport`, a status block) needs `strip_escape_codes` when
`terminal_profile.degrades_to_plain()`, and `output::stderr_text` on stderr.
A new leaf adds a row to `args/output.rs::COMMAND_LEAVES`, a schema file under
`cli/schemas/output/`, and conformance tests in `cli/tests/l1/output_commands.rs`
that spell the schema path inside each `#[test]`;
`every_leaf_has_a_schema_file_and_a_conformance_test` fails otherwise. A schema's
named types are closed, unlike its root object, so a type must list every field.
One Rust `Diagnostic` serializes into all 17 output schemas, so their
`diagnostic` and `data-location` types are identical
(`output_commands::every_schema_file_shares_one_diagnostic_type`); a new
diagnostic field goes into every file. `DocumentEnvelope.read_set` is always an
array; `ReportSink::reads_from(&DataRuntime)` reads the command's runtime when
the envelope is written, so every exit carries it without per-path pushes.
A schema root is an object: a scalar result is judged under `value`.

Link titles are JSON5 objects or plain text (`docs/topics/hyperlinking.md`); a
leftover `key=value` title is plain text with a warning until `md clean`
migrates it (`cleanup::link_titles`, see [cleanup.md](cleanup.md)). Image titles
keep their own syntax (`render/image_ref.rs`). Titles
(`dm.link.malformed_structured_title`, `dm.link.legacy_structured_title`) are
interpreted only by
`renderable::output::link_metadata::parse_link_title`, which reports a neutral
notice; `compose::policy::link_title_code` gives its code. `Link` stores the notice
(`title_notice`) and never prints. Composition reports them from
`compose::link_title_warnings`, keyed by the link's authored start: once before
anything runs (so a failed run keeps them and `--deny` stops first) and once
after the inline stages (a link a stage wrote). `md render`, which does not
compose, reads `Markdown::link_title_warnings` before cleanup and names the
same codes through `RENDER_DIAGNOSTIC_CODES`.

The reference report is `reference/validate.rs::ValidationReportView`, built in
`reference/report.rs`, implementing `CommandOutput`. `ReferenceValidationReport`
keeps `documents` (one entry per inclusion, depth-first, empty documents
included) so grouping by `(source, included_via)` and the document count never
infer scope from issues; neither `documents` nor `references` is serialized, and
issue JSON reports authored `line`/`column`, `section`, `inserted`, and
`included_via`, null where unknown (`ReferenceOrigin::line`/`span` stay prepared
coordinates, for graph walkers only; `origin.authored` is the projection).
`as_prose()` is the Markdown/HTML document;
`render_terminal` rebuilds the same structure with bare `file://` URIs
and `heading` as a link parameter. Compose and `md graph --validate` show errors
(compose prints nothing when there are none), `md validate refs` shows all
severities. Reasons and constructs live only in
`ReferenceIssueCode::reason_phrase` and `ReferenceSyntax::construct_phrase`.
HTML discovery (`reference/html.rs`) reads one Markdown HTML node as any number of
elements with `scan_elements`, and `LineMap` carries each back to the source past
line breaks and container prefixes; never classify an HTML node line by line.
`visit_html_elements` walks a document's HTML nodes in order, so a raw-text element
split across nodes is one element, and every reader drops what starts in its
`inert_ranges`; never decide inertness in one consumer. See
`docs/cli/validate.md#html-and-css-references`.

`md compose` accepts its Markdown default and the data family; `md render` both
families, defaulting to `terminal`, which is byte-identical to `plain` off a
color TTY. A data output of either prints one envelope (`file`, `outcome`,
`document`, `diagnostics`, `summary`, optional `perf`, `lifecycle_output`;
`markdown::output::command::DocumentEnvelope`; schemas in
`cli/schemas/output/`) for every parsed invocation, and stderr stays empty: no
`-v`, tracing, prompt, or lifecycle line reaches it. Lifecycle output there is
collected per step; in Markdown mode all of it, `stdout:` included, goes to
stderr, because stdout holds only the deliverable. Exit 0 succeeded, 1 ran and
failed (including a refused reference), 2 usage only; handlers return
`ReportedExit`, and only `main` exits. `cli/src/output/mod.rs` resolves target
and width (`OutputEnvironment`; a test passes one to spoof a stream) and owns
stderr (`stderr_terminal`, `stderr_report`: its own TTY and columns, unlimited
off a terminal, whatever `--width` says). `cli/src/artifact.rs::ProcessOpener`
is the `--show` seam: `MD_DRY_RUN` never launches, and `MD_DRY_RUN=fail` is a
host with no default app. The CLI only collects
(`cli/src/output/report.rs::ReportSink`); every code and record comes from
`compose::report_diagnostics` (`warning_diagnostic`, `error_diagnostics`,
`lifecycle_failure_diagnostics`), and a CLI-worded shell failure keeps its
typed diagnostics through `ClassifiedFailure`. `Markdown::compose_with_warnings`
returns a failure with the warnings reported before it (`PipelineRuntime`'s
failed-document stash); a failed pre-flight walk leaves its failed document's
warnings for `ComposeOptions::take_preflight_failure_warnings` (with the
triggers of the siblings walked before it, each trigger once), as does any
policy error `compose_preflight_approvals*` raises after a successful walk;
a successful one returns them in `ComposePreflightApprovals::warnings`; the
in-pipeline pre-approval gate (`validate_pre_approved`) adds its walk's
triggers to the composition report; and a
lifecycle call rejection carries its earlier warnings (`earlier_warnings`);
a failing document keeps the triggers its `as_markdown` children and gate
found (`NestedCompose::finish` → `PipelineRuntime::record_failed_triggers`).
`md compose`'s reference check is passive: its graph preparation runs no
`::shell` (`prepare_content` never re-enables an operation the request
disabled), and `ComposeOptions::for_reference_check` defers what only
composition can judge (values waiting on a frontmatter command, an expression
it cannot finish, a failing `when=`, an inclusion whose parser fails, an
included file that exists but cannot be loaded: composition rejects or
tolerates them). Every other graph build (`md validate refs`, `md graph`, the
library's graph, local query, and validation APIs) fails with composition's
typed error on an inclusion parse error (`reference/graph.rs::parsed_inclusions`),
an unloadable included file (`ReferenceAnalysisRuntime::load_inclusion`; a
`::toc-linking` target's headings are read raw, as composition reads them, and a selected file that cannot be read is `transclusion::read_failure_at` at the directive's `InclusionSite` in both, as a child load is),
and a failing `when=` (evaluated through `conditions::ConditionSite`, as the
transclusion stage does), and on a target its directive's admission rule
refuses: `transclusion::admit_local_target`/`admit_url_target`, the
resolver's own passive checks, applied to a `::file`/`prologue`/`epilogue`
child before it is opened (`graph.rs::admits_markdown_child`) and to `::code`
and `::url` records in `validate.rs` before the kind branches and the remote
batch (a `::url` record is always `RemoteUrl`; the local query is a lookup
and admits nothing); never let a reference reader turn a failure into an
absent inclusion. A `path#fragment` link into a file that cannot be loaded is
the `UnreadableTarget` issue, and a link or image target that is no valid
file reference (`[x](@)`) the `InvalidReference` issue at the link, never a
`ReferenceError`. A check that still fails (`check_references` returns
`Result`) is a composition failure with status 1, reported through
`error_diagnostics`; its graph build leaves its warnings for
`take_preflight_failure_warnings`. A `Diagnostic`'s `source.line`/`column` are
`Option` (null when no authored position is proven, never a placeholder), a
record that knows only its file keeps the file, a typed error with no file takes
the composed document (`report_diagnostics::with_document_fallback`, applied by
`ReportSink`), a warning's file is stamped from `ComposeReport::document`, and
only `ComposeWarning::location` (never `line_number`, a current-body count) is
published; `source.inserted` marks a construct that wrote the text; only a reference
record has `reference_details`, flattened into `reason`, `construct`,
`reference`, `failure`, and `included_via`. Skipped-trigger
warnings found by a pass that runs ahead of the delivering report (reference
validation via `ReferenceValidationReport::trigger_warnings`, which prepares
each transcluded child in its own context; the lifecycle call check via
`effective_schema_for`; the lifecycle bootstrap; the `--shell` walk via
`ComposePreflightReport::warnings`; the shell-approval walk via
`ComposePreflightApprovals::warnings`) are held in
`compose.rs::PendingTriggerWarnings`: merged once into the first report or
failure, or delivered by any exit that never composes.
The call precheck orders findings by authored position and keeps the prefix
before a rejection. Never print to stdout or stderr on a data-output path.

Every source-file argument opens through `cli/src/io::open_argument`:
`FileReference` grammar first, resolved in the launch context, with the
document context derived from the opening reference. Read [cli.md](cli.md)
before adding a route that reads a file argument or changing CLI style
layers.

## Testing and verification

Use the package-area recipes in `darkmatter/`: `just build`, `just test`,
`just lint`.

Use `just test-l2` only for real-terminal behavior and `just test-browser` for
headless browser behavior. Parser, schema, prompt, template, or configuration
changes require both a passive shipped-artifact corpus test and an end-to-end
test through the normal invocation path. Persisted values require a repeated
read/write/read round trip.

Expression grammar, inserted-text, failure-policy, undeclared-property,
undeclared-member, host-binding, and
warning-identity rules are in [expressions.md](expressions.md); read it
before changing the lexer, an evaluation surface, or a warning family.
Fixture rules (`CliProcessFixture`, `git init` for repositories), the
source-scan guards (context construction, glob implementation, spawn sites,
semantic results), and the entry-point parity matrix (including the glob
rows) are in [testing.md](testing.md); read it before writing an `md`
integration test, adding a file-resolving entry point, or touching a guard's
allowlist.

Do not run workspace-wide Cargo gates for a Darkmatter-only change; when a
public type or behavior changes, find consumers (Claudine) with Sniff/GitNexus.
