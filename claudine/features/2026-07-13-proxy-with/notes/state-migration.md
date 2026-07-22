# State migration map — seven legacy carriers → four ownership layers

Written during Phase 2. **This table is the checklist Phases 5–9 execute
against.** Every field of all seven legacy carriers appears exactly once
below, mapped to a layer or marked deleted with a reason.

Layer names are the Phase 2 types in `lib/src/composition/coordinator/`:

| Layer | Type | Survives a proxy? |
|---|---|---|
| Invocation inputs | `InvocationInputs` (immutable; `InvocationInputsDraft::freeze`) | yes |
| Run ledger | `RunLedger` (+ `LedgerMut` capability) | yes — extended, never reset |
| Handoff | `EvaluatedProxyRequest` → `ProxyHandoff` | it *is* the crossing |
| Prepared document | `PreparedDocument` | no — target prepared afresh |
| Active-document execution | `ActiveDocumentState` | no — discarded |

---

## 1. `CompositionPrepContext` (`cli/src/commands/wrap/composition/prep_context.rs:38`)

CLI-private prep-time discovery cache. Phase 5 dissolves it into invocation
inputs; the discovery scan itself stays in the CLI.

| Field | Destination | Notes |
|---|---|---|
| `original_ref` | Invocation inputs → `file_ref` | |
| `resolved_path` | Prepared document → `composition().resolved_path` | Already `#[allow(dead_code)]` on the carrier. |
| `source_parent` | **Deleted** | Derivable from `resolved_path.parent()`; already `#[allow(dead_code)]`. |
| `source_repo_root` | Prepared document → `composition().source_repo_root` | Source-specific, so it is *target*-specific after a proxy — Phase 9 recalculates it. |
| `cwd` | Invocation inputs → `launch_cwd` | |
| `selection_config` | **Deferred to Phase 5** | `SelectionConfig` is CLI-private today. Either lift it into the library or keep it a CLI-side input assembled into the canonical service. Not modeled in Phase 2. |
| `installed_snapshot` | Invocation inputs → `installed_snapshot` | |
| `launch_context` | Invocation inputs → `launch_discovery.launch_context` | |
| `env_context` | Invocation inputs → `launch_discovery.env_context` | |
| `launch_workspace` | Invocation inputs → `launch_discovery.launch_workspace` | |
| `launch_detection_error` | Invocation inputs → `launch_discovery.detection_error` | Preserves the `--repo`-set + scan-failed = abort contract. |

## 2. `PreparedComposition` (`lib/src/composition/types.rs:521`)

Survives as the body of `PreparedDocument`. Phase 5 gives it the stored
`ComposeContext` (R5); Phase 9 gives it the rebuilt launch plan.

| Field | Destination | Notes |
|---|---|---|
| `mode` | Prepared document (via `composition()`) | Duplicated from invocation inputs today; Phase 5 should read the invocation layer and drop the copy. |
| `resolved_path` | Prepared document | |
| `source_repo_root` | Prepared document | |
| `prompt` | Prepared document | |
| `effective_frontmatter` | Prepared document | Post-schema. Distinct from `PreparedDocument::overlay()`, which is pre-schema and immutable. |
| `selection_hints` | Prepared document | |
| `closure` | Prepared document | Target-owned after a proxy (R7); Phase 6 locks closure ownership. |
| `lifecycle` | Prepared document | |
| `compose_perf` | Prepared document | Per-document. Command-wide accumulation is ledger work — see carrier 6. |
| `dropped_optionals` | Prepared document | |
| `warnings` | Prepared document | |
| `deferred_lifecycle_keys` | Prepared document | |
| `rematerialize` | **Deleted or renamed in Phase 5** | Phase 5 task: demote to an internal input of the canonical service with an honest limited name, or delete in favor of the invocation/document layers. Its four fields are already covered: `set_overrides` → invocation inputs; `file_ref_fallback_dir` → invocation inputs; `pre_approved_commands` → run ledger `approval_cache`; `env_overrides` → invocation inputs. **Adding a field to it is explicitly not the answer.** Preserve the in-flight mutation at `prompt.rs:180-186` (newly approved commands folded back in) — that becomes a `LedgerMut` write. |

## 3. `CompositionExecutionRequest` (`lib/src/composition/types.rs:626`)

37 fields. Everything except `prepared`, `resolved_target`, and
`header_emitted` is invocation input.

