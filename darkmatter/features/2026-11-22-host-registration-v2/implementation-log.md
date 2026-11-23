---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/spec.md"
plan: "darkmatter/features/2026-11-22-host-registration-v2/plan.md"
implemented-by: "claude/opus"
started-phase: "1"
packages:
    - simplified-schema
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - simplified-schema/schemas/namespace-schema.yaml
    - simplified-schema/schemas/schema-trigger.yaml
    - simplified-schema/schemas/schema.yaml
    - simplified-schema/lib/generated/shipped.json
    - simplified-schema/lib/src/envelope.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/bundle.rs
    - simplified-schema/lib/src/resolve/buffers.rs
    - simplified-schema/lib/src/resolve/select.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/shipped/registry.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/triggers/assemble.rs
    - simplified-schema/lib/src/triggers/discovery.rs
    - simplified-schema/lib/src/triggers/envelope.rs
    - simplified-schema/lib/src/triggers/finding.rs
    - simplified-schema/lib/src/triggers/mod.rs
    - simplified-schema/lib/src/triggers/prepared.rs
    - simplified-schema/lib/src/yaml_documents/mod.rs
    - simplified-schema/lib/src/yaml_documents/registry.rs
    - simplified-schema/lib/src/yaml_documents/select.rs
    - simplified-schema/lib/tests/l1/bundle_resolution.rs
    - simplified-schema/lib/tests/l1/kind_envelopes.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/shipped_artifacts.rs
    - simplified-schema/lib/tests/l1/yaml_kind_selection.rs
    - darkmatter/schemas/darkmatter-host.yaml
    - darkmatter/lib/generated/shipped.json
    - darkmatter/lib/src/config/registration.rs
    - darkmatter/lib/src/config/registration/bundle.rs
    - darkmatter/lib/src/config/registration/manifest.rs
    - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
    - darkmatter/lib/src/markdown/output/command/data.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/host_registration_snapshot.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/cli/schemas/output/schema-triggers.yaml
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
    - darkmatter/dmls/src/providers/frontmatter/schema_value_hover.rs
    - claudine/lib/generated/shipped.json
    - claudine/lib/src/composition/schema/shipped.rs
    - claudine/lib/tests/l1/invalid_trigger_findings.rs
    - claudine/cli/tests/l1/invalid_trigger_warnings.rs
docs_updated_during_phase_1:
    - simplified-schema/docs/topics/language/schema-documents.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/selecting-schema-files.md
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/dmls/docs/diagnostics.md
    - claudine/docs/topics/schemas.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_2:
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/schemas/globals.lifecycle.json
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/host_publication.rs
    - simplified-schema/lib/src/resolve.rs
    - darkmatter/lib/src/config/load.rs
    - darkmatter/lib/src/config/registration.rs
    - darkmatter/lib/src/config/registration/manifest.rs
    - darkmatter/lib/src/config/registration/namespaces.rs
    - darkmatter/lib/src/config/registration/selection.rs
    - darkmatter/lib/src/lifecycle/host_profile.rs
    - darkmatter/lib/src/lifecycle/host_profile/tests.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/tests/host_fixtures/mod.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/host_namespaces.rs
    - darkmatter/lib/tests/l1/host_registration.rs
    - darkmatter/lib/tests/l1/host_selection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/cli/tests/l1/host_registration.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/benches/pipeline_work_a3.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/hosts.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/lifecycle_host_tests.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
    - darkmatter/dmls/tests/l1/claudine_host.rs
    - darkmatter/dmls/tests/l1/edit_action_editor.rs
    - darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/host_globals.rs
    - darkmatter/dmls/tests/l1/host_registration_watches.rs
    - darkmatter/dmls/tests/l1/lifecycle_event_views.rs
    - darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registered_host_editor_agreement.rs
    - darkmatter/dmls/tests/l1/registered_host_views.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/docs/topics/lifecycle/host-artifact.md
    - darkmatter/docs/topics/host-functions.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/schema.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_3:
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/triggers/assemble.rs
    - simplified-schema/lib/src/triggers/finding.rs
    - simplified-schema/lib/src/triggers/prepared.rs
    - darkmatter/lib/src/config/registration.rs
    - darkmatter/lib/src/config/registration/merge.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/tests/l1/host_trigger_merge.rs
    - darkmatter/lib/tests/l1/registered_yaml_kinds.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/cli/schemas/output/schema-validate.yaml
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/host_registration.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/yaml_contract.rs
    - darkmatter/dmls/src/overlay/yaml_document.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registered_trigger_conflict.rs
    - darkmatter/dmls/tests/l1/registered_yaml_kinds.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
