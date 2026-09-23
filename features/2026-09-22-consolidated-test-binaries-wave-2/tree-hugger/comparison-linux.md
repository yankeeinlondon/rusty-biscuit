# Identity comparison: identical

## `tree-hugger`

Migration manifest applied: yes

| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |
|---|---|---|---:|---:|---:|---:|---|
| linux | `none` | `L1` | 483 | 483 | 0 | 0 | none |
| linux | `none` | `L2` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `L3` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `browser` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-0` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-1` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-2` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-3` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-4` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-5` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-6` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-7` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-ci-8` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-0` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-1` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-2` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-3` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-4` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-5` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-6` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `override-default-7` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `real` | 483 | 0 | 483 | 0 | none |
| linux | `none` | `sanity` | 483 | 483 | 0 | 0 | none |

Platform-absent:

- `none`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures

## Notes

- linux/tree-hugger/none: override override-ci-3 was rewritten ('test(=test_detect_completes_in_reasonable_time)' → 'test(=integration::test_detect_completes_in_reasonable_time)')
