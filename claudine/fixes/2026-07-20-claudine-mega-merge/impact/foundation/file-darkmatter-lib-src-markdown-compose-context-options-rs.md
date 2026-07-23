# File hotspot upstream impact

- Revision: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/error-prop-and-file-resolution`
- Concrete graph identity: `File:darkmatter/lib/src/markdown/compose/context/options.rs`
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
    "uid": "File:darkmatter/lib/src/markdown/compose/context/options.rs",
    "name": "options.rs",
    "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs"
  },
  "incoming_counts": {
    "imports": 1
  },
  "outgoing_counts": {
    "imports": 2
  },
  "process_count": 0
}
```

## Impact

```json
{
  "target": {
    "id": "File:darkmatter/lib/src/markdown/compose/context/options.rs",
    "name": "options.rs",
    "type": "",
    "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs"
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
