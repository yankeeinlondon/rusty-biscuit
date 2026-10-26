---
spec: /Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/claudine/fixes/2026-10-25-fix-now/spec.md
plan: claudine/fixes/2026-10-25-fix-now/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 (P1.1) was inventory only; no source changed. Read "## Phase 1" of this log before starting Phase 2: it lists every consumer of the changed public types (Consumer inventory), the six runtime Box::leak sites P2.1 must remove, the seven DMLS LifecycleHost::for_document call sites that must take the one shared host, and the existing tests that each acceptance item can extend. The predecessor 2026-10-26-dmls-frontmatter-typing-allocation is fully implemented (commits 457b1a51a, cf25e93a1), so no prerequisite work is needed. Design decision P1-D1: check_command_blacklists (claudine/lib/src/harness/shell.rs, the pre-flight shape-approved check) also calls SessionPolicy::load. Leave it unchanged, because it is an approval path that keeps the underivable-root denial and is outside spec item 7.
---

# Implementation Log for 2026-10-25-fix-now (3 phases)

## Phase 1

Phase 1 consists of one task, P1.1 Contract inventory. It is investigative: no source, test,
doc, or skill file changed. Every finding below was confirmed with `rg` against
the worktree at `ea0a1c7db`. The pre-existing untracked file
`claudine/fixes/2026-10-25-fix-now/implementation-log.md` is this log, which was empty.

### Predecessor status (Decision 11 / 12)

`2026-10-26-dmls-frontmatter-typing-allocation` is **implemented**:

- `spec.md` has `implemented: true`, and its `implementation-log.md` has `completed_phase: 5`.
- Its code landed in `457b1a51a` ("share DMLS frontmatter typing analysis") and
  `cf25e93a1` ("reuse schema work across DMLS snapshots").
- `darkmatter/dmls/src/overlay/analysis.rs` defines `FrontmatterAnalysis` (line 247) and
  `CrossSnapshotSchema` (line 177).
- `host_lifecycle_profiles` (`darkmatter/lib/src/markdown/schemas/catalog.rs:573`) already looks up the
  cache before classifying a file. It caches non-catalog files as `None`, keeping the latest
  revision per path through `cache.retain(...)`.

No prerequisite work is needed. P2.1 must keep the non-catalog classification
entries inside its latest-per-path cache.

### Consumer inventory (P2.1 public API migration)

| Symbol | Consumers (production unless marked test) |
| --- | --- |
| `GlobalRequirement` (`&'static str` `name`/`same_as`, `const fn shape`) | `darkmatter/lib/src/markdown/schemas/catalog.rs` (19 refs; definition at :63, leak at :291); `claudine/lib/src/composition/schema/globals.rs` (5, compiled registration); `darkmatter/dmls/src/overlay/event_views.rs` (1); tests: `darkmatter/lib/src/lifecycle/host_profile/tests.rs`, `darkmatter/dmls/tests/l1/lifecycle_host_profile.rs`, `darkmatter/lib/tests/l1/variable_type_projection.rs`, `darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs` |
| `CatalogErrorReason::Mismatch(&'static str)` | construction `catalog.rs:368`; test match `darkmatter/lib/tests/l1/global_catalog.rs` |
| `UnknownVerbPolicy::IgnoreUnprovided(&'static [&'static str])` (`parse.rs:77–104`) | `darkmatter/lib/src/lifecycle/{parse.rs, mod.rs, host_profile.rs, md_host.rs, actions.rs}`; `claudine/lib/src/composition/lifecycle/mod.rs`; tests: `lifecycle/{tests.rs, tests/overloads.rs, preflight/tests.rs, executor/tests.rs, host_profile/tests.rs, run_guard/tests.rs, md_host/tests.rs}`, `darkmatter/lib/tests/l1/lifecycle_generated_operands.rs`, `claudine/lib/tests/l1/host_catalog.rs` |
| `EventRegistry::declare_host_global(&'static str)` / `register_global_catalog(&'static GlobalCatalog)` / `global_catalog() -> Option<&'static GlobalCatalog>` / `host_globals() -> &[&'static str]` (`registry.rs:475–901`) | `lifecycle/host_profile.rs:139,151,370,373`; `lifecycle/shell_template.rs:193–194`; `claudine/lib/src/composition/lifecycle/registry.rs:77–109`; `markdown/compose/preflight/collect.rs:1475` and `shell_expansion/site.rs:512` (via `with_host_globals(&[&str])`, `projection.rs:223`); `compose/context/options.rs:2082`; tests: `lifecycle/registry/tests.rs`, `lifecycle/host_profile/tests.rs`, `claudine/lib/tests/l1/host_catalog.rs:109` |
| `HostLifecycleProfile::from_artifact` | `catalog.rs:616` (discovery); tests: `host_profile/tests.rs`, `claudine/lib/tests/l1/host_catalog.rs`, `darkmatter/dmls/tests/l1/lifecycle_host_profile.rs` |
| `host_lifecycle_profiles` (eager `Vec`) | `darkmatter/dmls/src/overlay/event_views.rs:61` (only production consumer); `darkmatter/lib/benches/pipeline_work_support/mod.rs:23,180` |

