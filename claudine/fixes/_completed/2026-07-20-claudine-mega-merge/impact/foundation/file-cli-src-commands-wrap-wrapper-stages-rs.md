# File hotspot upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete graph identity: `File:claudine/cli/src/commands/wrap/wrapper_stages.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **LOW**

## Context summary

```json
{
  "status": "found",
  "symbol": {
    "uid": "File:claudine/cli/src/commands/wrap/wrapper_stages.rs",
    "name": "wrapper_stages.rs",
    "filePath": "claudine/cli/src/commands/wrap/wrapper_stages.rs"
  },
  "incoming_counts": {
    "imports": 1
  },
  "outgoing_counts": {
    "imports": 1
  },
  "process_count": 0
}
```

## Impact

```json
{
  "target": {
    "id": "File:claudine/cli/src/commands/wrap/wrapper_stages.rs",
    "name": "wrapper_stages.rs",
    "type": "",
    "filePath": "claudine/cli/src/commands/wrap/wrapper_stages.rs"
  },
  "direction": "upstream",
  "impactedCount": 1,
  "risk": "LOW",
  "epistemic": "exact",
  "summary": {
    "direct": 1,
    "processes_affected": 0,
    "modules_affected": 0
  },
  "byDepthCounts": {
    "1": 1
  },
  "affected_processes": [],
  "affected_modules": []
}
```
