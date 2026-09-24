# Identity comparison: identical

## `claudine`

Migration manifest applied: yes

| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |
|---|---|---|---:|---:|---:|---:|---|
| linux | `none` | `L1` | 4342 | 4342 | 0 | 0 | none |
| linux | `none` | `L2` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `L3` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `browser` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-0` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-1` | 4342 | 4342 | 0 | 0 | none |
| linux | `none` | `override-ci-2` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-3` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-4` | 4342 | 2 | 4340 | 0 | none |
| linux | `none` | `override-ci-5` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-6` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-7` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-ci-8` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-0` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-1` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-2` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-3` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-4` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-5` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-6` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `override-default-7` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `real` | 4342 | 0 | 4342 | 0 | none |
| linux | `none` | `sanity` | 4342 | 4342 | 0 | 0 | none |

Platform-absent:

- `none`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures

## Notes

- linux/claudine/none: override override-ci-3 was rewritten ('test(=test_detect_completes_in_reasonable_time)' → 'test(=integration::test_detect_completes_in_reasonable_time)')
