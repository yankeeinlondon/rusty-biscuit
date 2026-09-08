# run_sequence_steps — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `run_sequence_steps`
- File: `claudine/cli/src/commands/wrap/sequence/iterate.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/sequence/iterate.rs:run_sequence_steps",
    "name": "run_sequence_steps",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs",
    "startLine": 102,
    "endLine": 261
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/mod.rs:execute_sequence",
        "name": "execute_sequence",
        "filePath": "claudine/cli/src/commands/wrap/sequence/mod.rs"
      }
    ]
  },
  "outgoing": {
    "accesses": [
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.plan",
        "name": "plan",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.plan",
        "name": "plan",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.shared",
        "name": "shared",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.interrupted",
        "name": "interrupted",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.effective_fail_fast",
        "name": "effective_fail_fast",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.silent",
        "name": "silent",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/iterate.rs:SequenceRunContext.silent",
        "name": "silent",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequenceRunSummary.succeeded",
        "name": "succeeded",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequenceRunSummary.failed",
        "name": "failed",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequenceRunSummary.steps",
        "name": "steps",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ],
    "calls": [
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/iterate.rs:run_one_step",
        "name": "run_one_step",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/iterate.rs:emit_group_task_breakdown",
        "name": "emit_group_task_breakdown",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/iterate.rs:emit_step_status",
        "name": "emit_step_status",
        "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
      },
      {
        "uid": "Function:claudine/cli/src/log.rs:terminal",
        "name": "terminal",
        "filePath": "claudine/cli/src/log.rs"
      },
      {
        "uid": "Function:claudine/cli/src/log.rs:message",
        "name": "message",
        "filePath": "claudine/cli/src/log.rs"
      },
      {
        "uid": "Function:darkmatter/lib/src/markdown/schemas/validate.rs:ValidatorCache.with_capacity#1",
        "name": "with_capacity",
        "filePath": "darkmatter/lib/src/markdown/schemas/validate.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/perf/mod.rs:SequenceStepPerf",
        "name": "SequenceStepPerf",
        "filePath": "claudine/cli/src/perf/mod.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/perf/mod.rs:SequenceTaskPerf",
        "name": "SequenceTaskPerf",
        "filePath": "claudine/cli/src/perf/mod.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:SequenceRunSummary",
        "name": "SequenceRunSummary",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:SequenceStepResult",
        "name": "SequenceStepResult",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "run_sequence_steps", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/sequence/iterate.rs:run_sequence_steps",
    "name": "run_sequence_steps",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/iterate.rs"
  },
  "direction": "upstream",
  "impactedCount": 4,
  "risk": "HIGH",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 1,
    "modules_affected": 3
  },
  "byDepthCounts": {
    "1": 1,
    "2": 1,
    "3": 2
  },
  "affected_processes": [
    {
      "name": "execute_sequence",
      "type": "Function",
      "filePath": "claudine/cli/src/commands/wrap/sequence/mod.rs",
      "affected_process_count": 6,
      "total_hits": 6,
      "earliest_broken_step": 1
    }
  ],
  "affected_modules": [
    {
      "name": "Commands",
      "hits": 2,
      "impact": "indirect"
    },
    {
      "name": "Compose",
      "hits": 1,
      "impact": "indirect"
    },
    {
      "name": "Sequence",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
