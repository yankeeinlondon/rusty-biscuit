# Mega-Merge Acceptance Ledger

`open` is a Phase 1 planning state, not an acceptance classification. Every
row must be re-anchored to the immutable merged acceptance-candidate SHA and
changed to one of the specification's evidence statuses before closeout. Only
`passed` satisfies a required row.

## Owning sources

| Workstream | Frozen source |
|---|---|
| Error propagation | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97:claudine/features/2026-07-13-error-propogation/spec.md` |
| File resolution | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97:claudine/features/2026-07-13-file-resolution/spec.md` |
| Sequence Plus | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97:claudine/features/2026-07-11-sequence-plus/spec.md` |
| Sequence validation | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97:claudine/features/2026-07-11-sequence-plus/validation-matrix.md` |
| Proxy-with | `e348486c810969abe87a6b7209979034f5454b07:claudine/features/2026-07-13-proxy-with/spec.md` |
| Proxy acceptance map | `e348486c810969abe87a6b7209979034f5454b07:claudine/features/2026-07-13-proxy-with/notes/acceptance-map.md` |

## Error propagation

| ID | Contract | Named merged-tree test or audit | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| EP-01 | No typed in-process flattening; snapshots at erased boundaries | `error_guards::lossy_boundary_inventory_stays_closed` | L1 | all | Diagnostic registry | open | — | error spec AC1 |
| EP-02 | One complete Claudine diagnostic registry | `diagnostic_registry_is_source_complete` | L1 | all | Diagnostic registry | open | — | error spec AC2 |
| EP-03 | Renderer, `err.*`, and serialized output select one effective diagnostic/cause | `diagnostic_projection_parity_across_render_err_and_snapshot` | L1 | all | Lifecycle diagnostics | open | — | error spec AC3 |
| EP-04 | Motivating proxy-resolution failure is source-aware and actionable | `level2_mega_merge_proxy_resolution_source_aware` | L2 | macOS + Linux | CLI rendering | open | — | error spec AC4 |
| EP-05 | Proxy-resolution identity/detail parity across lifecycle routes | `level2_mega_merge_proxy_resolution_route_parity` | L2 | macOS + Linux | Coordinator + diagnostics | open | — | error spec AC5 |
| EP-06 | Excerpts, no-color, and exactly-once rendering remain stable | `level2_mega_merge_error_rendering_contract` | L2 | macOS + Linux | Terminal rendering | open | — | error spec AC6 |
| EP-07 | Structured detail conforms to locked catalog including present-null fields | `diagnostic_catalog_detail_shape_is_lossless` | L1 | all | Diagnostic registry | open | — | error spec AC7 |
| EP-08 | Exit, lifecycle, retry/resume/proxy decisions and `err.msg` remain behavior-neutral | `transport_migration_preserves_control_decisions` | L1 | all | Lifecycle protocol | open | — | error spec AC8 |
| EP-09 | Focused, L1, L2, and lint gates pass | `error_propagation_acceptance_gate_audit` | L1/L2 | required matrix | Phase owner | open | — | error spec AC9 |

## File resolution

