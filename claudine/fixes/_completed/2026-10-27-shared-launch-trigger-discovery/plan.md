---
kind: plan
created: 2026-10-27
total_phases: 3
phase: 4
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1:
- claudine/lib/src/composition/schema/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- claudine/lib/tests/l1/invalid_trigger_findings.rs
- claudine/lib/src/composition/schema/findings.rs
- claudine/lib/benches/pipeline_work_alloc.rs
docs_updated_during_phase_2:
- claudine/docs/topics/schemas.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/claudine/SKILL.md
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
- claudine/lib/src/composition/schema/mod.rs
- claudine/lib/tests/l1/invalid_trigger_findings.rs
- claudine/lib/src/composition/schema/findings.rs
- claudine/lib/benches/pipeline_work_alloc.rs
documentation:
- claudine/docs/topics/schemas.md
- .claude/skills/claudine/SKILL.md
completed_phase: 3
implemented: true
packages:
- claudine
---

# Shared launch trigger discovery implementation plan

## Summary and Definition of Done

Refactor Claudine's schema pre-validation to own one discovered `DarkmatterSchemas` for the duration of one call. Borrow that owner to prepare the cleanup schema, move it to replace caller records after cleanup, and borrow it again to prepare the verdict schema. Discovery and loaded trigger payloads are shared; trigger matching and effective-schema preparation still happen twice.

Completion means:

- A successful public `pre_validate_schema` call with a matching disk trigger counts exactly one `TriggerDiscovery` and two `EffectivePreparation` operations, measured around the call alone.
- Cleanup can remove the value selecting a trigger, and the final verdict stops applying that trigger. Cleaned overrides, drop attribution, caller file origins, validation phases, and typed errors retain their contracts.
- Findings are recorded before authored-schema resolution and delivered once per trigger identity per invocation. Discovery infrastructure errors return immediately without a second scan.
- Separate calls sharing the same context observe trigger creation, replacement, removal, and repair. Standalone cleanup and status still discover independently; authored-only loading still does no discovery.
- The existing D3 allocation benchmark is run on this macOS host for its three existing sizes, with counts and allocated bytes compared to the specification's baseline in `implementation-log.md` beside this plan. No registry or schema-owner clone consumes the saving.
- Claudine's `just test` and `just lint` pass, current topic documentation and the skill explain the call-local lifetime, and all changes remain in the starting worktree. Implementation is complete and ready for review; no commit, push, lifecycle-directory move, or human gate is part of this plan.

Paths below are repository-relative. Execute phases in order. Wave numbers are unique across the plan. Tasks within Wave 2 can run concurrently with the stated file ownership; the other waves are sequential. Every delegated session must receive the non-interactive-session constraints and must use one-shot commands with no prompts.

## Phase 1: Separate discovery from preparation

### Necessary Rules

1. **Use existing ownership APIs without changing Darkmatter.** All production edits belong in `claudine/lib/src/composition/schema/mod.rs`. `DarkmatterSchemas::with_file_resolution_context` preserves the registry, `with_caller_input_records` consumes and returns the owner while replacing its caller-origin map, and `effective_for(&self, ...)` borrows it (`darkmatter/lib/src/markdown/schemas/mod.rs`). Do not clone `DarkmatterSchemas` or `TriggerRegistry`, introduce a registry cache, add dependencies, or change public signatures.

2. **Extract two private helpers with these contracts.** Add `discover_schema_owner(source: &ResolvedCompositionSource, file_ref_fallback_dir: Option<&Path>, file_resolution_context: &FileResolutionContext, trigger_findings: &TriggerFindings) -> Result<DarkmatterSchemas, CompositionError>` and `prepare_effective_schema(source: &ResolvedCompositionSource, snapshot: Option<&serde_json::Value>, schemas: &DarkmatterSchemas) -> Result<Option<EffectiveSchema>, CompositionError>`. Spell `std::path::Path` and `biscuit_file::FileResolutionContext` qualified, matching the surrounding signatures. The first helper discovers from `file_resolution_context.for_source(&source.resolved_path)`, attaches `bundle::claudine_schema_bundle()` before discovery, records findings immediately after successful discovery, then configures the launch resolution context and optional fallback. The second retains the existing `matching_markdown`/`effective_for` dispatch and error translation. Evidence: these are the two responsibilities currently combined in `load_effective_schema`.

