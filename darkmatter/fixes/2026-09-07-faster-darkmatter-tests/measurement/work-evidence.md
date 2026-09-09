# Phase 9 eliminated-work evidence

Timing is not the proof in this table. Each claim is backed by an observable
counter or sentinel assertion selected by both full candidate L1 populations.
The accepted measurement report records twelve candidate executions for every
L1 identity below: two warm-ups and ten alternating suite runs.

| Work claim | Counter or sentinel | Public observable proof |
|---|---|---|
| Incidental host/repository discovery is excluded from deterministic CLI tests | Hostile inherited CWD, home/config/cache, `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `PATH`, terminal, rendering, and application values are sentinels. | `md_process_fixture::inherited_families_do_not_reach_the_child_and_fixture_defaults_win` asserts none reach the real child and fixture-owned values win. `spawn_site_guard::l1_tests_spawn_md_through_the_fixture_builder` asserts the complete deterministic L1 population has no raw `md` spawn bypass. |
| Passive schema and DMLS paths do not compose or evaluate executable content | `engine_build_count()` remains unchanged around expression validation, passive trigger matching, and every DMLS read-side request. The DMLS corpus includes the exact `$(echo pwned)`, `::shell touch SENTINEL_SHOULD_NOT_EXIST`, `predict_conflicts(...)`, missing-file, and remote-URL forms; the file sentinel remains absent. | `schemas_literal_expression::expression_validation_never_evaluates`, `meta_schema_phase4::semantic_types_match_triggers_by_passive_parse`, and `no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets` assert valid diagnostics/completion/hover/definition/reference/token outputs as well as the zero delta/absent sentinel. |
| Passive paths execute no effects | `engine_build_count()` is unchanged, and the shell-created file sentinel remains absent. | The same three tests assert dependent validation and language-server outputs before checking the counters, so a shortcut that skipped behavior would fail. |
| Passive and denied paths make no HTTP request | `network_attempt_count()` remains unchanged for passive library/DMLS paths. The CLI loopback fixture reports `request_count() == 0` and an empty request corpus for denied-host cases. | `no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets`, `compose_remote_caching::test_compose_remote_deny_all_fails_without_request`, and `test_compose_remote_epilogue_deny_all_fails_without_request`. Allowed/cache cases separately assert exact GET paths, loopback Host headers, refresh count 2, fallback count 2, and repeated TTL read count 1. |

The original loaded failure of
`test_compose_remote_prologue_allowed_host_fetches_url` is retained under
`rejected-http-regression/`. It used the shipped CLI path and exact original
frontmatter input. After accepted sockets were explicitly returned to blocking
mode, that identity passed twelve loaded candidate executions (0 failures, 0
retries; 0.246/0.294/0.590 seconds min/median/max) while preserving rendered
output and captured `/intro.md` request assertions.

Discovery and composition have no general-purpose production counters in this
source state. The evidence above is therefore deliberately scoped to the
eliminated test paths and uses hostile/executable sentinels rather than making
an unsupported repository-wide zero-work claim.
