---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/fixes/2026-10-23-frontmatter-key-order/spec.md
plan: darkmatter/fixes/2026-10-23-frontmatter-key-order/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: [biscuit-hash/lib/Cargo.toml, biscuit-hash/lib/src/json.rs, biscuit-hash/lib/src/lib.rs, biscuit-hash/justfile]
docs_updated_during_phase_2: [biscuit-hash/README.md, docs/dependencies.md]
docs_created_during_phase_2: [biscuit-hash/docs/json.md]
skills_files_updated_during_phase_2: [.claude/skills/biscuit-hash/SKILL.md, .claude/skills/biscuit-hash/library-api.md]
packages: [biscuit-hash, darkmatter, dmls, claudine]
decisions_for_review:
    - |-
        S1 found a second order-sensitive schema hash the plan does not name:
        `schema_digest` in darkmatter/lib/src/markdown/schemas/validate.rs
        (next to `schema_content_hash`, which the plan's N5 canonicalizes).
        I recorded it for the same treatment as N5: hash the canonical JSON
        bytes, keep the `judges_files` substring scan on any rendering (it
        does not depend on order). The alternative is to leave it
        order-sensitive and document why; that keeps validator identities
        for reordered-but-equal schemas distinct, which is only a cache miss,
        not a wrong answer. If the reviewer prefers that, Phase 3 drops one
        line change and adds a comment instead.
    - |-
        The plan asked for the baseline failure list from `just test`, but
        that recipe stops at the first failure. I ran
        `just test --no-fail-fast` once, on the unmodified tree, to get the
        full list (45 failures, as the spec draft reported). No alternative
        gives the full list without running every test.
    - |-
        The plan said biscuit-hash's `json` feature needs only `serde_json`.
        Moving Darkmatter's renderer byte-for-byte keeps its `Serialize`
        wrapper, which needs the `serde` crate as well, so `json` enables
        optional `serde` and `serde_json` (both already in the build, no new
        version). The alternative was a hand-written JSON writer with no
        `serde` dependency; that risks small byte differences in number and
        string escaping, which would change every compose cache key.
        Reversing this means rewriting `biscuit-hash/lib/src/json.rs` and
        proving byte equality again.
    - |-
        The plan asked to confirm that `just lint` in biscuit-hash covers the
        new feature-gated code. It did not: the shared lint recipe checks only
        default features, so `json`, `blake3`, and `argon2id` code was never
        linted. I changed the biscuit-hash `lint` recipe to run clippy with
        `--all-features`, matching its `test` recipe and the precedent in
        biscuit-visualized. The alternative was a one-off manual clippy run
        per phase. Reverting is a two-line justfile change.
    - |-
        Phase 4 went wider than the plan's list of HashMap-to-ordered-map
        changes. The plan named EffectiveState and its builder. The audit also
        found bare `fm` during frontmatter interpolation reading a HashMap
        (`FrontmatterSeedState`), fed by three public frontmatter shell APIs
        (`ResolvedShellValue::resolve`, `execute_resolved_shell_values`,
        `execute_shell_values_with`), so those take an ordered map too, and
        Claudine's sequence preflight loader followed. The alternative was to
        keep those signatures and convert inside, which leaves bare `fm` in a
        frontmatter template in random order. Reverting means restoring
        `HashMap` on those signatures and their Claudine callers.
    - |-
        The plan's list of merge argument reversals missed one:
        `prepare_frontmatter_for_compose` (compose/util.rs) merged external
        state into the document's own frontmatter with external state as the
        order base. It decides a transcluded child's key order, so I fixed it
        the same way (document is the base, document values still win). No
        reasonable alternative keeps the spec's "receiving document first"
        rule; leaving it would make child output parent-first.
    - |-
        Bare `fm` used to re-append the document's own `ctx` at the end of the
        object. It now keeps `ctx` at its authored position. The alternative
        (always last) is simpler to describe but contradicts author order.
        Reverting is a few lines in `EffectiveState::get_fm`.
    - |-
        Phase 5's 43 review-list failures were already green when the phase
        started: a parallel commit merged into this branch after Phase 4
        (58e2a5790, "preserve authored order and validate numeric ingress")
        updated the 40 conversion snapshots, the JSON5 and suggestion tests,
        and made `md schema validate --format json` sort its report keys
        (`sort_all_objects`) to keep that published output alphabetical. I
        reviewed and kept those changes rather than redoing them. The
        alternative for the JSON report is to let it follow construction
        order; that changes a published byte-for-byte output, so I left the
        sort. Reverting would mean re-recording the legacy JSON fixtures.
    - |-
        The review found that schema lowering still used swap-removes
        (`serde_json::Map::remove`), so some emitted JSON Schema keys were not
        in the order the code builds them: a root-union arm got `required`
        moved to its front, and a union arm with a description and annotations
        had them shuffled. The plan only asked to review outputs. I switched
        the removals in `convert.rs`, `resolve.rs`, and `scope.rs` to
        `shift_remove`, and built the property-union wrapper with its null
        arm already in place instead of removing and re-adding `anyOf`. As a
        result `anyOf` now leads that wrapper (before `default`/`description`),
        the same as for an optional single property. This re-recorded 20
        schema goldens and 1 conversion snapshot. Every changed golden was
        checked to be the same JSON with only key order changed. The
        alternative was to leave the swap order, which is deterministic and
        semantically harmless but does not match the code. A stronger option
        not taken: a clippy `disallowed-methods` ban on `Map::remove` across
        darkmatter, which would need a crate-wide audit. Reverting means
        restoring the three files and re-blessing the goldens with
        `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`.
    - |-
        `darkmatter/lib/src/markdown/schemas/resolve.rs` already carried
        another session's uncommitted edits (named-type work) when this phase
        started. My change there is nine one-word `remove` -> `shift_remove`
        hunks, separate from that work. Whoever commits this phase should
        expect the file to contain both sets of changes.
    - |-
        The plan and spec say `md frontmatter`, but the commands are top-level
        `md get`, `md set`, and `md rm`. The round-trip test drives those.
        `md set` treats a dotted name as a literal key, so nested edits are
        made by replacing the parent object; the test does exactly that.
message_to_agent: |-
    Phase 5 is done; `just test` in darkmatter is fully green (13768 passed).
    For Phase 6 (docs and final gates):
    - The 43 baseline failures were fixed by a parallel commit merged into
      this branch (58e2a5790), not by this fix; Phase 5 reviewed them and
      fixed the remaining swap-remove order in schema lowering (see the log).
    - For `docs/cli/hash.md`: `--strict --save` over a baseline saved without
      `--strict` (or vice versa) sees a changed hash and bumps
      `last_updated`, because a stored hash does not record its mode. That is
      the "normal save path" refresh for an affected strict baseline;
      `--diff` never rewrites anything. The mixed-mode case was observed by
      hand; the strict refresh and no-migration cases are tested in
      `darkmatter-cli::l1 frontmatter_key_order::*`.
    - The frontmatter commands are `md get|set|rm`, not `md frontmatter ...`;
      write docs with the real spelling.
    - `.claude/skills/darkmatter/schema.md` and several
      `darkmatter/docs/topics/schemas/*.md` pages carry another session's
      uncommitted edits; leave those hunks alone.
source_files_during_phase_3: [darkmatter/lib/Cargo.toml, darkmatter/lib/src/markdown/hash/mod.rs, darkmatter/lib/src/markdown/hash/compute.rs, darkmatter/lib/src/markdown/toc/mod.rs, darkmatter/lib/src/markdown/toc/types.rs, darkmatter/lib/src/markdown/schemas/validate.rs, darkmatter/lib/src/markdown/compose/cache/hashing.rs, darkmatter/lib/src/markdown/compose/cache/operation.rs, darkmatter/lib/src/markdown/compose/context/options.rs, darkmatter/lib/src/markdown/compose/expression/binding.rs, darkmatter/dmls/src/overlay/mod.rs]
docs_updated_during_phase_3: [darkmatter/docs/cli/hash.md]
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: [.claude/skills/darkmatter/frontmatter.md]
source_files_during_phase_4: [claudine/lib/src/composition/interpolation_conformance.rs, claudine/lib/src/composition/lifecycle/context/tests.rs, claudine/lib/src/composition/lifecycle/executor.rs, claudine/lib/src/composition/lifecycle/prewarm/classify.rs, claudine/lib/src/composition/preflight.rs, claudine/lib/src/composition/sequence/preflight/mod.rs, claudine/lib/src/composition/sequence/preflight/providers.rs, claudine/lib/src/composition/sequence/preflight/providers/tests.rs, claudine/lib/src/composition/sequence/preflight/reapproval.rs, claudine/lib/src/composition/sequence/task/tests.rs, darkmatter/lib/benches/phase6_interpolation.rs, darkmatter/lib/src/markdown/compose/cache/hashing.rs, darkmatter/lib/src/markdown/compose/conditions.rs, darkmatter/lib/src/markdown/compose/context/checked.rs, darkmatter/lib/src/markdown/compose/context/effective_state.rs, darkmatter/lib/src/markdown/compose/context/merge.rs, darkmatter/lib/src/markdown/compose/expression/fm_namespace.rs, darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs, darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs, darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs, darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs, darkmatter/lib/src/markdown/compose/inline/replacement.rs, darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs, darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs, darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs, darkmatter/lib/src/markdown/compose/page_blocks/engine.rs, darkmatter/lib/src/markdown/compose/preflight/collect.rs, darkmatter/lib/src/markdown/compose/replacement.rs, darkmatter/lib/src/markdown/compose/subtree.rs, darkmatter/lib/src/markdown/compose/tests/frontmatter.rs, darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs, darkmatter/lib/src/markdown/compose/transclusion/engine.rs, darkmatter/lib/src/markdown/compose/util.rs, darkmatter/lib/src/markdown/frontmatter.rs, darkmatter/lib/src/markdown/mapping_orders.rs, darkmatter/lib/src/markdown/reference/graph.rs, darkmatter/lib/src/markdown/schemas/clean.rs, darkmatter/lib/tests/l1/binding_contract.rs, darkmatter/lib/tests/l1/compose_expression_failure_contract.rs, darkmatter/lib/tests/l1/frontmatter_merge_order.rs, darkmatter/lib/tests/l1/main.rs, darkmatter/lib/tests/l1/shell_result_values.rs]
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: [.claude/skills/darkmatter/frontmatter.md]
source_files_during_phase_5: [darkmatter/lib/src/markdown/schemas/simplified/convert.rs, darkmatter/lib/src/markdown/schemas/resolve.rs, darkmatter/lib/src/markdown/schemas/scope.rs, darkmatter/lib/tests/l1/schema_json_key_order.rs, darkmatter/lib/tests/l1/main.rs, darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap, darkmatter/cli/tests/l1/frontmatter_key_order.rs, darkmatter/cli/tests/l1/main.rs, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-claudine.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-lifecycle.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence-types.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-style.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-sequence.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schema-yaml-spellings-root-union-conventional-schema-body.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schema-yaml-spellings-root-union-serialized-schema-body.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schemas-feature-review.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/validate-root-union-fileref.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/validate-root-union-none-match.json, darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/validate-root-union.json]
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: [.claude/skills/darkmatter/frontmatter.md]
---

# Implementation Log for 2026-10-23-frontmatter-key-order (6 phases)

## Phase 1

Read-only orientation: the S1 trace, the baseline test runs, and skill loading.
This phase changed no production or test code.

### S1: call-site trace (confirmed on the tree, 2026-10-23)

The decisions in N1-N10 stand. The trace found **more call sites than the plan
lists** for N1 and N8; each takes the same treatment as its listed siblings.

#### `canonical_json_sorted` (N1)

Defined at `darkmatter/lib/src/markdown/compose/cache/hashing.rs:199`
(`CanonicalJson` wrapper at :205, unit tests at :255-298). Callers:

- `compose/cache/hashing.rs` :46, :61, :164, :171 (listed in the plan)
- `compose/cache/operation.rs:77` (listed)
- **not listed:** `compose/context/options.rs` :2365 (`graph_context_fingerprint`),
  :2591/:2597, :2853/:2861/:2869/:2879, :3077, :3289/:3297/:3305/:3313/:3323
  (compose-options fingerprints)
- **not listed:** `compose/expression/binding.rs:771` (globals `identity()`)

All of these switch their import to `biscuit_hash::canonical_json` in Phase 3.
Because the bytes do not change, their keys stay identical.

#### `EffectiveState` API (N8)

- `EffectiveState.data: HashMap<String, Value>` (`effective_state.rs:146`),
  `data()` (:357), `EffectiveState::new(&HashMap, Option<&Value>, ComposeContext)`
  (:179), `EffectiveStateBuilder::with_frontmatter(HashMap)` (:633), and the
  `build` merge at :735.
- `markdown_child_options(parent, parent_data: &HashMap, ...)` at
  `compose/transclusion/engine.rs:1701`; its one caller is engine.rs:1482.

`EffectiveState::new` callers: `compose/context/checked.rs:90`, the
effective_state.rs unit tests (:804-:930), and Claudine
`lib/src/composition/lifecycle/executor.rs:1085`.

`EffectiveStateBuilder::with_frontmatter` callers in Darkmatter (production):
`compose/pipeline/mod.rs:728`, `compose/subtree.rs:313`,
`compose/replacement.rs` :248 and :458, `compose/conditions.rs:475` (plus
`HashMap::new()` at :560/:581), `compose/inline/replacement.rs:59`,
`compose/expression/lint.rs:541`, `compose/interpolation/evaluator.rs`
:547/:914, `compose/transclusion/engine.rs:1607`,
`compose/interpolation/rewrite.rs:439`, `compose/page_blocks/engine.rs`
:194/:258, `compose/interpolation/fatality_characterization.rs:166`,
`reference/graph.rs:344`, `compose/cache/hashing.rs:552`.
Tests and benches: `effective_state.rs` unit tests, `compose/tests/frontmatter.rs`,
`compose/tests/lookup_parity.rs:111`, `lib/tests/l1/compose_expression_failure_contract.rs:452`,
`lib/tests/l1/binding_contract.rs:26`, `lib/benches/phase6_interpolation.rs:88`,
`cli/tests/l1/compose_schema.rs:586`, and doc examples in `subtree.rs`,
`replacement.rs`, `evaluator.rs`.
Most call sites build their map with `HashMap::from`/`.into()`/`.collect()`, so
they only need the target type changed.

`.data()` consumers in Darkmatter: `compose/cache/hashing.rs:40`,
`compose/unknown_identifiers.rs:49`, `effective_state.rs:532`,
`transclusion/engine.rs:1482`, `compose/preflight/collect.rs:500`.

**Claudine is wider than the plan's "six sites".** `EffectiveStateBuilder::with_frontmatter`:

- `cli/src/commands/wrap/sequence/task_run.rs:79` (listed)
- `lib/src/composition/interpolation_conformance.rs:70` (listed)
- `lib/src/composition/preflight.rs:646` (listed)
- `lib/src/composition/lifecycle/executor.rs:1073` (listed; `EffectiveState::new` at :1085 is its second listed site)
- `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs` (listed; uses `EffectiveState`)
- **not listed:** `lib/src/composition/lifecycle/context/tests.rs:423`,
  `lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs:208`,
  `lib/src/composition/lifecycle/prewarm/classify.rs:205`,
  `lib/src/composition/sequence/task/sequence.rs:154`,
  `lib/src/composition/sequence/task/group.rs:804`,
  `lib/src/composition/sequence/task/templates.rs:47`,
  `lib/src/composition/sequence/preflight/runtime_fixture.rs:70`,
  `lib/src/composition/sequence/preflight/reapproval.rs:174`,
  `lib/src/composition/sequence/task/tests.rs` :389 and :5999

Claudine `.data()` consumers (each clones or re-collects the map, and most
re-feed it into `with_frontmatter`): `lib/src/composition/preflight.rs:1024`,
`sequence/task/group.rs:792`, `sequence/task/sequence.rs` :152/:187/:343/:457,
`sequence/preflight/mod.rs` :524/:556 (`loader.step_frontmatter`, an
`Arc<HashMap>` today), `sequence/task/mod.rs:525`, `sequence/task/templates.rs:42`.
Other Claudine files naming the types: `sequence/preflight/containers.rs`,
`invocation_context.rs`. Phase 4 must update them all and keep
`cargo check --tests -p claudine -p claudine-cli` clean.

#### Hash inputs fed by serialized JSON (Darkmatter lib + DMLS)

| Site | Purpose | Order today | Planned action |
|---|---|---|---|
| `markdown/hash/mod.rs:101` (`hash_frontmatter_map`) | non-strict frontmatter hash | nested order leaks | shared renderer (Phase 3) |
| `toc/mod.rs:441` | TOC normalized frontmatter hash | whole map order leaks | N4 |
| `schemas/validate.rs:1447` (`schema_content_hash`) | validator cache key | order leaks | N5 |
| **`schemas/validate.rs:1462` (`schema_digest`)** | second schema hash plus the `judges_files` text scan | order leaks | **not in the plan**; same treatment as N5 (canonical bytes for the hash; the `contains` scan is order-independent) |
| `dmls/src/overlay/mod.rs:1048` (`schema_cache_key`) | DMLS schema cache | already sorted (`SchemaConfig` is `bool` + `BTreeMap<String, {PathBuf, Vec<String>}>`) | N6 comment only |
| `compose/cache/hashing.rs:201` | compose cache keys | canonical already | N1 move |
| `reference/provenance.rs:98` | document represented-state identity | deliberately order-preserving (also hashes the raw frontmatter source) | keep; justify in the Phase 3 audit table |

Nothing else in `darkmatter/lib/src` or `darkmatter/dmls/src` serializes JSON
into a hash (35 `serde_json::to_*` calls checked; the rest are output or docs).

### Baseline run

Run on the unmodified tree, 2026-10-23, macOS.

- `darkmatter/`: `just test` stops at the first failure (fail-fast), so the
  baseline used `just test --no-fail-fast`: **10611 run, 10566 passed, 45
  failed, 10 skipped** (87.9 s). The count matches the spec draft's 45.
- `biscuit-hash/`: `just test`: **74 run, 74 passed**.

#### Baseline failures (darkmatter, 45)

Grouped: 1 frontmatter metadata test, 1 benchmark manifest identity, 1 JSON5
formatter, 1 legacy schema-validate byte comparison, 40
`schemas_convert_snapshots::*`, 1 `suggest_constraint_phase3` snapshot. Every
group is named in the plan's Phase 5 review list, plus the benchmark manifest
test that Phase 3 must turn green without editing the manifest.

- `darkmatter markdown::frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`
- `darkmatter::l1 benchmark_fixtures::benchmark_manifest_matches_recorded_identities`
- `darkmatter markdown::compose::expression::functions::tests::fn_list_formatting::as_json5_uses_the_formatter_object_idiom`
- `darkmatter-cli::l1 schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_with_description`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_mixed_with_string_fallback`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_time`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_string_min_max`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_url_bare`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_root_union_inline_only`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_as_union_arm`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_string_not_empty`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_string_pattern`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_date`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_url_scheme`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_datetime`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_string`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_default_and_required_atoms`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_full_document_schema`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_enum_with_default_and_required`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_property_union_hoists_matching_default`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_any`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_enum_members`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_file_bare`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_object`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_constrained_array`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_nested`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_required_value`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_property_union_hoists_required`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_array_with_description`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_file_match`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_boolean`
- `darkmatter::l1 suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_array`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_number`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_empty`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_numberlike`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_array_string_with_pattern_and_array_constraints`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_bare`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_number_integer_with_min`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_file_array_min_items`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_boolish`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_atom_email`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_inline_object_multi_line`
- `darkmatter::l1 schemas_convert_snapshots::snapshot_property_union_basic`

### Phase gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `change_class: package`, `packages: [biscuit-visualized, worktree,
  worktree-cli, test-toolkit, darkmatter, darkmatter-cli, repo-deps, sniff]`.
  The working tree also holds the branch's uncommitted merman work. This
  phase's two files alone give `packages: [darkmatter]`, `change_class:
  package`. Gates were therefore run, not skipped.
- `biscuit-visualized/`: `just test` 976 run, 976 passed, 3 skipped;
  `just lint` clean (exit 0).
- `darkmatter/`: the baseline run above is this phase's darkmatter run. Its
  45 failures are pre-existing (this phase changed no code); Phases 3-5 fix them.
- No dependent-package or cross-OS run: this phase changed no code.

### Skills loaded

`darkmatter`, `biscuit-hash`, `rust-testing`, `os`, `biscuit-visualized`
(session requirement). `rust-devops` not loaded: CI scope is not touched.

### Hash audit

Seeded by the S1 table above; Phase 3 completes it.

### Output updates

None yet (Phase 5).

### Strict-baseline correction

None yet (Phase 5).

### Departures

- S1 found call sites the plan does not list (see above); they take their
  siblings' treatment, so no decision changes.

## Phase 2

biscuit-hash `json` feature: the shared canonical-JSON renderer and its hash
conveniences. Darkmatter is not wired to it yet (Phase 3).

### What landed

- `biscuit-hash/lib/Cargo.toml`: `json = ["dep:serde", "dep:serde_json"]`
  with optional `serde` 1 and `serde_json` 1. It enables neither `xx_hash`,
  `blake3`, nor serde_json's `preserve_order`. A dev-dependency on
  `serde_json` with `preserve_order` lets the crate's own tests build maps in
  an explicit insertion order.
- `biscuit-hash/lib/src/json.rs` (new, `#[cfg(feature = "json")]`):
  - `canonical_json(&Value) -> String`: Darkmatter's `canonical_json_sorted` /
    `CanonicalJson` ported byte-for-byte (same `Serialize` wrapper, keys
    sorted by `str` order via borrowed `Vec<(&String, &Value)>`, compact
    `serde_json::to_writer`). The `unwrap()` / `unwrap_or_default()` pair is
    now two `expect`s with invariant messages (N1: infallible by
    construction, no `Result`).
  - `xx_hash_json(&Value) -> u64` (`json` + `xx_hash`) =
    `xx_hash_bytes(canonical_json(v).as_bytes())`.
  - `blake3_hash_json(&Value) -> String` (`json` + `blake3`) =
    `blake3_hash(&canonical_json(v))`, the existing 64-char hex return type.
- `biscuit-hash/lib/src/lib.rs`: module and re-exports gated per feature;
  crate docs list the `json` feature.
- `biscuit-hash/justfile`: `lint` now runs clippy with `--all-features` (see
  decisions_for_review).

### Requirement-to-test mapping

All in `biscuit-hash/lib/src/json.rs` `tests` (L1, F1 in-process at the
crate's public API; the boundary is the crate's exported functions). They run
under `just test` (`--all-features`): 13 tests, all passing.

| Requirement (plan Phase 2 Wave 3) | Test |
|---|---|
| ported behavior (moved Darkmatter tests) | `canonical_json_sorts_keys`, `canonical_json_preserves_array_order`, `canonical_json_handles_nested_objects`, `canonical_json_scalars` |
| recursive object-order independence (incl. objects inside arrays) | `object_key_order_is_ignored_at_every_depth` (fixture asserted to differ in insertion order first) |
| exact canonical bytes, representative fixture | same test, plus `canonical_json_sorts_keys` |
| array-order sensitivity | `array_element_order_is_significant` |
| empty `{}` / `[]` | `canonical_json_empty_containers` |
| scalar roots | `canonical_json_scalars` |
| Unicode keys | `unicode_keys_sort_by_str_order` |
| escaped strings | `strings_are_escaped_like_serde_json` |
| equals compact `serde_json::to_string` for sorted input | `sorted_input_matches_compact_serde_json` |
| input iteration order unchanged (`preserve_order` on) | `input_iteration_order_is_unchanged` |
| hash conveniences hash the renderer's bytes | `xx_hash_json_hashes_the_canonical_bytes`, `blake3_hash_json_hashes_the_canonical_bytes` |

Doctest: `json::canonical_json` example. The Input Robustness Matrix does not
apply (a renderer, not a reader).

The Darkmatter copies of the four moved tests stay until Phase 3 deletes
`canonical_json_sorted` with them; deleting them now would leave Darkmatter's
still-live function untested.

### Checkpoint 2

- `cargo check -p biscuit-hash --no-default-features --features json`: clean
- same with `json,xx_hash`, `json,blake3`, and default features: clean
- `cargo check -p biscuit-hash --tests --no-default-features --features json`: clean
- `cargo clippy -p biscuit-hash --lib --tests --no-default-features --features json -- -D warnings`: clean
  (`--all-targets` there fails on the pre-existing bench, which needs `xx_hash`)
- `just test` in `biscuit-hash/`: 87 run, 87 passed (baseline 74; +13 json)
- `just lint` in `biscuit-hash/`: clean, now `--all-features`
- `just doctest` in `biscuit-hash/`: clean
- Pre-existing, not fixed: `cargo test --doc --no-default-features --features
  json` (or `json,blake3`) fails on the crate-level `lib.rs` example, which
  imports `xx_hash` without a feature gate. No recipe or CI cell runs doctests
  without default features; it is unrelated to this fix.

### Phase gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `change_class: package`, `packages: [biscuit-hash, test-toolkit,
  darkmatter, sniff]`. Gates were run, not skipped.
- `biscuit-hash/`: `just test` 87/87 passed; `just lint` clean.
- `darkmatter/`: `just test --no-fail-fast`: **10611 run, 10566 passed, 45
  failed, 10 skipped**. The failing set was diffed against the Phase 1
  baseline list and is **identical** (no new failure, none resolved; this
  phase does not touch Darkmatter). Phases 3-5 own those 45.
  `just lint` clean.
- Dependents: nothing enables `json`, so dependents build exactly as before.
  `cargo check -p sniff -p test-toolkit --tests` clean (the two non-Darkmatter
  packages the scope calculator named).
- No cross-OS run: pure in-memory string rendering, no paths, processes, or
  `cfg(windows)` code.

### Departures

- Plan's feature wiring named only `serde_json`; the ported `CanonicalJson`
  wrapper implements `serde::Serialize`, so `json` also enables an optional
  `serde` 1 (already in the graph). See decisions_for_review.
- `biscuit-hash/justfile` `lint` changed to `--all-features` (decision below).
- The moved unit tests were copied, not yet removed from Darkmatter; Phase 3
  removes them with `canonical_json_sorted`.

## Phase 3

Darkmatter now requests `preserve_order` itself, and every order-insensitive
hash renders through biscuit-hash's `canonical_json`.

### What landed

- `darkmatter/lib/Cargo.toml`: `serde_json = { version = "1.0", features =
  ["preserve_order"] }` with a contract comment; `biscuit-hash` gains `json`
  next to `blake3`.
- `markdown/hash/mod.rs` `hash_frontmatter_map`: only the per-value rendering
  changed (`serde_json::to_string` -> `canonical_json`). Top-level key sort,
  `key:` prefix, literal two-character `\n` join, `xx_hash`, the empty map
  sentinel, ignore filtering, and the strict YAML path are unchanged. Doc
  comment now states recursive normalization and that strict follows author
  order. `hash_frontmatter_keys`'s comment implies nothing about nested maps,
  so it was left alone.
- `markdown/toc/mod.rs`: normalized hash is `xx_hash_json` of the whole
  frontmatter object (N4); empty-frontmatter sentinel (`0`, no assignment) and
  the raw hash unchanged. Field doc in `toc/types.rs` updated.
- `markdown/schemas/validate.rs`: `schema_content_hash` is `xx_hash_json`
  (N5); `schema_digest` (Phase 1's extra site) hashes `canonical_json` bytes
  and runs its `judges_files` substring scan on the same text. A doc block for
  `canonical_hash` had drifted above `schema_content_hash`; moved it back onto
  `canonical_hash` (comment-only).
- `dmls/src/overlay/mod.rs` `schema_cache_key`: comment only (N6).
- Compose cache (N1): `canonical_json_sorted` and `CanonicalJson` deleted from
  `compose/cache/hashing.rs` with their five unit tests (the four the Phase 2
  agent named plus `canonical_json_empty_object`; all are covered in
  `biscuit-hash/lib/src/json.rs`). Callers switched to
  `biscuit_hash::canonical_json`: `cache/hashing.rs`, `cache/operation.rs`,
  `context/options.rs` (all 15), `expression/binding.rs`.
- `cargo tree -e features -i serde_json -p darkmatter` (acceptance 7) shows:

  ```text
  │   └── serde_json feature "preserve_order"
  │       ├── darkmatter v0.1.0 (.../darkmatter/lib) (*)
  │       └── merman-core v0.9.1 (...)
  ```

  i.e. Darkmatter requests the feature directly, no longer only through merman.

### Hash audit (complete)

| Hash | Purpose | Order matters? | Action |
|---|---|---|---|
| `hash_frontmatter_map` non-strict | `md hash` semantic frontmatter | no | per-value `canonical_json` |
| `hash_frontmatter_map` strict | `md hash --strict` | yes (author order) | unchanged; nested order now observable (N9) |
| `hash_frontmatter_keys` | keys-only component | non-strict no / strict top-level only | unchanged |
| TOC `frontmatter_hash_normalized` | TOC comparison | no | `xx_hash_json` of whole map (N4) |
| TOC `frontmatter_hash` | raw YAML identity | yes | unchanged |
| `schema_content_hash` | structural validator cache key | objects no, arrays yes | `xx_hash_json` (N5) |
| `schema_digest` | context validator cache key + file-judging scan | objects no, arrays yes | `canonical_json` bytes |
| DMLS `schema_cache_key` | DMLS schema cache | already sorted (`bool` + `BTreeMap`) | comment only (N6) |
| compose cache keys (`effective_state_hash`, `context_hash`, operation keys, options fingerprints, globals identity) | compose caches | no | already canonical; renderer moved to biscuit-hash, bytes identical |
| `reference/provenance.rs` represented-state identity | document identity that also hashes the raw frontmatter source | yes, deliberately | keep: it pairs with the raw source bytes, so a reorder is a real represented-state change |
| `cicd_list` / `pr_list` provider cache keys | per-request provider query cache | n/a | typed query structs, deterministic field order; no change |
| `capture/capabilities.rs` `for_document` | text scan for `current.` / function names, not a hash | no | no change |

All other `serde_json::to_*` calls in `lib/src` and `dmls/src` are output,
tests, or doc examples.

Byte-identity evidence for the compose cache move: every `compose::cache::`,
`compose::context::options`, and `expression::binding` test passes unchanged,
and `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`
passes with the manifest unchanged (`replace_heavy` = `9e3319d168b8b47a`).

### Requirement-to-test mapping

Boundary: the crate's public hash results (`Markdown::compute_hash`,
`Markdown::hash_frontmatter`, `MarkdownToc::from`, `ValidatorCache`), fed by
parsed YAML/JSON text. F1, in-process, L1 (unit `#[cfg(test)]` modules, run
by `just test`). No CLI spawn: Phase 5 owns the `md hash` process tests.

