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
    - test-toolkit
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
    - |-
        Claudine's formal `kind: sequence` YAML is judged by the shipped `formal-sequence-file.yaml` kind after the sequence parser accepts the list, not before normalizing as the plan says. Checking first replaced the parser's more precise errors and would refuse a `sequence` that is a file reference, an expression, or a `$( … )` source, which work today. Alternative: check first and rewrite about eight tests, at the cost of weaker errors. Redo cost: small.
    - |-
        Claudine's build-time inventory check (`claudine/lib/build.rs`) is structural only: portable names, no links, case collisions, UTF-8, and an allowed `kind`. Full preparation of the bundle runs in the generator and the tests. Alternative: make SimplifiedSchema a build dependency for earlier, fuller errors, at the cost of slower builds of every Claudine consumer. Redo cost: moderate.
    - |-
        `claudine-config.yaml` is an embedded bundle member (a types-only library), so it is published with the registration and listed by `claudine registrations`. The plan moves it into `claudine/registration/` because the old publisher shipped it. Alternative: keep it in the folder but exclude it from the bundle, which needs an exclusion rule in the inventory walk. Redo cost: small.
    - |-
        A same-name copy of a Claudine registration file in a schema root is now an ordinary trigger layered above the registered layer; it no longer replaces the compiled-in file. A malformed or non-matching copy therefore no longer hides Claudine's triggers, and a property counts as a Claudine control only when its winning declaration is in the registered Claudine layer. Alternative: keep the old replace-by-name rule for in-process use, which would make in-process and installed results differ. Redo cost: moderate (embedded shadow tests rewritten).
    - |-
        One broken member refuses Claudine's whole registered profile, in process as well as installed, so the trigger matrix's per-origin `trigger_kind` cells are all errors and the old `unclaimed_trigger` case is gone. Alternative: let the in-process path skip a broken trigger, which would differ from the installed reader. Redo cost: small to moderate.
    - |-
        The prepared registered payloads are stored in Claudine's shipped artifact (`PREPARATION_REVISION` 7), which grew from 3.1 MB to 5.7 MB, so an in-process launch does not prepare the profile from text (about 0.2 s first load now, versus roughly 0.5 s from text in debug). Alternative: prepare from text at first use and keep the artifact small. Redo cost: moderate.
    - |-
        A registered root-union layer has no editor shape (`registered_trigger_layers` sets `shape: None`), so DMLS completion, hover, and declared expression names see nothing from `inline-composition.yaml`; validation is unaffected. Six DMLS editor tests now start their document with `agent: claude` instead of `prompt: hi`, assertions unchanged. This was already true of installed registrations after Phase 3. Alternative: give union arms an editor shape now (merge arm properties for completion). Redo cost: moderate; the six test inputs should be reverted when that lands.
    - |-
        SimplifiedSchema's `Prepared::new` now reuses a cached default plan (`DefaultPlan::for_identity`) library-wide instead of compiling one per call. It fixed a 10 s test caused by compiling the 590 KB sequence kind's defaults on every inline sequence plan. Alternative: cache only in Claudine's sequence check. Redo cost: small.
    - |-
        `registration.rs` in Claudine was split into `registration/published.rs` and `registration/legacy.rs` (re-exported at the same paths) to stay under the repository's 800-line production-file guard. Redo cost: none functionally.
    - |-
        The publisher's lock waits up to 10 s (the config writer waits 2 s), because a publish holds it across about 17 synced writes. Validity of the previous manifest (for retiring members) is read with Claudine's own closed serde check, because Darkmatter's `read_v2` is private and `load_snapshot` refuses exactly the interrupted state that still needs retirement. The test seam `publish_with` is always compiled (`#[doc(hidden)]`). Alternatives: export a Darkmatter envelope check; gate the seam behind `test-fixtures`. Redo cost: small each.
    - |-
        Setup rewrites an existing entry for the new manifest that is spelled differently (an absolute path) to the canonical `~/…` spelling, per the plan's 'canonical spelling at the first affected index' rule; an existing test was renamed to assert this. For an entry whose file is missing, only the first location its resolution would probe is compared. Legacy cleanup removes nothing at all if any link or unexpected entry is found in the old tree. Obsolete repository `register` entries are not reported, because setup had no existing place for such a report. Alternatives: leave other spellings alone; remove the old manifest even when the revision tree is obstructed; add a report. Redo cost: small each.
    - |-
        `claudine registrations` is dispatched after request initialization (home capture) and before Darkmatter configuration loads, and `just install` calls a public `publish-registrations` recipe that prints a file count rather than the whole JSON. Startup-passivity tests use `claudine errors` as the representative ordinary command. Alternatives: dispatch earlier with separate home capture; a private recipe. Redo cost: small.
    - |-
        Reload relevance is now decided by path alone on the editor's message thread, and the reload itself (on the background worker) re-reads and hashes the files. Before this phase, the message thread re-hashed a reported file and ignored the event when its bytes were unchanged; the plan asks that files be read and hashed only on the worker. Consequence: a file saved with identical bytes now costs one background reload after the 300 ms window, which finds the same registrations and changes no document's analysis. Alternative: have the worker compare the recorded hashes first and skip the full load when nothing changed. Redo cost: small (one check at the start of the reload).
    - |-
        A reload whose registrations (manifest paths, revisions, load failure) are unchanged but whose configuration-level warnings differ keeps the analysis version: the new warnings are published on the files they name, but open documents are not reanalyzed. Before, any warning change bumped the version and reanalyzed every open document. The plan asks for the effective revision and the diagnostics to be compared separately. Alternative: keep bumping the version on any warning change (simpler, more reanalysis). Redo cost: small; one existing unit assertion was flipped to match (a refusal that loads no profile keeps the version).
    - |-
        The settle window is zero everywhere except the running server: `HostRegistrations::default()` and `OverlayState` (used by in-process callers and many tests) reload on the next preparation, and the server sets 300 ms (`StartupHooks::registration_settle` can override it). The shared L1 server fixture also defaults to zero so the existing registration-watch tests keep their immediate behavior; the 300 ms window is exercised by the new `registration_reload_scheduling` tests on a manual clock and by the `hosts.rs` unit tests. Alternative: make the production window the default everywhere and have test flushes wait for pending reloads, which adds about 300 ms of real time per reported change in every host test. Redo cost: moderate (fixture default and test timing).
    - |-
        Only the watch-limited save fallback (a client that cannot root watchers at folders, or rejected them) schedules the debounced registration reload on save. A client with no watcher at all (`ServerRescan`), a `.dmls.toml` or client settings change, a package manifest or `.git` change, and other explicit full invalidations still reload registrations immediately, as the plan's 'preserve explicit configuration reload behavior' asks. Alternative: debounce the `ServerRescan` save too. Redo cost: one call site.
    - |-
        When a configuration's first load (nothing installed yet) is touched by a change while it runs, its result is installed anyway, so documents get a view, and another reload is scheduled after the window. A warm configuration's touched or superseded result is dropped and the installed view kept. Alternative: drop the first result too and leave documents on `md` until the second load. Redo cost: small.