3. **Retain `load_effective_schema` as the independent-discovery adapter.** Keep its visibility, signature, and callers unchanged; implement it by discovering an owner, moving it through `with_caller_input_records(callers)`, and borrowing it through `prepare_effective_schema`. Its production consumers are `prepare.rs::resolve_launch_schema`, `schema/classify.rs::build_schema_status_report_for_mode`, and `schema/translate.rs::{translate_schema_failure, handle_retry_error}`. These consumers retain their own discovery boundaries. `load_authored_schema` stays untouched and trigger-free.

4. **Give pre-validation one local owner.** `pre_validate_verdict` calls `discover_schema_owner` before optional cleanup and propagates discovery errors with `?`. Build original `override_records`, move the owner through `with_caller_input_records`, and borrow it for cleanup. Rebuild `build_effective_instance` and `override_records` from the returned cleaned source and overrides, replace caller records by moving the same owner, and borrow it for final preparation. No effective schema is reused between passes. Evidence: the current two calls to `load_effective_schema` are in `drop_invalid_optionals_with_origin` and `pre_validate_verdict`; Darkmatter counts each `effective_for_with_override` as `EffectivePreparation`.

5. **Share the cleanup implementation, preserving its infallible public API.** Replace private `drop_invalid_optionals_with_origin` with `drop_invalid_optionals_with_schemas(source: ResolvedCompositionSource, set_overrides: Option<Value>, data_keys: &BTreeSet<String>, schemas: &DarkmatterSchemas) -> (ResolvedCompositionSource, Option<Value>, Vec<DroppedOptional>)`. Keep its existing cleanup body, changing only schema acquisition to the borrowed preparation helper. Public `drop_invalid_optionals` discovers its own owner, configures original caller records, and invokes this helper with an empty data-key set. Discovery failure in this standalone adapter returns the original source/overrides and no drops, preserving its existing best-effort behavior. Pre-validation discovery failure returns the translated error immediately. Evidence: standalone cleanup currently absorbs every loader error; pre-validation's final loader currently returns it after repeating discovery.

6. **Separate discovery failure from effective preparation failure.** Both new helpers translate `SchemaError` through `schema_error_to_composition_error` with the document's resolved path and the typed cause. Individual invalid trigger envelopes, payloads, and imports remain successful discovery with findings. Infrastructure failures never become an empty registry or findings. Cleanup continues to tolerate `Ok(None)` and effective-preparation errors; final preparation returns its own error. Thus authored-schema failures still get a cleanup attempt and a final preparation, but a failed scan gets only one attempt. Evidence: `triggers/discovery.rs::scan_with_source` propagates root enumeration/text-read failures, while collecting envelope/payload failures; `schema/translate.rs::schema_error_to_composition_error` maps grammar/convert/shape errors to `SchemaParse`, other errors to `SchemaLoad`.

7. **Preserve matching identity and validation policy.** Keep `matching_markdown` unchanged: overlays change values but preserve the document path and its authored `$schema`. Keep cleanup's `validate_stored(..., None)` and final validation's `launch_phase_for_mode(mode)`, plus all required/eager/control exclusions, template deferral, literal-data handling, and `prefer_removed_grammar`. Keep drop order, reasons, `Frontmatter`/`Override` attribution, and `PreValidation` stage. Evidence: `matching_markdown`, `drop_invalid_optionals_with_origin`, `pre_validate_with_origin`, and `pre_validate_verdict` already encode these policies; the feature changes discovery lifetime alone.

8. **Do not update launch callers.** `cli/src/commands/schema_interactive/mod.rs` calls public pre-validation for first checks, rechecks, and file-confirmation; `cli/src/commands/wrap/sequence/jit.rs` calls layered pre-validation; `cli/src/commands/sequence.rs` uses standalone cleanup. All retain their signatures and separate-call freshness. Composition, transclusions, retry/proxy drivers, approval, and post-shell validation receive no shared owner. Evidence: the caller trace reaches the same public adapters, and the specification explicitly limits sharing to one pre-validation call.