| ID | Contract | Named merged-tree test or audit | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| FR-01 | `FileReference` is the sole syntax authority | `file_reference_authority_source_scan` | L1 audit | all | Biscuit File | open | — | file spec AC1 |
| FR-02 | Explicit relative is source-only with no fallback | `explicit_relative_proxy_target_never_falls_back` | L1 | Unix + Windows paths | Biscuit File | open | — | file spec AC2 |
| FR-03 | Bare references are repository-first then source | `implicit_reference_repository_first_then_source` | L1 | all | Biscuit File | open | — | file spec AC3 |
| FR-04 | Original router reference resolves without author rewrite | `motivating_router_reference_resolves_unchanged` | CLI E2E | macOS + Linux | Claudine CLI | open | — | file spec AC4 |
| FR-05 | Every proxy orchestration route resolves identically | `proxy_route_file_resolution_parity` | L2 | macOS + Linux | Coordinator | open | — | file spec AC5 |
| FR-06 | Composition, sequence, schema, expression, and transclusion share the contract | `claudine_darkmatter_reference_surface_parity` | L1 process integration | all | Claudine + Darkmatter | open | — | file spec AC6 |
| FR-07 | Docs, skill, implementation, completion, and tests agree | `file_reference_contract_documentation_audit` | audit | all | Biscuit File docs | open | — | file spec AC7 |
| FR-08 | Missing references retain typed ordered candidate detail | `missing_reference_preserves_candidates_and_probes` | L1 | all | Diagnostic registry | open | — | file spec AC8 |
| FR-09 | Native absolute/reference semantics are cross-platform | `file_reference_native_path_matrix` | L1/native | macOS + Linux + Windows | Biscuit File | open | — | file spec AC9 |
| FR-10 | Affected area L1/L2/lint gates pass | `file_resolution_acceptance_gate_audit` | L1/L2 | required matrix | Phase owner | open | — | file spec AC10 |
| FR-11 | Sequence `~` stays home-pinned on native Windows | `sequence_home_reference_is_home_pinned` | native L1 | Windows | Sequence | open | — | file spec AC11 |
| FR-12 | Document-backed resolution consumes one explicit request context | `document_resolution_never_recaptures_ambient_state` | L1 source/runtime guard | all | Request context | open | — | file spec AC12 |
| FR-13 | Probing distinguishes missing from permission/I/O failure | `candidate_probe_failure_classification_and_order` | L1 | all | Biscuit File | open | — | file spec AC13 |
| FR-14 | Child documents re-anchor source while retaining invocation context | `child_document_context_reanchors_without_moving_launch` | L1 | all | Request context | open | — | file spec AC14 |
| FR-15 | Every changed-default caller is audited/migrated explicitly | `file_reference_default_caller_audit` | audit | all | Workspace owners | open | — | file spec AC15 |

## Sequence Plus specification criteria

| ID | Contract | Named merged-tree test or audit | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| SP-01 | Retained scalar/object/inline/fail-fast/missing/dry-run/Ctrl+C behavior | `sequence_retained_behavior_matrix` | L1 + L3 | all/native | Sequence | open | — | sequence spec criterion 1 |
| SP-02 | Typed errors cover every rejected construct | `sequence_errors_cli` | CLI E2E | all | Sequence parser | open | — | sequence spec criterion 2 |
| SP-03 | Typed/string list forms cover quotes, CRLF, Unicode, foreign values, null, empty | `sequence_dynamic_sources_and_list_format_matrix` | L1 | all | Sequence + Biscuit File | open | — | sequence spec criterion 3 |
| SP-04 | YAML/JSON/JSON5/JSONL/NDJSON and reference variants share `FileReference` | `sequence_sources_cli` | CLI E2E | all | Sequence resolution | open | — | sequence spec criterion 4 |
| SP-05 | JIT visibility, overlay precedence, exact shell bytes, disk boundaries | `sequence_jit_and_exact_command_matrix` | L1 | all | Sequence runtime | open | — | sequence spec criterion 5 |
| SP-06 | Group ordering, errors, merge, outputs, caps, no prompts, Ctrl+C | `sequence_parallel_group_semantics_matrix` | L1 + L3 | all/native | Sequence tasks | open | — | sequence spec criterion 6 |
| SP-07 | Terminal rendering preserves width, channels, concurrency, ANSI integrity | `level2_sequence_task_stream_capture_matrix` | L2 | macOS + Linux | Task stream | open | — | sequence spec criterion 7 |
| SP-08 | Platform-sensitive spawn/path/env/interrupt/newline behavior is portable | `sequence_cross_platform_runtime_matrix` | native | macOS + Linux + Windows | Process ownership | open | — | sequence spec criterion 8 |
| SP-09 | Test placement follows repository tier rules | `sequence_test_placement_guard` | L1 audit | all | Test architecture | open | — | sequence spec criterion 9 |
| SP-10 | Sequence docs/CLI/skill describe final clean-break grammar | `sequence_documentation_link_audit` | audit | all | Docs owner | open | — | sequence spec criterion 10 |
| SP-11 | Claudine L1/L2/lint gates pass | `sequence_acceptance_gate_audit` | L1/L2 | required matrix | Phase owner | open | — | sequence spec criterion 11 |

