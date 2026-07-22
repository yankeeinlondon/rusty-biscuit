# Phase 1 SHA Ledger

Recorded by: Codex `/root` automated integration session
Recorded at: 2026-07-22T05:18:00Z (2026-07-21T22:18:00-0700)

## Frozen inputs

| Role | Ref | Frozen SHA | Verification command |
|---|---|---|---|
| Execution seed | `refs/heads/claudine` | `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | `git rev-parse refs/heads/claudine` |
| Foundation | `refs/heads/error-prop-and-file-resolution` | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` | `git rev-parse refs/heads/error-prop-and-file-resolution` |
| Proxy | `refs/heads/proxy-with` | `e348486c810969abe87a6b7209979034f5454b07` | `git rev-parse refs/heads/proxy-with` |
| Foundation merge base | `claudine` ↔ `error-prop-and-file-resolution` | `8fc8711434b01327479297af9b40a67685409d00` | `git merge-base refs/heads/claudine refs/heads/error-prop-and-file-resolution` |
| Proxy merge base | `claudine` ↔ `proxy-with` | `6cdb8bf56321c3747d5ea16a1241e47c2bff7fce` | `git merge-base refs/heads/claudine refs/heads/proxy-with` |

All five values were verified at the recorded time. The feature tips and merge
bases exactly match the reviewed plan inputs. The execution seed is newer than
the reviewed seed; see [`reviewed-seed-audit.md`](./reviewed-seed-audit.md).

## Integration branch

- Branch: `claudine-mega-merge-integration-20260721-phase1`
- Worktree: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-mega-merge-integration-20260721-phase1`
- Base: exact execution seed above
- Initial status: clean (`## claudine-mega-merge-integration-20260721-phase1`); current expected changes are inventoried in [`dirty-worktree-inventory.md`](./dirty-worktree-inventory.md)
- `rerere.enabled`: `true`

## Freeze record

Freeze owner: Codex `/root`, acting for this non-interactive execution.
Freeze effective time: 2026-07-22T05:18:00Z.

The three source refs must remain at the SHAs above until Phase 6 closes. This
ledger is the in-repository collaborator notice. No external collaboration
connector was available in this session, so no out-of-band notification was
sent. Reverify all three refs before every merge and at final promotion.

## Command record

```text
git rev-parse refs/heads/claudine refs/heads/error-prop-and-file-resolution refs/heads/proxy-with
git merge-base refs/heads/claudine refs/heads/error-prop-and-file-resolution
git merge-base refs/heads/claudine refs/heads/proxy-with
git worktree add -b claudine-mega-merge-integration-20260721-phase1 /Users/ken/.claudine/worktrees/rusty-biscuit/claudine-mega-merge-integration-20260721-phase1 1fdbfb3e47dac357923368c3f4437a565a4d3810
git merge-base --is-ancestor 72a5843af470ba75c1ae6f6e1ccf16ba10a427eb HEAD
git config rerere.enabled true
```

## Phase 6 candidate audit

| Role | Ref or object | SHA | Status |
|---|---|---|---|
| Audited integration candidate | `refs/heads/claudine-mega-merge-integration-20260721-phase1` | `df13f68dd7ad3ef22ef7e324dbdc213ed75afcd6` | immutable audit object; acceptance blocked |
| Current `claudine` | `refs/heads/claudine` | `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | still matches freeze |
| Frozen foundation | `refs/heads/error-prop-and-file-resolution` | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` | still matches freeze; not an ancestor of candidate |
| Frozen proxy | `refs/heads/proxy-with` | `e348486c810969abe87a6b7209979034f5454b07` | still matches freeze; not an ancestor of candidate |

No Phase 5 evidence-commit SHA exists: required acceptance rows are not all
green and the execution request prohibits staging or committing. No ref was
fast-forwarded and the freeze remains open.
