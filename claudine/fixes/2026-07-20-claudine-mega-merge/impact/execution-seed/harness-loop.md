# run_harness_loop_inner — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `run_harness_loop_inner`
- File: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:run_harness_loop_inner",
    "name": "run_harness_loop_inner",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs",
    "startLine": 272,
    "endLine": 286
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:run_harness_loop",
        "name": "run_harness_loop",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:HarnessLoopState.new#1",
        "name": "new",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:prepare_attempt_phase",
        "name": "prepare_attempt_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:execute_attempt_phase",
        "name": "execute_attempt_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:classify_attempt_phase",
        "name": "classify_attempt_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "run_harness_loop_inner", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:run_harness_loop_inner",
    "name": "run_harness_loop_inner",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
  },
  "direction": "upstream",
  "impactedCount": 5,
  "risk": "HIGH",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 1,
    "modules_affected": 3
  },
  "byDepthCounts": {
    "1": 1,
    "2": 2,
    "3": 2
  },
  "affected_processes": [
    {
      "name": "run_composition_body",
      "type": "Function",
      "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs",
      "affected_process_count": 5,
      "total_hits": 5,
      "earliest_broken_step": 1
    }
  ],
  "affected_modules": [
    {
      "name": "Composition",
      "hits": 2,
      "impact": "direct"
    },
    {
      "name": "Wrap",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Lifecycle",
      "hits": 1,
      "impact": "indirect"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
