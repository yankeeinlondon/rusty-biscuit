# execute_sequence — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `execute_sequence`
- File: `claudine/cli/src/commands/wrap/sequence/mod.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/sequence/mod.rs:execute_sequence",
    "name": "execute_sequence",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/mod.rs",
    "startLine": 37,
    "endLine": 483
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/sequence.rs:run_sequence_inner",
        "name": "run_sequence_inner",
        "filePath": "claudine/cli/src/commands/sequence.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/hints.rs:parse_selection_hints_from_frontmatter",
        "name": "parse_selection_hints_from_frontmatter",
        "filePath": "claudine/lib/src/composition/hints.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/select.rs:classify_agent_resolution",
        "name": "classify_agent_resolution",
        "filePath": "claudine/lib/src/composition/select.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/select.rs:build_picker_plan_with_hints",
        "name": "build_picker_plan_with_hints",
        "filePath": "claudine/lib/src/composition/select.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/select.rs:resolve_model_with_hints",
        "name": "resolve_model_with_hints",
        "filePath": "claudine/lib/src/composition/select.rs"
      },
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
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
        "uid": "Function:claudine/cli/src/perf/render.rs:emit_report",
        "name": "emit_report",
        "filePath": "claudine/cli/src/perf/render.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:ResolvedExecutionTarget",
        "name": "ResolvedExecutionTarget",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:ProviderPickerPlan",
        "name": "ProviderPickerPlan",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:SequenceStepDraft",
        "name": "SequenceStepDraft",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Function:biscuit-terminal/lib/src/components/status.rs:Status.render#1",
        "name": "render",
        "filePath": "biscuit-terminal/lib/src/components/status.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.model",
        "name": "model",
        "filePath": "claudine/cli/src/commands/compose/mod.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/compose/mod.rs:SharedComposeArgs.dry_run",
        "name": "dry_run",
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
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.original_ref",
        "name": "original_ref",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.resolved_path",
        "name": "resolved_path",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.resolved_path",
        "name": "resolved_path",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.resolved_path",
        "name": "resolved_path",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.markdown",
        "name": "markdown",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:ResolvedCompositionSource.markdown",
        "name": "markdown",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequencePlan.steps",
        "name": "steps",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequencePlan.steps",
        "name": "steps",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequencePlan.document_fail_fast",
        "name": "document_fail_fast",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:SequenceExecutionOptions.fail_fast_override",
        "name": "fail_fast_override",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ]
  },
  "processes": [
    {
      "id": "proc_181_execute_sequence",
      "name": "Execute_sequence → Color_mode",
      "step_index": 1,
      "step_count": 4
    },
    {
      "id": "proc_32_execute_sequence",
      "name": "Execute_sequence → BlockContent",
      "step_index": 1,
      "step_count": 6
    },
    {
      "id": "proc_33_execute_sequence",
      "name": "Execute_sequence → Split_lines",
      "step_index": 1,
      "step_count": 6
    },
    {
      "id": "proc_34_execute_sequence",
      "name": "Execute_sequence → Escape_sequence_end",
      "step_index": 1,
      "step_count": 6
    },
    {
      "id": "proc_71_execute_sequence",
      "name": "Execute_sequence → Default",
      "step_index": 1,
      "step_count": 5
    },
    {
      "id": "proc_180_execute_sequence",
      "name": "Execute_sequence → Parse_duration_secs",
      "step_index": 1,
      "step_count": 4
    }
  ]
}

---
**Next:** If planning changes, use impact({target: "execute_sequence", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/sequence/mod.rs:execute_sequence",
    "name": "execute_sequence",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/sequence/mod.rs"
  },
  "direction": "upstream",
  "impactedCount": 4,
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
      "hits": 4,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