decisions-for-review:
    - |-
        Format 1 manifests still load through the old profile loader during this phase. The plan's Rule 5 says format 1 is refused with an upgrade warning. That warning is implemented in the new format 2 reader (`bundle::load_snapshot`), but `load_registrations` still loads format 1 into host profiles, because host profiles are not built from a format 2 snapshot until Phase 2, and dozens of existing tests use format 1 fixtures that Phase 2 converts. The old format 1 contract text now lives privately in `manifest.rs`; the shipped `darkmatter-host.yaml` is format 2. Alternative: refuse format 1 everywhere now, which would break every registered-host test until Phase 2. Redo cost if reversed: none beyond what Phase 2 already removes.
    - |-
        The fixture bootstrap task was not applied as written. The plan says to point three tests' includes of the deleted `claudine/schemas/sequence-types.yaml` at the benchmark copy. In this working tree that file exists again (untracked, from in-progress schema edits in the same tree), and the benchmark copy is a different, older shape that imports `loose-body` from a different `lifecycle.yaml`, so repointing would break those tests. The includes were left on the live file. If the live file is not kept, Phase 4's move to the v2 inventory replaces these includes anyway; until then the three tests need either that file or a matched set of benchmark copies.
    - |-
        Member paths in a format 2 manifest allow only the characters `A-Z a-z 0-9 . _ -` in each segment (a leading dot is allowed). The plan asks for "portable ASCII filename segments" without naming the set. A wider set (spaces, `+`, `~`) would be portable on most systems but invites quoting and shell trouble. Widening it is a one-function change in `manifest.rs`.
    - |-
        The snapshot refusal is returned boxed (`Box<SnapshotRefusal>`) to satisfy clippy's large-error lint, rather than boxing only its dependency lists. Either is a small edit.
    - |-
        A manifest is treated as format 1 (and gets the upgrade warning) when `format_version` is the number 1, or when it carries `identify`, `triggers`, or `globals` with any version other than 2. With `format_version: 2`, a stray `identify` is an ordinary unknown-field error. The alternative is to call any legacy field an upgrade case; that would hide genuine authoring mistakes in new manifests.
    - |-
        Explicit `null` for the optional envelope fields (`name`, `description`, `types` on triggers; optional namespace-schema fields) is refused by the envelope parsers, not by the shipped kind definitions, because the schema language admits null for any optional property. Whole-YAML validation of such a file (for example in the editor) therefore does not flag the null yet; the docs mark that as planned. The alternative is a new schema-language option that forbids null on an optional key, which is a larger language change.
    - |-
        Duplicate trigger/library `name` labels inside one schema root reuse the existing trigger-match error type located at `name`, so a types-only library that takes part in a duplicate is worded as a "schema trigger" finding. A file shadowed by a same-named file in a higher-priority root still counts toward its own root's names. A dedicated error variant would mean touching every exhaustive match of that error.
    - |-
        Registered YAML kinds: two different registered definitions claiming one kind reuse the existing duplicate cause, listing files in host-then-path order, so the message names files but not hosts. A registered selection reuses the existing "selected by kind" result; the host is read from the definition's source. Copies collapse when the caller-supplied sameness key (meant to be host revision plus path) is equal. A registered definition whose claim fails is refused when it is built and never indexed, so it cannot block a kind; Phase 2 is expected to refuse the whole registration in that case. Adding a host-carrying cause or failure entries is small to moderate work.
    - |-
        The bundle resolver is a thread-local scope consulted inside the existing file-selection and read functions (the same pattern as unsaved editor buffers), not a parameter threaded through every resolver call. Embedded bundles use a virtual root spelled `<name>`. Because the envelope for a file-backed example requires `kind: example` and a bundle refuses `kind: example` members, no `example(file)` can succeed inside a bundle in v2; the scope also governs any host read made through the shared read function while it is in force, so a host must evaluate template slot data outside the scope. Threading an explicit context instead would touch every resolver signature.
    - |-
        Claudine still publishes format 1 manifests until Phase 4, and the loader now refuses format 1 with the upgrade warning. Claudine's registration tests that load its published manifest through Darkmatter (installed registration, identification, refresh, publication round trips) are therefore expected to fail between Phase 2 and Phase 4. The alternative was to keep a format 1 loader alive until the cutover, which the plan removes in this phase. Redo cost: none; Phase 4 publishes format 2.
    - |-
        Claudine's committed `globals.lifecycle.json` is regenerated with a transitional `catalog_hash`: `namespace_catalog_hash({"globals.yaml": member_digest(HOST_CATALOG_TEXT)})`, a one-entry map over its existing single catalog, so the artifact is format 2 and its freshness test still passes. Phase 4 replaces this with the hash of Claudine's real namespace files. Alternative: leave Claudine's artifact at format 1 until Phase 4, which would fail Claudine's freshness test now. Redo cost: one line in `claudine/lib/src/composition/lifecycle/mod.rs`.
    - |-
        `triggers()` and `yaml_kinds()` are in bundle-path order (the order of `files` has no effect, matching the revision rule). Matched triggers layer in that order, so a host that relied on manifest order now gets path order. DMLS's A3 completion golden changed accordingly (`group` before `seq`). Alternative: keep authored order, which would make two installs with reordered `files` layer differently despite an equal revision. Redo cost: small (sort site in `prepare_members`).
    - |-
        The lifecycle binding check is `host_globals` ⊆ the namespace catalog when `global_catalog` is true. Event views' declaration roots are not separately checked against the catalog, because they are already validated by the registry rebuild (a reserved or unknown root is refused there). Alternative: also cross-check every event view root. Redo cost: small, in `host_profile.rs::read`.
    - |-
        An empty namespace (`$schema: {}`) declares a global with no members and is valid, as the input robustness matrix allows. Alternative: refuse an empty payload. Redo cost: one check in `namespaces.rs::prepare_one`.
    - |-
        When two manifests of one host conflict, both claims' YAML definitions stay in `RegisteredHosts::yaml_kinds()`, so a conflicting kind is reported by the shared YAML selection as a duplicate claim rather than silently disappearing. Markdown selection still refuses a conflicted host. Alternative: drop a conflicting host's kinds entirely. Redo cost: small, in `selection.rs`.
    - |-
        `RegisteredHosts::yaml_kinds()` is built and tested, but `md schema validate` and DMLS do not pass it to `select_with_registered` yet; that wiring is Phase 3's YAML tier. The docs mark it planned.
    - |-
        Two DMLS diagnostic codes were removed with legacy discovery: `dm.lifecycle.incomplete_analysis` and `dm.schema.host_catalog`. Both described schema-root host catalogs, which no longer exist; a registered host's unusable artifact or function catalog refuses the registration with `dm.host.registration` instead. Alternative: keep the codes for the refused-registration case, which would duplicate `dm.host.registration`. Redo cost: moderate (re-add emitters and tests).
    - |-
        A trigger or library `name` that fails the label pattern is refused by the envelope parser without a line (`line: None`); the test asserts the file, member, and `/name` in the message. Locating it needs the envelope parser to return the key span. Redo cost: small to moderate in SimplifiedSchema's envelope parsing.
    - |-
        `md`'s run-level registration warnings show the message on stderr without the file and line (they are present in `--output json`). Changing `output::report_run_notices` affects every run-level notice, including configuration ones, so it was left as is.
    - |-
        Root unions are admitted only in registered trigger payloads (`resolve_registered_trigger_payload`, `PayloadForm::ObjectOrUnion`); embedded and schema-root trigger payloads still refuse a root union. A union arm must be inline (a `Name@file` arm is refused). Phase 4 needs Claudine's inline-composition union payload to take the registered path. Alternative: admit unions for every trigger, which changes the root-trigger merge contract. Redo cost: small (the gate is one `form` argument).
    - |-
        A layer stack with a root-union host layer is merged by `merge_around_union`: the baseline and lower layers merge, the higher layers and document merge on top of them, and then the union, with the keys the higher layers declare removed from its arms, is merged last. A document `$schema` that is itself a root union over a union host layer is a `SchemaError::Baseline`. A higher layer's `closed` root is not carried over the union, because the union arms own the root. Alternative: lower the union to per-arm full stacks, which multiplies validator size. Redo cost: moderate.
    - |-
        Origins and default-file attribution now cover root-union schemas: each key any arm declares is attributed to the highest layer declaring it in any arm (`declares_property`, `top_level_keys`). This replaces the old rule that a root-union schema has no origins, and one existing unit test changed from asserting empty origins to asserting per-key origins (`root_union_document_schema_merges_trigger_per_arm_with_arm_origins`). This is needed so a registered union layer's defaults and related locations work. Alternative: attribute only for host unions. Redo cost: small.
    - |-
        A registered template payload is resolved whole (`without_properties(&[])`), with no discard of properties a higher layer redeclares, and its checked layer is not memoized; `pending_templates_for` lists it whole. That makes a conflict visible even under an override, at the cost of resolving slots a higher layer would discard. Alternative: discard first, as root triggers do, which would let an authored override hide a conflict. Redo cost: small.
    - |-
        `RegisteredHostProfile::checked_layer` memoizes static results (successes and conflicts) per sorted matched set with no bound. The number of distinct matched sets is at most the power set of a host's triggers and in practice a handful. Alternative: a bounded LRU. Redo cost: small.
    - |-
        Examples are compared as part of the constraint set (reason: "their constraints differ"), not with a separate message. A registered bundle cannot carry a `kind: example` file (refused as a listed member, unreadable unlisted), so no registered payload can declare an `example(...)` today and the case has no L1 test. Redo cost: small if bundles later admit example files.
    - |-
        `SchemaError::HostTriggerConflict` is boxed (`HostTriggerConflict(Box<simplified_schema::HostTriggerConflict>)`) to keep `SchemaError` under clippy's large-error threshold, following `UnresolvedSlots`.
    - |-
        `md schema validate --output json` gives a conflict's `schema` record `code: dm.host.trigger_conflict` and a new `related` array (the two trigger files); the output schema gained `related`. DMLS relates each trigger file at a zero range (the top of the file), because the property's span in the trigger is not carried through the checked layer. Locating the declarations needs the payload's source map in `merge::check`. Redo cost: moderate.
    - |-
        A whole YAML document now waits for its configuration's host registrations in DMLS (`registry_demand`, `prepare_need`), and the contract key folds in the installed registrations' version. Their install republishes the document's current version, so an editor can see one extra publication of the same version; `lsp_session::meta_schema_phase6_shipped_schema_activation_and_current_error` now waits for the version 2 publication instead of the next one. Alternative: suppress a republish whose diagnostics are unchanged. Redo cost: small.