| Requirement | Test |
|---|---|
| Acceptance 1, fixed historical entry format | `markdown::hash::compute::tests::non_strict_frontmatter_hash_keeps_the_historical_entry_format` (expected `xx_hash("a:[2,1]\\nb:{\"x\":2,\"y\":1}")`, pinned hex `26b30b2982c8c394`, through helper, legacy method, and `Fm` kind) |
| Acceptance 1, benchmark manifest unchanged | `darkmatter::l1 benchmark_fixtures::benchmark_manifest_matches_recorded_identities` (Phase 1 baseline failure, now passing) |
| Acceptance 2, three reorder pairs; non-strict equal, strict differ, across helper + legacy + `fm`/`simple`/`structured`/`detailed` frontmatter components; nested-only pairs keep strict keys-only equal; top-level pair changes strict keys-only; body outputs unaffected | `markdown::hash::compute::tests::frontmatter_hash_matrix_ignores_key_order_only_when_non_strict` |
| Array element order stays significant (scalar and object arrays) | `markdown::hash::compute::tests::non_strict_frontmatter_hash_changes_on_array_element_reorder` |
| Acceptance 4, TOC: top-level, nested, array-contained reorder ignored; array reorder changes | `markdown::toc::tests::normalized_frontmatter_hash_ignores_key_order_at_every_depth` |
| TOC bytes for sorted input equal compact JSON (old stored values for sorted documents unchanged) | `markdown::toc::tests::normalized_frontmatter_hash_matches_compact_json_for_sorted_frontmatter` |
| TOC empty-map sentinel | `markdown::toc::tests::normalized_frontmatter_hash_is_zero_for_empty_frontmatter` |
| Acceptance 4, schema: reordered objects share a validator, reordered arrays do not (structural path, `schema_content_hash`) | `markdown::schemas::validate::tests::reordered_schema_keys_share_a_structural_validator` |
| Same, context-keyed path (`schema_digest`) | `markdown::schemas::validate::tests::reordered_schema_keys_share_a_context_validator` |