## Sequence Plus validation-matrix criteria

| ID | Contract | Named merged-tree test or audit | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| VM-AC01 | Retained behavior including interruption exit derivation | validation matrix AC1 named suite | L1 + L3 | all/native | Sequence | open | — | validation matrix AC1 |
| VM-AC02 | Typed rejected-construct coverage | validation matrix AC2 named suite | CLI E2E | all | Sequence parser | open | — | validation matrix AC2 |
| VM-AC03 | Source/list-form coverage | validation matrix AC3 named suite | L1 | all | Sequence sources | open | — | validation matrix AC3 |
| VM-AC04 | File sources and `FileReference` coverage | validation matrix AC4 named suite | CLI E2E | all | Sequence resolution | open | — | validation matrix AC4 |
| VM-AC05 | JIT semantics and exact-command parity | validation matrix AC5 named suite | L1 | all | Sequence runtime | open | — | validation matrix AC5 |
| VM-AC06 | Group semantics and deterministic merge | validation matrix AC6 named suite | L1 | all | Sequence tasks | open | — | validation matrix AC6 |
| VM-AC07 | Task-stream rendering | validation matrix AC7 named suite | L1 + L2 | macOS + Linux | Task stream | open | — | validation matrix AC7 |
| VM-AC08 | Cross-platform process and interruption evidence | validation matrix AC8 native matrix | L1/L3/native | macOS + Linux + Windows | Process ownership | open | — | validation matrix AC8 |
| VM-AC09 | Test-placement guard | validation matrix AC9 audit | L1 audit | all | Test architecture | open | — | validation matrix AC9 |
| VM-AC10 | Documentation | validation matrix AC10 audit | audit | all | Docs owner | open | — | validation matrix AC10 |
| VM-AC11 | Verification commands | validation matrix AC11 gate audit | L1/L2/L3 | required matrix | Phase owner | open | — | validation matrix AC11 |

## Proxy-with acceptance map

The named evidence for each source row remains the exact test list in the
frozen acceptance-map row; Phase 4 must re-run and re-anchor it rather than
copying its historical checkmark.

