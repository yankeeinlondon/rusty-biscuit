---
spec: /Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/claudine/fixes/2026-10-25-fix-now/spec.md
plan: claudine/fixes/2026-10-25-fix-now/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - dmls
    - claudine
    - biscuit-speaks
human_review: false
source_files_during_phase_2:
    - biscuit-speaks/lib/Cargo.toml
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/harness/shell/policy.rs
    - claudine/lib/src/harness/shell/tests.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/lifecycle_host_tests.rs
    - darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/src/lifecycle/host_profile.rs
    - darkmatter/lib/src/lifecycle/host_profile/tests.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - scripts/cross-check.sh
docs_updated_during_phase_2:
    - darkmatter/docs/topics/lifecycle/host-artifact.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
docs_updated_during_phase_3:
    - biscuit-speaks/docs/tts-caching.md
    - biscuit-speaks/docs/dependencies.md
    - claudine/docs/topics/pre-flight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
source_code:
    - biscuit-speaks/lib/Cargo.toml
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/harness/shell/policy.rs
    - claudine/lib/src/harness/shell/tests.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/lifecycle_host_tests.rs
    - darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/src/lifecycle/host_profile.rs
    - darkmatter/lib/src/lifecycle/host_profile/tests.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - scripts/cross-check.sh
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
documentation:
    - darkmatter/docs/topics/lifecycle/host-artifact.md
    - darkmatter/dmls/docs/diagnostics.md
    - biscuit-speaks/docs/tts-caching.md
    - biscuit-speaks/docs/dependencies.md
    - claudine/docs/topics/pre-flight-checks.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/dmls.md
completed_phase: 3
implemented: true
pipeline_refreshed:
    - 2026-10-25-host-function-catalog-leak
decisions_for_author: []
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

## Phase 2

All six Phase 2 tasks (P2.1–P2.6) are implemented and checked off in the plan.
P2.6 was done by the orchestrator. The other tasks were done by foreground
subagents, in three rounds with no overlapping files:

1. P2.2, P2.3 and P2.4+P2.5, in parallel;
2. P2.1 part A (ownership, cache, observed text, lazy discovery);
3. P2.1 part B (the DMLS shared diagnostics host).

The orchestrator then ran the phase gates once, with no subagent running.

### Design Decisions

- **P2-D1: Wave order changed to run P2.1 last and on its own.**
    - **decision:** run P2.2, P2.3, P2.4/P2.5 (and P2.6) first, then P2.1 in two sequential parts.
    - **alternatives considered:** the plan's order (P2.1, P2.2 and P2.3 in parallel, then Wave 3); a separate worktree per worker.
    - **why:** P2.1 rewrites public darkmatter types that claudine compiles against, so a P2.3 worker running alongside it would have tested claudine against a darkmatter in mid-edit. Rule 6 forbids creating worktrees. The Wave 3 tasks do not use the P2.1 APIs, so the plan's reason for putting them after Wave 2 does not apply.
    - **downsides:** P2.1, the longest task, ran without parallelism.
- **P2-D2 (P2.1): ownership wrappers.**
    - **decision:**
        - Add `CatalogHandle { Static(&'static GlobalCatalog), Shared(Arc<GlobalCatalog>) }` and `VerbList { Static(&'static [&'static str]), Shared(Arc<[String]>) }`, both with `From` conversions.
        - Both compare by content, not pointer.
        - `host_globals()` returns `&[Cow<'static, str>]`.
        - A crate-private `DeclaredTypes::with_registered_host_globals` accepts those names without changing the public `with_host_globals`.
    - **alternatives considered:** pointer equality; an iterator return type.
    - **why:** existing tests compare a rebuilt registry with the compiled one. Callers keep slice semantics.
    - **downsides:** comparing two handles costs a deep comparison of the catalogs.
