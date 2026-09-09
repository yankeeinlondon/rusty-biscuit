pnpm --dir "/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/tools/test-audit" exec tsx src/cli.ts counters compare ../../sniff/fixes/2026-09-07-faster-sniff-tests/baseline/local-c2dee9217/work-counts.json ../../sniff/fixes/2026-09-07-faster-sniff-tests/baseline/local-c2dee9217/work-counts.json --config ../../sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json --markdown
| Signal | Environment | Baseline | Candidate | Delta | Ratio |
|---|---|---:|---:|---:|---:|
| `filesystem-walk` | macos-local-clean | 639 | 639 | +0 | 1.000 |
| `filesystem-io` | macos-local-clean | 17337 | 17337 | +0 | 1.000 |
| `filesystem-inventory` | macos-local-clean | 1367 | 1367 | +0 | 1.000 |
| `repo-structure` | macos-local-clean | 185 | 185 | +0 | 1.000 |
| `git` | macos-local-clean | 19 | 19 | +0 | 1.000 |
| `process` | macos-local-clean | 0 | 0 | +0 | 1.000 |
| `remote-requests` | macos-local-clean | 0 | 0 | +0 | 1.000 |
| `network-wan` | macos-local-clean | 0 | 0 | +0 | 1.000 |
GATE EXIT=0
