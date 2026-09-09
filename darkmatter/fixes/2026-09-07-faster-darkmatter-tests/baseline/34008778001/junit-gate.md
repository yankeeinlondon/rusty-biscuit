@rusty-biscuit/test-audit@0.1.0 (node v22.20.0)

### run 34008778001 (main@03ce3f8c1, PR #68)

| Environment | Tier | Package | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips | Retries |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | L1 | `darkmatter` | 278.6 s | 167.4 s | 668.2 s | 6332 | 0 | 0 | 0 |
| `ubuntu-latest` | L1 | `darkmatter-cli` | 236.9 s | 47.1 s | 184.9 s | 665 | 0 | 0 | 0 |
| `ubuntu-latest` | L1 | `dmls` | 221.2 s | 7.8 s | 31.1 s | 643 | 0 | 0 | 0 |
| `ubuntu-latest` | L1 | `zed-dmls-cli` | 156.9 s | 0.1 s | 0.4 s | 27 | 0 | 0 | 0 |
| `ubuntu-latest` | L2 | `darkmatter-cli` | 227.9 s | 7.1 s | 7.1 s | 69 | 0 | 0 | 0 |
| `ubuntu-latest` | L2 | `dmls` | 164.3 s | 1.7 s | 1.7 s | 3 | 0 | 0 | 0 |
| `ubuntu-latest` | browser | `darkmatter` | 277.4 s | 40.6 s | 40.6 s | 86 | 0 | 0 | 0 |
| `macos-latest` | L1 | `darkmatter` | 408.4 s | 174.6 s | 523.0 s | 6333 | 0 | 0 | 0 |
| `macos-latest` | L1 | `darkmatter-cli` | 388.3 s | 47.7 s | 143.1 s | 665 | 0 | 0 | 0 |
| `macos-latest` | L1 | `dmls` | 385.4 s | 10.6 s | 31.7 s | 643 | 0 | 0 | 0 |
| `macos-latest` | L1 | `zed-dmls-cli` | 287.4 s | 0.6 s | 1.7 s | 27 | 0 | 0 | 0 |
| `macos-latest` | L2 | `darkmatter-cli` | 321.8 s | 13.2 s | 13.2 s | 69 | 0 | 0 | 0 |
| `macos-latest` | L2 | `dmls` | 315.9 s | 4.1 s | 4.1 s | 3 | 0 | 0 | 0 |
| `windows-latest` | L1 | `darkmatter` | 697.0 s | 238.0 s | 950.1 s | 6319 | 0 | 0 | 0 |
| `windows-latest` | L1 | `darkmatter-cli` | 565.4 s | 80.6 s | 315.4 s | 667 | 0 | 0 | 0 |
| `windows-latest` | L1 | `dmls` | 528.2 s | 10.8 s | 42.9 s | 645 | 0 | 0 | 0 |
| `windows-latest` | L1 | `zed-dmls-cli` | 404.8 s | 2.2 s | 7.1 s | 26 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `darkmatter` | 53.9 s | 229.1 s | 895.4 s | 6332 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `darkmatter-cli` | 19.2 s | 66.8 s | 253.5 s | 665 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `dmls` | 2.9 s | 8.1 s | 32.1 s | 643 | 0 | 0 | 0 |
| `wsl2-ubuntu` | L1 | `zed-dmls-cli` | 0.9 s | 0.1 s | 0.5 s | 27 | 0 | 0 | 0 |

**Note:** Build/setup, runner elapsed, and summed duration are three separate columns:
- **Build/setup**: manifest `duration_s` minus runner time (compile + setup overhead)
- **Runner elapsed**: nextest wall time for the run (from `<testsuites time>`)
- **Summed duration**: Σ of all test case times (exceeds elapsed under parallelism)

The summed column is comparable across runner core counts; the others are not.