- **P2-D3 (P2.1): cache shape.**
    - **decision:**
        - One entry per path: `Profile(Arc)`, `Refused(reason)`, or `NotHostCatalog`.
        - Reads happen outside the mutex; lookup, build and replace happen under it.
        - An invalid catalog is evicted, not cached, and is re-parsed on its next visit, as before.
        - Listing a visited root evicts cached paths under that root that are no longer listed. An unreadable root counts as an empty listing.
    - **alternatives considered:** building outside the lock; caching invalid catalogs.
    - **why:** no unchanged revision is ever built twice, and nothing beyond the winner is read just to prune.
    - **downsides:** builds for different paths run one at a time. An entry for a file that is shadowed but still present stays until its root's listing drops it.
- **P2-D4 (P2.1): discovery test seam.**
    - **decision:** a thread-local `catalog::discovery_probe` (build counter, read log, injected read failure), compiled under `cfg(any(test, feature = "work-counters"))`. That feature is the existing cross-crate test-only mechanism, and DMLS's dev-dependency enables it.
    - **alternatives considered:** a new feature; a public testing API.
    - **why:** it observes the production read path with no process-global races.
    - **downsides:** it sees only discovery on the test's own thread.
- **P2-D5 (P2.1 part B): provider hook.**
    - **decision:**
        - `Provider` gains two defaulted methods, `reads_lifecycle_host()` and `diagnostics_with_host(ctx, &LifecycleHost)`. The frontmatter and DSL providers override both.
        - `ProviderRegistry::diagnostics` delegates to a private `diagnostics_discovering(ctx, discover)`, which runs discovery once inside `catch_unwind`.
        - On a discovery panic, each host-reading provider logs the existing `provider panicked` event and contributes nothing.
        - A document without frontmatter gets `LifecycleHost::md()` without discovery, as neither consumer discovered for it before.
    - **alternatives considered:** matching on provider names; adding an `Option<&LifecycleHost>` parameter to `diagnostics`; one pseudo-provider log line for the panic.
    - **why:** the standalone `diagnostics(ctx)` entry points are unchanged, and the panic log is identical to the one the same panic produced before.
    - **downsides:** the two trait methods must stay consistent. One discovery panic produces two log lines.
- **P2-D6 (P2.2): ElevenLabs writer seam.**
    - **decision:**
        - A private `writer: AtomicWriter` field defaults to `write_atomic` and can be replaced only through a `#[cfg(test)] with_writer`.
        - `snapshot::write` takes the writer as a parameter.
        - An audio `JoinError` maps to `TempFileError` with the text "audio cache publication did not complete".
        - The test gate reports entry on a Tokio channel, so the releasing task can await it, and waits for release on a std `recv_timeout(1.5 s)` watchdog.
    - **alternatives considered:** a global hook; std channels on both sides.
    - **why:** no global state, and the injected writer runs inside the real closure.
    - **downsides:** one `Arc` clone per write.
- **P2-D7 (P2.3): policy read observer and file split.**
    - **decision:**
        - A per-thread `#[cfg(test)] policy_reads` observer sits inside the shared `load_policy_file` helper.
        - The tests use a relative root as the underivable root.
        - The user-policy loading code (`policy_paths_at`, `user_blacklist`, `load_policy_file`, `SessionPolicy`, `policy_reads`) moved to `claudine/lib/src/harness/shell/policy.rs`, and the inline tests moved to `shell/tests.rs`. `shell.rs` re-exports `user_blacklist` and `policy_reads`, so every public path is unchanged.
    - **alternatives considered:** an entry in the `test_placement` `EXCEPTIONS` list; trimming lines.
    - **why:** the claudine-cli `test_placement` guard failed `shell.rs` at 809 production and 370 inline-test lines (limits 800 and 300). Its exceptions list is empty by policy.
    - **downsides:** `shell.rs`'s diff is larger than the behavior change.
