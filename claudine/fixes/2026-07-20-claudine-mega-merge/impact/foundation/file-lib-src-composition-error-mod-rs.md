# File hotspot upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete graph identity: `File:claudine/lib/src/composition/error/mod.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **MEDIUM**

## Context summary

```json
{
  "status": "found",
  "symbol": {
    "uid": "File:claudine/lib/src/composition/error/mod.rs",
    "name": "mod.rs",
    "filePath": "claudine/lib/src/composition/error/mod.rs"
  },
  "incoming_counts": {
    "imports": 8
  },
  "outgoing_counts": {
    "imports": 6
  },
  "process_count": 0
}
```

## Impact

```json
{
  "target": {
    "id": "File:claudine/lib/src/composition/error/mod.rs",
    "name": "mod.rs",
    "type": "",
    "filePath": "claudine/lib/src/composition/error/mod.rs"
  },
  "direction": "upstream",
  "impactedCount": 10,
  "risk": "MEDIUM",
  "epistemic": "exact",
  "summary": {
    "direct": 8,
    "processes_affected": 0,
    "modules_affected": 0
  },
  "byDepthCounts": {
    "1": 8,
    "2": 2
  },
  "affected_processes": [],
  "affected_modules": []
}
```