message-to-agent: |-
    Phase 3 landed Rule 8 and Rule 9. `config::registration::merge::check` turns a host's matched payloads into one `CheckedLayer` (or a `TriggerConflict`, boxed as `SchemaError::HostTriggerConflict`, code `dm.host.trigger_conflict`); `RegisteredHostProfile::checked_layer(&matched)` memoizes static results, and `markdown/schemas/mod.rs::registered_trigger_layers` uses it (template payloads resolve whole, unmemoized). Registered payloads are loaded by `simplified_schema::triggers::assemble::resolve_registered_trigger_payload` with `PayloadForm::ObjectOrUnion`, so one root union is allowed and merged by `merged_trigger_schema`'s `merge_around_union`; embedded and root triggers still refuse unions. For Phase 4: Claudine's inline-composition union payload must ship as a registered trigger (inline arms only, no `Name@file` arm) and must not share alternative-defining keys (`prompt`, `sections`, ...) with Claudine's other triggers. Registered `yaml-schema` kinds are the lowest tier of `prepare_yaml_contract` in the CLI and DMLS (`YamlContract::registered()`). Claudine's format 1 round-trip tests (`host_publication`, `installed_registration`, `host_registration_refresh`) still fail with the format 1 upgrade warning until Phase 4 publishes format 2. Tests: `darkmatter/lib/tests/l1/host_trigger_merge.rs`, `registered_yaml_kinds.rs`, DMLS `registered_trigger_conflict.rs` and `registered_yaml_kinds.rs`, and two CLI tests at the end of `darkmatter/cli/tests/l1/host_registration.rs`.