- **P2-D8 (P2.4): bounded validator cache.**
    - **decision:** `format_assertion` delegates to a private `format_assertion_in(cache, …)`. Production passes `LazyLock<ValidatorCache>`, and the test passes the existing public `ValidatorCache::with_capacity(1)`. Eviction is proven by reference count and `Arc::ptr_eq`.
    - **alternatives considered:** a peek API on `validate.rs`; a global override.
    - **why:** the test exercises the exact production path, and `validate.rs` needs no change.
    - **downsides:** none significant.
- **P2-D9: `cross-check.sh` long-path clone fix (Rule 5).**
    - **decision:** the Windows leg's first `git clone` now passes `-c core.longpaths=true`.
    - **alternatives considered:** leave it; enable long paths by hand on the rig.
    - **why:** a fresh clone checked out files before the later `git config core.longpaths true` ran, so the first run on a new clone directory failed with "Filename too long". `scripts/ci/test_cross_check.py` (55 tests) passes.
    - **downsides:** none.

### Requirement → test mapping

| Spec acceptance item | Evidence |
| --- | --- |
| 1 Owned profiles release | `catalog::discovery_tests::a_replaced_revision_releases_the_old_profile_once_its_holders_drop`; `host_profile::tests::owned_globals_and_ignored_verbs_round_trip_with_unchanged_bytes` |
| 2 Failures cached, nothing retained | `discovery_tests::a_refused_artifact_is_cached_by_reason_and_rebuilt_only_when_it_changes`, `a_cached_success_is_dropped_when_its_catalog_stops_loading`, `a_non_catalog_file_s_new_revision_replaces_its_entry` |
| 3 Ownership and wire contracts | `discovery_tests::the_host_text_loader_builds_from_the_supplied_bytes` (file absent or different on disk; parity of errors with `from_host_file`); `host_profile::tests::owned_collection_fields_refuse_every_invalid_shape_and_admit_an_empty_list`; existing committed-artifact and compiled-host registration tests pass. Inspection: the `Box::leak` calls in `catalog.rs` and `host_profile.rs` were removed. The host-function catalog leaks are untouched (out of scope). |
| 4 Lazy discovery, one host | `discovery_tests::discovery_stops_reading_at_the_first_usable_profile` (reads are exactly `[a.yaml, a.lifecycle.json]`), `discovery_keeps_root_order_sorted_names_and_shadowing`; `dmls overlay::event_views` `the_host_is_the_first_usable_profile_and_nothing_after_it_is_read`; `dmls providers::lifecycle_host_tests::{one_lifecycle_discovery_serves_a_combined_pass_and_a_repair_shows_next_request, a_broken_host_catalog_is_still_reported_in_a_combined_lifecycle_pass, a_lifecycle_discovery_panic_drops_only_the_frontmatter_and_dsl_diagnostics}`. A temporary revert of the dispatcher made the count tests fail (2 reads), so the tests can fail. |
| 5 Publications off the worker | `elevenlabs::tests::{default_voice_preparation_publishes_record_and_audio_off_the_async_worker, a_default_voice_record_on_an_audio_cache_hit_publishes_off_the_async_worker, an_audio_write_error_fails_preparation_and_a_record_write_error_only_warns, a_panicking_audio_write_fails_preparation_and_a_panicking_record_write_only_warns, a_caller_dropped_after_write_entry_still_publishes_the_owned_audio}`. With the writes temporarily made inline, all three progress tests failed within 1.5–3.1 s without hanging. |
| 6 Validator eviction | `translate::tests::{an_evicted_validator_keeps_judging_for_its_assertion, a_file_assertion_judges_syntax_without_looking_for_the_file, a_custom_format_assertion_applies_the_registered_format, a_validator_build_failure_names_the_check}` |
| 7 Import parsing | `tests/l1/schemas_named_import_errors.rs` (5 tests: the `kind: schema` / `types: [` file, numeric admission, non-schema YAML, a table of unsupported-envelope / root-union / whole-file-reference / duplicate-key / trailing / second-document cases, and a control). Inspection: the preliminary `serde_yaml_ng::from_str` was removed. Existing DMLS `named_type_references` (16) and CLI `schema_triggers` (28) tests pass. |
| 8 Definition projection | Inspection: `map_bodies` builds `Definition { body, origin: clone, json_key: clone }` with no struct-update clone. `recursive_schema*` and `shipped_schema_corpus` pass (139 tests). |
| 9 Blacklist only | `executor::tests::shell_templates::user_blacklist::{a_dispatch_reads_the_blacklist_and_never_opens_a_malformed_whitelist, a_blacklist_edit_applies_at_the_next_dispatch, unloadable_user_rules_are_empty_and_built_ins_still_refuse, valid_blacklist_rules_apply_among_malformed_lines_which_still_warn}`; `harness::shell::tests::{full_approval_reads_both_policy_files, full_approval_denies_an_underivable_policy_root}` |
| 10 Scoped gates | below |

