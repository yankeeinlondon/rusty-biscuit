# materialize_harness_prompt — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `materialize_harness_prompt`
- File: `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/prompt.rs:materialize_harness_prompt",
    "name": "materialize_harness_prompt",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/prompt.rs",
    "startLine": 204,
    "endLine": 358
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:materialize_attempt_prompt_phase",
        "name": "materialize_attempt_prompt_phase",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/prompt.rs:compose_rematerialize_resolves_ctx_agent_from_env",
        "name": "compose_rematerialize_resolves_ctx_agent_from_env",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/prompt.rs"
      },
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/prompt.rs:proxy_target_preflight_approves_frontmatter_shell_and_rematerializes",
        "name": "proxy_target_preflight_approves_frontmatter_shell_and_rematerializes",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/prompt.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/prepare.rs:bind_agent_workspace",
        "name": "bind_agent_workspace",
        "filePath": "claudine/lib/src/composition/prepare.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/prepare.rs:prepare_inline",
        "name": "prepare_inline",
        "filePath": "claudine/lib/src/composition/prepare.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/resolve.rs:validate_file_permissions",
        "name": "validate_file_permissions",
        "filePath": "claudine/lib/src/composition/resolve.rs"
      },
      {
        "uid": "Function:claudine/lib/src/composition/runtime_state.rs:layered_set_overrides",
        "name": "layered_set_overrides",
        "filePath": "claudine/lib/src/composition/runtime_state.rs"
      },
      {
        "uid": "Struct:claudine/cli/src/commands/wrap/harness_orch/types.rs:MaterializedHarnessPrompt",
        "name": "MaterializedHarnessPrompt",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/prepare.rs:PrepareOptions",
        "name": "PrepareOptions",
        "filePath": "claudine/lib/src/composition/prepare.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:ResolvedCompositionSource",
        "name": "ResolvedCompositionSource",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Struct:claudine/lib/src/composition/types.rs:RematerializeInputs",
        "name": "RematerializeInputs",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ],
    "accesses": [
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.mode",
        "name": "mode",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.source_path",
        "name": "source_path",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.base_prompt",
        "name": "base_prompt",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.overlay",
        "name": "overlay",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.prompt_tail",
        "name": "prompt_tail",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.next_prompt_override",
        "name": "next_prompt_override",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.rematerialize",
        "name": "rematerialize",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.rematerialize",
        "name": "rematerialize",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.rematerialize",
        "name": "rematerialize",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.runtime_state",
        "name": "runtime_state",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState.runtime_state",
        "name": "runtime_state",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:RematerializeInputs.set_overrides",
        "name": "set_overrides",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:RematerializeInputs.set_overrides",
        "name": "set_overrides",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:RematerializeInputs.file_ref_fallback_dir",
        "name": "file_ref_fallback_dir",
        "filePath": "claudine/lib/src/composition/types.rs"
      },
      {
        "uid": "Property:claudine/lib/src/composition/types.rs:RematerializeInputs.file_resolution_context",
        "name": "file_resolution_context",
        "filePath": "claudine/lib/src/composition/types.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "materialize_harness_prompt", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/harness_orch/prompt.rs:materialize_harness_prompt",
    "name": "materialize_harness_prompt",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/harness_orch/prompt.rs"
  },
  "direction": "upstream",
  "impactedCount": 5,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 3,
    "processes_affected": 0,
    "modules_affected": 1
  },
  "byDepthCounts": {
    "1": 3,
    "2": 1,
    "3": 1
  },
  "affected_processes": [],
  "affected_modules": [
    {
      "name": "Harness_orch",
      "hits": 5,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