Each fixture asserts it really differs in key order (or that the parsed data
are equal) before asserting the hash, so the reorder is load-bearing. The
Input Robustness Matrix does not apply (no reader changed).

Targeted run: `cargo nextest run -p darkmatter --lib -E 'test(/markdown::hash::/)
| test(/markdown::toc::/) | test(/schemas::validate::/) | test(/compose::cache::/)
| test(/compose::context::options/) | test(/expression::binding/)'`: 402 passed.

### Docs and skill

- `darkmatter/docs/cli/hash.md` "Normalization Behavior": the non-strict and
  strict frontmatter lines now state recursive normalization and author order
  (the old lines became wrong with this phase). Phase 6 still owns the fuller
  hash doc work (strict-baseline refresh guidance).
- `.claude/skills/darkmatter/frontmatter.md`: new "Key Order and Hashing"
  section.

### Departures

- Deleted five, not four, Darkmatter canonical-JSON unit tests: the fifth
  (`canonical_json_empty_object`) tested the same deleted function and is
  covered by biscuit-hash's `canonical_json_empty_containers`.
- Updated `docs/cli/hash.md` now rather than waiting for Phase 6, because this
  phase made two of its lines wrong.

### Phase gates

- `darkmatter/`: `just test --no-fail-fast` (run from the worktree):
  **10620 run, 10576 passed, 44 failed, 10 skipped** (120 s). The failing set
  was diffed against the Phase 1 baseline list: **no new failures**; one
  resolved (`benchmark_fixtures::benchmark_manifest_matches_recorded_identities`).
  The other 44 are pre-existing and on Phase 5's review list (frontmatter
  metadata test, JSON5 formatter, legacy schema-validate byte comparison, 40
  `schemas_convert_snapshots::*`, `suggest_constraint_phase3`). The count is 9
  above Phase 2's 10611: this phase adds 8 tests and deletes 5 (net +3); the
  other +6 come from the concurrent dmls work on the shared tree.
