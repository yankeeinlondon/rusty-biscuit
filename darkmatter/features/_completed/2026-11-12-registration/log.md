---
feature: 2026-11-12-registration
implementation_1: "2026-11-14T03:45:17-08:00"
implementation_2: "2026-11-14T07:01:33-08:00"
implementation_3: "2026-11-14T07:52:03-08:00"
implementation_4: "2026-11-14T08:42:49-08:00"
---

# Registration review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-14T03:45:17-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began
- the review has no `human_in_the_loop` items and no blocked findings; no recurrence list (first iteration)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Foreign composition discards host-event schema errors | fixed | every child below verified; compose keeps selected-host excluded-key problems via shared `pending_rendering_sets_aside` |
| R1-F1.1 | Foreign `start: 5` → compose rejects | fixed | CLI `compose_foreign_documents::a_host_start_of_the_wrong_shape_is_rejected`; lib `the_host_schema_rejects_a_host_only_event_md_never_runs` |
| R1-F1.2 | Foreign `blocked: 5` → compose rejects | fixed | `a_host_blocked_of_the_wrong_shape_is_rejected` |
| R1-F1.3 | Foreign `success: 5` → compose rejects | fixed | `a_host_success_of_the_wrong_shape_is_rejected` |
| R1-F1.4 | Foreign `failure: 5` → compose rejects | fixed | `a_host_failure_of_the_wrong_shape_is_rejected` |
| R1-F1.5 | Claudine `loop: [1, 2]` with `agent: claude` → selected-host composition rejects | fixed | claudine `installed_registration_md_compose_judges_an_event_it_never_runs` (published profile, problem at `/loop`) |
| R1-F1.6 | Shared `initialize: 5` → both reject (control) | clean | `a_shared_initialize_of_the_wrong_shape_is_rejected` (md lifecycle parser rejects first) |
| R1-F1.7 | Shared `cancelled: 5` → both reject (control) | clean | `a_shared_cancelled_of_the_wrong_shape_is_rejected` |
| R1-F1.8 | Shared `finalize: 5` → both reject (control) | clean | `a_shared_finalize_of_the_wrong_shape_is_rejected` |
| R1-F1.9 | Renamed-away `composed: 5` → both reject (control) | clean | `a_renamed_away_composed_is_rejected` |
| R1-F1.10 | Renamed-away `failed: 5` → both reject (control) | clean | `a_renamed_away_failed_is_rejected` |
| R1-F1.11 | Ordinary `timeout: 5` → both accept (control) | clean | `a_numeric_timeout_converts_and_is_accepted` |
| R1-F2 | Sequence-only Claudine documents miss the main host schema | fixed | `claudine.yaml` selects on `sequence`; drift guard asserts identical `match:` blocks |
| R1-F2.1 | `sequence: []` + `timeout: [1, 2]` → timeout rejected | fixed | `installed_registration_sequence_selects_the_main_schema` (passive and compose reject `/timeout`) |
| R1-F2.2–F2.9 | `start`/`blocked`/`success`/`failure`/`loop`/`prompt`/`agent`/`model` selectors → reject timeout (controls) | clean | `main_schema_rejects_timeout!` one test per key, both readers |
| R1-F2.10 | `sequence: []` + `composed: [stop]` → rejects md spelling | fixed | `installed_registration_sequence_document_rejects_the_md_event_spelling` |
| R1-F2.11 | `agent: claude` + `composed: [stop]` → rejects (control) | clean | `installed_registration_agent_document_rejects_the_md_event_spelling` |
| R1-F2.12 | Shared-event-only document → md host (control) | clean | `installed_registration_shared_events_alone_keep_the_md_contract` plus existing identification tests |
| R1-F3 | Claudine leaves configured URL transclusion disabled | fixed | `compose_options_with_context` enables remote transclusion at every Claudine options site; policy decides |
| R1-F3.1 | Claudine `::file` URL under effective `network: allow` → included | fixed | `network_transclusion::network_allow_transcludes_a_remote_file` |
| R1-F3.2 | Claudine `::code` URL → included as code | fixed | `network_allow_transcludes_a_remote_code_block` |
| R1-F3.3 | Claudine `prologue` URL → included | fixed | `network_allow_transcludes_a_remote_prologue` |
| R1-F3.4 | Claudine `epilogue` URL → included | fixed | `network_allow_transcludes_a_remote_epilogue` |
| R1-F3.5 | Read-side expression `load_data(url)` → success (control) | clean | `shell_permission_parity::shared_fixture` URL rows |
| R1-F3.6 | md/public composition with remote enabled → success (control) | clean | md network tests unchanged and passing |
| R1-F3.7 | Inline, loop, sequence, preflight preparation options inherit the enabled switch | fixed | all routes build options in `canonical_compose_options`; sequence discovery fixed; `shell_discovery_reads_a_remote_child_only_under_network_allow`; deny (0 requests) and redirect tests |
| R1-F4 | Per-run host approvals bypass invalid permissions | fixed | invalid policy refused before per-run grants at projection and every gate |
| R1-F4.1 | Remote `::file` gate, invalid network shapes + `--allow host` → config error, no request | fixed | lib `network_permission::` invalid rows (null, 123, [], "", sometimes) on `::file`: error naming `/permissions/network`, 0 requests |
| R1-F4.2 | Remote `::code` gate, same → error, no request | fixed | same matrix, `::code` surface |
| R1-F4.3 | Remote expression gate, same → error, no request | fixed | same matrix, expression surface |
| R1-F4.4 | Schema-slot URL gate, same → error, no request | fixed | same matrix, schema-slot surface |
| R1-F4.5 | Valid `allow`/`deny` + allowed host → success (control) | clean | `valid_allow_…`, `valid_deny_…` fetch on all four surfaces |
| R1-F4.6 | Network field validator shapes (control) | clean | existing `config_fields`/`config_load` tests pass |
| R1-F4.7 | Local schema-slot gate refuses invalid permissions (control) | clean | first branch refuses; suites pass |
| R1-F4.8 | Claudine shell permission reader refuses (control) | clean | `Decider::new` before cache/handler; claudine permission/parity 226/226 |
| R1-F4.9 | YAML loader document shapes (control) | clean | existing loader tests pass |
| R1-F4.10 | `permissions: null` → config error, no request | fixed | lib and CLI `null_permissions_outrank_an_exact_host` |
| R1-F4.11 | `file-read: [123]` → config error, no request | fixed | `an_invalid_file_read_entry_outranks_an_exact_host` |
| R1-F4.12 | `shell-commands: [frobnicate echo]` → config error, no request | fixed | `an_invalid_shell_rule_outranks_an_exact_host` |
| R1-F4.13 | `permissions: ./missing.yaml` → reference error naming both files, no request | fixed | lib and CLI `a_missing_permissions_reference_outranks_an_exact_host` (CLI names `~/.darkmatter.yaml:1:14` and the target) |
| R1-F4.14 | Omitted permissions `{}` + allowed host → success (control) | clean | lib `omitted_permissions_fetch_every_surface_for_an_exact_host`; CLI `omitted_permissions_fetch_an_exact_host` (1 request) |
| R1-F5 | DMLS ignores template shell approvals | fixed | DMLS verdict uses md template parser, projection, coverage |
| R1-F5.1 | DMLS template allow, `echo {{ title }}` → approved | fixed | DMLS `shell_permission_classification` rule-kind row and shared-table row (approved = md) |
| R1-F5.2 | Template allow, literal `echo hi` → approval required | clean | row "a template allow never covers a literal" (approval required = md) |
| R1-F5.3 | `exact echo hi` → approved (control) | clean | row "an exact allow" |
| R1-F5.4 | `prefix echo` → approved (control) | clean | row "a prefix allow" |
| R1-F5.5 | `prefix echo` + `!template …`, literal → approval required (control) | fixed | literal row clean; shape row (`echo {{ title }}`) was approved by DMLS while md asks, now approval required |
| R1-F5.6 | md template execution → approved (control) | clean | every DMLS row compared against md pre-flight decision |
| R1-F5.7 | md/Claudine parity (control); parity fixture asserts the editor decision | fixed | parity fixture now asserts the DMLS decision for template rows; claudine `parity` 77/77 |
| R1-F6 | Union validation skips conflicting default candidates | fixed | shortcuts gated on `alternatives_supply_defaults`; complete candidates compared |
| R1-F6.1 | Strict whole-document `{b: y}` → ambiguity | fixed | `a_value_one_arm_accepts_as_written_is_ambiguous_with_an_arm_that_completes_it` (strict) |
| R1-F6.2 | Soft whole-document `{b: y}` → ambiguity | fixed | same test, soft |
| R1-F6.3 | Strict nested `{shape: {b: y}}` → ambiguity at `shape` | fixed | open arms: `a_nested_value_one_open_arm_accepts_as_written_is_ambiguous_at_its_property`; closed SimplifiedSchema arms have one successful candidate (verified with `md`), pinned by `a_closed_nested_arm_completed_with_a_default_cannot_take_the_other_arms_keys` |
| R1-F6.4 | Soft nested → ambiguity at `shape` | fixed | same tests, soft |
| R1-F6.5 | Empty whole-document settlement → ambiguity (control) | clean | `root_arms_that_complete_the_value_differently_are_ambiguous` |
| R1-F6.6 | Empty nested settlement → ambiguity (control) | clean | `an_empty_nested_value_with_differing_candidates_is_ambiguous_in_both_modes`; `md` probe reports ambiguity at `shape` |
| R1-F6.7 | Equal candidates / literal-tag selection (control) | clean | existing equal-candidate/discriminant tests plus `a_distinguishing_literal_keeps_a_complete_value_unambiguous` |
| R1-F6.8 | Darkmatter `md schema validate` root/nested → located ambiguity | fixed | CLI `conforms_schema_validate_locates_alternatives_that_complete_a_valid_value_differently` (root `""`, nested `/shape` with open arms) |
| R1-F7 | Windows source-scan timings fail the L1 speed gate | fixed | single-threaded guard with Git pre-filter; Windows 7.334 s → 1.085 s |
| R1-F7.1 | `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` → under budget, no in-test threads | fixed | Windows filtered run 1.085 s; package-run timing in final verification |
| R1-F7.2 | Same scan on macOS (control) | clean | macOS alone 0.367 s |
| R1-F7.3 | Bounded fixture control | clean | 0.031 s macOS / 0.133 s Windows; new pre-filter correctness test |
| R1-F7.4 | Related glob ownership scan (control) | clean | 1.295 s alone macOS, 1.822 s Windows, single-threaded, unchanged |
| R1-F8 | Eager default failures lose their schema-default origin | fixed | default provenance retained and attributed in Simplified Schema, md validate, md compose, DMLS |
| R1-F8.1 | Simplified Schema strict → default origin retained | fixed | `required_defaults::eager_file_defaults::strict_…` (`/properties/doc/default`) |
| R1-F8.2 | Simplified Schema soft → default origin retained | fixed | `required_defaults::eager_file_defaults::soft_…` |
| R1-F8.3 | `md schema validate` → identifies schema default | fixed | `conforms_schema_validate_attributes_a_default_problem_to_the_default`, `schema_validate_attributes_a_missing_eager_default_target_to_the_default` (with `hint:` line) |
| R1-F8.4 | `md compose` → identifies schema default | fixed | `conforms_compose_…`, `composition_defaults::eager_file_defaults::composition_…`, `compose_hint_directs_a_default_problem_to_the_importing_types_file`, pass-2 test |
| R1-F8.5 | Existing eager default target → success (control) | clean | library and compose tests: success with applied default reported |
| R1-F8.6 | Lazy file default → success (control) | clean | library and compose tests: success with target absent or present |
| R1-F8.7 | Structurally invalid numeric default (control) | clean | existing invalid-numeric-default tests pass |
| X1 | Discovered: claudine-cli `document_environment_routes::successive_sequence_steps_each_read_the_map_they_captured` fails (steps read the launch map) | fixed | test-gating defect; gated on `test-fixtures`; 16/16 module tests with the feature |
| X2 | Discovered: claudine-cli `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path` measured 5.55 s | clean | loaded host (load ≈ 23 on 16 cores, from other host apps); package run 6.613 s (< 10 s), alone 1.301 s (≤ 5 s); cleared under the loaded-host rule |
| X2.1 | Same rule: `compose_initialize_staged_boot::initialize_shells_are_forbidden_even_with_yolo_across_entry_paths` | clean | package run 5.345 s, alone 1.564 s; cleared |
| X2.2 | Same rule: `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged` | clean | package runs 8.330 s and 7.744 s, alone 2.746 s; cleared |
| X3 | Discovered: claudine-cli `wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline` panicked once in the loaded package run (perf report TR-1 reconciliation, `cli/src/perf/tree.rs:445`) | unresolved; R2-F2 | The isolated 0.673 s pass and full 12103/12103 rerun did not repair it. Review 2 reproduced exit 101 in initialized direct and inline composition with `--perf` and a 0.1 s command: preparation timing stops before actual composition. See the dated correction below. |