| ID | Contract | Named merged-tree test or audit | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| PW-01 | Proxied target uses the canonical preparation service | proxy acceptance-map AC1 named suite | L1 | all | Preparation service | open | — | proxy map row 1 |
| PW-02 | Only coordinator changes identity; harness returns typed `Proxy` | proxy acceptance-map AC2 named suite | L1 | all | Coordinator | open | — | proxy map row 2 |
| PW-03 | All four routes return one consumed/rejected handoff | proxy acceptance-map AC3 named suite | L2 | macOS + Linux | Coordinator | open | — | proxy map row 3 |
| PW-04 | Evaluation creates request; coordinator resolves/commits | proxy acceptance-map AC4 named suite | L1 | all | Coordinator | open | — | proxy map row 4 |
| PW-05 | Clean proxy gives closure to target without synthetic source closure | proxy acceptance-map AC5 named suite | L2 | macOS + Linux | Lifecycle coordinator | open | — | proxy map row 5 |
| PW-06 | Compose/inline/sequence-step retain command state | proxy acceptance-map AC6 named suite | L1 | all | Command owners | open | — | proxy map row 6 |
| PW-07 | Proxied target owns loop with direct-equivalent count | proxy acceptance-map AC7 named suite | L2 | macOS + Linux | Active document | open | — | proxy map row 7 |
| PW-08 | Body/frontmatter/lifecycle/schema/shell share one stored context | proxy acceptance-map AC8 named suite | L1 | all | Preparation service | open | — | proxy map row 8 |
| PW-09 | Context/env agent/model agree with direct execution | proxy acceptance-map AC9 named suite | L2 | macOS + Linux | Launch plan | open | — | proxy map row 9 |
| PW-10 | Mutable launch facets rebuild per target | proxy acceptance-map AC10 named suite | L2/native | all | Launch plan | open | — | proxy map row 10 |
| PW-11 | Initialize follows narrow gate and precedes full preflight | proxy acceptance-map AC11 named suite | L2 | macOS + Linux | Preparation stages | open | — | proxy map row 11 |
| PW-12 | Stabilized reread occurs without double initialize | proxy acceptance-map AC12 named suite | L2 | macOS + Linux | Preparation stages | open | — | proxy map row 12 |
| PW-13 | Entry-reason stage matrix is exact | proxy acceptance-map AC13 named suite | L1 | all | Preparation stages | open | — | proxy map row 13 |
| PW-14 | Retry refreshes canonically and retains overlay/provenance | proxy acceptance-map AC14 named suite | L1/L2 | macOS + Linux | Retry coordinator | open | — | proxy map row 14 |
| PW-15 | Resume key matches exact launch bundle and names changes | proxy acceptance-map AC15 named suite | L2/native | all | Session compatibility | open | — | proxy map row 15 |
| PW-16 | Budgets persist/reset at documented boundaries | proxy acceptance-map AC16 named suite | L1 | all | Loop control | open | — | proxy map row 16 |
| PW-17 | Every fresh target audits exact shell bytes | proxy acceptance-map AC17 named suite | L2 | macOS + Linux | Preflight | open | — | proxy map row 17 |
| PW-18 | Key/value proxy accepts optional static-key mapping | proxy acceptance-map AC18 named suite | L1 | all | Lifecycle grammar | open | — | proxy map row 18 |
| PW-19 | Positional proxy remains valid; sibling `with` remains ambiguous | proxy acceptance-map AC19 named suite | L1 | all | Lifecycle grammar | open | — | proxy map row 19 |
| PW-20 | `with` resolves once, preserving typed whole values | proxy acceptance-map AC20 named suite | L1 | all | Overlay evaluation | open | — | proxy map row 20 |
| PW-21 | Invalid interpolation aborts atomically | proxy acceptance-map AC21 named suite | L1 | all | Overlay evaluation | open | — | proxy map row 21 |
| PW-22 | Precedence, shallow replace, and null removal are exact | proxy acceptance-map AC22 named suite | L1/L2 | macOS + Linux | Overlay layering | open | — | proxy map row 22 |
| PW-23 | Stored overlay remains immutable pre-schema input | proxy acceptance-map AC23 named suite | L1 | all | Overlay layering | open | — | proxy map row 23 |
| PW-24 | Overlay can satisfy schema; invalid overlay fails pre-launch | proxy acceptance-map AC24 named suite | L1 | all | Schema stage | open | — | proxy map row 24 |
| PW-25 | Overlay control-plane values cannot bypass target policy | proxy acceptance-map AC25 named suite | L2 | macOS + Linux | Policy/preflight | open | — | proxy map row 25 |
| PW-26 | Overlay survives immediate refresh but not downstream hop | proxy acceptance-map AC26 named suite | L2 | macOS + Linux | Overlay lifetime | open | — | proxy map row 26 |
| PW-27 | `with` never changes source/target bytes or hashes | proxy acceptance-map AC27 named suite | L2 | macOS + Linux | Persistence boundary | open | — | proxy map row 27 |
| PW-28 | Diagnostic identity matches direct/initialize/recovery routes | proxy acceptance-map AC28 named suite | L2 | macOS + Linux | Diagnostics | open | — | proxy map row 28 |
| PW-29 | Failed handoff is event-aware and emitted once | proxy acceptance-map AC29 named suite | L2 | macOS + Linux | Lifecycle coordinator | open | — | proxy map row 29 |
| PW-30 | Overlay is redacted; output uses `TerminalRenderable` | proxy acceptance-map AC30 named suite | L2 | macOS + Linux | Terminal rendering | open | — | proxy map row 30 |

