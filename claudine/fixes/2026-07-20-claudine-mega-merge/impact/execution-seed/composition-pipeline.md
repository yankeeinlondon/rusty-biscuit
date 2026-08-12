# provider_run_handoff — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `provider_run_handoff`
- File: `claudine/cli/src/commands/wrap/composition/pipeline.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/composition/pipeline.rs:provider_run_handoff",
    "name": "provider_run_handoff",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs",
    "startLine": 1216,
    "endLine": 1402
  },
  "epistemic": "exact",
  "incoming": {},
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/context.rs:LifecycleTiming.from_instants#3",
        "name": "from_instants",
        "filePath": "claudine/lib/src/composition/lifecycle/context.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/context.rs:LifecycleCurrent.capture_at_event#1",
        "name": "capture_at_event",
        "filePath": "claudine/lib/src/composition/lifecycle/context.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.new#6",
        "name": "new",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/composition/runner.rs:run_composition_body",
        "name": "run_composition_body",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx",
        "name": "CompositionRunCtx",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/lifecycle/executor.rs:StackExecutionContext",
        "name": "StackExecutionContext",
        "filePath": "claudine/lib/src/composition/lifecycle/executor.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/lifecycle/mod.rs:LifecycleRuntimeContext",
        "name": "LifecycleRuntimeContext",
        "filePath": "claudine/lib/src/composition/lifecycle/mod.rs"
      }
    ],
    "uses": [
      {
        "uid": "Macro:claudine/cli/src/commands/wrap/composition/pipeline.rs:proceed_phase",
        "name": "proceed_phase",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.request",
        "name": "request",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.verbose",
        "name": "verbose",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.verbose",
        "name": "verbose",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.perf_collector",
        "name": "perf_collector",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.document_start",
        "name": "document_start",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.term",
        "name": "term",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.external_guard",
        "name": "external_guard",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:CompositionAttempt.skip_preflight",
        "name": "skip_preflight",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/pipeline.rs:EnvironmentPhase.env_plan",
        "name": "env_plan",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/launch_workspace.rs:LaunchWorkspaceContext.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/lib/src/composition/launch_workspace.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/launch_workspace.rs:LaunchWorkspaceContext.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/lib/src/composition/launch_workspace.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/launch_workspace.rs:LaunchWorkspaceContext.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/lib/src/composition/launch_workspace.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "provider_run_handoff", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/composition/pipeline.rs:provider_run_handoff",
    "name": "provider_run_handoff",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
  },
  "direction": "upstream",
  "impactedCount": 0,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 0,
    "processes_affected": 0,
    "modules_affected": 0
  },
  "byDepthCounts": {},
  "affected_processes": [],
  "affected_modules": []
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