### Progress

- starting the work on 'R1-F1 foreign composition discards host-event schema errors' at 03:45:52
        - defect class: deferring execution of values also suppressed final validation of those values, even when no later stage validates them
        - repair: the compose schema filter keeps excluded-key problems when a host is selected (`with_selected_host`), setting aside only what `md schema validate` sets aside as awaiting rendering; that rule is now one shared function `pending_rendering_sets_aside` in `lib/src/markdown/schemas/mod.rs`
        - Claudine's escape hatch is unchanged: no selected host means excluded-key problems are still dropped, and Claudine's `with_host_globals` makes `with_selected_host` a no-op
        - sibling sites checked: md CLI foreign branch (fixed); md's own lifecycle branch (clean, `md_host::parse_lifecycle` rejects bad shapes first); preflight discovery `collect.rs:528` (clean, inherits options); Claudine `prepare.rs:477` (clean, `post_shell_validate` later judges the full frontmatter); Claudine `preflight.rs:196` (clean, discovery pass); Claudine sequence `wrap/sequence/mod.rs:143` (clean, deferred verdict, each step validates); passive `ValidationOptions::excluded_keys` (clean, tests only); DMLS (clean, no usage)
        - tests: CLI `compose_foreign_documents::` 18/18 (one test per row via `event_verdicts!`); lib `foreign_document_compose::` 13/13; claudine `installed_registration::` 12/12; mutation check (branch disabled) failed exactly the four host-only rows, the lib test, and the Claudine test
        - docs: `darkmatter/docs/topics/host-registration.md` deferred-keys bullet; `.claude/skills/darkmatter/compose.md`
- work completed for 'R1-F1 foreign composition discards host-event schema errors' at 03:58:51
- starting the work on 'R1-F2 sequence-only Claudine documents miss the main host schema' at 03:58:51
        - defect class: host identification and the selectors that supply that host's main schema disagreed (`claudine-document.yaml` identifies by nine keys; `claudine.yaml` selected eight)
        - repair: `claudine/schemas/claudine.yaml` now carries the five non-null `sequence` alternatives spelled like the other keys; shared events alone still do not select it; no payload duplicated into `sequence.yaml`; `claudine/lib/generated/shipped.json` regenerated (`just shipped-schemas`; only two digests changed; no preparation code changed, so `PREPARATION_REVISION` unchanged)
        - implementation decision #46 (omit `sequence` from the main trigger) is reversed, as the review recommends
        - drift guard: the existing guard pinned the bug (main = identifying minus `sequence`); it now asserts `claudine.yaml`'s `match:` equals `claudine-document.yaml`'s `match:` exactly (`claudine_identification::claudine_yaml_selects_exactly_the_identified_documents_and_never_on_shared_events_alone`)
        - sibling triggers: `sequence.yaml` (comment only), `inline-compose.yaml` (metadata only, clean), other schema files are not triggers
        - tests: `installed_registration.rs` covers both passive validation and composition through the published profile; `main_schema_rejects_timeout!` gives one test per identifying key; parity table, shipped corpus, and schema matrix pointers updated; claudine scoped run 967/967, darkmatter `shipped_artifacts::` 47/47, claudine `just lint` exit 0
        - docs: `claudine/schemas/README.md`, `claudine/docs/topics/schemas.md`, `claudine/docs/topics/frontmatter-properties.md`, `.claude/skills/claudine/SKILL.md`
        - discovered: `claudine-cli::l1 document_environment_routes::successive_sequence_steps_each_read_the_map_they_captured` fails alone (steps read the launch map instead of `a`/`b`); it also fails with the R1-F2 schema change reverted, and the test file predates this feature's commits; added as ledger row X1 for this cycle
        - discovered: `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path` measured 5.55 s in a scoped run; added as ledger row X2 (re-measure in the final package run)
