# Phase 2 Requirement-to-Test Map

This map was completed before applying the foundation tree to the integration
worktree. Tests named below are present at frozen foundation revision
`43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` unless marked as an integration
addition.

| Changed behavior | Public result and variants | Targeted evidence |
|---|---|---|
| Typed diagnostics survive discovery, erasure, rendering, lifecycle `err.*`, and serialization | Semantic versus transparent causes; registered versus unstructured errors; present-null detail; plain/color/OSC 8 rendering; initialize and terminal routes | `diagnostics::discovery::tests`, `diagnostics::snapshot::tests`, `diagnostics::restored::tests`, `error_guards`, `effective_diagnostic_render`, `characterization_error_routes`, and `level2_typed_error_render_capture` |
| Diagnostic snapshots persist losslessly | Every facet, detail object, message, nested cause, unknown additive fields, and stable repeated serialization | `snapshot_round_trips_every_facet_detail_message_and_cause` performs a second byte-identical serialization cycle; `a_selected_snapshot_round_trips_through_json` covers selector output |
| `FileReference` owns grammar and cross-platform classification | Bare, `./`, `../`, `@`, `!`, `~`, vault, recursive, POSIX absolute, Windows drive/UNC, drive-relative, URL, and malformed rooted magic payloads | `reference_grammar`, especially `explicit_vs_implicit_relative_are_distinguished_without_fs`, `magic_payload_must_remain_relative_after_the_documented_sigil`, and the Windows classification cases |
| Bare paths resolve repository-first; explicit relative paths never fall back | Both candidates present, repository-only, source-only fallback, no repository, deduplication, missing candidates, permission/I/O probe failures | `precedence_flip`, `detailed_resolution`, and `resolution_context` |
| The original router input resolves unchanged | Exact input `prompts/_implement/implement-suggestions.md`; negative pair `./prompts/_implement/implement-suggestions.md`; ordered missing-candidate output | `level2_implicit_reference_resolves_repository_first_in_tmux`, `level2_explicit_reference_stays_source_relative_and_fails_in_tmux`, and `level2_implicit_no_match_lists_two_ordered_candidates_in_tmux` |
| Request context is captured once and re-anchored for child documents | Present/missing repository and home values; ambient mutation; in-repository and explicitly trusted external sources; completion/execution equivalence | `resolution_context`, `completion_round_trip`, and `completion_resolution_round_trip` |
| Rooted payloads following magic `@` are rejected on every host and cannot reach candidate probing | POSIX roots, Windows drive roots, and UNC roots through parse, planning, resolution, completion, sequence, and transclusion | `magic_payload_must_remain_relative_after_the_documented_sigil`, `rooted_magic_payloads_never_reach_planning_or_resolution`, `completion_rejects_rooted_magic_payloads_before_enumerating_roots`, and `rooted_magic_payload_is_rejected_across_file_sequence_and_transclusion_surfaces` |
| Darkmatter composition, schema, reference, expression, and transclusion use the shared request context | Repository/source collisions, invalid and permission failures, nested source origins, schema `file(...)`, reference graph enumeration/validation, frontmatter native versus quoted values | `reference_integration`, `compose_transclusion_resolves_repository_first_on_collision`, `compose_transclusion_uses_source_doc_repository_not_launch_cwd`, and focused compose/schema/transclusion module tests |
| Sequence Plus preserves JIT state, source/list grammar, tasks/groups, deterministic merges, exact shell bytes, ordering, and process ownership | YAML/JSON/JSON5/JSONL/NDJSON; CSV/TSV/lines/Markdown lists; quoted, CRLF, Unicode, numeric/boolean, null, and empty values; serial/parallel success and failure | `sequence_sources_cli`, `sequence_jit`, `sequence_groups`, `sequence_errors_cli`, composition sequence/task tests, and `level2_sequence_task_stream_capture` |
| Shipped prompt/config artifacts remain parseable after parser/schema changes | Every tracked Markdown artifact under `prompts/` is inspected without executing side effects | Integration addition `shipped_prompt_corpus_parses_frontmatter` |
| A real shipped artifact follows the normal CLI resolution path | `prompts/implement.md` executes its shipped initialize proxy through `claudine compose` and delivers the target rather than the router body | Integration addition `shipped_implement_prompt_runs_real_router_target` |
| Root workspace test selection works without per-package feature arguments | Exact failing input `just test biscuit-test-harness` invokes the selected package under macOS Bash 3.2 with `set -u`; the existing Messenger feature-argument path remains unchanged | The exact root command is the regression test; `just test biscuit-test-harness` must reach nextest and pass |

No production behavior will be considered verified by a broad package gate
alone. Focused suites run first; package-area `just test` and `just lint` are
the broader Phase 2 gates. Real-terminal assertions run only through
`just test-l2` if required by a plan row; the Phase 2 completion request names
L1 and lint as the mandatory Claudine gates.
