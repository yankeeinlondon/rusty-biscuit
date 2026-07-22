# Hotspot Symbol Resolution

GitNexus `query`, `context`, and graph-schema-backed `cypher` inventory resolved
the plan's ambiguous labels. Each upstream file report uses the exact graph UID
`File:<full-repository-path>`; supplementary function/type reports record the
primary execution symbol where useful.

| Ambiguous plan label | Concrete file identity | Representative concrete symbol |
|---|---|---|
| `prep` | `File:claudine/cli/src/commands/compose/prep.rs` | `execute_loop_or_single` |
| wrap composition `mod` | `File:claudine/cli/src/commands/wrap/composition/mod.rs` | `execute_composition_request` |
| wrap composition `pipeline` | `File:claudine/cli/src/commands/wrap/composition/pipeline.rs` | `provider_run_handoff` |
| wrap composition `runner` | `File:claudine/cli/src/commands/wrap/composition/runner.rs` | `run_composition_body` |
| `wrapper_stages` | `File:claudine/cli/src/commands/wrap/wrapper_stages.rs` | `run_execution_stage` |
| `loop_control` | `File:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` | `run_harness_loop_inner` |
| `control_dispatch` | `File:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs` | `dispatch_terminal_control` |
| harness `proxy` | `File:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs` | `run_target_initialize` |
| harness `prompt` | `File:claudine/cli/src/commands/wrap/harness_orch/prompt.rs` | `materialize_harness_prompt` |
| harness `types` | `File:claudine/cli/src/commands/wrap/harness_orch/types.rs` | `HarnessPromptState` |
| `overlay` | `File:claudine/cli/src/commands/wrap/overlay.rs` | `merge_frontmatter_overlay` |
| sequence `iterate` | `File:claudine/cli/src/commands/wrap/sequence/iterate.rs` | `run_sequence_steps` / proxy-tip `run_step_proxy_loop` |
| sequence `mod` | `File:claudine/cli/src/commands/wrap/sequence/mod.rs` | `execute_sequence` |
| sequence `phase1c` | `File:claudine/cli/src/commands/wrap/sequence/phase1c.rs` | `run_phase_1c_with_schema` |
| composition error `mod` | `File:claudine/lib/src/composition/error/mod.rs` | `Enum:...:CompositionError` |
| error render `mod` | `File:claudine/lib/src/composition/error/render/mod.rs` | `file_reference_detail` |
| lifecycle `context` | `File:claudine/lib/src/composition/lifecycle/context.rs` | `LifecycleErrorInfo` |
| lifecycle `executor` | `File:claudine/lib/src/composition/lifecycle/executor.rs` | `StackExecutionContext` |
| looping `engine` | `File:claudine/lib/src/composition/looping/engine.rs` | `execute_loop_with_lifecycle` |
| composition `mod` | `File:claudine/lib/src/composition/mod.rs` | module owner for canonical composition exports |
| composition `preflight` | `File:claudine/lib/src/composition/preflight.rs` | `resolve_shell_approvals` |
| composition `prepare` | `File:claudine/lib/src/composition/prepare.rs` | `prepare_direct` |
| composition `types` | `File:claudine/lib/src/composition/types.rs` | `PreparedComposition`, `CompositionExecutionRequest`, `CallerInputLayers` |
| Darkmatter `options` | `File:darkmatter/lib/src/markdown/compose/context/options.rs` | `ComposeOptions` |

Proxy-tip query additionally resolved the new coordinator owner as
`Impl:claudine/lib/src/composition/coordinator/document.rs:PreparedDocument`.
