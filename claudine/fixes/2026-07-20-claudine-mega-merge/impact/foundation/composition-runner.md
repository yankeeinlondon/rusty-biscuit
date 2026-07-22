# run_composition_body — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `run_composition_body`
- File: `claudine/cli/src/commands/wrap/composition/runner.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/composition/runner.rs:run_composition_body",
    "name": "run_composition_body",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs",
    "startLine": 94,
    "endLine": 532
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/composition/pipeline.rs:provider_run_handoff",
        "name": "provider_run_handoff",
        "filePath": "claudine/cli/src/commands/wrap/composition/pipeline.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/lifecycle/context.rs:LifecycleErrorInfo.from_error_or_action#2",
        "name": "from_error_or_action",
        "filePath": "claudine/lib/src/composition/lifecycle/context.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/preflight.rs:resolve_shell_approvals",
        "name": "resolve_shell_approvals",
        "filePath": "claudine/lib/src/composition/preflight.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/composition/mod.rs:SingleCompositionOutcome",
        "name": "SingleCompositionOutcome",
        "filePath": "claudine/cli/src/commands/wrap/composition/mod.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
        "name": "HarnessPromptState",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.request",
        "name": "request",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.target",
        "name": "target",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.provider",
        "name": "provider",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.effective_repo_root",
        "name": "effective_repo_root",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.launch_workspace",
        "name": "launch_workspace",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.launch_cwd",
        "name": "launch_cwd",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.binary_path",
        "name": "binary_path",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.lifecycle_effect_engine",
        "name": "lifecycle_effect_engine",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.emitter",
        "name": "emitter",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.lifecycle_settings",
        "name": "lifecycle_settings",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.lifecycle_messaging",
        "name": "lifecycle_messaging",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.lifecycle_context",
        "name": "lifecycle_context",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.term",
        "name": "term",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.document_start",
        "name": "document_start",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.shell_options",
        "name": "shell_options",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.silent",
        "name": "silent",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.quiet",
        "name": "quiet",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.is_inline",
        "name": "is_inline",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.profile",
        "name": "profile",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.effective_non_interactive",
        "name": "effective_non_interactive",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.args_before_prompt",
        "name": "args_before_prompt",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.child_cwd",
        "name": "child_cwd",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.use_structured",
        "name": "use_structured",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/composition/runner.rs:CompositionRunCtx.structured_codex_output",
        "name": "structured_codex_output",
        "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
      }
    ]
  },
  "processes": [
    {
      "id": "proc_255_run_composition_body",
      "name": "Run_composition_body → Color_mode",
      "step_index": 1,
      "step_count": 4
    },
    {
      "id": "proc_256_run_composition_body",
      "name": "Run_composition_body → Parse_duration_secs",
      "step_index": 1,
      "step_count": 4
    },
    {
      "id": "proc_109_run_composition_body",
      "name": "Run_composition_body → Default",
      "step_index": 1,
      "step_count": 5
    },
    {
      "id": "proc_110_run_composition_body",
      "name": "Run_composition_body → Escape_sequence_end",
      "step_index": 1,
      "step_count": 5
    },
    {
      "id": "proc_257_run_composition_body",
      "name": "Run_composition_body → Record_event_emission",
      "step_index": 1,
      "step_count": 4
    }
  ]
}

---
**Next:** If planning changes, use impact({target: "run_composition_body", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/composition/runner.rs:run_composition_body",
    "name": "run_composition_body",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/composition/runner.rs"
  },
  "direction": "upstream",
  "impactedCount": 1,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 0,
    "modules_affected": 1
  },
  "byDepthCounts": {
    "1": 1
  },
  "affected_processes": [],
  "affected_modules": [
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