- `darkmatter/`: `just lint` clean (exit 0, worktree paths only).
- Dependents (serial, scoped to code that reads the changed hashes; no public
  API changed, and `preserve_order` was already unified into every workspace
  build via merman):
  - `cargo nextest run -p claudine --lib -E 'test(/composition::closure/) |
    test(/composition::completion/) | test(/composition::prepare/) |
    test(/composition::types/) | test(/linking::compatibility/)'`: 227 passed
  - `cargo nextest run -p claudine-cli -E 'test(/wrap_inline_compose/) |
    test(/agent_text_is_data/) | test(/commands::wrap::inline/)'`: 62 passed
  - `cargo nextest run -p messenger -E 'test(/research/)'`: 1 passed
- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `change_class: package`, `packages: [darkmatter]`. Gates run, not
  skipped. By then the tree held only plan/log edits, because another
  process had already committed this phase's code (below).
- No cross-OS run: in-memory hashing and string rendering only; no paths,
  processes, or `cfg` code.

### Environment notes

- In this shell `CDPATH` includes `/Users/ken/coding/personal/rusty-biscuit`,
  so a relative `cd darkmatter` lands in the **main checkout**. One audit grep
  and one first `just lint` ran there by mistake. Both were redone against
  the worktree with absolute paths, and no file outside the worktree was
  edited. (The main checkout has an unrelated, pre-existing
  `darkmatter/lib/Cargo.toml` edit that is not from this fix.)