---

# Implementation Log for 2026-11-22-host-registration-v2 (6 phases)

## Phase 1

- Started 2026-11-22. The worktree already carried uncommitted edits from the
  `2026-11-22-object-composition` review cycle (SimplifiedSchema, DMLS, docs,
  and a restructuring of `claudine/schemas/` that deleted
  `lifecycle-events.yaml` and added an untracked `sequence-types.yaml`)
  before this phase began; they are not part of this phase.
- Work ran as four subagents (kind envelopes, manifest v2 + verified
  snapshot, YAML registered kinds in parallel; then the bundle resolver),
  each verifying only its own tests. The orchestrator regenerated the shipped
  artifacts once (`just shipped-schemas` from the root, all three
  artifacts) after the first three finished, then ran the gates.

### Fixture bootstrap

- `claudine/schemas/sequence-types.yaml` is present in this tree, so the
  includes in `shipped_corpus.rs`, `claudine_schema_bundle.rs`, and
  `claudine_schema_discovery.rs` compile as they are. Not repointed (see
  `decisions-for-review`).

### Kind envelopes (SimplifiedSchema)

- `schema-trigger.yaml` admits optional `name`, `description`, `types`;
  `schema.yaml` split into two closed arms with `name` only in the
  types-only arm; new `namespace-schema.yaml` registered as the fourth
  envelope definition with a passive parser
  (`parse_namespace_schema_document`). A namespace file is refused as a
  `$schema` target or import source (`NotSchemaReason::NamespaceCatalog`).
- Trigger labels, descriptions, and local types are carried by
  `TriggerEnvelope` and the prepared bundle record; a trigger's own payload
  sees its local types; other files cannot import them.
- Per-root (and per-embedded-bundle) duplicate `name` detection reports and
  excludes every participant; diagnostics show `name (path)`.
- `md schema triggers` rows and their output schema carry `name`.
- `PREPARATION_REVISION` bumped in SimplifiedSchema (3→4) and Claudine (4→5).
- Existing fixtures that used `name:`/`description:` as examples of unknown
  trigger keys now use `title:`/`notes:`.
- Tests: `simplified-schema/lib/tests/l1/kind_envelopes.rs` (15 tests: field
  shapes for each new field, namespace envelope shapes, namespace refused as
  target/import, trigger-local types private, duplicate names in one root /
  two roots / shadowed / embedded bundle, `$extends` of a library type keeps
  its declaring file).

### Manifest envelope and verified snapshot (Darkmatter)

- `darkmatter/schemas/darkmatter-host.yaml` is the closed format 2 contract.
  `manifest::read_v2` detects format 1 first and returns a located
  upgrade warning; member path rules (prefix, slashes, portable segments,
  reserved device names, trailing dots, escape, alias, case collision,
  symlink/junction components, remote); digest keys must equal the authored
  members exactly.
- `config/registration/bundle.rs`: `load_snapshot` / `load_snapshot_with`
  (reader seam) read raw bytes once, verify BLAKE3 digests, re-read the
  manifest once on a missing member or mismatch and retry only when its
  bytes changed, never mixing two releases. Revision is a `blake3_hash_json`
  of the normalized meaning (sorted files, sorted function pairs, normalized
  digest map); the raw manifest hash is kept separately. Dependencies and
  candidates from both attempts are kept on success and refusal.