**Runtime `Box::leak` sites P2.1 must remove:** `catalog.rs:291` (dynamic global
names), `catalog.rs:613` (catalog per key), `host_profile.rs:370` (host globals),
and `host_profile.rs:389,392` (ignored verbs). Out of scope:
`markdown/compose/expression/catalog/mod.rs` and `compose/context/catalog.rs`
(host-*function* catalogs, `2026-10-25-host-function-catalog-leak`). Test-only
leaks are not runtime leaks. These are `lifecycle/signatures.rs:314` and the
`tests/l1/*` files.

**DMLS per-pass discovery sites.** `LifecycleHost::for_document(ctx)` is called in
`providers/frontmatter.rs:86,1188,1215`, `providers/dsl.rs:363`, and
`diagnostics/frontmatter.rs:93`; the test file
`diagnostics/frontmatter/severity_tests.rs:124,149` also calls it. Each call
rediscovers. P2.1 threads one host from the diagnostics dispatcher
(`providers/mod.rs`) to the frontmatter and DSL diagnostics paths. Hover and completion
call sites, such as `providers/frontmatter.rs:86,1188,1215`, remain standalone entry points
that may discover on their own (Decision 4).

### Other surfaces (confirmed present as the spec describes)

- **Item 3:** `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:458` writes audio with inline `write_atomic`,
  and `:579` writes the record with inline `snapshot::write`. The production
  `tokio` features in `biscuit-speaks/lib/Cargo.toml:24` are `process, io-util, fs`, so
  `rt` is missing. The `rt-multi-thread` feature comes only from the dev-dependency at
  `:34`.
- **Item 4:** `translate.rs:1323` has the unbounded `Mutex<HashMap<String, Arc<Validator>>>`. The replacement
  `ValidatorCache` / `structural_validator_for` is at
  `markdown/schemas/validate.rs:79,323`.
- **Item 5:** `resolve.rs:2615–2617` holds the preliminary `serde_yaml_ng::from_str` parse and its stale
  comment.