- A concurrent process working the dmls incremental-graph feature in the same
  worktree committed the whole working tree at 18:03, this phase's code
  included (`bbde223e6 fix(darkmatter): make semantic hashes key-order
  insensitive`, `09d4679d0 planning(darkmatter): record Phase 3 close ...`).
  This phase made no commits. The plan/log frontmatter and these final log
  entries are the only uncommitted Phase 3 changes.

## Phase 4

Merge order: the receiving document fixes key order in every merge, and the
compose state carries ordered maps end to end. Darkmatter and Claudine compile
and pass against the new API.

### What landed

- **Recursive merge precedence (N7).** `deep_merge(base, overlay, ConflictWinner)`
  in `compose/context/effective_state.rs`. `base` always fixes key order at
  every depth; `ConflictWinner::{Base, Overlay}` only picks which side's value
  survives a non-object conflict. The null rule is unchanged: a `null` on the
  winning side yields to the other side. `deep_merge_override` (literal null,
  used by `set` overrides) already had the child as base and is untouched.
  Argument reversals removed:
  - `EffectiveStateBuilder::build` `PreferDocument` (was `deep_merge(external, fm)`)
  - `EffectiveState::new` (same reversal)
  - **not in the plan:** `compose/util.rs` `prepare_frontmatter_for_compose`,
    which writes external state into the document's own frontmatter in place
    (`deep_merge(external, &current)`). This is the merge that orders a
    transcluded child's frontmatter, so it was the main order defect.
  - `merge_replace_maps` and the `ctx` merge in `context/merge.rs` already had
    the right base; they only switched to the new signature.