**Input robustness:**

- The host-catalog YAML loader's document shapes are covered by the parity cases in `the_host_text_loader_builds_from_the_supplied_bytes`: a duplicate key, a non-string key, a wrong type, and the reserved `fm`.
- The artifact collection fields (`host_globals`, `ignore_unprovided`) are covered for absent, null, wrong type, mixed and all-invalid lists, plus a valid empty list.
- The named-import loader's shapes are covered by the table test.

**Code-smell search:**

- `from_host_text`'s name probe uses `.ok()`, `filter_map(as_str)` and `unwrap_or_default()`. It is a permissive probe; the strict `load` stays authoritative, and the parity test proves it.
- The discovery skips (unreadable root or file, non-UTF-8 file) are intentional, as before.
- `#[serde(default)]` does not appear in these readers.
- `lifecycle/host/artifact.rs:520` has a `.ok()`. That file is untouched and outside this scope.
- The policy fallbacks for a missing or unreadable blacklist are intentional, as the spec requires.

### Gates and evidence (macOS host)

- **darkmatter `just test`:**
    - Run 1: 1 failure, `dmls::l1 startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`. Run alone, it passed 3 of 3.
    - Run 2: 1 failure, `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`. Phase 1 had already recorded this test as load-sensitive. Run alone, it passed 3 of 3.
    - Both compare a captured log sequence into which a load-dependent `WARN dmls::validation_timing: slow schema validation` line is inserted. They are recorded as **load-sensitive**, and neither covers code this phase changed.
    - Run 3: **12,593 passed, 10 skipped, 0 failed**.
    - The 6 slow tests are all existing corpus and parity tests; none is new.
- **darkmatter `just lint`:** pass.
- **biscuit-speaks `just test`:** 597 passed, 20 skipped. **`just lint`:** pass.
- **claudine `just test`:**
    - Run 1: the `claudine-cli::l1 test_placement::repository_test_placement` guard failed on `harness/shell.rs` (see P2-D7). Fixed by the split.
    - Rerun: **11,654 passed, 10 skipped, 0 failed**. The 283 slow tests are existing CLI sequence suites under full-suite load; none is new.
- **claudine `just lint`:**
    - Run 1: failed on clippy "items after a test module" in `shell/policy.rs`. Fixed by moving `policy_reads` to the end of the file.
    - Rerun: pass.
- **Part A compile checks:**
    - `cargo check -p darkmatter --benches --examples`, with and without `work-counters`;
    - `cargo check -p claudine --examples`;
    - `cargo check -p dmls --all-targets`;
    - `cargo clippy -p darkmatter --features work-counters --all-targets -D warnings`.

    All are clean.