| Field | Destination | Notes |
|---|---|---|
| `mode` | Invocation inputs → `mode` | |
| `file_ref` | Invocation inputs → `file_ref` | |
| `prepared` | Prepared document | The layer split: this request currently conflates command-lifetime and document-lifetime state in one struct. |
| `resolved_target` | Prepared document | Provider/model selection is target-specific and rebuilt on every proxy (R6, Phase 9). |
| `explicit_provider` | Invocation inputs | Stays authoritative over target frontmatter (R6). |
| `excluded` | Invocation inputs | |
| `yolo` | Invocation inputs | |
| `include` | Invocation inputs | |
| `model` | Invocation inputs | |
| `output` | Invocation inputs → `output_policy.format` | |
| `system_prompt_args` | Invocation inputs | Delivery *and content* are resume-compatibility-key facets (Phase 11). |
| `timeout` | Invocation inputs | |
| `step_timeout` | Invocation inputs | |
| `stall_timeout` | Invocation inputs | |
| `operation` | Invocation inputs | |
| `sandbox` | Invocation inputs | |
| `repo` | Invocation inputs | |
| `dry_run` | Invocation inputs | Dry-run fires no lifecycle events, so it never reaches a dynamic proxy (Phase 6). |
| `mcp` | Invocation inputs | |
| `mcp_use` | Invocation inputs | |
| `strict` | Invocation inputs | |
| `session_interactive` | Invocation inputs | |
| `session_interactive_source` | Invocation inputs | |
| `quiet` | Invocation inputs → `output_policy.quiet` | |
| `silent` | Invocation inputs → `output_policy.silent` | |
| `env_overrides` | Invocation inputs | |
| `shared_approval_cache` | Run ledger → `approval_cache` | Becomes non-optional: the ledger always has one. Phase 9 must confirm no freeze blocks a *target* from requesting approval for newly discovered commands. |
| `sequence` | Invocation inputs → `output_policy.sequence` | |
| `installed_snapshot` | Invocation inputs | |
| `prep_launch_workspace` | Invocation inputs → `launch_discovery.launch_workspace` | |
| `prep_launch_context` | Invocation inputs → `launch_discovery.launch_context` | |
| `prep_env_context` | Invocation inputs → `launch_discovery.env_context` | |
| `prep_launch_detection_error` | Invocation inputs → `launch_discovery.detection_error` | |
| `header_emitted` | **Deleted** | A render-progress flag, not state any layer owns. Phase 12 routes status through `TerminalRenderable` components; the renderer tracks its own emission. |
| `provider_args` | Invocation inputs → `provider_args` | |
| `provider_args_explicit` | Invocation inputs → `provider_args_explicit` | |
| *(sequence step scope)* | Invocation inputs → `step_overrides` | `SequenceStepOverlay` is supplied per step; a sequence proxy retains the step's scoped inputs and timing identity (Phase 6). |

## 4. `HarnessPromptState` (`cli/src/commands/wrap/harness_orch/types.rs:21`)

The carrier the motivating bug lives in: `control_dispatch.rs:209-213` mutates
`source_path`/`original_ref` in place to effect a proxy. Phase 6 deletes that
mutation and this carrier's identity fields with it.

| Field | Destination | Notes |
|---|---|---|
| `mode` | Invocation inputs → `mode` | |
| `source_path` | Prepared document → `composition().resolved_path` | **Was the mutation site.** Only the coordinator may commit active-document identity, and only by preparing a new `PreparedDocument` from a `ProxyHandoff`. |
| `original_ref` | Handoff → `ProxyHandoff::authored_target` / invocation inputs → `file_ref` | Direct runs take the caller's `file_ref`; proxied runs take the handoff's authored target. |
| `base_prompt` | Prepared document → `composition().prompt` | |
| `overlay` | Prepared document → `overlay()` (`DocumentOverlay`) | Today an untyped `IndexMap` mutated in place. Phase 8 makes it the immutable, evaluated, pre-schema input. |
| `prompt_tail` | Active-document execution | Cleared on proxy today (`control_dispatch.rs:213`); discarding the whole execution-state layer subsumes that. |
| `next_prompt_override` | Active-document execution → `ProviderAttempt::resume_followup` | |
| `next_resume_session_id` | Active-document execution → `ProviderAttempt::session_id` | Lives in the attempt slice so retry drops it and resume retains it. |
| `rematerialize` | See carrier 2 | |

## 5. `MaterializedHarnessPrompt` (`cli/src/commands/wrap/harness_orch/types.rs:41`)

Produced by the second composer that Phase 5 retires.

| Field | Destination | Notes |
|---|---|---|
| `frontmatter` | Prepared document → `composition().effective_frontmatter` | |
| `prompt` | Prepared document → `composition().prompt` | |
| `env_overrides` | Prepared document | Target-specific (`AGENT`/`MODEL` resolve per document); Phase 9 rebuilds them on every proxy. Distinct from the invocation-input `env_overrides`, which are caller intent. |
| `inline_closure_plan` | Prepared document → `composition().closure` | |
| `live_frontmatter` | Active-document execution | Per-attempt `RefCell` lifetime is correct and must be preserved: Phase 4 resolves `with:` against this so a preceding `set_frontmatter` in the same stack is visible. |

## 6. `HarnessLoopState` (`cli/src/commands/wrap/harness_orch/loop_control.rs:177`)

