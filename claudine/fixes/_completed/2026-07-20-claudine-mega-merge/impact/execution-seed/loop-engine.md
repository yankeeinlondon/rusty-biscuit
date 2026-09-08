# execute_loop_with_lifecycle — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `execute_loop_with_lifecycle`
- File: `claudine/lib/src/composition/looping/engine.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **HIGH**

## Context

```json
{
  "status": "found",
  "symbol": {
    "uid": "Function:claudine/lib/src/composition/looping/engine.rs:execute_loop_with_lifecycle",
    "name": "execute_loop_with_lifecycle",
    "kind": "Function",
    "filePath": "claudine/lib/src/composition/looping/engine.rs",
    "startLine": 301,
    "endLine": 710
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs:run_loop_lifecycle",
        "name": "run_loop_lifecycle",
        "filePath": "claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs:run_loop_lifecycle_emitting_terminal",
        "name": "run_loop_lifecycle_emitting_terminal",
        "filePath": "claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/loop_run.rs:run_loop_with_overrides",
        "name": "run_loop_with_overrides",
        "filePath": "claudine/cli/src/commands/compose/loop_run.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/context.rs:LifecycleErrorInfo.from_harness_error#1",
        "name": "from_harness_error",
        "filePath": "claudine/lib/src/composition/lifecycle/context.rs"
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
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.execute_event#2",
        "name": "execute_event",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_next_iteration#0",
        "name": "reset_for_next_iteration",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_next_iteration#0",
        "name": "reset_for_next_iteration",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:route_init_failure_typed",
        "name": "route_init_failure_typed",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:execute_loop_catch_protocol",
        "name": "execute_loop_catch_protocol",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:run_loop_gate",
        "name": "run_loop_gate",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:build_loop_stack_context",
        "name": "build_loop_stack_context",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:capture_loop_lifecycle_globals",
        "name": "capture_loop_lifecycle_globals",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:decide_rate_limit_action",
        "name": "decide_rate_limit_action",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/engine.rs:should_continue_after_cap",
        "name": "should_continue_after_cap",
        "filePath": "claudine/lib/src/composition/looping/engine.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/expression.rs:evaluate_condition",
        "name": "evaluate_condition",
        "filePath": "claudine/lib/src/composition/looping/expression.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/types.rs:LoopIterationOutput.success#1",
        "name": "success",
        "filePath": "claudine/lib/src/composition/looping/types.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/looping/types.rs:LoopIterationOutput.failure#3",
        "name": "failure",
        "filePath": "claudine/lib/src/composition/looping/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/looping/types.rs:LoopIterationContext",
        "name": "LoopIterationContext",
        "filePath": "claudine/lib/src/composition/looping/types.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.execute_event#2",
        "name": "execute_event",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_next_iteration#0",
        "name": "reset_for_next_iteration",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRunGuard.reset_for_next_iteration#0",
        "name": "reset_for_next_iteration",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext.repo_root",
        "name": "repo_root",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext.launch_area",
        "name": "launch_area",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext.launch_area",
        "name": "launch_area",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext.launch_area",
        "name": "launch_area",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/runtime.rs:LifecycleCatchResult.evaluation_error_signal",
        "name": "evaluation_error_signal",
        "filePath": "claudine/lib/src/composition/lifecycle/runtime.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/runtime.rs:LifecycleCatchResult.evaluation_error",
        "name": "evaluation_error",
        "filePath": "claudine/lib/src/composition/lifecycle/runtime.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/runtime.rs:LifecycleCatchResult.setup_error",
        "name": "setup_error",
        "filePath": "claudine/lib/src/composition/lifecycle/runtime.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/lifecycle/runtime.rs:LifecycleCatchResult.control",
        "name": "control",
        "filePath": "claudine/lib/src/composition/lifecycle/runtime.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/looping/types.rs:LoopExecutionOptions.max_iterations",
        "name": "max_iterations",
        "filePath": "claudine/lib/src/composition/looping/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/looping/types.rs:LoopExecutionOptions.fail_fast",
        "name": "fail_fast",
        "filePath": "claudine/lib/src/composition/looping/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "execute_loop_with_lifecycle", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/lib/src/composition/looping/engine.rs:execute_loop_with_lifecycle",
    "name": "execute_loop_with_lifecycle",
    "type": "Function",
    "filePath": "claudine/lib/src/composition/looping/engine.rs"
  },
  "direction": "upstream",
  "impactedCount": 15,
  "risk": "HIGH",
  "epistemic": "exact",
  "summary": {
    "direct": 3,
    "processes_affected": 0,
    "modules_affected": 3
  },
  "byDepthCounts": {
    "1": 3,
    "2": 12
  },
  "affected_processes": [],
  "affected_modules": [
    {
      "name": "Tests",
      "hits": 12,
      "impact": "direct"
    },
    {
      "name": "Looping",
      "hits": 2,
      "impact": "direct"
    },
    {
      "name": "Compose",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
