# ComposeOptions — upstream impact

- Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`
- Exact-seed revalidation: context and summary impact re-run on 2026-07-21; no source/config/artifact diff from the prior captured index
- Indexed worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`
- Concrete symbol: `ComposeOptions`
- File: `darkmatter/lib/src/markdown/compose/context/options.rs`
- Direction: upstream
- Minimum confidence: 0.8
- Maximum depth: 3
- Tests included: yes
- Risk: **LOW**

## Context

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'ComposeOptions'. Use uid, file_path, or kind to disambiguate.",
  "candidates": [
    {
      "uid": "Impl:darkmatter/lib/src/markdown/compose/context/options.rs:ComposeOptions",
      "name": "ComposeOptions",
      "kind": "",
      "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs",
      "line": 410,
      "score": 0.9
    },
    {
      "uid": "Struct:darkmatter/lib/src/markdown/compose/context/options.rs:ComposeOptions",
      "name": "ComposeOptions",
      "kind": "",
      "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs",
      "line": 43,
      "score": 0.9
    }
  ]
}

---
**Next:** If planning changes, use impact({target: "ComposeOptions", direction: "upstream", repo: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}) to check blast radius. To see execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```

## Impact

```json
{
  "status": "ambiguous",
  "message": "Found 2 symbols matching 'ComposeOptions'. Blast radius differs per candidate (max 0 impacted at risk LOW). Disambiguate with target_uid (or file_path/kind) for a single authoritative result.",
  "target": {
    "name": "ComposeOptions"
  },
  "direction": "upstream",
  "totalCandidates": 2,
  "impactedCount": 0,
  "risk": "UNKNOWN",
  "maxImpactedCount": 0,
  "maxRisk": "LOW",
  "candidates": [
    {
      "uid": "Impl:darkmatter/lib/src/markdown/compose/context/options.rs:ComposeOptions",
      "name": "ComposeOptions",
      "kind": "",
      "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs",
      "line": 410,
      "score": 0.9,
      "impactedCount": 0,
      "risk": "LOW",
      "direct": 0
    },
    {
      "uid": "Struct:darkmatter/lib/src/markdown/compose/context/options.rs:ComposeOptions",
      "name": "ComposeOptions",
      "kind": "",
      "filePath": "darkmatter/lib/src/markdown/compose/context/options.rs",
      "line": 43,
      "score": 0.9,
      "impactedCount": 0,
      "risk": "LOW",
      "direct": 0
    }
  ]
}

---
**Next:** Review d=1 items first (WILL BREAK). To check affected execution flows, READ gitnexus://repo//Users/ken/.claudine/worktrees/rusty-biscuit/claudine/processes.
```
