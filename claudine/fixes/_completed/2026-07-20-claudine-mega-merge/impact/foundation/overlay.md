# merge_frontmatter_overlay — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `merge_frontmatter_overlay`
- File: `claudine/cli/src/commands/wrap/overlay.rs`
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
    "uid": "Function:claudine/cli/src/commands/wrap/overlay.rs:merge_frontmatter_overlay",
    "name": "merge_frontmatter_overlay",
    "kind": "Function",
    "filePath": "claudine/cli/src/commands/wrap/overlay.rs",
    "startLine": 6,
    "endLine": 17
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/cli/src/commands/wrap/harness_orch/prompt.rs:materialize_harness_prompt",
        "name": "materialize_harness_prompt",
        "filePath": "claudine/cli/src/commands/wrap/harness_orch/prompt.rs"
      }
    ]
  },
  "outgoing": {},
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "merge_frontmatter_overlay", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/cli/src/commands/wrap/overlay.rs:merge_frontmatter_overlay",
    "name": "merge_frontmatter_overlay",
    "type": "Function",
    "filePath": "claudine/cli/src/commands/wrap/overlay.rs"
  },
  "direction": "upstream",
  "impactedCount": 5,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 0,
    "modules_affected": 1
  },
  "byDepthCounts": {
    "1": 1,
    "2": 3,
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