message-to-agent: |-
    Phase 5 landed debounced DMLS registration reloads (`darkmatter/dmls/src/overlay/hosts.rs`): per-configuration `due`/`generation`, `REGISTRATION_SETTLE` = 300 ms (measured publish 0.306 ms), router timer + `start_due_host_reloads` before every message, path-only matching on the router thread, `invalidate_unwatched_inputs` for the save fallback. The docs already describe the 300 ms wait in `darkmatter/docs/topics/host-registration.md` ("Changes are followed without a restart") and `dmls/docs/diagnostics.md`; Phase 6 still needs `dmls/docs/editors/README.md` and the `use-dmls` caller skill to mention it, plus the stale Claudine/DMLS doc text listed by Phase 4 (`claudine/docs/topics/schemas.md` "eight files" table, `claudine/schemas/README.md`, `.claude/skills/claudine/cli-commands.md`, `darkmatter/dmls/docs` mentions of `globals.yaml`, `darkmatter/docs/topics/schemas/dmls-schema-support.md`). Caller skills have not been republished in Phases 4 or 5 (`just publish-skills` not run). L1 tests of the window use `LspFixture::start_with_registration_settle` with a `ManualClock`; every other `LspFixture` uses a zero window.
source_files_during_phase_4:
    - claudine/registration/action.yaml
    - claudine/registration/claudine-config.yaml
    - claudine/registration/composition.yaml
    - claudine/registration/content-policy.yaml
    - claudine/registration/document.yaml
    - claudine/registration/formal-sequence-file.yaml
    - claudine/registration/group.yaml
    - claudine/registration/inline-composition.yaml
    - claudine/registration/lifecycle-events.yaml
    - claudine/registration/linking.yaml
    - claudine/registration/loop.yaml
    - claudine/registration/seq.yaml
    - claudine/registration/sequence-document.yaml
    - claudine/registration/sequence.yaml
    - claudine/registration/shared.yaml
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine-config.yaml
    - claudine/schemas/claudine-document.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/document.yaml
    - claudine/schemas/globals.lifecycle.json
    - claudine/schemas/globals.yaml
    - claudine/schemas/inline-compose.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/schemas/linking.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/schemas/sequence.yaml
    - claudine/lib/build.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/generated/globals.lifecycle.json
    - claudine/lib/generated/shipped.json
    - claudine/lib/examples/lifecycle_artifact.rs
    - claudine/lib/examples/shipped_schemas.rs
    - claudine/lib/benches/pipeline_work.rs
    - claudine/lib/benches/pipeline_work_alloc.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/inventory.rs
    - claudine/lib/src/composition/inventory/walk.rs
    - claudine/lib/src/composition/inventory/tests.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/registration.rs
    - claudine/lib/src/composition/registration/legacy.rs
    - claudine/lib/src/composition/registration/published.rs
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/schema/globals.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/shipped.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/declarations/tests.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/lib/src/config/atomic.rs
    - claudine/lib/src/config/schema.rs
    - claudine/lib/src/config/setup.rs
    - claudine/lib/tests/l1/claudine_identification.rs
    - claudine/lib/tests/l1/claudine_schema_parity.rs
    - claudine/lib/tests/l1/claudine_schema_shipped_corpus.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_trigger_selection_parity.rs
    - claudine/lib/tests/l1/container_schema_recursion.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
    - claudine/lib/tests/l1/embedded_inventory_preflight.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/host_publication.rs
    - claudine/lib/tests/l1/installed_registration.rs
    - claudine/lib/tests/l1/installed_registration_agreement.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/registration_inventory.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/tests/l1/shipped_artifacts.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/registrations.rs
    - claudine/cli/src/commands/init_wizard.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/host_registration_refresh.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/registrations_command.rs
    - claudine/cli/tests/l1/shipped_prompts.rs
    - claudine/cli/src/commands/wrap/runaway_guard.rs
    - claudine/cli/tests/l1/implement_plan_commit_message.rs
    - claudine/cli/tests/l1/lifecycle_refusal_reports.rs
    - claudine/cli/tests/l1/lifecycle_edit_action.rs
    - claudine/justfile
    - darkmatter/lib/src/config/init.rs
    - darkmatter/lib/src/config/registration.rs
    - darkmatter/lib/src/config/registration/bundle.rs
    - darkmatter/lib/src/config/registration/manifest.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/config_init.rs
    - darkmatter/lib/tests/l1/config_widening.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/embedded_registration.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/init_command.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/config_contracts.rs
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/dmls/tests/l1/config_contracts.rs
    - darkmatter/dmls/tests/l1/edit_action_editor.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/shipped/wire_support.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/tests/l1/phase_projection.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_identities.rs
    - tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs
    - scripts/shipped_schemas.py
    - justfile
    - Cargo.lock
    - .github/ci/permission-inputs.json
