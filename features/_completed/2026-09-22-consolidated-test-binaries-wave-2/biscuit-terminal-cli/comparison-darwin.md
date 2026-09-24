# Identity comparison: identical

## `biscuit-terminal-cli`

Migration manifest applied: yes

| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |
|---|---|---|---:|---:|---:|---:|---|
| darwin | `none` | `L1` | 317 | 317 | 0 | 0 | none |
| darwin | `none` | `L2` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `L3` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `browser` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-0` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-1` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-2` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-3` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-4` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-5` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-6` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-7` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-ci-8` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-0` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-1` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-2` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-3` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-4` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-5` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-6` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `override-default-7` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `real` | 317 | 0 | 317 | 0 | none |
| darwin | `none` | `sanity` | 317 | 317 | 0 | 0 | none |
| darwin | `terminal-tests` | `L1` | 452 | 376 | 76 | 0 | none |
| darwin | `terminal-tests` | `L2` | 452 | 76 | 376 | 0 | none |
| darwin | `terminal-tests` | `L3` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `browser` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-0` | 452 | 76 | 376 | 0 | none |
| darwin | `terminal-tests` | `override-ci-1` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-2` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-3` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-4` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-5` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-6` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-7` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-ci-8` | 452 | 1 | 451 | 0 | none |
| darwin | `terminal-tests` | `override-default-0` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-1` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-2` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-3` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-4` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-5` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `override-default-6` | 452 | 1 | 451 | 0 | none |
| darwin | `terminal-tests` | `override-default-7` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `real` | 452 | 0 | 452 | 0 | none |
| darwin | `terminal-tests` | `sanity` | 452 | 376 | 76 | 0 | none |

Platform-absent:

- `none`: not-evaluated (hosts: darwin) — captures from one host only; platform-absent needs the other hosts' captures
- `terminal-tests`: not-evaluated (hosts: darwin) — captures from one host only; platform-absent needs the other hosts' captures

## Notes

- darwin/biscuit-terminal-cli/none: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- darwin/biscuit-terminal-cli/none: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- darwin/biscuit-terminal-cli/terminal-tests: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- darwin/biscuit-terminal-cli/terminal-tests: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