- **Ordered maps (N8).** `EffectiveState::data()`, its field,
  `EffectiveState::new`, `EffectiveStateBuilder::with_frontmatter`, and
  `markdown_child_options`' `parent_data` are `serde_json::Map`. The same
  treatment, beyond the plan, for the other order-observable path found in the
  audit: `FrontmatterSeedState` (bare `fm` during frontmatter interpolation)
  and the three public frontmatter shell APIs that feed it
  (`ResolvedShellValue::resolve`, `execute_resolved_shell_values`,
  `execute_shell_values_with`). `fm_namespace::resolve_fm_namespace_in_map`
  became identical to `resolve_fm_namespace_in_object` and was deleted.
- **Bare `fm`** (`EffectiveState::get_fm`, `FmPath::Whole`) swaps the
  document's own `ctx` in at its authored position instead of re-appending it
  at the end.
- **Swap-remove trap.** Under `preserve_order`, `serde_json::Map::remove` is a
  swap remove: the last key jumps into the removed slot. Switched to
  `shift_remove` in `markdown_child_options` (`prologue`/`epilogue`/`ctx`),
  preflight `collect.rs` (`$schema`), and `schemas/clean.rs` `parse_instance`
  (`$schema`). `md frontmatter rm` already used `shift_remove`.
- **Shallow merges** (`Frontmatter::merge_with`, `set_defaults`) were already
  correct on the `IndexMap`; tests now pin the contract and the docs state it.