9. **Keep finding identity and delivery unchanged.** Only move the point that calls `TriggerFindings::record`, placing it in successful discovery before both authored-schema preparations. Do not alter `Ledger::admit`, its first-reason rule, or `take_undelivered`. A scan that fails before returning a registry publishes none of its partial findings. Evidence: `schema/findings.rs` owns the existing full-source-identity ledger; discovery only exposes findings after returning a complete registry.

10. **Use hermetic public-API L1 tests.** Extend `lib/tests/l1/invalid_trigger_findings.rs`, using its `Fixture`, `RequestSnapshot`, explicit home, and `build_resolution_context` pattern. Add helpers there rather than a second fixture framework or test binary. Construct source/context and warm unrelated setup before taking counter snapshots. Counter tests use `#[cfg(feature = "work-counters")]` and `darkmatter::work_counters::{snapshot, Work}`; Claudine forwards the feature. No existing D3 counter assertion was found in Claudine's Rust sources; add one using the existing counter infrastructure rather than claiming to update a preexisting assertion. Evidence: `lib/Cargo.toml` forwards `darkmatter/work-counters`, declares the consolidated `tests/l1/main.rs` root, and enables counters in CI metadata; `claudine/justfile::test` enables counters locally.

11. **Use an unreadable text value to test infrastructure failure portably.** Write bytes `[0xff]` to an enumerated `.yaml` file. Expect public `SchemaLoad`, the affected path in the message, one discovery and zero effective preparations, then overwrite it with a valid trigger and prove the next call succeeds using the same source/context. Do not use permission bits, filename case collisions, races, or new injection seams. Evidence: `FilesystemRoots::root_file_text` in `triggers/discovery.rs` uses `read_to_string` and translates invalid UTF-8 into `SchemaError::Io` before envelope parsing, on every supported OS.

12. **No parser expansion or robustness matrix is required.** This change rearranges discovery and effective preparation; it changes no schema/config field validator, deserializer, file-format loader, or grammar acceptance. Preserve all existing malformed-trigger and authored-schema tests; add error-path orchestration assertions described below. The user-requested input matrix adds no new field or document cases for this ownership-only refactor. The rematching fixture uses an OR group of string and number guards rather than `any(required)`: `triggers/matcher.rs::presence_follows_the_shared_effective_calculation` proves that unknown/any admits absence. Evidence: the proposed production diff stays in the orchestration module and calls unchanged Darkmatter loaders.

13. **No spike or wider measurement.** The specification already measures the repeat at 47–50% of warm pre-validation allocations and explicitly asks for only D3 rows at 1, 10, and 100 schema-root files on one host. Run `cargo bench --color=never -p claudine --features work-counters --bench pipeline_work_alloc -- D3` from the repository root. Its `main` substring filter selects D3 alone; retain all six existing rows, including one-discovery controls. No A4 rerun, timing threshold, new workload, or additional measurement host is included.

14. **Update current records, not the specification.** Update `docs/topics/schemas.md`, `schema/findings.rs` module docs, affected schema module comments, the D3 function's stale duplicate-scan comment, and the schema-discovery paragraph in `.claude/skills/claudine/SKILL.md`. The README is an overview and needs no change for this internal optimization. Preserve the specification as its dated snapshot and document implementation evidence in `implementation-log.md`. Topic docs must not link to or name this feature. Evidence: current finding docs say every effective-schema load rediscovers, while the skill says `load_effective_schema` records findings; both descriptions need correction.

### Wave 1

- [x] **Extract schema helpers**
  - Implement the two private helpers and the existing `load_effective_schema` adapter according to Rules 1–3 and 6. Keep the existing schema-error translator as the sole mapping authority.
  - Preserve bundle registration, disk shadowing, root ordering, and authored-schema precedence by continuing to use the same Darkmatter discovery and preparation APIs.

