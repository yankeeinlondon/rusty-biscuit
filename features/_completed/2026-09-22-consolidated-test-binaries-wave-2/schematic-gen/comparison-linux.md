# Identity comparison: identical

## `schematic-gen`

Migration manifest applied: yes

| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |
|---|---|---|---:|---:|---:|---:|---|
| linux | `none` | `L1` | 563 | 558 | 2 | 3 | none |
| linux | `none` | `L2` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `L3` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `browser` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-0` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-1` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-2` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-3` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-4` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-5` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-6` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-7` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-ci-8` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-0` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-1` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-2` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-3` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-4` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-5` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-6` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `override-default-7` | 563 | 0 | 560 | 3 | none |
| linux | `none` | `real` | 563 | 2 | 558 | 3 | none |
| linux | `none` | `sanity` | 563 | 558 | 2 | 3 | none |
| linux | `terminal-tests` | `L1` | 566 | 558 | 5 | 3 | none |
| linux | `terminal-tests` | `L2` | 566 | 3 | 560 | 3 | none |
| linux | `terminal-tests` | `L3` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `browser` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-0` | 566 | 3 | 560 | 3 | none |
| linux | `terminal-tests` | `override-ci-1` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-2` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-3` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-4` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-5` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-6` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-7` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-ci-8` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-0` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-1` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-2` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-3` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-4` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-5` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-6` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `override-default-7` | 566 | 0 | 563 | 3 | none |
| linux | `terminal-tests` | `real` | 566 | 2 | 561 | 3 | none |
| linux | `terminal-tests` | `sanity` | 566 | 558 | 5 | 3 | none |

Platform-absent:

- `none`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures
- `terminal-tests`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures

## Notes

- linux/schematic-gen/none: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/schematic-gen/none: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/schematic-gen/terminal-tests: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/schematic-gen/terminal-tests: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
