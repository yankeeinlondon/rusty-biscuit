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
message-to-agent: |-
    Phase 1 left format 1 manifests loading through the old profile path on purpose: `load_registrations` still calls `manifest::read` (v1, validated against a private legacy contract in `manifest.rs`), because host profiles were not yet built from format 2. Phase 2 must switch `load_registrations` to `config::registration::bundle::load_snapshot` (format 1 then gets the upgrade warning it already returns), build profiles from the `VerifiedSnapshot`, and delete `manifest::read` and the legacy contract. Use `simplified_schema::resolve::bundle::{BundleSources::on_disk(snapshot.bundle_root(), snapshot.bytes().clone()), with}` for every member resolution so reads come only from verified bytes; `BundleReads` gives attempted/missing paths for watches. For YAML, wrap each registered yaml-schema in `yaml_documents::RegisteredDefinition::new(RegisteredSource{host, path, file}, resolved, sameness)` (sameness = host revision + path) and pass `RegisteredKinds` to `yaml_documents::select_with_registered`. A namespace file is parsed with `simplified_schema::parse_namespace_schema_document`; the reserved-name checks (expression roots, Darkmatter globals, `null`) are not done yet and belong to Phase 2 catalog preparation. Explicit null on optional envelope fields is refused only by the envelope parsers, not the kind definitions. `claudine/schemas/sequence-types.yaml` exists untracked in this tree (from other in-progress work) and three tests include it; see decisions-for-review.
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