- [x] **Reuse local owner**
  - Depends on helper extraction. Implement Rules 4–9 in `pre_validate_verdict` and standalone cleanup; replace the private origin cleanup function with the borrowed-owner implementation.
  - Review docs/comments attached to each changed symbol. Correct the module's affected schema-error description to distinguish skipped individual triggers from failed discovery.
  - Checkpoint: source inspection shows one discovery call on the pre-validation path, two independent preparation calls, caller-record replacement between them, and no schema-owner/registry clone or stored owner.

## Phase 2: Prove behavior and document lifetime

### Wave 2

The following tasks depend on Phase 1 and can run concurrently. One agent owns `invalid_trigger_findings.rs`; another owns topic/skill/findings documentation and the benchmark comment. Neither changes production control flow. Run focused verification after the wave joins so concurrent test builds do not duplicate work.

- [x] **Cover shared discovery**
  - Extend `claudine/lib/tests/l1/invalid_trigger_findings.rs` with all cases below. Retain the module's registration in `tests/l1/main.rs`; no new target is needed. Use `test_toolkit::trace_phase!` to mark fixture setup and observed work where useful.
  - Add a successful matching disk-trigger control with a valid required number. Measure only `pre_validate_schema` and assert discovery = 1 and effective preparations = 2. Repeat against the same source/context and assert the same delta independently.
  - Add a trigger matching numeric or string `count` via `match: { group: [{ count: "string(required)" }, { count: "number(required)" }] }` and declaring `count: number(optional)` plus `ticket: string(required)` in its payload. Control: valid `count` without `ticket` returns `MissingProperties`. Edited case: wrong-type `count` is dropped, the final verdict succeeds without `ticket`, and the returned source has no `count`. Assert one `Frontmatter`/`PreValidation` drop with a nonempty reason and the 1/2 counter delta.
  - Repeat the rematching case with an invalid caller `count` override and no authored `count`. Keep another caller override `input` constrained by a trigger that matches before and after cleanup to `file(required;match(./launch-only.txt);eager)`. Select this second trigger with a separate authored `gate: yes` and `match: { gate: "string(required)" }`; preserve `gate` during cleanup. This follows `schema/tests.rs::provided_file_match_partial_reports_unresolved_file_reference` and `pre_validate_schema_defers_caller_eager_file_to_canonical_preparation`. Place that file only at the explicit launch anchor. Assert the returned override object keeps `input`, removes `count`, reports an `Override`/`PreValidation` drop, and succeeds. Add a control with `input: absent-file` that returns `CompositionError::UnresolvedFileReference` for `input`. This proves surviving caller origins remain wired after replacing records.
  - Extend existing mixed-valid/invalid and repeated-check cases to assert findings are available before error delivery, preserve filename order and first reason, and deliver no duplicates after subsequent pre-validations, standalone cleanup, and status checks. Change the invalid trigger's reason between calls and assert the first reason stays in the ledger.
  - Extend authored-load coverage with the malformed inline declaration `$schema: { spec: "file(required, match(**/*spec*.md))" }` from `schema/tests.rs::schema_parse_error_for_grammar_failure_names_property_and_keeps_path_load_distinct`, asserting `SchemaParse`, the document source path and property `spec`, and already-recorded trigger findings. Keep the existing missing-file `SchemaLoad` and authored-reference-to-trigger tests.
  - Strengthen the no-applicable-schema case to compare original and returned frontmatter and overrides and assert zero drops. A skipped unrelated trigger still records its warning; its existence must not create an effective schema.
  - Add a same-source/same-context freshness sequence: absent trigger succeeds; created trigger requiring absent `ticket` fails; modified trigger requiring a present `title` succeeds; removed trigger succeeds; malformed trigger yields a finding and succeeds; repaired trigger requiring `ticket` fails. Use direct writes/removal between synchronous calls, without sleeps or metadata manipulation. Keep one finding ledger throughout and assert its delivered warning is not emitted again after repair.
  - Add the invalid-UTF-8 infrastructure case in Rule 11, asserting no partial findings and successful recovery on the next call. Also exercise standalone cleanup on the failing scan: it returns unchanged source/overrides and no drops.
  - Counter assertions are feature-gated; behavior assertions remain active without counters. All new cases execute in process, without provider, terminal, browser, shell, or ambient user-schema dependencies.

