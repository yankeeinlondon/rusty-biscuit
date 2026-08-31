# file_reference_detail — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `file_reference_detail`
- File: `claudine/lib/src/composition/error/render/mod.rs`
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
    "uid": "Function:claudine/lib/src/composition/error/render/mod.rs:file_reference_detail",
    "name": "file_reference_detail",
    "kind": "Function",
    "filePath": "claudine/lib/src/composition/error/render/mod.rs",
    "startLine": 213,
    "endLine": 235
  },
  "epistemic": "exact",
  "incoming": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/composition/error/render/mod.rs:CompositionError.detail#0",
        "name": "detail",
        "filePath": "claudine/lib/src/composition/error/render/mod.rs"
      }
    ]
  },
  "outgoing": {
    "calls": [
      {
        "uid": "Function:claudine/lib/src/diagnostics/mod.rs:null_detail_for",
        "name": "null_detail_for",
        "filePath": "claudine/lib/src/diagnostics/mod.rs"
      },
      {
        "uid": "Function:darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs:suggest_sibling_files",
        "name": "suggest_sibling_files",
        "filePath": "darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs"
      }
    ]
  },
  "processes": []
}

---
**Next:** If planning changes, use impact({target: "file_reference_detail", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "target": {
    "id": "Function:claudine/lib/src/composition/error/render/mod.rs:file_reference_detail",
    "name": "file_reference_detail",
    "type": "Function",
    "filePath": "claudine/lib/src/composition/error/render/mod.rs"
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
      "name": "Render",
      "hits": 1,
      "impact": "direct"
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