- Tests: `darkmatter/lib/tests/l1/host_registration_snapshot.rs` (30 tests
  through the public loader: control, reorder and copy keep the revision,
  format 1 warning location, envelope field shapes, digest-key mismatch both
  ways, digest text shape, every path rule including in-root `./a/../b`,
  junction/symlink component, remote member, unchanged-manifest refusal,
  changed-manifest retry, interleaved releases never combined, repeated load
  equality).

### YAML claims (SimplifiedSchema)

- `RegisteredSource`, `RegisteredDefinition`, `RegisteredKinds`, and
  `select_with_registered` in `yaml_documents`; `select` is unchanged and
  delegates with no registered tier. Claimed-kind extraction is one shared
  helper for shipped, local, and registered definitions.
- Tests: `simplified-schema/lib/tests/l1/yaml_kind_selection.rs` (12 tests:
  control, shipped and local precedence, local duplicate / incomplete / root
  failure not shadowed, competing registered claims sorted by host then
  path, equal copies collapse, no/non-string kind, kind over root
  `$schema`, unchanged `select`, a definition claiming no kind).

### Bundle resolver (SimplifiedSchema)

- `simplified_schema::resolve::bundle` (`BundleSources::{on_disk, embedded}`,
  `with`, `BundleReads`, `BundleBoundary`, `SchemaError::BundleBoundary`,
  `SelectionFailure::Bundle`). Imports, payload targets, `$extends`/spread
  operands, examples, and template-returned imports are decided against the
  verified table before any probe or open; unlisted, escaping, remote, bare
  name, and root-search references are refused; built-in shipped names stay
  allowed. DMLS's exhaustive hover match gained an arm.
- Tests: `simplified-schema/lib/tests/l1/bundle_resolution.rs` (13 tests:
  table wins over disk, nested `$extends`, payload indirection, unlisted not
  read, escape, remote, bare/`~`/absolute refused, built-in allowed, example
  cases, embedded root, thread carry, malformed bundle input, template
  import checked by both gates).

### Requirement-to-test mapping

| Rule / task | Tests |
|---|---|
| Rule 1 envelopes, namespace kind | `kind_envelopes::*` (field shapes, namespace shapes, target/import refusal) |
| Rule 2 labels, local types, per-root uniqueness, CLI name | `kind_envelopes::files::*`, `darkmatter-cli` output conformance tests |
| Rule 3 closed bundle, path rules | `host_registration_snapshot::*` path and envelope cells |
| Rule 4 verified-byte resolution | `bundle_resolution::*` |
| Rule 5 hash once, retry, revision, format 1 warning | `host_registration_snapshot::*` retry/release/revision/format cells |
| Rule 9 YAML lookup tier | `yaml_kind_selection::*` |

### Gates

- `just shipped-schemas` (root): regenerated all three artifacts.
- `simplified-schema`: `just test` — 2125 + minimal-feature run passed;
  `🐢 L1 run recorded: 2525 test(s), none at or above 5.0s`. `just lint` passed.
- `darkmatter`: `just test` — 17584 passed, 10 skipped;
  `🐢 L1 run recorded: 17584 test(s), none at or above 5.0s`. `just lint`
  passed (including the zed-dmls wasm check).
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing
  stranded.
- Dependents: `cargo nextest run -p claudine -p claudine-cli -E
  'test(/trigger|schema|shipped|envelope|registration|sequence|compose/)'` —
  3789 passed.
- `just publish-skills` in `darkmatter/` (the `use-dmls` caller skill
  changed): published `use-darkmatter` and `use-dmls`.

### Cross-OS check