- [x] **Document call lifetime**
  - Update `claudine/docs/topics/schemas.md` around trigger discovery and invalid-trigger warnings. State that one pre-validation shares loaded trigger rules/payloads across cleanup and verdict, while matching each pass's current values. Show the `count` cleanup example and explain that the final verdict no longer selects its trigger.
  - Add a compact Mermaid flow: discover/record findings → original callers and values → cleanup preparation → cleaned callers and values → verdict preparation → owner released. Show scan errors exiting before preparation.
  - State that edits between calls are visible, edits between passes do not replace loaded trigger contents, authored `$schema` still resolves per preparation, and discovery is not an atomic filesystem snapshot. Keep separate composition/status/standalone boundaries clear.
  - Update `schema/findings.rs`'s discovery description and `.claude/skills/claudine/SKILL.md`'s corresponding paragraph. Preserve invocation-level deduplication and first-observation semantics.
  - Update only the stale D3 description in `lib/benches/pipeline_work_alloc.rs`; keep the fixture, sizes, repetitions, allocator, output labels, and one-discovery control unchanged.

Validation checkpoint after Wave 2: from the absolute `claudine/` working directory run `just test-library --features work-counters invalid_trigger_findings::`. Confirm every new test appears and passes. Existing `schema/tests.rs` caller-relative, mode-dependent, templated/data, eager-file, literal-token, and retired-grammar coverage remains unchanged and will run in Phase 3.

## Phase 3: Validate savings and close implementation

### Wave 3

- [x] **Measure allocation savings**
  - Depends on Phase 2. Run the Rule 13 benchmark once on this host after the implementation stabilizes. Record exact command, current base revision, uncommitted change state, `rustc --version --verbose`, bench profile, and raw six D3 rows in the adjacent `implementation-log.md`. Use `GIT_TERMINAL_PROMPT=0` for Git commands. Do not commit for measurement identity.
  - Put baseline and new warm allocated bytes, discovery counts, preparation counts, absolute bytes saved, and percentage saved side by side for each pre-validation size. Baselines are 331,851 / 1,940,903 / 18,023,939 B, with counts 2/2; baseline one-discovery controls are 157,365 / 961,891 / 9,003,409 B. Identify these as the specification's historical baseline, not a fresh same-tree control run.
  - Expect pre-validation counts 1/2 and one-discovery controls 1/0. The historical subtraction estimates are 174,486 / 979,012 / 9,020,530 B remaining; these are explanatory estimates, not exact allocation thresholds. New pre-validation bytes must decrease at all three sizes. Inspect and eliminate owner/registry copies and accidental rediscovery responsible for a failed comparison, then rerun D3 after the corrective code change. Timing is informational only.

- [x] **Run completion checks**
  - Run `just test` and then `just lint` from the absolute starting worktree's `claudine/` directory, using explicit tool working directories to avoid `CDPATH`. These are the final area gates; do not run full-workspace tests, L2/L3, providers, or Darkmatter's separate gates. Darkmatter code is outside this plan and remains unchanged.
  - Resolve failures caused by this implementation and rerun the affected checks; preserve unrelated working-tree edits. Never run `cargo fmt` or formatting repair commands. Required lint checks may report formatting; fix touched lines manually.
  - Review the final diff for one call-local owner, two matches/preparations, replaced caller origins, early finding recording, unchanged public signatures, no new cache/API/dependency/CI scope, and portable fixture paths. Retain existing CLI `launch_trigger_schemas` and `compose_schema_cli` tests as the wiring evidence; no new CLI process matrix is introduced.
  - Record exact validation commands/results and any concrete implementation departure in `implementation-log.md`; state that the benchmark was measured only on macOS and do not claim runtime evidence from other OSes. All new tests remain portable L1 tests for the existing environment schedule.
  - Mark plan tasks complete only after their observable outputs pass. Leave the active feature directory in place, with the final state “implementation complete, ready for review.” Human review follows the entire implementation and does not interrupt any phase.