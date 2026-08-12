# run_harness_loop_inner — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `run_harness_loop_inner`
- File: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:run_harness_loop_inner",
    "name": "run_harness_loop_inner",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs",
    "startLine": 282,
    "endLine": 296
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
**Next:** If planning changes, use impact({target: "run_harness_loop_inner", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
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
    "modules_affected": 4
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
      "name": "Wrap",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Composition",
      "hits": 1,
      "impact": "indirect"
    },
    {
      "name": "Harness_orch",
      "hits": 1,
      "impact": "direct"
    },
    {
      "name": "Lifecycle",
      "hits": 1,
      "impact": "indirect"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
