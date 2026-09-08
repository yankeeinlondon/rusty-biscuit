@rusty-biscuit/test-audit@0.1.0 (node v22.20.0)

### run 34008778001 (main@03ce3f8c1, PR #68)

| Environment | Tier | Package | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips | Retries |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | L1 | `sniff` | 153.2 s | 28.8 s | 108.0 s | 1787 | 0 | 0 | 0 |
| `ubuntu-latest` | L1 | `sniff-cli` | 231.8 s | 17.2 s | 68.6 s | 790 | 0 | 0 | 0 |
| `ubuntu-latest` | L2 | `sniff-cli` | 133.7 s | 1.3 s | 1.3 s | 2 | 0 | 0 | 0 |
| `macos-latest` | L1 | `sniff` | 275.9 s | 50.1 s | 143.7 s | 1809 | 0 | 0 | 0 |
| `macos-latest` | L1 | `sniff-cli` | 314.1 s | 29.9 s | 89.2 s | 790 | 0 | 0 | 0 |
| `macos-latest` | L2 | `sniff-cli` | 225.4 s | 2.6 s | 2.5 s | 2 | 0 | 0 | 0 |
| `windows-latest` | L1 | `sniff` | 440.7 s | 165.3 s | 164.9 s | 1783 | 0 | 0 | 0 |
| `windows-latest` | L1 | `sniff-cli` | 553.8 s | 111.2 s | 111.1 s | 786 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `sniff` | 2.8 s | 86.2 s | 282.9 s | 1787 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `sniff-cli` | 1.8 s | 41.2 s | 161.3 s | 790 | 0 | 0 | 0 |

**Note:** Build/setup, runner elapsed, and summed duration are three separate columns:
- **Build/setup**: manifest `duration_s` minus runner time (compile + setup overhead)
- **Runner elapsed**: nextest wall time for the run (from `<testsuites time>`)
- **Summed duration**: Σ of all test case times (exceeds elapsed under parallelism)

The summed column is comparable across runner core counts; the others are not.