- `just check-tier-coverage darkmatter`: nothing stranded.
- Dependent packages: claudine and claudine-cli consume the changed darkmatter types and are covered by claudine's full `just test`. DMLS and darkmatter-cli are inside darkmatter's `just test`.
- **Cross-OS, native Windows** (`build-win-native`, through `just cross-check … --os windows` with substring filters for the new and changed tests). All passed:
    - darkmatter: 22
    - dmls: 31
    - claudine: 72
    - biscuit-speaks: 71
- **Not run:** Linux and WSL2. CI's pull-request Linux leg will cover Linux.
- `affected_scope.py` over the working tree gives `packages: [biscuit-speaks, claudine, darkmatter, dmls, repo-deps, test-toolkit, claudine-cli]` and `change_class: package`, so the gates could not be skipped (they were run above).

## Phase 3

Phase 3 is documentation, the acceptance audit, the integrated gates, and the
pipeline refresh. It changed no source code.

### P3.1 Current documentation

- **Already current (reviewed, unchanged):** `darkmatter/docs/topics/lifecycle/host-artifact.md`
  ("Reading it", with its Mermaid flow) and `darkmatter/dmls/docs/diagnostics.md`
  ("The lifecycle host is found once per pass") were written during Phase 2. They
  cover one retained revision per visited path, caller-held lifetimes, observed
  invalidation, first-usable lazy discovery, one host per diagnostics pass, panic
  isolation, and fresh discovery on each request. The darkmatter skill's
  `lifecycle.md` and `dmls.md` match.
- **`biscuit-speaks/docs/tts-caching.md`** ("Atomic Writes"): both ElevenLabs
  publications run on Tokio's blocking pool and are awaited; a table gives the
  fatal audio / warning-only record split; dropping the caller does not cancel
  an owned publication, so the entry ends complete or absent.
- **`biscuit-speaks/docs/dependencies.md`:** there was no Tokio entry to update,
  so a paragraph was added explaining why the production `tokio` dependency
  enables `rt` (`spawn_blocking`) and that it does not rely on dev-dependency
  feature unification. No crate was added or removed.
- **`claudine/docs/topics/pre-flight-checks.md`** ("Approval Policy"): a table
  and paragraph contrast an approval decision (reads blacklist and whitelist;
  an underivable root denies) with template dispatch (reads only the blacklist,
  again for every command; missing, unreadable, or underivable gives no user
  rules while the built-ins still apply), plus the malformed-line and
  no-rediscovery rules.
- **Stale comments:** searched the changed darkmatter, DMLS, and claudine files
  for runtime-leak, once-per-editor, process-lifetime, and classifier wording.
  The only `Box::leak` left under `lifecycle/` and `schemas/` is the test fixture
  in `lifecycle/signatures.rs`, which is out of scope. `FORMAT_VALIDATORS`'s doc
  already describes the bounded cache, and `resolve.rs` no longer has the
  preliminary-parse comment. Nothing else needed changing.
- **Old error wording:** no doc quotes the old serde message for numeric admission.
  `docs/topics/schemas/definition.md` already describes the `not valid YAML: ...`
  schema-document error, which is still the behavior.
- No README described the changed behavior, so none was changed. The claudine and
  biscuit-speaks skills make no claims about lifetimes or policy reads that the
  change affects.

### P3.2 Acceptance audit

- **Items 1–9:** the Phase 2 "Requirement → test mapping" was checked against
  the code. Each named test exists in a file compiled by an L1 binary or as an
  inline `#[cfg(test)]` module, and no test name or module carries a tier
  marker (`level2_`, `level3_`, `real_`, `slow_`, `browser_`). All of them ran in
  this phase's darkmatter `just test` (darkmatter, DMLS) or Phase 2's claudine and
  biscuit-speaks runs.
- **Observers sit on production paths:** `discovery_probe::read`/`note_build` are
  the production read and build calls in `catalog.rs`; without the cfg they are
  plain `std::fs::read` and a no-op. `policy_reads::observe` sits inside
  `load_policy_file`, which both runtime and approval loading call. The ElevenLabs
  gate writer runs inside the real `spawn_blocking` closure, and its
  `recv_timeout` watchdog releases the writer when an inline-write regression
  happens. The capacity-one validator test goes through `format_assertion_in`,
  which production also calls.