docs_updated_during_phase_4:
    - claudine/schemas/README.md
    - claudine/docs/cli/index.md
    - claudine/docs/dependencies.md
    - claudine/docs/topics/configuration.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/docs/topics/lifecycle/host-artifact.md
docs_created_during_phase_4:
    - claudine/docs/cli/registrations.md
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/module-map.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/simplified-schema/defaults.md
    - claudine/docs/use-skills/use-claudine/config.md
source_files_during_phase_5:
    - darkmatter/dmls/src/overlay/hosts.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registration_reload_scheduling.rs
    - darkmatter/dmls/tests/l1/host_registration_watches.rs
    - darkmatter/dmls/tests/l1/registered_yaml_kinds.rs
docs_updated_during_phase_5:
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
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

## Phase 4

- Started 2026-11-23. Tasks: Source migration (Rules 10–11), Embedded
  inventory (Rule 12), Fixed publication (Rule 13), then Wave 4:
  Registration command (Rule 14) and Config migration (Rule 15) in parallel,
  then the Publisher checkpoint. Each task ran as one foreground subagent
  that verified only its own slice; the orchestrator ran the gates.

### Source migration (Rules 10–11)

- `claudine/registration/` is the only source of Claudine's bundle.
  `action.yaml`, `document.yaml`, `linking.yaml`, and `claudine-config.yaml`
  moved in from `claudine/schemas/`; `content-policy.yaml` is a byte copy of
  `content-policy/schemas/content-policy.yaml`; the old `schemas/sequence.yaml`
  trigger became `registration/sequence-document.yaml` and its named types
  `registration/sequence.yaml`; `lifecycle-events.yaml` is now the
  authoritative types-only lifecycle library. `claudine.yaml`,
  `inline-compose.yaml`, `lifecycle.yaml`, `sequence.yaml`, and
  `sequence-types.yaml` under `claudine/schemas/` are deleted.
- `shared.yaml::common-attributes` carries the common controls;
  `composition.yaml` and both arms of `inline-composition.yaml` extend it.
  Exactly one of `composition` and `inline-composition` applies to a
  document now, which changed the recorded trigger-selection parity table
  (prompt rows select only `inline-composition.yaml`), two matrix operand
  paths, and the embedded shadow tests.
- Claudine config validation and DMLS's configuration contract read
  `claudine/registration/claudine-config.yaml`; `.github/ci/permission-inputs.json`
  names the new path.
- Interim: embedded bundles still refuse a root union, so the in-process
  inline trigger was a single-object runtime copy
  (`composition/schema/runtime-inline-composition.yaml`) until the embedded
  inventory task routed in-process selection through the registered profile.
- New tests: `claudine/lib/tests/l1/registration_inventory.rs` (7 tests:
  whole install and exact content-policy copy; shared events alone and a
  misspelled kind select nothing; prompt/sections arms with null
  counterparts, both values, and `kind: inline-compose` with neither;
  sequence presence, wrong types, literal mismatch; a sequence layer cannot
  weaken the union; formal sequence YAML requires an inline `step[]`; setup
  and config consume the canonical config definition).
- Scoped runs: `claudine` 1115/1115 of the touched filters, `claudine-cli`
  121/121, Darkmatter/SimplifiedSchema corpus filters 76/76, `darkmatter-cli`
  + DMLS filters 310/310. Format 1 round-trip tests (`host_publication`,
  `installed_registration`, `host_registration_refresh`) still fail with the
  upgrade warning, as expected until publication.

### Embedded inventory (Rule 12), first pass

- `claudine/lib/build.rs` walks `claudine/registration/` with the shared
  pure module `composition/inventory/walk.rs` (compiled by the build script
  through `#[path]` and by the library), emits a sorted include table, and
  a `rerun-if-changed` for the folder and every file. The build check is
  structural (portable names, no symlinks/junctions, case collisions,
  UTF-8, allowed member kinds); full preparation runs in the generator and
  tests. `serde_yaml_ng` became a build dependency.
- `claudine::composition::inventory` exposes `members()`, `member(path)`,
  `members_of(kind)`, `bundle_bytes()`, `bundle_sources()`,
  `bundle_context()`, `LIFECYCLE_ARTIFACT`, and the bundle constants.
