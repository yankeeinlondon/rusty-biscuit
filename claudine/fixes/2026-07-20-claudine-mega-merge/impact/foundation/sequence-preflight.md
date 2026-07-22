# run_phase_1c_with_schema — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `run_phase_1c_with_schema`
- File: `claudine/cli/src/commands/wrap/sequence/phase1c.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:run_phase_1c_with_schema",
    "name": "run_phase_1c_with_schema",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs",
    "startLine": 61,
    "endLine": 151
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
        "uid": "Function:claudine/lib/src/composition/schema/mod.rs:InteractiveSchemaOptions.allowed#0",
        "name": "allowed",
        "filePath": "claudine/lib/src/composition/schema/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/jit.rs:StepComposeContext.launch_area",
        "name": "launch_area",
        "filePath": "claudine/cli/src/commands/wrap/sequence/jit.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/sequence/jit.rs:StepComposeContext.shared",
        "name": "shared",
        "filePath": "claudine/cli/src/commands/wrap/sequence/jit.rs"
      }
    ],
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/schema/mod.rs:InteractiveSchemaOptions.allowed#0",
        "name": "allowed",
        "filePath": "claudine/lib/src/composition/schema/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/schema_interactive/mod.rs:resolve_interactive_options",
        "name": "resolve_interactive_options",
        "filePath": "claudine/cli/src/commands/schema_interactive/mod.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:failures_slice",
        "name": "failures_slice",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:into_sequence_missing",
        "name": "into_sequence_missing",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:run_phase_1c_attempt",
        "name": "run_phase_1c_attempt",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:collect_sequence_missing_values",
        "name": "collect_sequence_missing_values",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:find_first_unsupported",
        "name": "find_first_unsupported",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:merge_overrides",
        "name": "merge_overrides",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/sequence/phase1c.rs:SequencePreflight",
        "name": "SequencePreflight",
        "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "run_phase_1c_with_schema", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/sequence/phase1c.rs:run_phase_1c_with_schema",
    "name": "run_phase_1c_with_schema",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/phase1c.rs"
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
