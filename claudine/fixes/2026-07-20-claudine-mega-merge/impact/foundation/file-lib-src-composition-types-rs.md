# File hotspot upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete graph identity: `File:claudine/lib/src/composition/types.rs`
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
    "uid": "File:claudine/lib/src/composition/types.rs",
    "name": "types.rs",
    "filePath": "claudine/lib/src/composition/types.rs"
  },
  "incoming_counts": {
    "imports": 12
  },
  "outgoing_counts": {
    "imports": 3
  },
  "process_count": 0
}
```

## Impact

```json
{
  "target": {
    "id": "File:claudine/lib/src/composition/types.rs",
    "name": "types.rs",
    "type": "",
    "filePath": "claudine/lib/src/composition/types.rs"
  },
  "direction": "upstream",
  "impactedCount": 18,
  "risk": "MEDIUM",
  "epistemic": "exact",
  "summary": {
    "direct": 12,
    "processes_affected": 0,
    "modules_affected": 0
  },
  "byDepthCounts": {
    "1": 12,
    "2": 4,
    "3": 2
  },
  "affected_processes": [],
  "affected_modules": []
}
```