- `globals.yaml` and `claudine-document.yaml` are deleted; Claudine's
  globals are prepared from the three namespace files
  (`schema::globals::prepare_namespace_catalog`), and the lifecycle
  artifact's `catalog_hash` is the hash of those files. The shipped record
  gained YAML kind records (`PreparedKind`, `formal_sequence_kind`);
  `PREPARATION_REVISION` 5 → 6.
- The lifecycle artifact (`claudine/lib/generated/globals.lifecycle.json`)
  is a candidate emitted by Claudine's generator (`--lifecycle-output`) and
  staged by `scripts/shipped_schemas.py` with the other artifacts;
  `examples/lifecycle_artifact.rs` is deleted and the old
  `regen-lifecycle-artifact` recipe delegates to the root
  `just shipped-schemas`.
- Formal `kind: sequence` YAML is judged by the prepared embedded kind in
  `sequence/mod.rs`, after the sequence parser accepts the list (see
  `decisions-for-review`).
- Generation now decodes the two lower registries once each (the bundle
  scope has no uncached mode); the generation test is renamed
  `generation_consults_no_artifact_but_the_lower_registries_for_namespaces_and_kinds`.
- Tests: 9 walk unit tests (`composition::inventory::tests`),
  `registration_inventory::the_embedded_inventory_is_a_fresh_walk_of_the_registration_folder`,
  `shipped_artifacts::staged_generation_refuses_a_stale_lifecycle_member_and_writes_nothing`,
  `embedded_inventory_preflight::inline_direct_and_sequence_preflight_use_the_embedded_inventory`
  (empty home, cwd outside any repository),
  `declarations::tests::a_formal_yaml_sequence_is_judged_by_the_prepared_kind`,
  and a coordinator test in `tools/test-toolkit`.
- Deferred by this pass and done by a follow-up task (below): in-process
  selection through the checked embedded registered profile and removal of
  the single-object runtime inline stand-in.

### Embedded inventory (Rule 12), registered profile

- In-process selection now goes through Darkmatter's registration path.
  `schema::bundle::claudine_hosts()` (`OnceLock`) verifies the compiled
  manifest and members from memory (`VerifiedSnapshot::from_embedded`,
  virtual root `<claudine>`), loads them with `registration::load_embedded`,
  and wraps the profile as the only `RegisteredHosts`. `discover_schema_owner`
  and the benches use `with_claudine_registration(..)`. Embedded triggers
  carry `TriggerSource::Embedded { bundle: "claudine", name: "schemas/<file>" }`,
  `location` `<claudine>/schemas/<file>`, and no host file; a registered
  trigger's duplicate check uses its disk identity, or its embedded identity
  plus payload digest.
- `composition/schema/runtime-inline-composition.yaml` and
  `RUNTIME_INLINE_COMPOSITION` are deleted; the inline union
  (`inline-composition.yaml`, `PayloadForm::ObjectOrUnion`) applies in
  process.
- Shipped artifact: `PREPARATION_REVISION` 6 → 7; the record holds the
  prepared registered payloads (`payload.registered`, one record per trigger
  with its bundle path and payload digest), served by
  `shipped::registered_payload`. `claudine/lib/generated/shipped.json`
  regenerated with `just shipped-schemas` (3.1 MB → 5.7 MB).
- Darkmatter: `manifest::embedded_members`, `bundle::from_embedded`,
  `registration::{load_embedded, RegisteredTrigger::{location, is_embedded}}`;
  SimplifiedSchema wire namespaces gained `Bundle*` variants so prepared
  payloads of a virtual-root bundle serialize.
- Departures from the plan:
  - A disk copy of a Claudine registration file in a schema root is an
    ordinary trigger layered above the registered layer, never a
    replacement: it no longer hides the registered triggers, and
    `is_claudine_owned` is true only when the winning origin is a trigger of
    the registered Claudine layer. The embedded shadow tests were rewritten
    for this (`claudine_schemas_embedded.rs`), including a malformed or
    non-matching disk copy, which now leaves the registered triggers in
    force.
  - Registration refuses the whole profile on one broken member, so the
    matrix `trigger_kind` cells are `Error` for every origin and
    `unclaimed_trigger` is gone; "each trigger alone" metadata cases are
    passive only (built with `profile_of(without_member)`).
  - DMLS's Claudine fixtures install the real bundle in the repository and
    register it; the "outside the area" halves use documents outside the
    repository. `darkmatter-cli`'s `schema_validate` Claudine bundle drops
    the inline file.
  - Six DMLS editor tests write `agent: claude` instead of `prompt: hi`: a
    registered root-union layer has no editor shape
    (`registered_trigger_layers` sets `shape: None`), so completion, hover,
    and declared expression names see nothing from `inline-composition.yaml`.
    That was already true for an installed registration; see leftovers.
  - `Prepared::new` (SimplifiedSchema) now takes its `DefaultPlan` from
    `DefaultPlan::for_identity`. The formal `kind: sequence` check of the
    first pass compiled the 590 KB kind's defaults on every inline sequence
    plan (about 120 ms each in debug), which made
    `template_owner_values_tests::every_inline_template_reads_its_owners_prepared_values`
    take 10.1 s alone; it now takes 1.4 s.
