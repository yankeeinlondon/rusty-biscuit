# CompositionError — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `CompositionError`
- File: `claudine/lib/src/composition/error/mod.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **UNKNOWN**

## Context

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'CompositionError'. Use uid, file_path, or kind to disambiguate.",
  "candidates": [
    {
      "uid": "Enum:claudine/lib/src/composition/error/mod.rs:CompositionError",
      "name": "CompositionError",
      "kind": "",
      "filePath": "claudine/lib/src/composition/error/mod.rs",
      "line": 107,
      "score": 0.9
    },
    {
      "uid": "Impl:claudine/lib/src/composition/error/mod.rs:CompositionError",
      "name": "CompositionError",
      "kind": "",
      "filePath": "claudine/lib/src/composition/error/mod.rs",
      "line": 2612,
      "score": 0.9
    }
  ]
}

---
**Next:** If planning changes, use impact({target: "CompositionError", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'CompositionError'. Blast radius differs per candidate (max 0 impacted at risk LOW). Disambiguate with target_uid (or file_path/kind) for a single authoritative result.",
  "target": {
    "name": "CompositionError"
  },
  "direction": "upstream",
  "totalCandidates": 2,
  "impactedCount": 0,
  "risk": "UNKNOWN",
  "maxImpactedCount": 0,
  "maxRisk": "LOW",
  "candidates": [
    {
      "uid": "Enum:claudine/lib/src/composition/error/mod.rs:CompositionError",
      "name": "CompositionError",
      "kind": "",
      "filePath": "claudine/lib/src/composition/error/mod.rs",
      "line": 107,
      "score": 0.9,
      "impactedCount": 0,
      "risk": "LOW",
      "direct": 0
    },
    {
      "uid": "Impl:claudine/lib/src/composition/error/mod.rs:CompositionError",
      "name": "CompositionError",
      "kind": "",
      "filePath": "claudine/lib/src/composition/error/mod.rs",
      "line": 2612,
      "score": 0.9,
      "impactedCount": 0,
      "risk": "LOW",
      "direct": 0
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```