- **Code-smell search** (`#[serde(default)]`, `.ok()`, `unwrap_or_default()`,
  `filter_map(Result::ok)` across the diff): the remaining hits are the
  intentional fallbacks already recorded in Phase 2. An unreadable root counts
  as an empty listing, an unreadable or non-UTF-8 candidate is skipped and loses
  its cache entry, the `from_host_text` name probe is permissive with strict
  `load` authoritative, and a missing or unreadable blacklist gives no user rules.
  No `#[serde(default)]` is on a load-bearing field.
- **Consumers:** the Phase 1 inventory (static registrations,
  `CatalogErrorReason::Mismatch`, artifact writers, fixtures, examples, the
  pipeline benchmark support) all compile. Phase 2 ran
  `cargo check -p darkmatter --benches --examples` with and without
  `work-counters`, plus the claudine examples and DMLS all-targets, and Phase 3
  made no source change since then. Byte-for-byte artifact comparisons (the
  claudine `host_catalog` freshness test and the round-trip test) pass.
- No acceptance item lacks evidence, and no requested scope was deferred.

### Design Decisions

- **P3-D1: Reuse Phase 2's claudine and biscuit-speaks gate results.**
    - **decision:** run darkmatter's `just test`, `just lint`, and
      `just check-tier-coverage` in this phase; do not rerun claudine's or
      biscuit-speaks's full suites.
    - **alternatives considered:** rerun all three areas' suites, as P3.3 lists.
    - **why:** Phase 3 changed only Markdown. `affected_scope.py` over this phase's
      three new doc edits selects only `darkmatter` (a darkmatter test reads one of
      them as a test input). Claudine's and biscuit-speaks's source is byte-identical
      to what passed their full `just test` / `just lint` at the end of Phase 2.
      P3.3 itself says to rerun only checks a repair invalidated.
    - **downsides:** the claudine and biscuit-speaks evidence is from the end of
      Phase 2 rather than this phase.
- **P3-D2: Add, not update, the Tokio dependency entry.**
    - **decision:** add a paragraph to `biscuit-speaks/docs/dependencies.md`.
    - **alternatives considered:** leave the file alone because no crate changed.
    - **why:** the plan assumed an existing Tokio entry, but there was none. The
      production `rt` feature is a non-obvious dependency choice that a reader
      would otherwise see only in `Cargo.toml`.
    - **downsides:** none.