- Guards that went red when whole packages ran, fixed here:
  `claudine/lib/src/composition/registration.rs` was over the 800-line
  production limit (`test_placement`), so its inventory and retired-cleanup
  sections moved to `registration/{published,legacy}.rs`, re-exported at
  the same paths; the error-guard allowlist gained `prepare_registered` and
  lost a stale `prepare_payload` entry; Darkmatter's context guard allows
  `embedded_context`; `shipped_schema_corpus` dropped the deleted
  `claudine/schemas/{globals,claudine-document}.yaml` entries.
- Tests added: `embedded_inventory_preflight::the_in_process_inline_launch_enforces_the_prompt_or_sections_union`,
  `claudine_schemas_embedded::a_schema_root_trigger_overrides_a_registered_property`,
  five `shipped_artifacts` record refusals (another trigger's record, a
  missing record, another revision's payload digest, a payload with both a
  shape and union arms, an absent digest), and
  `darkmatter/lib/tests/l1/embedded_registration.rs` (8 tests:
  verification from memory, digest mismatch, missing member, bundle names,
  members outside the bundle, `load_embedded` refusing a disk snapshot,
  embedded trigger provenance, a prepared payload served).
- Timing: `inline_direct_and_sequence_preflight_use_the_embedded_inventory`
  0.86 s before, 1.05–1.16 s after (the first in-process profile load, about
  0.2 s, is in it); the new union test 0.57 s.

### Fixed publication (Rule 13)

- `claudine/lib/src/composition/registration.rs` is rewritten:
  `RegistrationInventory::claudine()` holds every published file (manifest,
  lifecycle artifact, `schemas/…` members) with its kind, optional name, and
  exact bytes; `publish(home, inventory)` writes the fixed tree under
  `~/.claudine/registrations/darkmatter/` behind the OS file lock
  `~/.claudine/registrations/.darkmatter.lock` (deadline 10 s, backoff
  20 ms). Preflight (no-follow metadata plus a pure reparse-point
  classifier) runs before the lock file is created and again under the
  lock. Only changed members are written; retired entries come from the
  previous valid format 2 manifest; regular temps with the reserved prefix
  `.claudine-registration-` are removed; unknown files are kept; the
  manifest is written last and only when its bytes differ.
  `PublishOutcome::{Current, Published{changes}}`.
- `config/atomic.rs::atomic_write_registration` reuses the sibling-temp
  writer and its Windows persist retry with the reserved prefix.
- Legacy paths for setup migration: `LEGACY_MANIFEST_REFERENCE`,
  `legacy_manifest_path`, `legacy_revisions_dir`.
- Test seam: `publish_with(home, inventory, PublishHooks { lock_deadline,
  observer })`, whose observer can stop at `MemberWritten`, `Retired`, or
  `BeforeManifest`.
- Tests: 12 in `host_publication.rs` (fresh format 2 load, identical
  no-op with unchanged mtimes, single changed member, interruption at a
  member and before the manifest then convergence, readers at every step,
  two cooperating publishers, lock timeout, temp cleanup with unknown files
  kept, retirement, an invalid/escaping previous manifest removes nothing,
  misplaced entries stop before any write), 6 `obstructions::*` symlink
  tests (Unix), 5 `registration::tests` unit tests including the
  reparse-bit classifier, and 2 `atomic::tests` (reserved prefix; Windows
  retry, `#[cfg(windows)]`). `installed_registration.rs` moved to the new
  layout. Scoped run: 73/73; `claudine-cli host_registration_refresh` 5/5.

### Config migration (Rule 15)

- `darkmatter::config::init::replace_manifest_registration(context, scope,
  reference, old_path)` beside the unchanged `register_manifest`, with
  `replace_manifest_change(observation, …)` as its planning half (the
  `setup_change` pattern) so a test can change the file between read and
  commit. Entries match by `registration::manifest_location_identity`: the
  matched path, or for a missing file the first probed candidate in the
  context anchored at the config file, keyed by
  `registration::location_identity` (nearest existing ancestor
  canonicalized, rest appended). The first affected entry is rewritten to
  the given spelling, the rest removed in descending order, one
  `ConfigTransaction` with the observed bytes. `RegisterReport` gained
  `replaced`.
- Departure: an entry naming the new manifest in another spelling (an
  absolute path) is now rewritten to `~/…` instead of left as written;
  `installed_registration_same_manifest_spelled_absolutely_*` changed to
  assert the canonical entry.
- `setup_claudine`: publish → handshake (only a Claudine write failure
  stops) → `replace_manifest_registration(User, MANIFEST_REFERENCE,
  legacy_manifest_path)` → on success
  `composition::registration::remove_legacy_registration(home)`
  (`ClaudineSetup.cleanup`). Cleanup preflights `~/.claudine`, the old
  manifest, and every entry of `~/.claudine/darkmatter-host/` with the
  publisher's no-follow classifier and removes nothing on a link
  (`CleanupError::Obstructed`) or an unexpected entry. Repository
  configuration is never read for migration; obsolete repository entries
  are not reported (no existing place for it).
- CLI (`init_wizard.rs::write_with_darkmatter_setup`): publication at
  `info`, a replaced entry and each removed retired path as messages, a
  cleanup error as a warning; a fresh headless setup still prints nothing.

### Registration command (Rule 14)

