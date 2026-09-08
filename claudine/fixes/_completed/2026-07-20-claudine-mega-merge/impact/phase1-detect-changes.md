# Phase 1 Change Detection

Command equivalent:

```text
detect_changes({scope: "compare", base_ref: "main", worktree: "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-mega-merge-integration-20260721-phase1"})
```

GitNexus summary:

| Field | Result |
|---|---:|
| Changed symbols | 4,645 |
| Changed files | 423 |
| Affected symbols/process entries | 30 |
| Risk | CRITICAL |

This result was refreshed against execution seed `72a5843a` and describes the
entire long-lived `claudine` execution seed relative
to `main`; it is not scoped to the untracked Phase 1 evidence artifacts. The
reported processes include pre-existing Claudine composition/generation and
Rendezvous/Sniff flows. It therefore cannot serve as proof that a future staged
checkpoint contains only documentation. When staging is authorized, rerun
`detect_changes` with `scope: "staged"` and review that smaller result before
the checkpoint commit.
