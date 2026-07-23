# HarnessPromptState — upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete symbol: `HarnessPromptState`
- File: `claudine/cli/src/commands/wrap/harness_orch/types.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **UNKNOWN**

## Context

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'HarnessPromptState'. Use uid, file_path, or kind to disambiguate.",
  "candidates": [
    {
      "uid": "Impl:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
      "name": "HarnessPromptState",
      "kind": "",
      "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs",
      "line": 56,
      "score": 0.9
    },
    {
      "uid": "Struct:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
      "name": "HarnessPromptState",
      "kind": "",
      "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs",
      "line": 20,
      "score": 0.9
    }
  ]
}

---
**Next:** If planning changes, use impact({target: "HarnessPromptState", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution/processes.
```

## Impact

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'HarnessPromptState'. Blast radius differs per candidate (max 27 impacted at risk MEDIUM). Disambiguate with target_uid (or file_path/kind) for a single authoritative result.",
  "target": {
    "name": "HarnessPromptState"
  },
  "direction": "upstream",
  "totalCandidates": 2,
  "impactedCount": 0,
  "risk": "UNKNOWN",
  "maxImpactedCount": 27,
  "maxRisk": "MEDIUM",
  "candidates": [
    {
      "uid": "Struct:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
      "name": "HarnessPromptState",
      "kind": "",
      "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs",
      "line": 20,
      "score": 0.9,
      "impactedCount": 27,
      "risk": "MEDIUM",
      "direct": 5
    },
    {
      "uid": "Impl:claudine/cli/src/commands/wrap/harness_orch/types.rs:HarnessPromptState",
      "name": "HarnessPromptState",
      "kind": "",
      "filePath": "claudine/cli/src/commands/wrap/harness_orch/types.rs",
      "line": 56,
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
