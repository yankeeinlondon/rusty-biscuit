# execute_loop_or_single — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `execute_loop_or_single`
- File: `claudine/cli/src/commands/compose/prep.rs`
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
    "uid": "Function:claudine/cli/src/commands/compose/prep.rs:execute_loop_or_single",
    "name": "execute_loop_or_single",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/compose/prep.rs",
    "startLine": 678,
    "endLine": 834
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/compose/prep.rs:run_composition_inner",
        "name": "run_composition_inner",
        "filePath": "claudine/cli/src/commands/compose/prep.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:darkmatter/lib/src/markdown/compose/context/runtime.rs:ComposeContext.capture_for_document#2",
        "name": "capture_for_document",
        "filePath": "darkmatter/lib/src/markdown/compose/context/runtime.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/loop_run.rs:emit_compose_warnings",
        "name": "emit_compose_warnings",
        "filePath": "claudine/cli/src/commands/compose/loop_run.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/loop_run.rs:emit_rate_limit_halt",
        "name": "emit_rate_limit_halt",
        "filePath": "claudine/cli/src/commands/compose/loop_run.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/mod.rs:CompositionKind.mode#0",
        "name": "mode",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/mod.rs:CompositionKind.prepare_with_schema#2",
        "name": "prepare_with_schema",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/prep.rs:build_loop_options",
        "name": "build_loop_options",
        "filePath": "claudine/cli/src/commands/compose/prep.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/prep.rs:build_and_run_loop",
        "name": "build_and_run_loop",
        "filePath": "claudine/cli/src/commands/compose/prep.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/prepare.rs:PrepareOptions",
        "name": "PrepareOptions",
        "filePath": "claudine/lib/src/composition/prepare.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Function:claudine/cli/src/commands/compose/mod.rs:CompositionKind.mode#0",
        "name": "mode",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/compose/mod.rs:CompositionKind.prepare_with_schema#2",
        "name": "prepare_with_schema",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.dry_run",
        "name": "dry_run",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.silent",
        "name": "silent",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.perf",
        "name": "perf",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.perf",
        "name": "perf",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.perf",
        "name": "perf",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.source_repo_root",
        "name": "source_repo_root",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.source_repo_root",
        "name": "source_repo_root",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/prep_context.rs:CompositionPrepContext.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/prep_context.rs"
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
      },
      {
        "uid": "Property:claudine/lib/src/composition/launch_workspace.rs:LaunchWorkspaceContext.child_cwd",
        "name": "child_cwd",
        "filePath": "claudine/lib/src/composition/launch_workspace.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/launch_workspace.rs:LaunchWorkspaceContext.child_cwd",
        "name": "child_cwd",
        "filePath": "claudine/lib/src/composition/launch_workspace.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/preflight.rs:PreFlightResult.approved_commands",
        "name": "approved_commands",
        "filePath": "claudine/lib/src/composition/preflight.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/preflight.rs:PreFlightResult.approved_commands",
        "name": "approved_commands",
        "filePath": "claudine/lib/src/composition/preflight.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.markdown",
        "name": "markdown",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "execute_loop_or_single", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/compose/prep.rs:execute_loop_or_single",
    "name": "execute_loop_or_single",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/compose/prep.rs"
  },
  "direction": "upstream",
  "impactedCount": 5,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 1,
    "modules_affected": 1
  },
  "byDepthCounts": {
    "1": 1,
    "2": 2,
    "3": 2
  },
  "affected_processes": [
    {
      "name": "run_composition_inner",
      "type": "Function",
      "filePath": "claudine/cli/src/commands/compose/prep.rs",
      "affected_process_count": 1,
      "total_hits": 1,
      "earliest_broken_step": 1
    }
  ],
  "affected_modules": [
    {
      "name": "Compose",
      "hits": 5,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