- `just cross-check darkmatter --os windows`: pass.
- `just cross-check simplified-schema --os windows`: the first run failed
  `kind_envelopes::files::a_duplicate_name_in_one_root_marks_and_excludes_both_files`.
  The test fixture joined a forward-slash relative path onto the root, so the
  expected path displayed with mixed separators, while discovery reports
  native `\` separators. Fixed the fixture (`Repo::write` joins per
  segment); the second run passed. `just lint` in `simplified-schema/`
  passed again after the fix.

### Handoff

- Phase 2 builds host profiles from `bundle::load_snapshot`, feeds
  `snapshot.bundle_root()` / `snapshot.bytes()` into
  `simplified_schema::resolve::bundle::BundleSources::on_disk` +
  `resolve::bundle::with`, indexes registered YAML definitions with
  `yaml_documents::{RegisteredDefinition, RegisteredKinds}`, and removes the
  interim format 1 loader (`manifest::read` and its private legacy contract).

## Phase 2

- Started 2026-11-22. The orchestrator wrote the core (catalog preparation,
  member classification, profile loading, selection, removal of legacy
  discovery, the v2 fixture constructors, and the docs); three subagents then
  migrated the library tests and benches, DMLS, and the `md` CLI tests, each
  verifying only what it changed. Gates ran once, after all three finished.

### Catalog preparation

- `config/registration/namespaces.rs` (new): `prepare_catalog` prepares
  every `namespace-schema` member from verified bytes inside
  `simplified_schema::resolve::bundle::with`, returning the `GlobalCatalog`,
  each namespace's location, and the exact `{bundle path → digest}` map of
  the namespace files and every bundled file their resolution read
  (`BundleReads::served`); `namespace_catalog_hash` is the lifecycle
  `catalog_hash`. Names are checked with the expression-root and
  Darkmatter-global rules (`check_root`, `null`); duplicates are refused on
  every declaration. `simplified_schema::resolve::resolve_namespace_payload`
  resolves a namespace's `$schema` with its private `types`.
- Lifecycle artifact format 2: `lifecycle_artifact(options, effects,
  catalog_hash)`, `HostLifecycleProfile::from_artifact_at(catalog, hash,
  path, artifact)`; format 1 is refused; `global_catalog: true` requires
  `host_globals` ⊆ the catalog. Claudine's committed artifact is regenerated
  with a transitional one-entry hash (see `decisions-for-review`).
- `darkmatter/lib/tests/host_fixtures/mod.rs` rewritten for v2
  (`Bundle::{acme, acme_named, relay}`, `with_file`, `without_file`,
  `Fixture::{install_bundle, write_member, corrupt_member, readdress,
  edit_manifest, root_catalog}`), with one function catalog and facts pair.
  DMLS builds a Claudine-shaped bundle from frozen fixtures in
  `dmls/tests/l1/claudine_host.rs`.

### Host selection

- `load_manifest` classifies each `files` member by `kind` (trigger,
  `yaml-schema`, `namespace-schema`, types-only library), refuses any other
  kind, duplicate labels, and imports outside the bundle; every trigger must
  pass `admit_identifying_trigger`. Triggers and YAML kinds are in
  bundle-path order. Refused entries leave `LoadedRegistrations::watches`
  (manifest, every member, unlisted import targets).
- `select` returns `Selected{selection, matched}`; `RegisteredHosts` indexes
  YAML claims (`yaml_kinds()`). `DarkmatterSchemas::registered_matches`,
  composition host capture, and DMLS `HostView` read the matched indices.
- Removed: `HostSelection::Legacy`, `LegacyHost`, schema-root host catalog,
  lifecycle, and function discovery (`host_catalogs`,
  `host_lifecycle_profiles`, `host_function_catalogs`, `from_host_text`),
  `lifecycle_artifact_path`, `from_artifact`, the format 1 manifest
  contract, `manifest_member_candidates`, `catalog/discovery_tests.rs`, and
  the DMLS codes `dm.lifecycle.incomplete_analysis` and
  `dm.schema.host_catalog`.
- Converted: DMLS `host_globals.rs`, `lifecycle_host_profile.rs`,
  `lifecycle_event_views.rs`, `edit_action_editor.rs`,
  `host_function_diagnostics.rs`, `registered_host_*`,
  `host_registration_watches.rs`, `frontmatter_response_contracts.rs`
  (A3 completion golden reordered), `benches/pipeline_work_a3.rs`; library
  `benches/pipeline_work_support` (format 2 bundle built in a temp dir);
  negative legacy tests now assert a root-local catalog supplies no host.
- Claudine tests `host_catalog.rs` and `host_publication.rs` were updated to
  the new APIs so the suite compiles.

### Reader checkpoint

- `just shipped-schemas` in `simplified-schema/` and `darkmatter/`: both
  artifacts byte-identical (no prepared input changed).
- Watches: `host_registration::a_missing_member_is_refused_and_watched`,
  `…does_not_match_its_digest…`, `…unlisted_import…its_target_watched`;
  DMLS `host_registration_watches::a_refused_registration_watches_its_manifest_and_members_and_loads_once_repaired`
  (missing parent folder watched, repair loads).

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| Rule 6 catalog, multi-file, transitive hash, path-independent identity, spans | `host_namespaces::*` (multi-file order, `at == (2, 12)`, transitive library hash, two-folder identity) |
| Binding check, version 1 and stale artifacts | `host_profile::tests` (format 1, wrong hash, unbound `host_globals`), `host_namespaces::an_artifact_binding_a_namespace_the_catalog_lacks_is_refused`, `host_registration::a_format_1_lifecycle_artifact_is_refused` |
| Member classification and envelopes | `host_registration::a_member_that_is_no_registered_kind_is_refused_at_its_entry`, `…invalid_trigger…`, `…matcher_only_trigger…`, label tests, `host_namespaces::an_explicit_null_or_a_wrong_shape…`, `…reserved_or_malformed_namespace_name…`, `…declared_twice…` |
| YAML readers | `host_registration::the_member_yaml_reader_refuses_every_malformed_document` |
| Broad trigger branches | `host_registration::an_identifying_trigger_on_md_owned_keys_is_too_broad_at_its_match` |
| Reordered lists, moved copies | `host_selection::reordered_files_keep_the_revision_and_the_selection`, `…moved_copy_collapses…`, `host_namespaces::one_bundle_installed_in_two_folders_has_one_identity` |
| Same-host disagreement, multiple hosts | converted `host_registration` agreement/conflict tests, `host_selection` ambiguity tests |
| Literal mismatch, path qualifiers | `host_selection::a_literal_trigger_matches_only_an_equal_value`, `…path_qualified_trigger…` |
| Malformed presence operands, shared-event-only documents | `host_selection::a_malformed_presence_operand_or_a_shared_event_only_document_stays_md` |
| Matched trigger indices | `host_selection::matched_lists_exactly_the_matching_triggers`, `…layers_the_host_triggers_in_bundle_path_order` |
| Grant-independent passive analysis | `host_registration::loading_is_independent_of_permissions_and_writes_nothing` |
| No legacy discovery | `host_selection::a_root_local_globals_lifecycle_and_function_catalog_supply_no_host`, `host_functions::a_function_catalog_in_a_schema_root_supplies_no_host_and_no_document_schema`, DMLS `host_globals`/`lifecycle_event_views` negatives |
| YAML claims indexed | `host_selection::a_registered_yaml_kind_is_indexed_by_the_kinds_it_claims` |
| CLI wiring (one positive, located refusals) | `darkmatter-cli` `host_registration::a_format_2_bundle_types_the_document_its_trigger_matches`, `…format_1_manifest_is_refused…`, `…member_that_fails_its_digest…` |
| Watches on refusal | see Reader checkpoint |

Input robustness matrix rows covered this phase: manifest scalars and paths
and digests (Phase 1 snapshot tests plus two `load_entries` checks), member
envelopes, lifecycle JSON, YAML readers, and strict JSON (existing
`artifact.rs` and facts tests). Config mutation is unchanged this phase
(Phase 4). Not done: a label that fails its pattern is refused without a
line (see `decisions-for-review`).

### Gates

- `simplified-schema`: `just test` — 2125 passed; `just lint` passed.
- `darkmatter`: `just test` — 17594 passed, 10 skipped;
  `🐢 L1 run recorded: 17594 test(s), none at or above 5.0s`. `just lint`
  passed (including the zed-dmls wasm check).
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing
  stranded.
- Dependents: `cargo nextest run -p claudine -p claudine-cli -E
  'test(/host|registration|identif|lifecycle_artifact|publication|globals|catalog|schema|shipped|trigger/)'`
  — 1723 passed, 28 failed. All 28 are `host_publication::*`,
  `installed_registration::*`, and `host_registration_refresh::*`, which
  load Claudine's published format 1 manifest and now get the upgrade
  warning; expected until Phase 4. `cargo clippy -p claudine --tests` clean.
- `just publish-skills` in `darkmatter/` (`use-dmls` changed): published.
- Windows (`just cross-check <package> --os windows`): `darkmatter` 12743
  passed, 66 skipped; `darkmatter-lsp` 2112 passed, 9 skipped;
  `darkmatter-cli` 2639 passed, 85 skipped. No `os-failing` needed.

## Phase 3

- Started 2026-11-22, finished 2026-11-23. Tasks: Trigger layer (Rule 8),
  YAML tier (Rule 9), Consumer checkpoint. All three are checked off in the
  plan.

### What was built

- **Checked trigger layer.** `darkmatter/lib/src/config/registration/merge.rs`
  (new) compares every pair of a host's matched payloads per top-level
  property: subtype in both directions
  (`is_definition_subtype_with_definitions` over a combined definition
  table), then presence, null admission, defaults, descriptions, and the
  remaining constraints separately; a structural comparator that is
  coinductive over named types coalesces recursive aliases. The unique
  narrowest class wins and every other payload's declaration is removed, so
  the winner keeps its origin. Root checks: at most one root union, no root
  rules when two or more payloads contribute, and no other payload may
  declare a key that tells the union's arms apart.
  `RegisteredHostProfile::checked_layer` memoizes static results per sorted
  matched set; `DarkmatterSchemas::resolve_trigger_layers` and
  `pending_templates_for` use it (templates resolve whole and bypass the
  memo). A conflict is `SchemaError::HostTriggerConflict` (boxed), reported
  as `dm.host.trigger_conflict` by `report_diagnostics::schema_error_diagnostics`
  (compose and validate), the CLI's JSON `schema` record (`code`, `related`),
  and DMLS (`schema_prepare_diagnostic`, related information in both trigger
  files).
- **Registered root unions.** `resolve_registered_trigger_payload` with
  `PayloadForm::ObjectOrUnion` admits one inline root union in a registered
  payload; `merged_trigger_schema` layers around it (`merge_around_union`),
  and origins and default files are attributed per arm key.
- **YAML tier.** `prepare_yaml_contract` passes
  `RegisteredHosts::yaml_kinds()` to `select_with_registered`; the CLI
  attaches the run's hosts for YAML inputs; DMLS prepares the YAML
  document's host registrations and folds their version into the contract
  key. `YamlContract::registered()` names the host, path, and file.

### Requirement to test

| Requirement | Test |
| --- | --- |
| Equivalent declarations coalesce, first path is origin | `host_trigger_merge::equivalent_declarations_coalesce_under_the_first_trigger` |
| Enum narrows string | `host_trigger_merge::a_narrower_enum_wins_over_a_string_whose_other_rules_agree` |
| Required vs optional | `host_trigger_merge::a_required_string_and_an_optional_enum_conflict_on_presence` |
| Nullability, descriptions, constraints, defaults | `host_trigger_merge::equivalent_types_that_differ_in_nullability_description_constraint_or_default_conflict` (no example case; see `decisions-for-review`) |
| Nested rules | `host_trigger_merge::nested_declarations_are_walked` |
| Recursive aliases | `host_trigger_merge::recursive_definitions_compare_by_structure_not_storage` |
| A narrower third still exposes an incompatible pair | `host_trigger_merge::a_narrower_third_declaration_cannot_hide_an_incompatible_pair` |
| Inherited keys | `host_trigger_merge::an_inherited_property_is_compared_as_declared` |
| Unmatched conflicts | `host_trigger_merge::conflicting_triggers_that_never_match_together_are_no_conflict` |
| Memo per matched set | `host_trigger_merge::a_static_result_is_memoized_per_matched_set` |
| Arm order, ordinary keys in every arm | `host_trigger_merge::ordinary_properties_merge_into_each_arm_of_the_root_union_in_arm_order` |
| Competing alternative key | `host_trigger_merge::another_trigger_may_not_declare_a_key_that_defines_the_union` |
| Second root union, nondefault root rules | `host_trigger_merge::a_second_root_union_or_root_rules_with_two_payloads_conflict_on_the_root` |
| Proven local override; conflict survives an authored override | `host_trigger_merge::a_document_override_applies_to_a_prepared_layer_but_never_clears_a_conflict`, `host_trigger_merge::a_root_trigger_refines_a_prepared_union_layer` |
| Passive unresolved templates | `host_trigger_merge::a_registered_template_stays_passive_and_is_listed_whole` |
| Same-file suppression | `host_selection::a_registered_trigger_also_visible_in_a_root_applies_once` (existing) |
| Conflict code and sources: library | `host_trigger_merge::the_conflict_is_one_coded_error_on_the_document` |
| Conflict code and sources: CLI (`FileOutcome::schema_error`, compose) | `darkmatter-cli` `host_registration::two_matched_triggers_that_disagree_are_one_coded_conflict_naming_both` |
| Conflict code and sources: DMLS (`schema_prepare_diagnostic` related) | `darkmatter-lsp` `registered_trigger_conflict::a_conflict_between_matched_triggers_is_one_error_related_to_both_files` |
| Registered YAML tier: identity, precedence, copies, duplicates, no trigger layer | `registered_yaml_kinds::*` (9 tests, library) |
| CLI YAML tier | `darkmatter-cli` `host_registration::schema_validate_judges_a_yaml_document_by_a_registered_hosts_kind`, `…refuses_a_kind_two_registered_hosts_define_differently` |
| DMLS YAML tier, `selection_diagnostic` for kind collisions, reload | `darkmatter-lsp` `registered_yaml_kinds::a_registered_kind_judges_the_document_and_a_competing_definition_refuses_it_until_removed` |
| DMLS contract key follows the host version | `overlay::yaml_contract::tests::the_contract_key_follows_the_installed_registrations_version` |

### Changes to existing tests

- `markdown::schemas::…::root_union_document_schema_merges_trigger_per_arm_without_origins`
  was renamed `…_with_arm_origins` and now asserts per-key origins (root-union
  origins are now attributed; the `EffectiveSchema::origins` doc was stale
  and is corrected).
- `lsp_session::meta_schema_phase6_shipped_schema_activation_and_current_error`
  waits for the version 2 publication: the host-registration install now
  republishes version 1 of a YAML document first.
- `registered_yaml_kinds::a_local_definition_claiming_the_kind_wins_over_the_registered_one`
  compares the definition as a `Path` (it failed on Windows with mixed
  separators).

### Gates

- `darkmatter`: `just test` — 17626 passed, 10 skipped; slowest L1 test
  2.15 s. `just lint` passed.
- `simplified-schema`: `just test` — 2525 passed; `just lint` passed.
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing
  stranded.
- Dependents: `cargo check --tests` of `claudine`, `claudine-cli`,
  `claudine-gen`, `biscuit-icon-cli`, `biscuit-terminal-cli`,
  `sniff-detection-cli`, `biscuit-speaks-cli`, `messenger`, `messenger-cli`,
  `playa-cli`, `research`: clean. `claudine` filtered
  (`host|regist|trigger|yaml|schema|publication|catalog`): 1510 passed, 26
  failed; `claudine-cli` filtered: 414 passed, 2 failed. All 28 failures are
  `host_publication::*`, `installed_registration::*`, and
  `host_registration_refresh::*`, refused with the format 1 upgrade warning,
  as recorded in Phase 2; expected until Phase 4.
- `just publish-skills` in `darkmatter/`: published.
- Windows (`just cross-check darkmatter --os windows`): 12769 passed, 66
  skipped, after the path-comparison fix above. No `os-failing` needed.
