# run_execution_stage — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `run_execution_stage`
- File: `claudine/cli/src/commands/wrap/wrapper_stages.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **LOW**

## Context

```json
{
  "status": "found",
  "symbol": {
    "uid": "Function:claudine/cli/src/commands/wrap/wrapper_stages.rs:run_execution_stage",
    "name": "run_execution_stage",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/wrapper_stages.rs",
    "startLine": 406,
    "endLine": 580
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/mod.rs:run_provider_wrapper_inner",
        "name": "run_provider_wrapper_inner",
        "filePath": "claudine/cli/src/commands/wrap/mod.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/exec/spawn/inherited.rs:run_child",
        "name": "run_child",
        "filePath": "claudine/cli/src/commands/wrap/exec/spawn/inherited.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:run_harness_loop",
        "name": "run_harness_loop",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/session_report.rs:SessionPresence.started#5",
        "name": "started",
        "filePath": "claudine/cli/src/commands/wrap/session_report.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/wrapper_exec.rs:run_structured_stream_session",
        "name": "run_structured_stream_session",
        "filePath": "claudine/cli/src/commands/wrap/wrapper_exec.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/exec/mod.rs:ChildIoOptions",
        "name": "ChildIoOptions",
        "filePath": "claudine/cli/src/commands/wrap/exec/mod.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
        "name": "HarnessPromptState",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext",
        "name": "LifecycleRuntimeContext",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/messaging/resolve.rs:RuntimeMessagingSettings",
        "name": "RuntimeMessagingSettings",
        "filePath": "claudine/lib/src/messaging/resolve.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Property:claudine/cli/src/commands/wrap/flags.rs:WrapperArgs.model",
        "name": "model",
        "filePath": "claudine/cli/src/commands/wrap/flags.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/flags.rs:WrapperArgs.model",
        "name": "model",
        "filePath": "claudine/cli/src/commands/wrap/flags.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/flags.rs:WrapperArgs.timeout",
        "name": "timeout",
        "filePath": "claudine/cli/src/commands/wrap/flags.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/flags.rs:WrapperArgs.stall_timeout",
        "name": "stall_timeout",
        "filePath": "claudine/cli/src/commands/wrap/flags.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "run_execution_stage", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/wrapper_stages.rs:run_execution_stage",
    "name": "run_execution_stage",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/wrapper_stages.rs"
  },
  "direction": "upstream",
  "impactedCount": 3,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 1,
    "modules_affected": 2
  },
  "byDepthCounts": {
    "1": 1,
    "2": 1,
    "3": 1
  },
  "affected_processes": [
    {
      "name": "async_main",
      "type": "Function",
      "filePath": "claudine/cli/src/main.rs",
      "affected_process_count": 6,
      "total_hits": 6,
      "earliest_broken_step": 1
    }
  ],
  "affected_modules": [
    {
      "name": "Compose",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Wrap",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
