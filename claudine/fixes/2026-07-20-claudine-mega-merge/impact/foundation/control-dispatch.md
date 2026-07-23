# dispatch_terminal_control — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `dispatch_terminal_control`
- File: `claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **UNKNOWN**

## Context

```json
{
  "status": "found",
  "symbol": {
    "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:dispatch_terminal_control",
    "name": "dispatch_terminal_control",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs",
    "startLine": 54,
    "endLine": 314
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:start_lifecycle_phase",
        "name": "start_lifecycle_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:run_finalize_with_recovery",
        "name": "run_finalize_with_recovery",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:drive_terminal_recovery",
        "name": "drive_terminal_recovery",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs:dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run",
        "name": "dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs:terminal_proxy_failure_projects_event_and_property_separately",
        "name": "terminal_proxy_failure_projects_event_and_property_separately",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_retry_from_failure_continues_and_resets_guard",
        "name": "dispatch_retry_from_failure_continues_and_resets_guard",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_retry_from_finalize_continues_and_resets_guard",
        "name": "dispatch_retry_from_finalize_continues_and_resets_guard",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_resume_from_finalize_seeds_prompt_state",
        "name": "dispatch_resume_from_finalize_seeds_prompt_state",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_retry_exhausts_after_budget",
        "name": "dispatch_retry_exhausts_after_budget",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_resume_with_session_seeds_prompt_state",
        "name": "dispatch_resume_with_session_seeds_prompt_state",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs:dispatch_resume_without_session_aborts_typed",
        "name": "dispatch_resume_without_session_aborts_typed",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs:dispatch_defer_aborts_not_implemented",
        "name": "dispatch_defer_aborts_not_implemented",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs:dispatch_stop_falls_through",
        "name": "dispatch_stop_falls_through",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs:dispatch_error_aborts_without_changing_stop_semantics",
        "name": "dispatch_error_aborts_without_changing_stop_semantics",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs:dispatch_no_control_falls_through",
        "name": "dispatch_no_control_falls_through",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Struct:claudine/lib/src/composition/error/mod.rs:FileReferenceContext",
        "name": "FileReferenceContext",
        "filePath": "claudine/lib/src/composition/error/mod.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/lifecycle/runtime.rs:LifecycleTransitionInput",
        "name": "LifecycleTransitionInput",
        "filePath": "claudine/lib/src/composition/lifecycle/runtime.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:proxy_path_identity",
        "name": "proxy_path_identity",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:proxy_handoff_allowed",
        "name": "proxy_handoff_allowed",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:resolve_proxy_target",
        "name": "resolve_proxy_target",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:resolve_proxy_target_in_context",
        "name": "resolve_proxy_target_in_context",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:ControlBudgets.retry",
        "name": "retry",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:ControlBudgets.resume",
        "name": "resume",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.chain",
        "name": "chain",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:ProxyTracking.pending",
        "name": "pending",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.original_ref",
        "name": "original_ref",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.prompt_tail",
        "name": "prompt_tail",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "dispatch_terminal_control", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:dispatch_terminal_control",
    "name": "dispatch_terminal_control",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs"
  },
  "direction": "upstream",
  "impactedCount": 19,
  "risk": "HIGH",
  "epistemic": "exact",
  "summary": {
    "direct": 15,
    "processes_affected": 0,
    "modules_affected": 4
  },
  "byDepthCounts": {
    "1": 15,
    "2": 3,
    "3": 1
  },
  "affected_processes": [],
  "affected_modules": [
    {
      "name": "Tests",
      "hits": 15,
      "impact": "direct"
    },
    {
      "name": "Harness_orch",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Harness",
      "hits": 1,
      "impact": "indirect"
    },
    {
      "name": "Loop_control",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
