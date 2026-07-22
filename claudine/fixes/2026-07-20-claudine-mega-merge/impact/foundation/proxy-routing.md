# run_target_initialize — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `run_target_initialize`
- File: `claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:run_target_initialize",
    "name": "run_target_initialize",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs",
    "startLine": 13,
    "endLine": 131
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:adopt_proxy_lifecycle_phase",
        "name": "adopt_proxy_lifecycle_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs:target_initialize_proxy_failure_projects_event_and_property_separately",
        "name": "target_initialize_proxy_failure_projects_event_and_property_separately",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs:target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error",
        "name": "target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs:target_initialize_routes_to_failure_with_raise_surfaces_failure_evaluation_error",
        "name": "target_initialize_routes_to_failure_with_raise_surfaces_failure_evaluation_error",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:resolve_proxy_target",
        "name": "resolve_proxy_target",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/control.rs:resolve_proxy_target_in_context",
        "name": "resolve_proxy_target_in_context",
        "filePath": "claudine/lib/src/composition/lifecycle/control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_proxy#0",
        "name": "reset_for_proxy",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs:run_catch_protocol",
        "name": "run_catch_protocol",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs:surface_protocol_evaluation",
        "name": "surface_protocol_evaluation",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs:run_lifecycle_event",
        "name": "run_lifecycle_event",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/error/mod.rs:FileReferenceContext",
        "name": "FileReferenceContext",
        "filePath": "claudine/lib/src/composition/error/mod.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_proxy#0",
        "name": "reset_for_proxy",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "run_target_initialize", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:run_target_initialize",
    "name": "run_target_initialize",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
  },
  "direction": "upstream",
  "impactedCount": 6,
  "risk": "HIGH",
  "epistemic": "exact",
  "summary": {
    "direct": 4,
    "processes_affected": 0,
    "modules_affected": 3
  },
  "byDepthCounts": {
    "1": 4,
    "2": 1,
    "3": 1
  },
  "affected_processes": [],
  "affected_modules": [
    {
      "name": "Tests",
      "hits": 3,
      "impact": "direct"
    },
    {
      "name": "Harness_orch",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Lifecycle",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