## Mandatory combined seams

| ID | Contract | Concrete planned test | Tier | Platform | Owner | Status | Candidate | Evidence |
|---|---|---|---|---|---|---|---|---|
| MM-S01 | Repository-first proxy target; missing detail parity across terminal/`err.*`/snapshot | `level2_mega_merge_s01_bare_proxy_resolution_diagnostic_parity` | L2 | macOS + Linux | Diagnostics + coordinator | open | — | spec seam 1 |
| MM-S02 | Explicit relative stays source-local; child context re-anchors | `mega_merge_s02_explicit_proxy_child_context_no_fallback` | L1 | Unix + Windows paths | File resolution | open | — | spec seam 2 |
| MM-S03 | Overlay into sequence step with target loop runs step once and preserves JIT/output | `level2_mega_merge_s03_proxy_with_sequence_loop_containment` | L2 | macOS + Linux | Sequence + coordinator | open | — | spec seam 3 |
| MM-S04 | Proxied schema failure matches direct with exact owed events | `level2_mega_merge_s04_schema_failure_route_and_event_parity` | L2 | macOS + Linux | Schema + lifecycle | open | — | spec seam 4 |
| MM-S05 | Initialize/terminal handoff failures retain event, closure, diagnostic identity | `level2_mega_merge_s05_handoff_failure_phase_identity` | L2 | macOS + Linux | Lifecycle coordinator | open | — | spec seam 5 |
| MM-S06 | Overlay survives retry/resume/immediate loop but not downstream proxy | `level2_mega_merge_s06_overlay_refresh_and_hop_lifetime` | L2 | macOS + Linux | Overlay lifetime | open | — | spec seam 6 |
| MM-S07 | Launch changes rebuild and incompatible resume is typed | `level2_mega_merge_s07_launch_rebuild_and_resume_incompatibility` | L2/native | all | Launch/session plan | open | — | spec seam 7 |
| MM-S08 | Compatibility key derives from exact spawned launch bundle | `mega_merge_s08_session_key_equals_spawned_bundle` | L1 | all | Launch/session plan | open | — | spec seam 8 |
| MM-S09 | Sequence preflight approves exact post-handoff shell bytes | `mega_merge_s09_sequence_handoff_approved_bytes_equal_executed` | L1 process integration | all | Sequence + preflight | open | — | spec seam 9 |
| MM-S10 | Parallel proxied failure keeps task attribution/order/settlement/merge/teardown | `level2_mega_merge_s10_parallel_proxy_failure_task_integrity` | L2/native | all | Sequence tasks | open | — | spec seam 10 |
| MM-S11 | Cross-repo authoring context changes while child CWD stays fixed | `level2_mega_merge_s11_cross_repo_context_vs_child_cwd` | L2/native | all | Context + launch plan | open | — | spec seam 11 |
| MM-S12 | Dry-run reports intent without traversal, lifecycle, side effects, mutation, or disclosure | `level2_mega_merge_s12_dry_run_is_static_and_redacted` | L2 | macOS + Linux | Coordinator + CLI | open | — | spec seam 12 |

## Phase 1 test-design mapping

Phase 1 changes no runtime behavior, parser, schema, template, prompt, or
configuration artifact. Therefore no regression test is added in this phase.
The rows above are the required behavior-to-test map before Phase 2/3
implementation changes. Parser/schema/template/configuration changes in those
phases must additionally add the passive shipped-artifact corpus test, a real
shipped-artifact end-to-end test, and persisted read/write/read coverage where
applicable before their owning row can pass.