- work completed for 'R1-F2 sequence-only Claudine documents miss the main host schema' at 04:11:03
- starting the work on 'R1-F3 Claudine leaves configured URL transclusion disabled' at 04:11:03
        - defect class: a consumer projects permission consent but never enables the composition operation that uses it
        - repair: new helper `claudine::composition::compose_options_with_context` enables remote transclusion for every Claudine composition; the captured `network` policy is the only decision point; the library keeps its deliberate disable switch (decision #26)
        - sibling sites fixed: `canonical_compose_options` (body, inline, initialize bootstrap, shell approval/preflight, loop, proxy, retry, resume, frontmatter-only variants); `system_prompt/prepare.rs`; `cli/.../wrap/overlay.rs` passthrough harness seed; `cli/.../wrap/sequence/mod.rs` sequence shell discovery (discovery and execution must agree or a remote child's `::shell` escapes pre-approval)
        - sibling sites clean: `document_expression_resolution_context` (local-only, no fetch); `shell_options.rs:345` (test-only); `md validate refs`/`md graph` (fetch nothing); DMLS/contract/reaper (build no `ComposeOptions`)
        - tests: new `claudine/lib/tests/l1/network_transclusion.rs` (7 tests: four targets under allow, all four refused under default deny with 0 requests, redirect refused with 0 requests, shell discovery reads a remote child only under allow); all 7 failed with the fix reverted; scoped claudine runs 93/93, 325/325, 13/13; claudine-cli scoped 246/246 (one test at 6.2 s under load re-measured alone at 2.9 s); claudine `just lint` exit 0
        - orchestrator fix (same class of drift): Darkmatter's `UrlExecutionDisabled` body/hint said "blocked by policy" and "Enable `allow_remote: true` in frontmatter or CLI options", neither of which exists; rewritten in `lib/src/markdown/compose/transclusion/types.rs` and its snapshot updated (`error_snapshots::transclusion::url_execution_disabled_shows_url` passes; 27/27 related)
        - the `allow_remote_transclusion` field doc in `darkmatter/lib/src/markdown/compose/context/options.rs` had drifted from the code and was corrected
        - note: Claudine may fetch an allowed URL twice (discovery then compose) because it has no shared remote-fetch runtime; consent is unchanged, so this was kept to bound the change
        - docs: `claudine/docs/topics/configuration.md`, `.claude/skills/claudine/SKILL.md`
- work completed for 'R1-F3 Claudine leaves configured URL transclusion disabled' at 04:19:40
- starting the work on 'R1-F4 per-run host approvals bypass invalid permissions' at 04:19:40
        - defect class: a per-run approval was consulted before the configuration error the request had already captured, so an invalid policy gave a permissive result
        - repair at the projection and at each gate: `NetworkPermission::fetch_policy` returns deny-all under `Invalid` whatever hosts the run names (covers `::file`, `::code`, read-side functions, git/cicd/pull-request provider functions, slot runtime, cache gate); `RemoteFetchRuntime::check_allowed` checks `config_error()` before hosts; `fetch_remote_text` keeps the located configuration message instead of a generic `RemoteDenied`; slot URL gate refuses every target under `SlotPermissions::Invalid` before this run's URL approvals; `GrantBook::network_allows` false under invalid
        - sibling sites clean: local slot file-read gate and listings/`find_files` (first branch refuses); md shell runtime snapshot, allow-once, cached decisions (`snapshot()` reads `request_policy` first; md has no `--yolo`); md lifecycle pre-flight; Claudine shell reader (`Decider::new` before cache and handler; `--yolo` is a handler); DMLS shell verdict (passive, invalid → unknown); DMLS slot-approval save (returns the error)
        - sibling site reordered: md shell pre-flight fast path reported `NotPreApproved` instead of the configuration error under an invalid block (never permissive); `request_policy` now runs first
        - scope: only the permissions block (and its reference) blocks permission decisions; `register` findings stay separate, as section 1 requires
        - decision: the URL refusal is located in its message (`file:line:col at /permissions/network`, `dm.compose.failure`), not a typed `dm.config.invalid` record; a typed record would require carrying `ConfigError` through string-typed fetch failures; the review requires a located error and no request, which this meets; kept
        - tests: lib `network_permission::` 24/24 (9 invalid rows plus 3 controls, all four surfaces per row; all 9 invalid rows failed with the fix reverted); CLI `config_notices::` 10/10 (3 invalid wiring cases failed with the fix reverted); scoped darkmatter 1682/1682, darkmatter-cli 194/194, claudine 226/226, darkmatter-lsp 110/110; `just lint` clean; slowest 0.26 s
        - docs: `darkmatter/docs/topics/remote-url-references.md` documented the wrong ordering and was corrected; `darkmatter/docs/topics/permissions.md`; `.claude/skills/darkmatter/SKILL.md`
- work completed for 'R1-F4 per-run host approvals bypass invalid permissions' at 04:32:35
- starting the work on 'R1-F5 DMLS ignores template shell approvals' at 04:32:35
        - defect class: the editor's passive shell verdict projected only part of the shared rule grammar (exact/prefix and locally split `!` entries; never template coverage or `!template` shape matches)
        - repair: `darkmatter/dmls/src/overlay/shell.rs` `verdict` now takes the site's type scope and uses md's own template parser, type projection, and coverage checks; `::shell` lines are read with md's now-public `authored_template` (directive options stripped as pre-flight does); built-in blacklist and `!` exclusions preserved
        - rule kind × consumer sweep: DMLS hover verdict was missing template, `!template` (value and shape), and chain template rules (fixed); DMLS `::shell` reading did not strip directive options (fixed); DMLS disallowed-command diagnostic now also checks a template's known words; md pre-flight, md runtime, and Claudine shell reader are clean for every rule kind, chains, and pipelines
        - discovered divergence fixed: `prefix echo` + `!template echo {{ string }}` on `echo {{ title }}` was approved by DMLS while md asks; now approval required
        - decision: DMLS judges a `{{ }}` command by its shape and never evaluates placeholders (as md's and Claudine's template paths do at pre-flight); consequence: an `exact` allow never approves a placeholder command, and an exact/template exclusion naming one of its programs leaves it unknown; DMLS can say unknown where md's runtime would approve, but never approved where md would ask; documented in `darkmatter/dmls/docs/features.md` and pinned by a test row; alternative (fill from static frontmatter) rejected as partial
        - shared parity fixture `darkmatter/lib/tests/fixtures/permission_parity/cases.json`: four rows added (declared-string template, template chain with unlisted segment, template exclusion as literal and by shape); DMLS now classifies every row through a real in-process LSP hover instead of recording templates unsupported
        - tests: darkmatter-lsp `shell|permission|parity` 105/105; darkmatter `permission_parity|shell_template|preflight_lifecycle` 101/101; claudine `parity` 77/77; `just lint` clean; three deliberate breakages (template coverage, value exclusion, shape exclusion) each failed the new tests
- work completed for 'R1-F5 DMLS ignores template shell approvals' at 04:44:35
- starting the work on 'R1-F6 union validation skips conflicting default candidates' at 04:44:35
        - defect class: accepting the original value early bypassed the required comparison of complete alternatives
        - repair: every "already valid" shortcut now returns early only when no union arm can supply a default (new public `simplified_schema::defaults::alternatives_supply_defaults`, conservative, follows local `$ref`s, memoized by `Arc` identity and cleared by `reset_process_caches`); otherwise each arm's complete candidate, including the value as written, is compared and distinct successful candidates give the existing located ambiguity error
        - sites fixed: `prepared.rs` `validate_strict`/`validate_soft` (new `accepted_as_written`); `defaults.rs` `prepare_defaults`; `defaults.rs` `Walk::union` (nested); `defaults.rs` `Walk::root_union` (dropped arms with no insertions, so the as-written value never competed); Darkmatter `EffectiveSchema::converted_candidate` (its own copy of the shortcut); Darkmatter `validate_converted` (skipped nested templates for an accepted value)
        - sites clean (different class): Darkmatter `coerce_frontmatter_with_pending` and `data_copy` (conversion-only, defaults excluded by contract); `rewrite.rs` `arm_accepts`/eager-file arm selection and `schema_validation.rs` root-arm applicability (arm selection, not defaulting); `validate_raw`, `judge_stored`, `required_defaults` (inherit via `complete`/`converted_candidate`)
        - F6.3/F6.4 and nested F6.8: SimplifiedSchema nested object arms are closed, so for the review's schema nested under `shape` the first arm's candidate `{b: y, a: 1}` is not a successful candidate and `{shape: {b: y}}` correctly passes unchanged; spec 3.2 makes only conflicting *successful* candidates ambiguous; orchestrator confirmed with the shipped `md`: nested `{b: y, a: 1}` is rejected by both arms, nested `{}` reports "ambiguous at `shape`: union arms [1, 2]", root `{b: y}` reports "ambiguous at the document root: union arms [0, 1]"; the nested defect is real for open arms and is fixed and tested with raw JSON Schema arms; closed-arm pass is pinned by `a_closed_nested_arm_completed_with_a_default_cannot_take_the_other_arms_keys`
        - pinned departure `a_value_one_arm_accepts_as_written_needs_no_default` replaced by `a_value_one_arm_accepts_as_written_is_ambiguous_with_an_arm_that_completes_it`
        - corpus impact: none; shipped unions with defaulting arms (`user-permissions | permissions-reference`, Claudine `tts`/`protect`, closed object arms) cannot accept one value two ways; no lowering change, so no artifact regeneration
        - tests: simplified-schema `just test` 2185/2185 plus `--no-default-features` 1828/1828; scoped darkmatter 425/425, darkmatter-cli 248/248 (new `conforms_schema_validate_locates_alternatives_that_complete_a_valid_value_differently`), claudine 234/234, extra corpus 1488/1488, work counters 706/706; `just lint` in simplified-schema and darkmatter exit 0; slowest 1.96 s
        - docs: `simplified-schema/docs/topics/validation.md`, `darkmatter/docs/inline/schema-validation.md`, `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`
- work completed for 'R1-F6 union validation skips conflicting default candidates' at 05:00:40
- starting the work on 'R1-F7 Windows source-scan timings fail the L1 speed gate' at 05:00:40
        - defect class: a whole-repository source scan opened every file itself and fanned out threads inside one test; on Windows the first open of each file costs about 0.8 ms per process, so the test overran the L1 budget and competed with nextest's parallelism
        - probe (temporary, removed): on `build-win` opening the 9,828 in-scope files took 7.9 s; re-reading all 172 MB in the same process 0.59 s; old 16-thread scan warm 0.27 s, cold 7.3 s; the cost is first opens, not decoding
        - repair: `darkmatter/lib/tests/l1/current_root_migration_guard.rs` is single-threaded; `git ls-files` plus a single-threaded `git grep` identify tracked, unmodified files containing neither needle, and only those are skipped; untracked, modified, assume-unchanged, and skip-worktree files and anything the path filters leave out are still opened and checked line by line; with no Git repository (extracted CI archive) every file is opened; same files, needles, and allowlist; `GIT_DIR` and similar variables removed from the child environment and `grep.fullName=false` forced
        - new correctness test `the_git_prefilter_skips_only_clean_tracked_files_without_a_needle` (real Git repo with edited-after-add, skip-worktree, assume-unchanged, untracked, and `features/` files; asserts the skip set is exactly `{clean.md}` and the scan equals an open-every-file scan); it fails if non-`H` entries are trusted
        - timings: Windows filtered run before 7.334 s, after 1.085 s; macOS alone before 0.162 s, after 0.367 s; fixture control 0.031 s macOS / 0.133 s Windows; new test 0.074 s / 0.517 s
        - sibling scans clean: other `thread::scope` uses are concurrency/race tests, not scans; other repository-root walks cover fixed directories single-threaded; `glob_implementation_guard::file_references_have_one_glob_implementation` (R1-F7.4) decodes 7 package `src` trees on one thread, 1.295 s alone on macOS and 1.822 s on Windows, unchanged
        - not done: a full Windows package run (filtered evidence only); measured inside the macOS package run in the final verification
        - skill: `.claude/skills/os/windows.md` records the per-process first-open cost on `build-win`
- work completed for 'R1-F7 Windows source-scan timings fail the L1 speed gate' at 05:19:11
- starting the work on 'R1-F8 eager default failures lose their schema-default origin' at 05:19:11
        - defect class: judging a completed (default-inserted) value lost which values came from schema defaults, so a check that runs only on the completed value reported the problem as if the author had written it
        - repair (first subagent): `AppliedDefault` gains `pointer` (same shape as `InvalidDefault`, e.g. `/properties/doc/default`) and `origin`; `ValidationProblem` gains `default`, set by new public `simplified_schema::defaults::attribute_problems` (message names the default; line/column cleared; `schema_path` keeps the failing keyword because `is_file_check` reads it); Darkmatter `validate_instance`/`complete` records each default's declaring layer; the compose pipeline keeps `applied_defaults` across both passes (covers revalidation after interpolation, trigger-added defaults, authored-copies re-judge); new structured `Diagnostic::schema_default`; `md schema validate` problems gain `default` and `defaults[]` gain `schema_location`/`schema_file`; DMLS ranges such problems at the property declaration (was the frontmatter root, an invented position) with `data.schema_default`
        - sibling sites: `Prepared::judge` (strict and soft) fixed; Darkmatter `validate_instance` and every passive/phase path fixed; compose verdicts both passes fixed; DMLS fixed; nested defaults under a present parent fixed (deepest default wins); Claudine config/launch validation and Darkmatter config load fixed through the shared paths (no Claudine source change); constraint checks on defaults clean (caught earlier as `InvalidDefault`); `match()` on a default clean (suggest-only); DMLS eager-file NoMatch unchanged (hover's job), pinned
        - follow-up subagent closed three gaps the first left: (1) the `md compose` hint said "Correct the frontmatter…" for a default-sourced problem; it now names the schema default and its file via shared `darkmatter::markdown::errors::schema_default_repair` (mixed authored/default problems get both lines); `md schema validate` formatted output adds a `hint:` line; DMLS `data.schema_default` carries `schema_file`; (2) a default declared inside a `Name@file` imported type named the importing layer; resolution now records the declaring file per property path (`ResolvedSchema::default_files`, `resolve::DefaultFiles`), and `EffectiveSchema` prefers it; (3) the pass-2 path is tested (`a_default_a_post_shell_trigger_inserts_is_attributed_in_the_second_pass`: a trigger matching only after shell expansion inserts an eager-file default whose target is missing)
        - decision: `pointer` stays the location in the lowered effective schema, reported beside the declaring file; a pointer into the declaring file cannot be mapped back from the lowered schema; documented
        - known limits, not spec defects (every such problem is still attributed to its default with a schema pointer and no invented position; only the named file is the layer's): defaults inside list items or under pattern keys; same path declared by several union arms (first arm's file); prepared/embedded trigger payloads; library-only `SimplifiedSchema`/`Prepared` leave `origin` `None`
        - spec departure (recorded, spec left as written): spec 3.2 shows `AppliedDefault { path, value }`; it now also carries `pointer` and `origin`; docs updated
        - tests: simplified-schema `just test` 2192/2192 plus minimal 1828/1828; darkmatter scoped 611/611 and 119/119; darkmatter-cli 452/452; darkmatter-lsp 205/205; doctest 1/1; `just lint` in simplified-schema and darkmatter exit 0; DMLS attribution test fails without the fix
        - docs: `darkmatter/docs/cli/schema.md`, `darkmatter/docs/cli/data-contracts.md`, `darkmatter/docs/inline/schema-validation.md`, `darkmatter/dmls/docs/diagnostics.md`, `simplified-schema/docs/topics/validation.md`, 17 `cli/schemas/output/*.yaml`, `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`
- work completed for 'R1-F8 eager default failures lose their schema-default origin' at 06:05:39
- starting the work on 'X1 claudine-cli sequence steps read the launch environment map' at 06:05:39
        - root cause: not a production defect; `document_environment_routes` relies on the `test-fixtures`-only `CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS` seam (`claudine/cli/src/request.rs:43`) but its module in `claudine/cli/tests/l1/main.rs` was not gated on that feature (already true at `main`); a build without the feature (the direct `cargo nextest run -p claudine-cli` used by the R1-F2 subagent) cannot pass it, while `just test` and CI pass `--features test-fixtures`
        - defect class: a test that relies on a `test-fixtures`-only `CLAUDINE_TEST_*` seam without being gated on that feature
        - fixed: `claudine/cli/tests/l1/main.rs` gates `document_environment_routes` (12 of 16 failed without the feature); `claudine/cli/tests/l1/lifecycle_message_drain.rs` gates `stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown` (sets the gated `CLAUDINE_TEST_DRAIN_BUDGET_MS`; without the feature it waited 10.9 s and failed)
        - siblings clean: `completion_delayed`, `compose_caller_file_provenance`, `group_branch_lifetime`, `sequence_initialize_include_preflight`, `tts_prewarm` already gated; `CLAUDINE_TEST_DRAIN_STARTED`/`TEARDOWN_HOLD` gated on `terminal-tests` and used only by L2/L3 binaries requiring it
        - production per-step captures confirmed correct with the feature: `iterate.rs:375`, `task_run.rs:824/855`, `container.rs:263`, `iterate.rs:919`, `compose/prep.rs:472`
        - tests: failing test alone with the feature 1/1 (0.648 s); claudine-cli scoped without feature 802/802, with feature 1153/1153; claudine scoped 765/765; `just lint` exit 0
- work completed for 'X1 claudine-cli sequence steps read the launch environment map' at 06:16:36
- starting final verification (package `just test` and `just lint` in darkmatter, simplified-schema, claudine) at 06:16:36
        - first darkmatter package run: 2 failures in `darkmatter-cli::l1 claudine_schema_bundle::` (`sequence_documents_select_the_sequence_trigger_and_validate_through_the_cli`, `depth_four_container_controls_validate_and_their_innermost_defects_are_located`) pinned the pre-R1-F2 expectation that sequence documents do not select `claudine.yaml`; expectations and their comments updated (R1-F2 consumer); module 11/11
        - stale prose found by grep and corrected (R1-F2 consumer): `claudine/docs/topics/schemas.md` said a sequence-only document matches `sequence.yaml` alone and is not offered root event keys
        - first claudine package run stopped on X3 (fail-fast); rerun with `--no-fail-fast` passed all tests
- work completed for final verification at 06:48:08

### Completion Report

- **outcome: eight requested repairs implemented; readiness incomplete because X3 remains unresolved (R2-F2)**; elapsed 3 h 03 min (03:45:17 → 06:48:08)
- parent findings: total 8 = fixed 8 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1, R1-F2, R1-F3, R1-F4, R1-F5, R1-F6, R1-F7, R1-F8: fixed; every child row is fixed or clean with evidence in the ledger
        - discovered rows: X1 fixed (test-gating defect); X2, X2.1, X2.2 cleared under the loaded-host speed rule; X3 unresolved (R2-F2), despite the passing rerun
- open repair-ledger IDs: X3 (now R2-F2); review 2 separately records newly discovered R2-F1
- recorded decisions (ledger only; none meets the author-review bar, so `human_review` is unchanged)
        - R1-F4: URL refusals under an invalid permissions block are located in the message (`file:line:col at /permissions/network`), not a typed `dm.config.invalid` record
        - R1-F5: DMLS judges `{{ }}` commands by shape at pre-flight; it may say unknown where md's runtime fill would approve, never approved where md would ask
        - R1-F6: F6.3/F6.4 ambiguity applies to open nested arms; closed SimplifiedSchema arms correctly have one successful candidate (confirmed with `md`)
        - R1-F8: `AppliedDefault` gained `pointer` and `origin` beyond spec 3.2's shape (docs updated, spec left as written); named-file precision limits listed under R1-F8
        - R1-F2: reverses implementation decision #46, as the review recommends
- verified repairs (summary)
        - R1-F1: `md compose` judges foreign host events as `md schema validate` does; Claudine's runtime escape hatch unchanged
        - R1-F2: `sequence` selects Claudine's main schema; exact-match drift guard; `shipped.json` regenerated
        - R1-F3: Claudine enables remote transclusion everywhere it composes; `network` decides each fetch
        - R1-F4: invalid permissions refuse every URL read before per-run host grants, at projection and gates
        - R1-F5: DMLS shell verdict uses md's template parser, projection, and coverage
        - R1-F6: union validation compares complete candidates whenever an arm can add defaults
        - R1-F7: migration guard single-threaded with a Git pre-filter (Windows 7.3 s → 1.1 s; macOS package run 1.145 s)
        - R1-F8: default-inserted value problems carry their schema default, location, and declaring file through Simplified Schema, md, and DMLS
- final verification (after the last change, no subagents running)
        - `just lint` in simplified-schema, darkmatter, claudine: exit 0 each
        - `just test` in darkmatter: 16680/16680 passed, none at or above 5.0 s (verifies R1-F1, F3 hint, F4, F5, F6.8, F7, F8)
        - `just test` in simplified-schema: 2192/2192 plus minimal build 1828/1828, none slow (R1-F6, R1-F8)
        - `just test --no-fail-fast` in claudine: 12103/12103 passed; 3 tests 5.3–8.3 s under load ≈ 23 on 16 cores, each ≤ 2.75 s alone (X2 rows) (R1-F1.5, R1-F2, R1-F3, X1)
        - `just check-tier-coverage` darkmatter, claudine, simplified-schema: exit 0
        - Windows `just cross-check` (filtered to changed modules): darkmatter 185/185, darkmatter-cli 442/442, darkmatter-lsp 53/53, simplified-schema 41/41, claudine 105/105; all exit 0
- no deferred performance measurement
- changed files: 118 working-tree entries; source and tests in `simplified-schema/lib`, `darkmatter/lib`, `darkmatter/cli` (including all `cli/schemas/output/*.yaml`), `darkmatter/dmls`, `claudine/lib`, `claudine/cli`, `claudine/schemas`, `claudine/lib/generated/shipped.json`; docs in `darkmatter/docs`, `darkmatter/dmls/docs`, `claudine/docs`, `claudine/schemas/README.md`, `simplified-schema/docs`; skills `claudine`, `darkmatter`, `simplified-schema`, `os`; per-finding files are listed in each progress entry above

## Review correction — 2026-11-14 (iteration 2)

The X3 ledger row and completion summary incorrectly treated the recorded performance-report panic as clean because it passed on rerun. The passing run totals above are historical observations and remain unchanged; the conclusion that the defect was cleared was wrong. The initialized route in `claudine/cli/src/commands/compose/prep.rs` records `prep_phase` before `run_staged_single` runs initialization and performs actual preparation. The report then counts the later composition as a structural child of that earlier window. A child lasting longer than the preliminary window makes the debug report panic; a loaded host is not required.

I verified this through the existing isolated `CliProcessFixture`, with a local fake Goose provider, user permission `exact sleep 0.1`, and a document declaring `initialize`. `claudine compose --goose --perf doc.md` with body `::shell sleep 0.1` exited 101 at `cli/src/perf/tree.rs:445` with `perf report failed TR-1 reconciliation` (final reproduction headline 773.120042 ms). The same document without `--perf`, and the same body without `initialize`, exited 0. `inline-compose --perf` also exited 101 when the same directive was placed in its Markdown `prompt` (headline 528.109917 ms); its non-initialized control exited 0. An initialized sequence step with the same prompt completed with an aggregate report. All providers were local fixtures, no real agent ran, and the temporary probes were removed. The remaining code defect is R2-F2; this correction itself is not another finding or a human-review item.

## Implementation of Review Findings #2

> **started at:** 2026-11-14T07:01:33-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- review frontmatter `implemented` confirmed `false` before repairs began
- the review has `human_in_the_loop: null` and no blocked findings; recurrence: none (R2-F1 is not R1-F6; R2-F2 is the unresolved repair-ledger `X3`, not R1-F7)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Required defaults are skipped beneath pattern keys | fixed | every child below verified; shared `Walk::object` repaired; final `just test` simplified-schema 2209/2209 + minimal 1845/1845, darkmatter 16682/16682 |
| R2-F1.1 | Prefix pattern `<starting::x->`, existing `x-build: {}` → strict accepts and reports default; soft inserts `3` | fixed | `required_defaults::object_children::a_prefix_pattern_key` (strict borrowed + default at `x-build.retries`, pointer; soft inserts 3); red before fix |
| R2-F1.2 | Suffix pattern `<ending::-build>` → same | fixed | `object_children::a_suffix_pattern_key`; red before fix |
| R2-F1.3 | Regex pattern `<pattern::^x-.*$>` → same | fixed | `object_children::a_regex_pattern_key`; red before fix |
| R2-F1.4 | Catch-all plus unrelated `^other-` pattern → catch-all applies and supplies default | fixed | `object_children::a_catch_all_key_beside_an_unrelated_pattern`; `the_catch_all_does_not_judge_a_key_a_pattern_covers`; red before fix |
| R2-F1.5 | Literal property control → accepts with default | clean | `object_children::a_literal_key` |
| R2-F1.6 | Catch-all without patterns control → accepts with default | clean | `object_children::a_catch_all_key_without_patterns` |
| R2-F1.7 | Literal property plus unrelated pattern control → accepts with default | clean | `object_children::a_literal_key_beside_an_unrelated_pattern` |
| R2-F1.8 | Array `items` traversal control | clean | `object_children::an_array_item` |
| R2-F1.9 | Tuple `prefixItems` traversal control | clean | `object_children::a_tuple_item` |
| R2-F1.10 | `$defs`/`$ref` traversal control | clean | `object_children::a_local_reference` |
| R2-F1.11 | `allOf` traversal control | clean | `object_children::an_all_of_member` |
| R2-F1.12 | Darkmatter `md schema validate` with prefix/suffix/regex fixtures → accept and report default | fixed | darkmatter-cli `output_commands::conforms_validate_and_compose_complete_an_entry_a_pattern_key_selects` (prefix/suffix/regex exit 0, default with `schema_location`); red before fix |
| R2-F1.13 | Darkmatter `md compose` with same fixtures → `Retries 3` | fixed | same test: composed document contains `Retries 3`; red before fix |
| R2-F1.14 | DMLS schema diagnostics → no `dm.schema.missing_required` for `/x-build` | fixed | darkmatter-lsp `required_defaults::a_required_default_beneath_a_pattern_key_is_present` (no missing-required for `/x-build`; control `r` still reported); red before fix |
| R2-F1.15 | Added: explicit value kept; literal+pattern both apply; pattern-child violation still invalid; every matching pattern applies; uncompilable pattern is a schema error | fixed | `object_children::an_explicit_value_under_a_pattern_key_is_kept`, `a_key_a_literal_and_a_pattern_both_cover_takes_both_schemas_defaults`, `a_value_breaking_the_pattern_child_contract_is_still_invalid`, `every_pattern_a_key_matches_applies`, `a_pattern_that_does_not_compile_is_a_schema_error` |
| R2-F2 | Initialized composition can panic when performance reporting is enabled | fixed | every child below verified; prep window closed after `build_execution_request` in `run_staged_single`; final claudine `just test` 12116/12116 |
| R2-F2.1 | Direct initialized `compose --perf` → exit 0, consistent report | fixed | `wrap_perf::initialized_compose_perf_prep_window_contains_composition` exit 0, composition under prep phase; exited 101 with old stamp |
| R2-F2.2 | Inline initialized `inline-compose --perf` → exit 0, consistent report | fixed | `wrap_perf::initialized_inline_compose_perf_prep_window_contains_composition`; exited 101 with old stamp; body workaround removed after X-R2-1 |
| R2-F2.3 | Direct ordinary `--perf` control | clean | `wrap_perf::compose_perf_emits_report_to_stderr`, `compose_perf_stdout_matches_non_perf` |
| R2-F2.4 | Inline ordinary `--perf` control | clean | `wrap_perf::inline_compose_perf_emits_report_to_stderr`, `inline_compose_perf_stdout_matches_non_perf` |
| R2-F2.5 | Initialized without `--perf` control | clean | `just test-cli compose_initialize` 47/47 through the changed staged route |
| R2-F2.6 | Sequence initialized step `--perf` control | clean | new control `sequence_perf::sequence_perf_initialized_step_composition_nests_under_its_step` (composition under `step 1: one`) |
| R2-F2.7 | Wrapper-only `CommandPerfCollector::new` control | clean | `perf::` unit tests incl. `command_perf_collector_full_report`, `perf_tree_reconciles_wrapper`; wrapper collector carries no composition child |
| R2-F2.8 | Sibling: ordinary route, zero-window wrapper/default timings, `new_with_composition`/`UnderPrep`, env-setup substages, loop route | clean | each child window contained in its parent per code inspection; full claudine run green |
| X-R2-1 | Discovered: inline-compose `prompt` `::shell` permitted by `shell-commands` (spec §2.2) refused as not pre-approved → runs; unpermitted → normal approval refusal | fixed | `inline_prompt_shell_preflight::inline_prompt_permitted_command_runs_into_the_delivered_prompt`, `..._unpermitted_..._refused_as_needing_approval`, `inline_prompt_permitted_command_runs_after_initialize`, `inline_sequence_step_prompt_command_is_approved_by_the_graph_audit`, `inline_retry_prompt_command_with_new_bytes_is_audited_before_it_runs`, `inline_proxy_target_prompt_command_is_audited_before_it_runs`; lib `inline_shell_preflight_*`, `an_inline_prompt_document_is_audited_on_its_prompt_not_its_body`, `inline_late_value_check_walks_the_prompt_not_the_body` |

### Progress

- starting the work on 'R2-F1 required defaults beneath pattern keys' at 07:01:50
        - delegated to a Rust subagent (simplified-schema walker plus sibling sweep across simplified-schema, darkmatter, DMLS)
- starting the work on 'R2-F2 initialized composition perf panic' at 07:02:30
        - run concurrently with R2-F1 because the two repairs share no files or packages (R2-F1: simplified-schema/darkmatter; R2-F2: claudine CLI); both subagents were restricted to filtered test runs to limit host load
        - defect class (R2-F1): the walk that supplies required defaults skips object entries selected by a key pattern, and drops catch-all traversal whenever any pattern exists
        - repaired `Walk::object` in `simplified-schema/lib/src/defaults.rs`: every `patternProperties` schema whose regex matches an existing key is visited (literal property too), `additionalProperties` only for keys no literal property or pattern covers; patterns compile through the package's `TextPattern` (same matcher as `KeyMatcher::Pattern`), cached per walk
        - decision: an uncompilable pattern is `SchemaError::BuildValidator` naming the pattern (alternative "no match", as the coercion shape builder does, rejected as a silent skip); the validator already rejects such a schema first
        - sibling sweep: `Walk::visit` (`$ref`, `allOf`, unions), `Walk::array`, `supplies_default`/`unions_supply_defaults` (walk every child map) clean; `Prepared`, `EffectiveSchema::complete`/`converted_candidate`, `config/load.rs::judge_setting`, DMLS diagnostics all delegate to this walker (wiring rows .12–.14); non-default pattern-aware code (`schema_validation.rs::fragments_below`, `schema_declares_property`, `bound_operands.rs`, `json_shape.rs`) already pattern-correct, clean
        - proven red before the fix: with the old block temporarily restored, 7 of 17 `object_children` tests and both Darkmatter wiring tests failed (DMLS published `"retries" is a required property` at `/x-build`)
        - docs: `simplified-schema/docs/topics/validation.md` documents dictionary entries under required defaults
- work completed for 'R2-F1 required defaults beneath pattern keys' at 07:15:02
        - defect class (R2-F2): a child timing is assigned to a parent report window that closed before that child's work started
        - `run_staged_single` now receives `prep_substages` and `compose_entry` and records `prep_phase` after `build_execution_request`, before `execute_staged_composition` (the ordinary route's boundary); reconciliation tolerance and `debug_assert_reconciles` unchanged
        - decision: `initialize` and the stabilized reread stay in `prep → unattributed` rather than a new Structural substage (report layout unchanged)
        - sibling sweep: ordinary route stamp, `main.rs` zero-window wrapper/default timings, `CommandPerfCollector::new`, `new_with_composition`/`UnderPrep`, `UnderStep` sequence placement (Breakdown child inside `wall_clock`), loop route (no timings), env-setup substages: all clean
        - proven red: both new regressions exited 101 with "perf report failed TR-1 reconciliation" with the old stamp temporarily restored
        - side note (not a spec defect): after a proxy handoff only the first active document carries startup timings, so that run prints no report; behavior predates this feature and the spec sets no requirement on it
        - discovered X-R2-1: an inline-compose `prompt` `::shell` covered by a user `shell-commands` rule is refused "command not pre-approved … a bug in the pre-flight scanner"; reproduced by the orchestrator with the debug binary; the R2-F2 inline regression repeats the directive in the body as a workaround until X-R2-1 is repaired
- work completed for 'R2-F2 initialized composition perf panic' at 07:15:02
- starting the work on 'X-R2-1 inline prompt shell directives not pre-approved' at 07:15:02
        - root cause: inline runs compose a stand-in document (frontmatter + `prompt` text) but pre-flight discovery walked the source body, so the prompt's commands were never discovered and a body command that never runs could be approved or asked about; pre-existing at HEAD, reachable through this feature's spec §2.2 permission contract
        - defect class: a pre-flight discovery pass walks a different text than the composition pass that executes its commands
        - fix: new shared `prepare::document_for_mode` (also used by `prepare_inline_on`); `approve_document_shell`, `preflight_document_shell`, `validate_document_shell_late_values` take a `CompositionMode`; "not pre-approved" guard unchanged, no bypass
        - sibling sites fixed with tests: eager inline path, `initialize`-staged reread, loop iteration audit, sequence graph audit, sequence JIT probe, sequence late-value check, harness retry/resume adoption (`preflight_harness_document` now audits inline; its comment claiming a separate inline audit was wrong and is corrected); proxy into an inline target and system prompts/`::file` clean
        - the R2-F2 inline and sequence perf regressions no longer repeat the directive in the body; both still pass
        - decision: mode passed explicitly to the three audit functions (alternative: storing it on `ResolvedCompositionSource` or `PrepareOptions`; rejected because a source may be composed either way)
        - noted, not a defect of this spec: inline composition diagnostics count lines in the stand-in document (`Origin: line 10` for an 8-line file); pre-existing, the spec states no requirement on inline diagnostic line mapping, and discovery and execution now report the same coordinates
        - docs: `claudine/docs/topics/pre-flight-checks.md` (discovery walks the composed text), `.claude/skills/claudine/cli-commands.md`
- work completed for 'X-R2-1 inline prompt shell directives not pre-approved' at 07:31:54
- starting final verification at 07:31:54
- final verification (after the last change, no subagents running)
        - `just lint` in simplified-schema, darkmatter, claudine: exit 0 each
        - `just test` in simplified-schema: 2209/2209 plus minimal build 1845/1845, none at or above 5.0 s (R2-F1)
        - `just test` in darkmatter: 16682/16682 passed, none at or above 5.0 s (R2-F1.12–.14, no regression elsewhere)
        - `just test` in claudine: 12116/12116 passed; 3 tests 5.4–7.6 s with load average ≈ 20 on 16 cores (loaded host, 10 s allowance); alone 1.55 s, 1.50 s, 2.98 s, so cleared as load-sensitive: `compose_initialize_staged_boot::initialize_shells_are_forbidden_even_with_yolo_across_entry_paths`, `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path`, `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged` (R2-F2, X-R2-1)
        - `just check-tier-coverage` simplified-schema, darkmatter, claudine: exit 0
        - no cross-OS run: the repairs change no path, process, or OS-specific code (a regex walk, a timing stamp location, and which text pre-flight discovers); CI's nightly covers Windows/WSL2

### Completion Report

- **outcome: complete**; started 07:01:33, finished about 07:42 (≈ 41 min elapsed)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R2-F1 fixed: 14 required rows (6 fixed, 8 clean controls) plus R2-F1.15 added cases
        - R2-F2 fixed: 7 required rows (2 fixed, 5 clean controls) plus R2-F2.8 sibling sweep
- discovered and fixed in this cycle: X-R2-1 (inline `prompt` shell directives never pre-approved; pre-existing, violated spec §2.2)
- open finding IDs: none
- noted, not a defect of this spec: inline-composition diagnostics report line numbers from the composed stand-in document rather than the authored file; pre-existing, and the spec says nothing about how inline diagnostics map to lines
- no deferred performance measurement
- changed files (this iteration)
        - R2-F1: `simplified-schema/lib/src/defaults.rs`, `simplified-schema/lib/tests/l1/required_defaults.rs`, `simplified-schema/docs/topics/validation.md`, `darkmatter/cli/tests/l1/output_commands.rs`, `darkmatter/dmls/tests/l1/required_defaults.rs`
        - R2-F2: `claudine/cli/src/commands/compose/prep.rs`, `claudine/cli/src/perf/mod.rs`, `claudine/docs/topics/composition.md`, `claudine/cli/tests/l1/wrap_perf.rs`, `claudine/cli/tests/l1/sequence_perf.rs`
        - X-R2-1: `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/composition/preflight.rs`, `claudine/lib/src/composition/prepare/tests.rs`, `claudine/lib/src/composition/sequence/preflight/tests.rs`, `claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs`, `claudine/lib/src/invocation_context/document_tests.rs`, `claudine/lib/tests/l1/network_transclusion.rs`, `claudine/cli/src/commands/compose/prep.rs`, `claudine/cli/src/commands/wrap/composition/staged_boot.rs`, `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`, `claudine/cli/src/commands/wrap/sequence/jit.rs`, `claudine/cli/src/commands/wrap/sequence/jit/tests.rs`, `claudine/cli/src/commands/wrap/sequence/task_run.rs`, `claudine/cli/tests/l1/inline_prompt_shell_preflight.rs` (new), `claudine/cli/tests/l1/main.rs`, `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/cli-commands.md`

## Implementation of Review Findings #3

> **started at:** 2026-11-14T07:52:03-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- review frontmatter `implemented` confirmed `false` before repairs began
- the review has `human_in_the_loop: null`, no blocked findings, and `recurrence: false`; one finding (R3-F1, high) with fourteen child rows

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Inline shell diagnostics publish generated document positions | fixed | every required child verified below plus discovered R3-F1.15–.18; `PromptOrigin` translates published origins at the shared projection; final `just test` claudine 12129/12129, darkmatter 16683/16683 |
| R3-F1.1 | Direct inline approval → name the real file and `prompt`, no generated line | fixed | lib `prepare::tests::inline_shell_preflight_names_the_prompt_property_not_a_generated_line` (multi-line); CLI `inline_prompt_shell_preflight::inline_prompt_unpermitted_command_without_a_terminal_is_refused_as_needing_approval` (direct route); red before fix (`doc.md:10`) |
| R3-F1.2 | Initialized reread approval → same rule | fixed | same lib test (initialized shape) and CLI test (initialized route) |
| R3-F1.3 | Inline loop approval → same rule | fixed | same lib test (loop shape) and CLI test (loop route) |
| R3-F1.4 | Sequence graph approval → attribute the step's authored prompt | fixed | lib `sequence::preflight::tests::template_position::an_inline_prompt_step_refusal_names_the_prompt_property`; red before fix |
| R3-F1.5 | Retry/resume harness approval → prompt origin preserved across attempts | fixed | CLI `inline_prompt_shell_preflight::inline_retry_prompt_refusal_names_the_prompt_property` (was `doc.md:16`); resume shares `preflight_harness_document` → `approve_document_shell` |
| R3-F1.6 | Proxy target approval → attribute the target's prompt | fixed | CLI `inline_prompt_shell_preflight::inline_proxy_target_prompt_refusal_names_the_prompt_property` |
| R3-F1.7 | Single-line prompt projection → prompt property, not the body's line | fixed | lib `inline_shell_preflight_names_the_prompt_property_not_a_generated_line` (single-line shape) |
| R3-F1.8 | Shell-block command projection → no generated block coordinates | fixed | same lib test (shell-block shape) |
| R3-F1.9 | Actual inline shell execution failure → real source + prompt origin, no invented excerpt | fixed | lib `prepare::tests::inline_shell_execution_failure_names_the_prompt_of_the_real_file` (origin `Frontmatter { prompt, line: None }`, context = real file text); CLI `inline_prompt_shell_preflight::inline_prompt_command_failure_names_the_real_file_and_prompt` (no `unknown`, no `line 10`) |
| R3-F1.10 | Passive late-value validation → returned error carries prompt origin and real source | fixed | lib `prepare::tests::inline_late_value_failure_names_the_prompt_of_the_real_file` asserts the returned `ShellExpansionFailed` origin and context |
| R3-F1.11 | Chained-document control → body line 6 kept | clean | lib `inline_shell_preflight_keeps_body_child_and_frontmatter_origins`; chained late-value control keeps `Body { line: 6 }` |
| R3-F1.12 | Transcluded child control → `child.md:3` kept | clean | same lib test (approval); lib `inline_shell_execution_failure_in_a_child_keeps_the_child_origin` (execution, `Body { line: 3 }` in `child.md`) |
| R3-F1.13 | Frontmatter shell-value control → line 3 kept | clean | same lib test: line 3 kept, now also `property: "value"` |
| R3-F1.14 | Prompt expression and missing-file controls → unchanged | clean | these are not shell errors; `PromptOrigin::composition_error` only touches `ShellExpansionFailed`/`PreFlightDiscoveryFailed(ShellExpansion)`, so their path is unchanged by construction |
| R3-F1.15 | Discovered: interactive approval prompt request for byte-path commands showed `Body { line: 10 }` | fixed | lib `prepare::tests::inline_shell_preflight_prompts_with_the_prompt_property`; red before fix |
| R3-F1.16 | Discovered: inline composition warnings printed prompt-text line numbers | fixed | lib `prepare::tests::inline_prompt_warnings_carry_no_generated_line` (root warning has no line; child keeps `Some(3)`); red before fix |
| R3-F1.17 | Sibling: `harness/audit.rs` passthrough audit | clean | reads the real source file, never the generated document |
| R3-F1.18 | Discovered in reconciliation: remote (URL) child failure also has an `unknown` context and would be moved onto the source's `prompt` | fixed | `network_transclusion::an_inline_prompt_keeps_a_remote_childs_shell_failure_its_own`; red with the subagent's heuristic, green after `shows_generated_text` |

### Progress

- starting the work on 'R3-F1 inline shell diagnostics publish generated positions' at 07:52:30
        - defect class: a projection that moves authored frontmatter text into a generated Markdown body loses its source identity, so the generated body's coordinates are reported as authored file positions
        - delegated to one Rust subagent (single finding; no parallel work)
        - subagent report (rust-developer, 07:53–08:20)
                - fix (scope corrected 2026-11-14): `prepare::PromptOrigin` rewrites published origins in single-document and sequence-graph discovery (entries and candidates), approval refusals and the byte-path approval prompt request, unchanged-body `ShellExpansionFailed` errors, warnings, and late-value errors; errors after body rewrites and shell-block error wrappers remain untranslated (see the dated correction below); Darkmatter's own discovery/execution keys are untouched, so the not-pre-approved guard (matches exact bytes and template shapes, not origins) and the origin-keyed `DocumentShellValues` store are unchanged
                - decision: reuse `ShellCommandOrigin::Frontmatter { key: "prompt", line: None }` with a real `SourceContext` (no numeric line, no generated excerpt; the review's smallest acceptable repair); added Darkmatter `ShellExpansionError::location_mut` and `ComposeWarning::document`; rejected: a new origin variant threaded through about 20 Darkmatter construction sites, because the origin-keyed late-value store would need a lockstep two-sided change
                - public change: `property: Option<String>` on `CompositionError::ShellApprovalUnavailable` and `ShellCommandDenied`; `composition.shell_approval` registry detail gains `property`; messages read ``doc.md (frontmatter `prompt`)``
                - extra sites discovered and fixed: R3-F1.15 (interactive approval prompt request showed `Body { line: 10 }`), R3-F1.16 (inline composition warnings printed prompt-text lines); R3-F1.17 `harness/audit.rs` clean (reads the real file)
                - red before green: with the translation disabled, 4 of 10 library and 4 of 9 CLI tests failed with the review's exact symptoms (`doc.md:10`, retry `doc.md:16`, `unknown`, `Origin: line 10`)
                - docs: `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/cli-commands.md`
        - orchestrator reconciliation
                - the subagent's context discriminant treated any `unknown`-path context as the generated document; a remote (`https://…`) child also has no file path, and its `::shell` runs under `network: allow`, so its failure would have been moved onto the source file's `prompt` (a fabricated attribution of the same defect class); added as R3-F1.18
                - fix: `PromptOrigin` now holds the generated document and moves a file-less context only when it shows that document's text (`shows_generated_text`); every constructor passes the generated document
                - test: `network_transclusion::an_inline_prompt_keeps_a_remote_childs_shell_failure_its_own` (loopback responder, `network: allow`); red with the previous heuristic (origin `Frontmatter { prompt }`, context `work/doc.md`), green after
                - docs: `pre-flight-checks.md` states a remote child's failure keeps its own text and line
- work completed for 'R3-F1 inline shell diagnostics publish generated positions' at 08:22:10
- starting final verification at 08:22:10
- final verification (after the last change, no subagents running)
        - `just lint` in claudine and darkmatter: exit 0 each
        - `just test` in claudine, first attempt: stopped after 6993 of 12129 with six `completion_delayed::*` failures; cause was my own concurrent `just lint`, which rebuilt `target/debug/claudine` without `test-fixtures` mid-run; the six pass alone (`just test-cli completion_delayed`, 6/6), not a code defect
        - `just test` in claudine, clean rerun: 12129/12129 passed; 2 tests over 5 s with load average ≈ 20 on 16 cores (loaded host, 10 s allowance): `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path` 6.44 s (alone 1.76 s), `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged` 8.29 s (alone 3.14 s); cleared as load-sensitive, same two recorded in iteration 2 and untouched by this repair
        - `just test` in darkmatter: 16683/16683 passed, none at or above 5.0 s
        - `just check-tier-coverage` claudine and darkmatter: exit 0
        - no cross-OS run: the repair changes no process or OS-specific code; the only path logic reuses `biscuit_file::canonicalize_simplified`; the new library tests use `mkdir` of an existing directory, which fails under `sh` and `cmd` alike; CI's nightly covers Windows/WSL2

### Completion Report

- **outcome: complete**; started 07:52:03, finished about 08:35 (≈ 43 min elapsed)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R3-F1 fixed: 14 required rows (10 fixed, 4 clean controls) plus discovered R3-F1.15, R3-F1.16, R3-F1.18 (fixed) and R3-F1.17 (clean)
- open finding IDs: none
- no deferred performance measurement; no human-review item (the repair is the review's stated smallest acceptable repair)
- changed files (this iteration, all R3-F1)
        - `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (`ShellExpansionError::location_mut`, `Frontmatter` variant doc, test), `darkmatter/lib/src/markdown/compose/context/report.rs` (`ComposeWarning::document`)
        - `claudine/lib/src/composition/prepare.rs` (`PromptOrigin`, applied to approval, inline execution, warnings, late-value validation), `claudine/lib/src/composition/preflight.rs` (single-document and sequence-graph publication), `claudine/lib/src/harness/shell.rs`, `claudine/lib/src/composition/error/mod.rs`, `claudine/lib/src/composition/error/render/mod.rs`, `claudine/lib/src/diagnostics/registry.rs`
        - tests: `claudine/lib/src/composition/prepare/tests.rs`, `claudine/lib/src/composition/preflight/tests.rs`, `claudine/lib/src/composition/sequence/preflight/tests.rs`, `claudine/lib/tests/l1/network_transclusion.rs`, `claudine/cli/tests/l1/inline_prompt_shell_preflight.rs`, `claudine/cli/tests/l1/error_guards.rs`
        - docs: `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/cli-commands.md`
- final verification tied to R3-F1: claudine `just test` 12129/12129, darkmatter `just test` 16683/16683, both `just lint` and `just check-tier-coverage` exit 0


## Review correction — 2026-11-14 (iteration 4)

The iteration-3 progress entry originally said `PromptOrigin` translated published origins “at every site,” including inline composition errors without qualification. That behavior claim was too broad and is corrected in place above. The original fourteen review cases and four added ledger cases pass, but a normal `Update {{ word }}.` before `::shell mkdir taken` changes the temporary body, so `shows_generated_text` no longer recognizes its execution-error context. The error still reports `unknown` and a generated body line. Shell-block command failures, invalid opener options, and an unclosed shell block also bypass `PromptOrigin::composition_error` through `ShellBlock` or `PageBlock` error wrappers.

Verified through temporary tests of the public Claudine preparation API and the shipped CLI using `CliProcessFixture`, an isolated home, `exact mkdir taken`, an existing `taken` directory, and a fake Goose executable that was never launched. The CLI reported `Origin: line 11` for the interpolated nine-line document, block/command lines 13/14 for an eleven-line block document, and a context-free line 3 for `timeout=bad`. The unclosed block named `unknown`. The public tests independently retained those generated locations. These newly checked sibling paths are recorded in review 4; the original ledger's passing cases remain valid. All temporary test additions were restored byte-for-byte.

## Implementation of Review Findings #4

> **started at:** 2026-11-14T08:42:49-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began
- the review has no `human_in_the_loop` items and no blocked findings; recurrence list: R4-F1 repeats R3-F1's source-projection class (rewritten runtime contexts, `ShellBlock`/`PageBlock` wrappers)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R4-F1 | Inline source attribution misses rewritten prompts and block errors | fixed | every required child R4-F1.1–R4-F1.17 verified; discovered R4-F1.18–R4-F1.24 and R4-F1.26 fixed, R4-F1.25 clean; final claudine `just test` 12147/12147, darkmatter 16693/16693 |
| R4-F1.1 | Literal prompt, literal failing command → real file and `prompt`, no numeric line (control) | clean | `inline_shell_execution_failure_names_the_prompt_of_the_real_file` passes before and after |
| R4-F1.2 | Unrelated frontmatter interpolation, literal prompt body → real file and `prompt` (control) | clean | `composition::prepare::tests::inline_shell_failures_after_rewrites_and_in_blocks_name_the_prompt` frontmatter-only case |
| R4-F1.3 | `Update {{ word }}.` before `mkdir taken` → real file and `prompt`, no line/excerpt | fixed | same table test; red `at line 11` → green |
| R4-F1.4 | True `::block` removed before the command → same attribution | fixed | same table test; red `at line 12` → green |
| R4-F1.5 | `::shell mkdir {{ word }}` → same attribution | fixed | same table test; red `at line 11` → green |
| R4-F1.6 | Failing command in `::shell-block` → outer and inner attributed to `prompt`, positions suppressed | fixed | same table test plus `inline_shell_block_failures_keep_their_cause_and_reason`; red block lines 10/11 → green |
| R4-F1.7 | Same block after `Update {{ word }}.` → same | fixed | same table test; red block lines 11/12 → green |
| R4-F1.8 | `::shell-block timeout=bad` → real file and `prompt`, no unqualified temp line | fixed | same two tests; red `parse error at line 2` → green |
| R4-F1.9 | Unclosed shell block (`PageBlock`) → real file and `prompt`, no invented line | fixed | same two tests; red `PageBlock` with `unknown`, line 2 → green |
| R4-F1.10 | Approval publication for interpolated prompt / bad block option → real file, property `prompt` (control) | clean | interpolated and bad-option shapes added to `inline_shell_preflight_names_the_prompt_property_not_a_generated_line` |
| R4-F1.11 | Passive late-value validation controls → unchanged | clean | `inline_late_value_failure_names_the_prompt_of_the_real_file`, `inline_late_value_check_walks_the_prompt_not_the_body` |
| R4-F1.12 | Local transcluded child failure keeps child file and line 3 (control) | clean | `inline_shell_execution_failure_in_a_child_keeps_the_child_origin`; child case of `inline_directive_failures_name_the_prompt_and_a_childs_keep_its_own` keeps `child.md:3` |
| R4-F1.13 | Remote child failure keeps its text and line 1 (control) | clean | `network_transclusion::an_inline_prompt_keeps_a_remote_childs_shell_failure_its_own`, new `…_block_errors_its_own`, `…_collection_failure_its_own` |
| R4-F1.14 | Ordinary body and frontmatter-value controls keep authored locations | clean | `inline_shell_preflight_keeps_body_child_and_frontmatter_origins` |
| R4-F1.15 | Warnings: root no line, child line 3 (control) | clean | `inline_prompt_warnings_carry_no_generated_line` |
| R4-F1.16 | CLI rendering of interpolated directive, block command, invalid option, unclosed block → corrected source/property, provider never launches | fixed | four `macro_rules!` CLI rows in `inline_prompt_shell_preflight.rs` (interpolation, block command, invalid option, unclosed block); red with handling disabled → green; agent never launched; each < 1 s |
| R4-F1.17 | Comments and current docs describe the resulting coverage | fixed | `PromptOrigin`/`document_for_mode` docs rewritten; `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/cli-commands.md`, `darkmatter/docs/inline/{shell-blocks,toc-linking,file-links}.md`, `darkmatter/docs/errors/composition-problems.md`, `.claude/skills/darkmatter/compose.md` |
| R4-F1.18 | (discovered) `::map`/`::filter` errors in the prompt → real file and `prompt`; child controls keep their own | fixed | `composition::prepare::tests::inline_collection_and_pairing_failures_name_the_prompt` (10 cases red → green); Darkmatter `collection_blocks::provenance::a_context_identity_names_a_file_less_documents_collection_failures`; CLI `inline_prompt_collection_failure_names_the_prompt`; local/remote child controls |
| R4-F1.19 | (discovered) block-pair scanner messages embed a generated-body line → position-free message | fixed | `block_pairs::tests::reasons_are_position_free_and_their_line_is_a_field` red → green; crossed-close and unmatched-closer table cases |
| R4-F1.20 | (discovered) `::toc-linking` errors in the prompt → real file and `prompt`; a child's keep its own | fixed | `composition::prepare::tests::inline_toc_and_file_links_failures_name_the_prompt`; `inline_prompt_keeps_a_local_childs_toc_failure_its_own`; CLI `inline_prompt_toc_linking_failure_names_the_prompt`; `directive_error_document::*` |
| R4-F1.21 | (discovered) `::file-links` errors in the prompt → real file and `prompt`; a child's keep its own | fixed | same table test; CLI `inline_prompt_file_links_failure_names_the_prompt`; `directive_error_document::*` |
| R4-F1.22 | (discovered) `md compose` keeps the scanner's secondary lines for ordinary files after R4-F1.19 | fixed | `shell_block_integration::pairing_errors_keep_their_secondary_lines_for_an_ordinary_file` (5 rows) red → green; `md compose` prints `(opened at line 4)` |
| R4-F1.23 | (discovered) tolerated `::toc-linking`/`::file-links` warnings from the prompt carry no generated line | fixed | `inline_tolerated_toc_and_file_links_warnings_carry_no_generated_line`; `tolerated_toc_linking_and_file_links_warnings_name_a_line_only_without_an_identity` |
| R4-F1.24 | (discovered) `::file-links` `FileSystem` construction error carries no generated line | fixed | code inspection only: the `FileSystem` construction error drops its line for an identified document; a test would need a filesystem race between discovery and construction |
| R4-F1.25 | (discovered) a `::file` of the source itself in the prompt keeps the child's own attribution (ownership by identity, not source file) | clean | unreachable: self-transclusion is `CycleDetected` before the child runs; pinned by `inline_prompt_transcluding_its_own_source_is_a_cycle` |
| R4-F1.26 | (discovered) `md compose` names a child's `::toc-linking`/`::file-links` error's own file | fixed | `report_diagnostics::a_toc_or_file_links_failure_names_the_document_holding_it` (lib, red named `toc-root.md`) and CLI `compose_report_diagnostics::…` with `md compose --output json` |
| R4-F2 | Claudine skill incorrectly says pattern-key defaults are skipped | fixed | R4-F2.1 fixed; sweep clean |
| R4-F2.1 | `.claude/skills/claudine/SKILL.md` statement corrected; sweep other skills/docs for the same obsolete claim | fixed | `.claude/skills/claudine/SKILL.md` claim removed; behavior confirmed in `simplified-schema/lib/src/defaults.rs` and the `object_children` cases; targeted grep returns nothing |

- starting the work on 'R4-F1 inline source attribution misses rewritten prompts and block errors' at 08:43:30
        - defect class: source attribution of an inline run's generated body is inferred from mutable temporary text or applied to only some error wrappers, so later diagnostics publish temporary-body coordinates instead of the source's `prompt` property
        - subagent 1 (repair of R4-F1.1–R4-F1.17)
                - stable identity: Darkmatter `Markdown::with_context_identity(path)` makes every error context of a file-less document carry that path as `absolute` (display path stays `unknown`); Claudine's `document_for_mode` sets `<prompt of {path}>`, and `PromptOrigin` recognizes the generated body by identity only (`shows_generated_text` and the text-equality rule removed); remote children (no identity) and local children (own file) keep their attribution
                - Darkmatter now records the composed document's file on every `ShellBlockError` (option errors previously had none); side effect: `md compose` names the child file for a child's `::shell-block` option error instead of falling back to the root
                - new `CompositionError::InlinePromptDirectiveFailed { source_path, property: "prompt", message, command }` (code `composition.failed`) for root-owned `ShellBlock`, `PageBlock`, and line-carrying `Transclusion` errors; message built from fields with no position; block command cause carried in `command`, already moved to `prompt`
                - decisions: context identity rather than "every file-less context" (forbidden by the review; captures remote children); not a file source for the generated document (changes resolution/schema/capture); new Claudine variant rather than optional lines in Darkmatter types; block command failure folded into the new variant (keeps code `composition.failed`, drops earlier-command output)
                - sweep clean: interpolation/call-check errors (`Effective` with `origin_key: prompt`, no line), missing-file `Io`, context-free errors (cycle, depth, remote fetch, relevel), `MissingSourceContext` (unreachable with a resolution context), remote-child page-block errors (tolerated warning)
                - newly discovered and open after subagent 1: R4-F1.18 (`::map`/`::filter` errors use Darkmatter's separate `document` placeholder plus generated lines) and R4-F1.19 (block-pair scanner messages embed a generated-body line)
                - red evidence from temporary edits restored byte-for-byte (`cmp`)
                - scoped checks: claudine library 23/23, CLI 22/22; darkmatter 807/807; wider claudine 706/706 and darkmatter 1096/1096; `just lint` exit 0 in both
                - two load-sensitive tests at 5.12 s / 5.73 s (load ≈ 8.8), 1.57 s / 1.54 s alone, untouched by this change
- starting the work on 'R4-F1.18/R4-F1.19 collection and scanner-message attribution' at 09:12:42
        - subagent 2 (R4-F1.18, R4-F1.19)
                - R4-F1.18 fixed: Darkmatter `Markdown::identified_unfiled` puts the context identity on collection sites (`inline/collection_blocks.rs::anchor`, `call_precheck.rs::collection_rejection`); Claudine `PromptOrigin::translate` moves root-owned collection and `InIteration` errors to `InlinePromptDirectiveFailed`; `composition::prepare::tests::inline_collection_and_pairing_failures_name_the_prompt` 10 cases red → green; Darkmatter `collection_blocks::provenance::a_context_identity_names_a_file_less_documents_collection_failures`; CLI `inline_prompt_collection_failure_names_the_prompt`; local and remote child collection controls pass
                - R4-F1.19 fixed: `BlockPairError` display is position-free (line kept in fields, `line()`); `block_pairs::tests::reasons_are_position_free_and_their_line_is_a_field` red → green
                - CLI helper's "names the prompt" check made stricter (the previous check also matched the run header)
                - newly discovered: R4-F1.20 `::toc-linking` errors and R4-F1.21 `::file-links` errors carry generated-body lines with no file; root and child are indistinguishable
                - orchestrator review: the position-free scanner display dropped secondary lines (crossed close opener, enclosing shell block) from `md compose` for ordinary files; recorded as R4-F1.22 (no `md compose` diagnostic regression) and sent back for repair
                - checks: darkmatter 1302/1302, 351/351, 7/7; claudine 26/26, CLI 23/23, 318/318, 278/278; `just lint` exit 0 both; no test ≥ 5 s
- starting the work on 'R4-F1.20–R4-F1.22' at 09:35:05
        - subagent 3 (R4-F1.20–R4-F1.22)
                - R4-F1.20 fixed: `TocLinkingError` line-bearing variants gain `document: Option<PathBuf>` (file, or context identity via `Markdown::error_document`), set at the compose stage (`pipeline/phases.rs`), the engine (`transclusion/engine.rs`), and the pre-flight walk (`preflight/collect.rs`, found while making the child control pass); `reason()` gives a position-free message; Claudine matches by identity only (`is_identity`); `composition::prepare::tests::inline_toc_and_file_links_failures_name_the_prompt` red → green; `inline_prompt_keeps_a_local_childs_toc_failure_its_own`; CLI `inline_prompt_toc_linking_failure_names_the_prompt`; Darkmatter `directive_error_document::*`
                - R4-F1.21 fixed: same repair on `FileLinksError` (six variants); CLI `inline_prompt_file_links_failure_names_the_prompt`
                - R4-F1.22 fixed: `BlockPairError::note()` plus `note: Option<String>` on `PageBlockError::ParseDirective`, `ShellBlockError::Parse`, `CollectionError::Malformed`, shown by Darkmatter's display; Claudine publishes `message` only; `shell_block_integration::pairing_errors_keep_their_secondary_lines_for_an_ordinary_file` red → green
                - discovered and fixed R4-F1.23: tolerated `::toc-linking`/`::file-links` warnings embedded a generated line in their text; Darkmatter words them with `reason()` for an identified document (`Markdown::generated_identity`); `inline_tolerated_toc_and_file_links_warnings_carry_no_generated_line`, `tolerated_toc_linking_and_file_links_warnings_name_a_line_only_without_an_identity`
                - discovered and fixed R4-F1.24: `Transform("failed to create FileSystem for ::file-links at line N")` drops the line for an identified document; no test (requires a filesystem race), recorded as code-inspection evidence
                - sweep clean with reasons: `PageBlockError`/`ConditionError` (identity), `ShellExpansionError` (`location_mut`), `TransclusionError` ctx variants (`directive_reason`), `MissingSourceContext` (unreachable), `DeferredSetError` line warnings (only under recoveries Claudine never sets), `RemoteReadError.line` (discovery record), `BlockScanError` (language-server only), `ReferenceError` (no reference check in Claudine)
                - checks: darkmatter 1131/1131, 307/307, 3/3; claudine 414/414, CLI 25/25; `just lint` exit 0 both; slowest test ≈ 0.66 s
                - orchestrator review: Claudine's `owns_block`, `site`, and `warning` still decide ownership by the source file, so a `::file` of the source itself in the prompt could be attributed to `prompt` (R4-F1.25); `md compose` publishes a child's `::toc-linking`/`::file-links` error against the root document although the new `document` field names the child (R4-F1.26); sent to a fourth subagent
- starting the work on 'R4-F1.25/R4-F1.26' at 10:11:57
        - subagent 4 (R4-F1.25, R4-F1.26)
                - R4-F1.25 clean (unreachable): an inline prompt that transcludes its own source is refused as `CycleDetected` (the generated document enters the chain under the source path) before the child runs, and the pre-flight walk approves none of its commands; any real child that `is_source` would match is a cycle first (cycle detection keys on the canonical path); pin test `composition::prepare::tests::inline_prompt_transcluding_its_own_source_is_a_cycle`; one-line comment on `owns_block`
                - R4-F1.26 fixed: `report_diagnostics::leaf_diagnostic` takes a toc-linking/file-links error's file from its own `document()` (identities filtered by `named`); the tolerated-warning path (`phases.rs::tolerated_failure_warning`) names the child's document too; Darkmatter `report_diagnostics::a_toc_or_file_links_failure_names_the_document_holding_it` (red: named `toc-root.md`) and CLI `compose_report_diagnostics::a_toc_or_file_links_failure_names_the_document_holding_it` (`md compose --output json`); doc row in `darkmatter/docs/errors/composition-problems.md`
                - checks: darkmatter 695/695 and 651/651, claudine 415/415, CLI 25/25, `just lint` exit 0 both
        - orchestrator: moved the `transclusion_failure_is_fatal` doc paragraph, which sat above `tolerated_failure_warning` (drift present at HEAD, in a function this change edits), back onto its function
- work completed for 'R4-F1 inline source attribution misses rewritten prompts and block errors' at 10:26:40
- starting the work on 'R4-F2 Claudine skill incorrectly says pattern-key defaults are skipped' at 10:26:40
        - defect class: documentation still describes the retired limitation that defaults are not applied under pattern keys
        - R4-F2.1 fixed: `.claude/skills/claudine/SKILL.md` "The default engine never fills values under `patternProperties`, so event and provider maps list their keys explicitly." → "Event and provider maps list their keys explicitly." (behavior confirmed in `simplified-schema/lib/src/defaults.rs` pattern walk and the `object_children` cases)
        - sweep across `.claude/skills`, `claudine/docs`, `darkmatter/docs`, `simplified-schema/docs`, the three `lib/src` trees, and `claudine/schemas`: every other hit is clean (states the current fill behavior, e.g. `simplified-schema/docs/topics/validation.md:190`, or is about provenance listing/lowering/completion, not filling); targeted grep for "never/not/skip … pattern … default" returns nothing
- work completed for 'R4-F2 Claudine skill incorrectly says pattern-key defaults are skipped' at 10:27:17
- starting final verification at 10:27:17
- final verification (after the last change, no subagents running)
        - `just lint` in darkmatter and claudine: exit 0 each (only the existing linker `__eh_frame` message)
        - `just test` in darkmatter: 16693/16693 passed, none at or above 5.0 s
        - `just test` in claudine: 12147/12147 passed; 2 over 5 s with load average ≈ 28 on 16 cores (loaded host, 10 s allowance): `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path` 6.43 s (alone 1.88 s), `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged` 7.85 s (alone 3.20 s); both cleared as load-sensitive, the same two recorded in iterations 2 and 3, untouched by this cycle
        - `just check-tier-coverage` claudine and darkmatter: exit 0
        - simplified-schema: only a skill document changed for it (R4-F2), so no code gate applies
        - no cross-OS run: the repair changes error attribution and messages only, with no process, filesystem, or `#[cfg]` code; the one path value (the context identity) is built with `biscuit_file::to_portable_string` and only compared, never opened; CI's nightly covers Windows and WSL2

### Completion Report

- **outcome: complete**; started 08:42:49, finished 10:39:18 (about 2 h elapsed)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R4-F1 fixed: 17 required rows (9 fixed: .3–.9, .16, .17; 8 clean controls: .1, .2, .10–.15) plus discovered rows R4-F1.18–R4-F1.24 and R4-F1.26 (fixed) and R4-F1.25 (clean, unreachable)
        - R4-F2 fixed: R4-F2.1 fixed, sweep clean
- open finding IDs: none
- no deferred performance measurement and no human-review item: every decision follows the review's recommended repair (stable source identity, root-owned wrapper handling, real source plus `prompt` with no unproven position)
- user-visible changes worth noting for the next review
        - Claudine publishes root-owned directive failures of an inline prompt as `CompositionError::InlinePromptDirectiveFailed` (code `composition.failed`); a failing `::shell-block` command keeps its shell cause in `command` but no longer prints earlier commands' output
        - `md compose` names the child file for a child's `::shell-block` option error and a child's `::toc-linking`/`::file-links` error or tolerated warning, instead of the root
        - block-pairing secondary lines moved from the reason text into a separate `note` shown by Darkmatter's display
- changed files (this iteration)
        - Darkmatter lib: `darkmatter/lib/src/markdown/mod.rs`, `compose/block_pairs.rs`, `compose/call_precheck.rs`, `compose/inline/collection_blocks.rs`, `compose/{page_blocks,shell_blocks,collection_blocks,toc_linking,file_links}/*`, `compose/shell_expansion/site.rs`, `compose/pipeline/phases.rs`, `compose/transclusion/engine.rs`, `compose/preflight/collect.rs`, `compose/report_diagnostics.rs`, `errors/mod.rs`
        - Darkmatter tests: `darkmatter/lib/tests/l1/{directive_error_document.rs (new), main.rs, shell_block_integration.rs, diagnostic_source_provenance.rs, report_diagnostics.rs, collection_blocks/provenance.rs, error_snapshots/{page_block,toc_linking}.rs}`, `darkmatter/cli/tests/l1/compose_report_diagnostics.rs`
        - Claudine: `claudine/lib/src/composition/{prepare.rs, preflight.rs, error/mod.rs, error/render/mod.rs, error/render/provider.rs}`
        - Claudine tests: `claudine/lib/src/composition/prepare/tests.rs`, `claudine/lib/tests/l1/network_transclusion.rs`, `claudine/cli/tests/l1/{inline_prompt_shell_preflight.rs, error_guards.rs}`
        - docs and skills: `claudine/docs/topics/pre-flight-checks.md`, `darkmatter/docs/inline/{shell-blocks,toc-linking,file-links}.md`, `darkmatter/docs/errors/composition-problems.md`, `.claude/skills/claudine/{SKILL.md,cli-commands.md}`, `.claude/skills/darkmatter/compose.md`
- final verification tied to findings: R4-F1 by claudine `just test` 12147/12147 and darkmatter `just test` 16693/16693 (including every test named in the ledger); R4-F2 by the targeted documentation grep; `just lint` and `just check-tier-coverage` exit 0 in both areas