- Hidden `claudine registrations [<filepath>|publish]`
  (`claudine/cli/src/commands/registrations.rs`), dispatched after request
  initialization and before Darkmatter configuration loads. List and detail
  read only the embedded inventory; `publish` writes stdout only after a
  successful publish. Hash is `format!("{:016x}", xx_hash_bytes(..))`.
- `refresh_host_registration` and its startup branch are removed from
  `claudine/cli/src/main.rs`; `host_registration_refresh.rs` is replaced by
  `claudine/cli/tests/l1/registrations_command.rs` (8 tests: sorted listing
  with hashes and names; exact detail contents; refused unlisted spelling
  and `publish` not a member; publish installs the listing and a second
  publish is identical with unchanged mtimes; publish failure on stderr;
  `claudine errors` with a valid config leaves an obstructed registration
  path untouched, never follows a registration symlink, and repairs neither
  missing nor stale members), plus 2 unit tests. Slowest 0.09 s.
- `claudine/justfile`: `install` runs the new `publish-registrations`
  recipe, which resolves the release binary from `cargo metadata`'s target
  directory and the native suffix. Verified with `just -n install --root …`
  and by running `publish-registrations` with a temp `HOME`, a temp target
  directory, and a failing `claudine` first on `PATH` (17 files published,
  the `PATH` binary never ran; an obstructed home failed with the cause on
  stderr). No real `cargo install` ran.
- Docs: `claudine/docs/cli/registrations.md` (new, maintainers),
  `claudine/docs/cli/index.md`, `claudine/docs/topics/schemas.md`.

### Publisher checkpoint

- Inventory against published bytes: `claudine-cli`
  `registrations_command::registrations_publish_installs_the_listing_and_a_second_publish_changes_nothing`
  compares the JSON listing (path, kind, seed-0 XXH64, name) with the
  inventory and every installed file, manifest and lifecycle artifact
  included, byte for byte.
- Installed versus in-process agreement: new
  `claudine/lib/tests/l1/installed_registration_agreement.rs` (8 tests, one
  per document via `macro_rules!`): the bundle is published into a temp
  home and registered in its `~/.darkmatter.yaml`; each document is judged
  passively (`DarkmatterSchemas::validate`/`validate_yaml_document` with the
  loaded `RegisteredHosts`, the path `md schema validate` and DMLS take)
  and in process (inline, direct, Markdown sequence, or formal sequence
  launch). Documents: shared events only, `prompt` only, `sections` only,
  both (refused at `/sections`), a wrong-typed `sequence` (refused at
  `/sequence`), a composition with `agent`/`model`, a valid and an invalid
  formal `kind: sequence` file (refused at `/description`). All agree;
  0.65–1.3 s each. DMLS itself is not spawned: it uses the same
  `DarkmatterSchemas` entry points, and its registered-host editor
  agreement is covered by DMLS's own `registered_host_editor_agreement`.
- Sections boundary (intentional): `sections` is schema-validated by
  `inline-composition.yaml` (one of `prompt` or `sections`, the other
  null-only), but running a `sections` document is a later runtime feature;
  the inline launch reports `PromptPropertyMissing` after the schema passes,
  and `error/render/sequence_loop.rs` reports sections execution as
  upcoming. The agreement test counts that as an accepted schema verdict.
- Fixture migrations this phase: Claudine's schema tests read
  `claudine/registration/…` instead of `claudine/schemas/…`; the trigger
  selection parity table and matrix operand paths follow the
  composition/inline split; embedded shadow tests expect a disk copy to
  layer above the registered triggers; provenance expectations moved to
  `embedded:claudine/schemas/<file>`; DMLS and `darkmatter-cli` Claudine
  fixtures install and register the real bundle; `installed_registration`
  uses the fixed-path layout and a format 2 hostile repository bundle;
  `host_registration_refresh.rs` is replaced by `registrations_command.rs`;
  six DMLS editor tests start from `agent: claude` instead of `prompt: hi`
  (no editor shape for a registered root union yet). Frozen bench inputs
  keep their original paths and bytes.

### Gate fix: phase projection over a registered root union

- The first full `claudine` run failed two `claudine-cli` tests
  (`launch_trigger_schemas::inline_compose_deferral_does_not_suspend_the_payload`,
  `…::inline_completion_lists_a_payload_problem_beside_the_authored_table`);
  both failed alone too, so not load. Cause:
  `simplified_schema::phase::merge_with_effective` returned only the
  projected document when the effective schema's root was a union (the
  registered inline layer) and the document's `$schema` an object, so at
  `SchemaPhase::Launch`/`Completion` every baseline and trigger rule was
  lost and an invalid `model` passed the verdict. Fix: the projected
  document is merged into every arm (`merge_layer` per arm; its `$defs`
  join the root). `PROJECTION_REVISION` 2 → 3; the pinned projection
  identity in `schema_identities.rs` updated.
- Tests: `simplified-schema` `phase_projection::an_object_document_projects_into_every_arm_of_an_effective_root_union`
  (new); Claudine `embedded_inventory_preflight::a_document_schema_over_the_inline_union_keeps_the_union_at_launch`
  (new; an in-process probe showed the invalid document accepted before
  the fix); the two CLI tests above pass after it.

### Gates

- `darkmatter`: `just test` 17644 passed, 10 skipped, none ≥ 5 s (run
  before and after the projection fix); `just lint` passed.