- **Item 6:** `simplified/definitions.rs:224` uses `..definition.clone()` in `map_bodies`.
- **Item 7:** `claudine/lib/src/harness/shell.rs:587` `user_blacklist` calls
  `SessionPolicy::load`. The per-file load closure is at `:615`, and `policy_paths_at` is at `:131`.
  The runtime callers are `claudine/lib/src/composition/sequence/task/templates.rs:67` and the lifecycle executor through
  the `ShellHost::user_blacklist` trait (`darkmatter/lib/src/lifecycle/executor.rs:805`;
  Claudine's implementation is in `composition/lifecycle/executor.rs`).

### Acceptance-item → task and existing-coverage map

| Item | Owning task | Existing tests to extend / keep green | Missing (to add) |
| --- | --- | --- | --- |
| 1 Owned profiles release | P2.1 | `lifecycle/host_profile/tests.rs` (round trip, md profile, refusal reasons) | weak-reference reclamation after artifact and catalog replacement; retained profile, registry, and owned-ignore-policy across replacement; owned globals and verbs through parse and serialization |
| 2 Failures cached without retention | P2.1 | `host_profile/tests.rs::an_absent_stale_or_lacking_artifact_is_refused_with_its_reason` | build counter and cache inspection; repair on the next call; injected read error; release of removed, unreadable, invalid, or non-host catalogs |
| 3 Ownership and wire contract | P2.1 | `claudine/lib/tests/l1/host_catalog.rs`, `registry/tests.rs`, committed-artifact comparisons, `global_catalog.rs` | supplied-text loader test (no second read); `Box::leak` inspection |
| 4 Lazy discovery, one host | P2.1 | `darkmatter/dmls/tests/l1/lifecycle_host_profile.rs` (incomplete reasons, LSP diagnostics) | read observer (zero reads after the winner); shadowing; combined-pass discovery count; panic isolation; repair clears the warning |
| 5 Publications off the worker | P2.2 | `elevenlabs.rs` `mod tests` (:1308; staging, detached hits, record skips) | current-thread blocked-writer tests for audio and for the record on a cache hit; error and panic tests; caller-drop test |
| 6 Validator eviction | P2.4 | `translate.rs` assertion tests, `tests/l1/coercion_core.rs` | capacity-one eviction through the production builder |
| 7 Import parsing | P2.5 | `dmls/tests/l1/named_type_references.rs`, `cli/tests/l1/schema_triggers.rs`, `resolve.rs` tests | public-resolver `kind: schema` / `types: [` malformed import; numeric-admission wording |
| 8 Definition projection | P2.6 | `tests/l1/recursive_schema_golden.rs`, `recursive_schema_conversion.rs`, `shipped_schema_corpus.rs` | none (inspection only) |
| 9 Blacklist only | P2.3 | `shell.rs` tests (`blacklisted_command_rejected`, :867) | read observer with no whitelist opened; edit between dispatches; no or underivable root; missing or unreadable file through injection |
| 10 Scoped gates | P3.3 | n/a | n/a |

Task and file ownership for parallel work follows the plan's Wave 2 and Wave 3 ownership lists
without change. P2.1 alone owns `catalog.rs`, the lifecycle registry, parse, and
host_profile code, and the DMLS providers. P2.2 alone owns biscuit-speaks. P2.3 owns
`shell.rs`. P2.4, P2.5, and P2.6 own disjoint single files.

### Design Decisions

- **P1-D1 — `check_command_blacklists` stays on `SessionPolicy::load`.**
    - **decision:** Spec item 7 changes only `user_blacklist`. `check_command_blacklists`
      (`shell.rs:565`, used by Claudine pre-flight at `composition/preflight.rs:390`)
      also loads both files, but it is an approval-time path. It denies an
      underivable root, and spec item 7 requires that denial to be kept for approval.
      It stays unchanged.
    - **alternatives considered:** Route it through the blacklist-only helper while
      keeping the denial. Make it fully permissive like `user_blacklist`.
    - **why:** The spec names `user_blacklist` and the runtime dispatch callers only. Rule 3
      (surgical changes) applies, and changing the approval path's semantics would alter
      what the spec promises.
    - **downsides:** A pre-flight check of a shape-approved command still reads the
      whitelist once per command.
- **P1-D2 — DMLS hover and completion keep standalone discovery.** Only the combined
  diagnostics pass shares one host (Decision 4). The `LifecycleHost::for_document`
  call sites outside diagnostics stay as they are.
    - **alternatives considered:** Thread a host through every provider. Add a session cache.
    - **why:** The spec scopes one host to each diagnostics pass and excludes session caching.
    - **downsides:** A hover still performs one lazy discovery, which stops early after this fix.

### Gate decision

The phase changed only this log and the plan, both under `claudine/fixes/`.
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
returned `packages: ["darkmatter"]` and `change_class: "package"`. The selection reason was
"changed test input read by package(s) darkmatter".
`darkmatter/lib/tests/l1/shipped_schema_corpus.rs` validates every kinded document under
`claudine/fixes`. The gates therefore could **not** be skipped and were run:

- `just lint` (darkmatter): **pass**.
- `just test` (darkmatter), first run: 12,372 passed and 1 failed.
  `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
  compares a captured tracing-log sequence, and fail-fast left 198 tests unrun. Run alone with
  `cargo nextest run -p dmls -E 'test(a_held_preparation_leaves_edits_cancellation_and_other_requests_running)'`,
  it passed three times out of three. It is recorded as **load-sensitive**, and no code in this phase
  touches DMLS.
- `just test` (darkmatter), second run on a quiet host: **12,571 passed, 10 skipped, 0 failed**.
  This covers the shipped-schema corpus check of the edited plan and log.

Dependent-package tests were not run because no source changed. Cross-OS checks were not run
for the same reason: there is no OS risk.