| Field | Destination | Notes |
|---|---|---|
| `run` (`HarnessLoopCtx`) | Split across all four layers | The aggregate the coordinator replaces. Includes `initial_proxy_target`, one of the two banned optional proxy channels — Phase 6 deletes it (`loop_control.rs:79`, `:130`, `:167`). |
| `effect_engine` | Prepared document | Rooted at repo-root-or-child-CWD, both target-specific; Phase 9 rebuilds. |
| `harness_context` | Prepared document | `CachedHarnessLoopContext` is keyed on `source_path` + `repo_root`, so it is per-document by construction. |
| `attempt` | Active-document execution → `ProviderAttempt::number` | |
| `initial_materialized` | Prepared document | A hand-off of the first prepared document; the canonical service makes it unnecessary as a separate slot. |
| `harness_perf` | **Run ledger — deferred to Phase 9** | Spec R2 puts command-wide performance accumulation in the ledger, but `crate::perf::AgentExecutionPerf` is a CLI type and Phase 2 is library-only. Phase 9 either lifts the type into the library or gives the ledger a perf slot. Not modeled in Phase 2 rather than modeled badly. |
| `loop_start` | Run ledger → `command_started` | Command/sequence timing anchors are not reset by a handoff. `LedgerMut::begin_step` anchors a sequence step. |
| `control_budgets` | Active-document execution → `DocumentIteration::{retry,resume}_budget` | `ControlBudgets { retry, resume }` becomes two labeled `ControlBudget`s (R8 forbids an unlabeled shared counter). `control_budget_for` semantics preserved exactly by `ControlBudget::ceiling_for`. |
| `proxy_tracking.chain` | Run ledger → `chain` | |
| `proxy_tracking.pending` | **Deleted** | An out-of-band "the loop must re-parse before the next event" flag. `DocumentTransition::Proxy` makes the handoff explicit and its consumption non-optional. |

### Behavior correction to land in Phase 6

`RunLedger::new(origin, …)` seeds the chain with the originating document, so
the first hop is already checked against it. Today the four call sites lazily
push the *current* `source_path` immediately before checking
(`loop_control.rs:203-208`, `control_dispatch.rs:89`, `:189`, `:246`), each
with its own idea of whether the current document is in the chain yet. The
seeded ledger is equivalent for every existing case and removes the
open-coding. `LedgerMut::approve_hop` still delegates the decision to
`proxy_handoff_allowed`, so the coordinator and any surviving legacy caller
cannot disagree about what a cycle is.

Separately, `looping/engine.rs:439` passes a **single-element chain slice** to
`proxy_handoff_allowed`, so the loop route cannot see multi-hop history. The
invocation-wide ledger chain subsumes this; Phase 6 fixes the call site.

## 7. `LifecycleRunGuard` (`lib/src/composition/lifecycle/mod.rs:475`)

Emission bookkeeping for one document. Phase 6 audits its Drop net
(`mod.rs:846`) and `reset_for_proxy` (`:704`) against clean-handoff semantics —
the Drop net is a likely source of a synthetic source `finalize` after a proxy
has already handed ownership to the target.

| Field | Destination | Notes |
|---|---|---|
| `config` | Prepared document → `composition().lifecycle` | The `Cow` exists only so a proxy can repoint the guard at a re-parsed target config. Once the coordinator prepares the target as its own document, the `Cow` can become a borrow again. |
| `ctx` | Prepared document | `LifecycleRuntimeContext`; Phase 5 pins it to the stored `ComposeContext`. |
| `emitter` | Invocation inputs | Command-lifetime; unaffected by a handoff. |
| `initialize_emitted` | Active-document execution | Per-document: exactly one `initialize` per active document (Phase 7). |
| `start_emitted` | Active-document execution | |
| `provider_launched` | Active-document execution → `ProviderAttempt` | Phase 11 keeps deriving pre/post-stage re-entry from it (`control.rs:138` → `reenter_preflight: !provider_launched`). |
| `terminal_emitted` | Active-document execution | |
| `finalize_emitted` | Active-document execution | |
| `terminal_signal` | Active-document execution → `ProviderAttempt::last_outcome` | `AttemptOutcome` is the typed form. |
| `proxied` | **Deleted** | Exists because a proxy swaps the document *underneath* the guard, invalidating the `ctx.*` snapshot captured for the original. When the target is prepared as its own document with its own context, there is nothing to invalidate. `PreparedDocument::provenance()` answers "was this proxied to" for diagnostics. |

---

## Not carried by any legacy carrier

New in Phase 2, with no field to migrate:

- `ProxyProvenance` / `ActionLocation` — today a proxy's origin is not
  recorded at all, which is why `control_dispatch.rs` diagnostics cannot name
  the property that requested the hop. `ActionLocation` reuses the dotted
  `"{event}.stack[{i}].action[{j}]"` form from `preflight.rs:280-286`.
- `RunLedger::transitions` — R2's "transition provenance needed for final
  output and diagnostics".
- `ResolvedProxyTarget` / `HopApproval` — the construction boundary that makes
  a `ProxyHandoff` unbuildable without resolution plus a hop decision. When the
  file-resolution feature lands, `ResolvedProxyTarget::from_resolver` is the
  single seam it replaces.