- `claudine`: `just test` 12701 passed, 15 skipped, none ≥ 5 s (after the
  fix; the first run had the 2 failures above); `just lint` passed after
  two `clippy::result_large_err` allowances in test helpers.
- `simplified-schema`: `just test` 2526 passed (and 2126 in the
  no-default-features leg); `just lint` passed.
- `just check-tier-coverage` for `darkmatter`, `claudine`,
  `simplified-schema`: nothing stranded. `just slow-tests fresh`: fresh.
- Dependents: `cargo check --tests` of `claudine-gen`, `biscuit-icon-cli`,
  `biscuit-terminal-cli`, `sniff-detection-cli`, `biscuit-speaks-cli`,
  `messenger`, `messenger-cli`, `playa-cli`, `research`, `content-policy`,
  `test-toolkit`: clean. `test-toolkit`'s coordinator tests were run by the
  embedded inventory task (7 passed).

### Cross-OS check (Windows, build-win)

- `claudine`, attempt 1: 6938 passed, 4 failed. Three `shipped_artifacts`
  tests: the prepared registered payload's `DefaultFileRecord.file` was
  the platform path (`<claudine>\schemas\action.yaml` on Windows), so the
  artifact was not byte-identical across platforms. Fixed in
  `composition/schema/shipped.rs`: the record stores `/` separators
  (`portable_path`) and decodes to the native path (`native_path`); the
  Linux bytes are unchanged, so no regeneration or revision bump.
  One pre-existing test,
  `sequence::preflight::inputs::tests::a_referenced_template_never_selects_a_prompt_from_its_referrers_values`,
  wrote a Windows path inside a double-quoted YAML scalar (read as `\U`
  escapes); it now spells the path with `/`. Attempt 2: 6942 passed.
- `claudine-cli`, attempt 1: 4229 passed, 13 failed, none in code this
  phase changed: seven `implement_plan_commit_message` tests recorded
  `gix`'s Windows installation-config probe (`git config -lz
  --show-origin …`) through the fake `git` (now filtered beside the
  `--exec-path` probe the test already ignored); four
  `wrap::runaway_guard` unit tests read the rig's real
  `%USERPROFILE%\.claudine\config.yaml` because `isolate_empty_home` set
  only `HOME` (now also `USERPROFILE`);
  `lifecycle_refusal_reports::a_foreign_referenced_group_initialize_refusal_is_reported_by_its_step`
  expected a mixed-separator path (now joined per component); and
  `lifecycle_edit_action::a_sequence_container_runs_edit_in_its_start_and_finalize_around_its_children`
  ran POSIX `test` under `cmd` (now a `cmd` equivalent on Windows).
  Attempt 2: 4242 passed.
- `darkmatter`: 12786 passed, 66 skipped.
- After these fixes, Linux `claudine` `just test` 12701 passed again and
  `just lint` passed. No `os-failing` needed.
- Unrelated uncommitted files present in the tree and not part of this
  phase: `claudine/features/2026-11-18-ergonomic-sequences/spec.md`
  (modified) and `claudine/features/2026-11-22-current-pid/spec.md` (new).

## Phase 5

### Publication timing sample (Linux)

- Host: this Linux build host, load average 1.71 at the start. A
  temporary example (`claudine/lib/examples/publish_timing.rs`, deleted
  after the run, release build) used the Phase 4 publisher in-process.
  For each trial a fresh temporary home was seeded outside the timed
  interval with a valid publication of the same inventory whose every
  member had edited bytes (a trailing `# seed` comment on YAML members, a
  trailing newline on JSON members) and the manifest derived for those
  bytes. The changed publish of `RegistrationInventory::claudine()` was
  then timed with `Instant` from the publisher's `Locked` step (taken
  immediately before the first member write) until `publish_with`
  returned after replacing the manifest. Every trial asserted the
  manifest and all 17 files were written and that
  `darkmatter::config::registration::bundle::load_snapshot` loaded the
  result.
- One warm-up (0.327 ms), then five samples: 0.289, 0.288, 0.306, 0.267,
  0.298 ms. Slowest valid sample 0.306 ms, at or below 150 ms, so the
  settle window is **300 ms** (`REGISTRATION_SETTLE`).

### Reload scheduling (Rule 16)

- `dmls/src/overlay/hosts.rs`: each configuration scope (`HostsKey`) now
  has `due: Option<Instant>` and `generation`. A registration-input event
  (`invalidate_paths(paths, now)`) is matched by path only
  (`HostDependencies::touches`, which now also covers a directory at or
  above a searched folder, so a missing folder's creation or a moved
  install counts) and sets `due = now + settle`, resetting any deadline
  already set and bumping the generation. `expire(now)` turns a passed
  deadline into `stale`; `needs_load` (never loaded, or stale) gates
  `begin`, so a scope waiting for quiet keeps serving its installed view
  and queues nothing. `install(prepared, now)` drops a result whose
  generation or sequence is older than the scope's (a newer event
  supersedes an in-flight reload); a warm result a concurrent change
  touched re-arms the deadline instead of installing. `same_result` was
  split into `same_effect` (manifest paths, revisions, load error: the
  analysis version) and `same_diagnostics` (configuration-level warnings,
  replaced under the same version). `schedule_all(now)` is the save
  fallback's debounced form; `invalidate_all` stays immediate.
  `REGISTRATION_SETTLE` = 300 ms. The old router-side content hashing in
  `affected_by` is gone: the reload on the worker reads and hashes.