- **Doc drift fixed:** `EffectiveState::new` said "Uses `PreferExternal`" but
  the code made frontmatter win; the code is right and the doc now says so.
  `MappingOrders` and `mapping_key_order` docs claimed ordinary JSON maps keep
  canonical order; they now say parsed values agree with the index.
- **Claudine** (N8): 10 files. Every site that built a `HashMap` from an
  ordered map now passes the `Map` directly. Beyond the plan's six sites, the
  sequence preflight loader's `step_frontmatter` (`Arc<Map>`), the
  reapproval basis, and the provider-hint `classify_reads`/`Evaluator` chain
  switched to `Map`, because they feed `with_frontmatter` and
  `ResolvedShellValue::resolve`.

### Unordered-map audit

- `rg 'HashMap<String, (serde_json::)?Value>'` over darkmatter lib/cli/dmls
  source (non-test): the remaining sites are lookup-only and never reach
  output: `presentation_values` (EffectiveState, schema validation,
  evaluator), the `ctx`/binding/`current` caches and memo tables, and
  frontmatter interpolation's per-key `authored_values`/`original_values`/
  `authored_templates` (always walked in `templated_keys` Vec order).
- `.remove(` on order-observable maps: fixed as above. Left for Phase 5, not
  frontmatter: `schemas/simplified/convert.rs` (lines ~288-571, 1521) and
  `schemas/resolve.rs` `strip_schema_uri` and `required` removal swap-remove
  keys out of JSON Schema objects; those feed the `schemas_convert_snapshots`
  outputs on Phase 5's review list. `context/options.rs:1173` removes then
  reinserts on purpose (the last layer's position wins).
- Checkpoint grep for `into_iter().collect()` on frontmatter objects: none
  left on an order-observable path.

### Requirement-to-test mapping

| Requirement | Test(s) | Rung / tier |
|---|---|---|
| Shallow: base positions, incoming appended in order, overwritten key never moves (`{z,a}`+`{a,b}` → `z,a,b`, both values) | `frontmatter::tests::merge_with_never_moves_an_overwritten_key_and_appends_incoming_keys`, `merge_with_appends_incoming_only_keys_in_incoming_order` | F1 / L1 unit |
| Shallow: `ErrorOnConflict` still errors | `merge_with_error_on_conflict_still_errors_on_an_ordered_map` | F1 / L1 |
| Shallow: nested object replaced whole in incoming order | `merge_with_replaces_a_nested_object_whole_in_incoming_order` (base parsed from YAML) | F1 / L1 |
| `set_defaults` | `set_defaults_keeps_existing_positions_and_appends_missing_keys_in_order` | F1 / L1 |
| Metadata agrees with value order | `frontmatter::tests::parsed_frontmatter_mapping_order_metadata_agrees_with_value_order` (replaces the baseline-failing `parsed_frontmatter_retains_nested_mapping_order_as_metadata`, which asserted sorted order) | F1 / L1 |
| Recursive: base fixes order at every depth for both winners; null rule | `effective_state::tests::deep_merge_base_fixes_order_at_every_depth_whichever_side_wins` | F1 / L1 |
| Value precedence unchanged (scalar, null either side, arrays as leaves, object/scalar, root null) | `deep_merge_base_winner_matches_swapped_overlay_values` (11 cases vs the old swapped call) | F1 / L1 |
| Document/external merge, every strategy | `builder_keeps_document_order_under_every_strategy`, `effective_state_new_keeps_document_order_and_document_values` | F1 / L1 |
| Bare `fm` order, authored `ctx` in place, runtime `ctx` not leaked | `bare_fm_renders_document_order_with_authored_ctx_in_place` | F1 / L1 |
| `set` / `set.NAME` overrides, literal null | `apply_set_overrides_keeps_child_order_and_appends_new_keys` | F1 / L1 |
| `replace` precedence | `merge_replace_maps_keeps_base_order_and_overlay_values` | F1 / L1 |
| End to end, real compose pipeline: external state; transclusion inheritance (`prologue` not inherited); directive `set.NAME` | `l1 frontmatter_merge_order::*` (3 tests, 4 runs each so a random order cannot pass by chance; asserts composed frontmatter keys, nested keys, winning values, and the rendered `{{ fm }}` text) | F2 / L1 integration |

All three end-to-end tests were confirmed to **fail** with the old
`util.rs` argument order restored and pass with the fix. The
`builder_keeps_document_order_under_every_strategy` case fails on the old
`PreferDocument` reversal by construction.

### Phase gates

- `just lint` (darkmatter): clean.
- `just test --no-fail-fast` (darkmatter): **10643 run, 10600 passed, 43
  failed, 10 skipped**. The 43 are exactly the Phase 3 list (44) minus the
  metadata test fixed here; all are on Phase 5's review list (JSON5
  formatter, legacy schema-validate byte comparison, 40
  `schemas_convert_snapshots::*`, `suggest_constraint_phase3`).
- `cargo test -p darkmatter --doc`: 203 passed (`just test` does not run
  doctests; one stale `HashMap` doc example in `replacement.rs` was caught
  and fixed this way).
- `just check-tier-coverage darkmatter`: nothing stranded.
- `cargo check -p claudine -p claudine-cli --tests`: clean.
- Claudine targeted tests (dependents of the changed API):
  `cargo nextest run -p claudine --lib -E 'test(/composition::(interpolation_conformance|lifecycle::context|lifecycle::executor|lifecycle::prewarm|preflight|sequence::preflight|sequence::task)/)'`
  → 682 passed. `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/loop_control|task_run|sequence|codex_app_server|document_environment_routes/)'`
  → 1310 passed. (A first run without `--features test-fixtures` showed 5
  failures from missing fixture binaries; the claudine `test` recipe always
  passes that feature, and with it all pass.)
- Gate-skip calculator over the uncommitted tree:
  `packages: [darkmatter, test-toolkit]`, `change_class: package`. Gates run,
  not skipped.
- No cross-OS run: in-memory map ordering only; no paths, processes, or `cfg`.

### Environment notes

- The concurrent process committed this phase's code mid-phase at 20:15
  (`d15577cd6 fix(darkmatter): preserve authored frontmatter key order`,
  `183a542da planning(darkmatter): record Phase 4 close ...`). This phase made
  no commits. Uncommitted Phase 4 changes at the end: the `replacement.rs`
  doctest fix, the darkmatter skill edit, and the plan/log frontmatter and
  log entries.

## Phase 5

Failing-output review and CLI process tests. Run 2026-10-29 on macOS.

### Starting state

The 43 review-list failures left by Phase 4 were already passing on the
current tree. A parallel commit merged into this branch after Phase 4,
`58e2a5790 feat(darkmatter): preserve authored order and validate numeric
ingress` (2026-10-22), updated them. Each group was reviewed against the code
rather than accepted as given:

| Test(s) | Resolution | Justification |
|---|---|---|
| `fn_list_formatting::as_json5_uses_the_formatter_object_idiom` | expected text updated by 58e2a5790 | `as_json5` now prints `{package, 'dev deps'}`, the order the `json!` literal writes; before, it was alphabetical only because maps were sorted |
| `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical` | code fix in 58e2a5790; expected fixtures untouched | `md schema validate --format json` sorts its report (`sort_all_objects` in `cli/src/commands/schema/validate.rs`), keeping the published alphabetical byte output. Both routes (the legacy fixture and the command) agree without editing expected text |
| `schemas_convert_snapshots::*` (40), `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata` | snapshots updated by 58e2a5790; `$schema` placed first in `to_json_schema` | Each new snapshot lists keys in the order `convert.rs` inserts them (for example `type` then `format`; `type, items, uniqueItems, minItems, maxItems`). Serialization is deterministic (insertion-ordered `IndexMap`), so nothing was random. Verified by rerunning with every removal switched to `shift_remove`: no snapshot changed |
| `benchmark_manifest_matches_recorded_identities`, frontmatter metadata test | resolved in Phases 3 and 4 | see those sections |

### Code fix: swap removes in schema lowering

The review found the order defect the Phase 4 handoff predicted, on paths the
convert snapshots do not reach. Under `preserve_order`, `Map::remove` is a swap
remove. Probes through the public API showed two misorders:

- **Root-union arms** (`resolve_yaml_schema` on a `$schema` list):
  `strip_schema_uri` removes the arm's leading `$schema`, so the arm's last
  key (`required`) jumped to the front:
  `{"required":["title"],"type":"object",...}`.
- **Property-union arms** with a description and annotations: hoisting
  `default` off the arm moved `x-darkmatter-example` ahead of `description`,
  and the wrapper came out as `x-darkmatter-generated, default, anyOf`.

Fix:

- `schemas/simplified/convert.rs`: every `Map` removal is `shift_remove`.
  `union_property_to_schema` puts the `null` arm into the arms vector before it
  builds the wrapper, instead of removing and re-inserting `anyOf`. The
  wrapper therefore leads with `anyOf` and then `default`/`description`/
  `x-darkmatter-generated`, matching an optional single property.
- `schemas/resolve.rs` (`strip_schema_uri`, the `$defs` hoists, `merge_layer`'s
  `properties`/`required`, `merge_baseline`'s `$defs`, example rehoming) and
  `schemas/scope.rs` (`fold_definitions`): `shift_remove`.
- Not changed: `schemas/validate.rs` `arm_with_assertions_beside` builds an
  internal validator schema that is never shown, and the remaining `.remove`
  calls in `schemas/` are on `Vec`, `HashSet`, or cache maps.

Output updates caused by the fix:

- 1 conversion snapshot (`snapshot_property_union_hoists_matching_default`):
  `default` moves after `anyOf`.
- 20 `recursive_schema_golden` goldens (14 full JSON, 6 digests). To prove
  these are order-only, the full lowered JSON of all 104 manifest entries was
  dumped once under the old code and once under the new. 84 were
  byte-identical; the 20 that differ compare equal as parsed JSON (objects
  unordered, arrays ordered). None differed in content. The moved keys
  were `default`/`description` (the union wrapper change) and
  `required`/`additionalProperties` (the root-union swap). The goldens were
  then re-blessed with `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`.

### CLI process tests

The plan says `md frontmatter`, but the commands are `md get`, `md set`, and
`md rm`. `md set` treats a dotted name as a literal key, so a nested edit
replaces the parent object.

### Strict-baseline correction

Strict hashes follow author key order at every depth, and a stored strict
baseline is not migrated. After a nested-only reorder, `md hash --strict
--diff` exits 2 ("Frontmatter has changed, body remains unchanged
semantically") and leaves the file untouched. `md hash --strict --save`
refreshes it, and a following `--strict --diff` exits 0. Observed by hand
and not asserted: a stored hash does not record its mode, so `--strict --save`
over a non-strict baseline also counts as a change and bumps `last_updated`.

### Requirement-to-test mapping

| Requirement | Test(s) | Rung / tier |
|---|---|---|
| Review-list outputs follow construction order | the 43 review-list tests (passing), plus the `shift_remove` rerun showing no snapshot depends on swap order | F1 / L1 |
| Root-union arm keeps its lowered key order (regression; fails with swap remove) | `darkmatter::l1 schema_json_key_order::root_union_arms_keep_their_lowered_key_order` | F2 / L1, public `resolve_yaml_schema` |
| Union wrapper and arm keep their built key order (regression; fails with swap remove) | `schema_json_key_order::property_union_arm_and_wrapper_keep_their_built_key_order` | F2 / L1, public `to_json_schema` |
| Shipped schemas lower deterministically, order-only change | `recursive_schema_golden::*` (17 tests, 104 entries, real shipped schema files) | F2 / L1 |
| `md get/set/rm` round trip, repeated: nested and top-level order kept, replaced key keeps its position, new key appends, nested object replace, `rm` closes the gap, file returns byte-identical to the original | `darkmatter-cli::l1 frontmatter_key_order::get_set_rm_round_trip_keeps_author_order_at_every_depth` (2 passes) | F3 / L1 process |
| `md hash --save`/`--diff` after a nested-only reorder: no semantic change, file not rewritten, no `last_updated` bump | `frontmatter_key_order::hash_baseline_ignores_a_nested_only_reorder` | F3 / L1 process |
| Strict hash follows nested order; strict baseline not migrated; refresh via save | `frontmatter_key_order::strict_hash_follows_nested_author_order_and_is_refreshed_only_by_save` | F3 / L1 process |

Both new `schema_json_key_order` tests were run as probes against the old
code first; each showed the misordered output.

### Phase gates

- `just lint` (darkmatter): clean.
- `just test` (darkmatter): **13768 run, 13768 passed, 10 skipped**. No
  failures remain from the Phase 1 baseline.
- `just check-tier-coverage darkmatter`: nothing stranded.
- Dependents (Claudine reads lowered schemas):
  `cargo nextest run -p claudine -E 'test(/schema/)'` → 1091 passed;
  `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/schema/)'`
  → 266 passed.
- No public API changed, so doctests were not rerun.
- No cross-OS run: the changes are in-memory map ordering and process tests
  over temp files, with no path, process, or `cfg` differences.

### Environment notes

- `darkmatter/lib/src/markdown/schemas/resolve.rs` and several other files
  already carried another session's uncommitted edits when the phase
  started (see `git status` at start). This phase added only the
  `shift_remove` hunks to `resolve.rs` and touched none of the others.