- **P3-D3: Fix the two "load-sensitive" DMLS log tests instead of recording them again.**
    - **decision:** `registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
      and `startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`
      now accept a request's completion line as either `request complete` or
      `slow request`.
    - **alternatives considered:** record them as load-sensitive again, as Phases 1
      and 2 did.
    - **why:** the first darkmatter `just test` in this phase failed the
      registry test at the same point twice in a row, so it was not occasional
      noise. The captured log showed the cause. Request 502 took 732 ms, so
      `request_timing` logged it as `slow request` (WARN) rather than
      `request complete` (DEBUG), and the test's `matching("request complete")`
      found zero lines. Phase 2 had blamed the inserted `slow schema validation`
      WARN; that was a misreading. The startup test had the identical filter.
      Three other DMLS tests (`registry_observability.rs`, `stdio_subprocess.rs`)
      already match both messages, so this follows existing convention (Rule 5).
    - **downsides:** none. The assertions still require exactly one completion
      line per request id.

### P3.3 Gates and evidence (macOS host)

- **darkmatter `just test`:**
    - Run 1: 1 failure (`registry_preparation::a_held_preparation_…`); fail-fast left
      199 tests unrun. Run alone, it passed in 0.67 s.
    - Run 2: the same test failed at the same point. Diagnosed and fixed (P3-D3).
      Both fixed tests passed run alone.
    - Run 3: **12,593 passed, 10 skipped, 0 failed** (188 s; 92 slow, under full-suite load).
- **Slow-test check:** `dmls::l1 lifecycle_host_profile::*` (one line changed in
  Phase 2: `Box::leak` to `Arc::new`) were among the slow tests. Alone,
  `claudine_events_narrow_until_a_write_with_say` takes 4.4 s, which is under the
  5 s budget. A `sample` profile of that run puts the Phase 2 discovery work
  (`observe_candidate`, `from_host_text`, BLAKE3) at no more than about 10 of
  roughly 1,100 samples per thread. The time is in schema compilation and
  validation in the existing per-row LSP fixture, so this fix did not cause it.
  It is close to the budget and is noted here for review.
- **darkmatter `just lint`:** pass.
- **`just check-tier-coverage darkmatter`:** nothing stranded.
- **claudine and biscuit-speaks:** not rerun (P3-D1). Their source is unchanged
  since their Phase 2 full `just test` / `just lint` passes (claudine 11,654 passed;
  biscuit-speaks 597 passed).
- **Examples and benchmarks:** Phase 2's `cargo check -p darkmatter --benches --examples`
  (with and without `work-counters`) and `cargo check -p claudine --examples` stand.
  Phase 3 changed only two DMLS test files, which `just test` and `just lint`
  compiled. The production-only biscuit-speaks feature graph declares `rt`
  directly in `[dependencies]`.
- **Dependent packages:** the only source changes are two DMLS integration-test
  files, and nothing depends on them.
- **Cross-OS:** not run in this phase. The change filters log strings in tests and
  carries no OS risk. Native Windows passed for all new tests in Phase 2. Linux and
  WSL2 were not run in any phase; CI's pull-request Linux leg and the nightly
  WSL2 leg cover them.
- **Scope calculator:** `affected_scope.py` over the full uncommitted tree selects
  darkmatter, dmls, claudine, biscuit-speaks and others (`change_class: package`),
  so the gates could not be skipped.

### P3.4 Review handoff

- The final diff holds only the implementation, tests, and documentation for
  this fix, plus the Rule 5 repairs recorded in P2-D9 and P3-D3. The pre-existing
  changes were kept.
- **Intentional diagnostic changes:** the numeric-admission wording for named
  imports (spec item 5).
- **Deviations from the plan:** P3-D1 (claudine and biscuit-speaks gates reused from
  Phase 2) and P3-D2 (a new dependencies paragraph rather than an update).
- Implementation complete, ready for review. The fix directory stays in place.

## Pipeline Refresh

- `2026-10-25-host-function-catalog-leak` (next spec): **changed.** The spec
  said its host-function cache follows "the same rule" as this fix's lifecycle
  profile cache. That was corrected: this fix evicts a catalog that fails to load
  rather than caching it, and prunes paths that drop out of a visited root's
  listing. A spec design decision ("Relationship to the lifecycle profile
  cache") keeps the downstream policy and records why. Plan Decision 8 records
  that `host_function_set`, `ArtifactKey`, and the function cache are unchanged,
  and points Decision 5's test seam at the existing `catalog::discovery_probe`.
  The plan has not started.
- `2026-10-25-schema-diagnostic-provenance`: checked, unchanged. It edits
  `resolve.rs` merge and conflict code, not the named-import loader whose
  preliminary parse this fix removed.
- `2026-10-25-shared-acyclic-definitions`: checked, unchanged. It edits
  `simplified/definitions.rs` and `resolve.rs` broadly, and nothing it assumes
  depends on the old struct-update clone in `map_bodies` or on the preliminary
  parse.
- `2026-10-25-pipeline-work-reduction`: checked, unchanged. It names this fix
  only as a sibling track and the chain's first step; it assumes nothing about
  lifecycle discovery, policy reads, ElevenLabs publication, or validator caching.
