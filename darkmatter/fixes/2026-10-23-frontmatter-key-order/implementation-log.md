---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/fixes/2026-10-23-frontmatter-key-order/spec.md
plan: darkmatter/fixes/2026-10-23-frontmatter-key-order/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
message_to_agent: |-
    Phase 1 (read-only) is done; its findings are in the implementation log under
    "## Phase 1". Read that section before Phase 3 and Phase 4:
    - `canonical_json_sorted` has callers the plan does not list:
      compose/context/options.rs (graph_context_fingerprint and the
      compose-options fingerprints, ~15 calls) and compose/expression/binding.rs:771.
      Switch them all to `biscuit_hash::canonical_json` when the local copy is
      deleted in Phase 3.
    - schemas/validate.rs has a second schema hash, `schema_digest` (~:1462),
      next to `schema_content_hash`; give it the N5 treatment (see
      decisions_for_review).
    - reference/provenance.rs:98 hashes the frontmatter map in insertion order
      on purpose (document represented-state identity, also hashes the raw
      source); justify it as order-sensitive in the Phase 3 audit table.
    - N8 is much wider in Claudine than the plan's "six sites": 14
      `EffectiveStateBuilder::with_frontmatter` sites plus ~10 `.data()`
      consumers (including `loader.step_frontmatter`, an `Arc<HashMap>`).
      All are listed in the log.
    - Baseline: darkmatter `just test` fails fast; use
      `just test --no-fail-fast` to compare against the 45-test baseline list.
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
