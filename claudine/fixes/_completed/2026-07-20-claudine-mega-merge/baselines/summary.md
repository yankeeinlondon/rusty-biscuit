# Frozen Branch-Tip Baseline Summary

Every command ran independently from a detached worktree at the exact frozen
SHA. The non-interactive host policy capped each command at 60 seconds. A
timeout is baseline evidence, not a product failure and not a pass.

| Gate | Foundation `43c23c6` | Proxy `e348486c` |
|---|---|---|
| Biscuit File L1 | passed | passed |
| Biscuit File L2 | passed | passed |
| Biscuit File lint | passed | passed |
| Darkmatter L1 | timed-out | timed-out |
| Darkmatter L2 | timed-out | timed-out |
| Darkmatter lint | timed-out | timed-out |
| Biscuit Test Harness L1 | failed: root recipe `package_args[@]` unbound | failed: same root recipe defect |
| Biscuit Test Harness lint | passed | passed |
| Rendezvous check | timed-out | timed-out |
| Rendezvous L1 | timed-out | timed-out |
| Rendezvous lint | timed-out | timed-out |
| Claudine L1 | timed-out | timed-out |
| Claudine L2 | timed-out | timed-out |
| Claudine lint | timed-out | timed-out |
| Claudine Windows check | timed-out; compile-only evidence not obtained | failed: branch has no `check-windows` recipe |

Totals:

- Foundation: 4 passed, 1 failed, 10 timed out.
- Proxy: 4 passed, 2 failed, 9 timed out.
- Skipped assertions remain visible in the individual nextest outputs; no skip
  was promoted to `passed`.
- No native Linux or Windows runtime evidence was produced on this macOS host.

The individual files preserve command, revision, worktree, exit status,
classification, timeout policy, and separately labeled stdout/stderr.