- `overlay/mod.rs`: the overlay passes the server clock (the slot
  store's, which the router sets) to every call; `registry_need` uses
  `needs_load` after `expire`; new `invalidate_unwatched_inputs`,
  `set_registration_settle`, `next_host_reload`, `take_due_host_reloads`.
- `router.rs`: `StartupHooks::registration_settle` (default the constant),
  a `crossbeam_channel::after` arm on the next deadline, and
  `start_due_host_reloads` (a workspace pass plus `refresh_all_diagnostics`)
  before every message, so a manual-clock advance takes effect on the next
  round trip, ahead of its reply. The watch-limited save fallback calls
  `invalidate_unwatched_inputs` instead of the immediate full
  invalidation. Ordinary edits, trigger files, `$schema` files,
  `.dmls.toml`, context inputs, and `ServerRescan` rescans are unchanged.
- YAML: `YamlContractStore::key` already folds in the installed hosts'
  version and `prepare_yaml_contract` passes the same looked-up view to the
  worker (Phase 3); nothing changed there. The version now changes only
  with effective registrations, so a warnings-only reload leaves YAML
  contracts alone.
- `work_counters::host_loads()` counts registration loads for tests.
- `tests/common/mod.rs`: `LspFixture` defaults to a zero window (the
  previous immediate behavior) and `start_with_registration_settle` gives
  the production window on a caller's `ManualClock`.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| Two batches inside the window yield one load after quiet; deadline reset per batch | `overlay::hosts::tests::two_batches_within_the_window_give_one_load_after_quiet` (unit, controlled instants); `registration_reload_scheduling::a_burst_across_batches_reloads_once_after_quiet_and_a_repair_clears_the_warning` (L1 server, `ManualClock`, `host_loads()` delta 0 at 299 ms, 1 at 300 ms) |
| Old view kept while dirty, for Markdown and YAML open together | same L1 test (seq completion still offered, YAML error count unchanged, no warning before the reload) |
| A newer event supersedes an in-flight result | `overlay::hosts::tests::a_newer_event_supersedes_an_in_flight_reload` (explicit begin/run/install completion order); `a_change_during_a_first_preparation_reloads_it_and_an_older_one_never_wins` |
| An unchanged view changes no analysis generation | `overlay::hosts::tests::an_unchanged_reload_keeps_the_version_and_a_changed_configuration_replaces_it` (`generation()` unchanged); `registration_reload_scheduling::an_unchanged_reload_changes_no_analysis` (one load, zero root assemblies and YAML preparations after the report) |
| Watch coverage follows a moved or missing target; directory and missing-target events | `registration_reload_scheduling::watch_coverage_follows_a_moved_install` (watcher on `home` for `moved`, then the folder's creation event alone reloads); `overlay::hosts::tests::a_missing_manifest_is_watched_where_it_would_appear` (folder event); `a_repair_clears_the_published_refusal` (watches follow a moved entry) |
| Broken view published on the latest completed view; repair clears it without reopening | L1 burst test (warning on the relay manifest after the reload; cleared after restoring the member, documents never reopened); `overlay::hosts::tests::a_repair_clears_the_published_refusal` |
| Unrelated config and a normal edit remain immediate | `registration_reload_scheduling::a_normal_edit_and_an_unrelated_schema_change_are_not_held` (a `$schema` file change and a `didChange` analyzed with no clock advance and no load) |
| Explicit reload immediate; unrelated path schedules nothing; save fallback waits | `overlay::hosts::tests::an_explicit_reload_is_immediate_and_an_unrelated_path_schedules_nothing`; `registration_reload_scheduling::a_save_fallback_client_reloads_after_the_window` (client without `relativePatternSupport`) |
| Existing behavior preserved | the existing `host_registration_watches`, `registered_host_views`, `registered_yaml_kinds`, `claudine_*`, `config_contracts`, and startup/registry suites (zero window) |

Not tested at the server level: holding a reload in flight with the
registry gate while a newer event arrives. The registry gate holds every
document and workspace job, which makes the open/flush sequence of these
fixtures deadlock-prone; the supersede rule is decided entirely inside
`HostRegistrations::install`, which the unit tests drive with an explicit
completion order.

Robustness matrix: not applicable. This phase adds no file-format or
configuration reader; the window is a constant, not configuration.

### Verification

- Targeted: `cargo nextest run -p darkmatter-lsp --lib -E 'test(overlay::hosts::) | test(host_registrations_watch_missing)'`
  (11 passed); `--test l1 -E 'test(registration_reload_scheduling::)'`
  (5 passed); the DMLS lib plus the L1 host, YAML, registration, Claudine,
  watch, schema-scope, config-contract, invalid-trigger, and startup tests
  (1299 passed).
- Gates, darkmatter area: `just lint` clean (after removing an unused
  test field); `just test` 17653 passed, 10 skipped, `none at or above
  5.0s` (load average about 11 during the run on this host). Only this
  log changed after that run. `just check-tier-coverage darkmatter`:
  nothing stranded.
- Dependents: no workspace package depends on `darkmatter-lsp`.
- Windows (`just cross-check darkmatter-lsp --os windows`, build-win):
  2124 passed, 9 skipped. No `os-failing`.
- The affected-scope calculator was not used to skip gates; both gates ran.
- Unrelated uncommitted files from earlier work remain in the tree and
  were not touched by this phase.
