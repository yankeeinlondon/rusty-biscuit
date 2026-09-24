# Identity comparison: identical

## `biscuit-file`

Migration manifest applied: yes

| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |
|---|---|---|---:|---:|---:|---:|---|
| linux | `fetch` | `L1` | 777 | 777 | 0 | 0 | none |
| linux | `fetch` | `L2` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `L3` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `browser` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-0` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-1` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-2` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-3` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-4` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-5` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-6` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-7` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-ci-8` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-0` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-1` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-2` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-3` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-4` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-5` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-6` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `override-default-7` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `real` | 777 | 0 | 777 | 0 | none |
| linux | `fetch` | `sanity` | 777 | 777 | 0 | 0 | none |
| linux | `none` | `L1` | 752 | 752 | 0 | 0 | none |
| linux | `none` | `L2` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `L3` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `browser` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-0` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-1` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-2` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-3` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-4` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-5` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-6` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-7` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-ci-8` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-0` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-1` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-2` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-3` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-4` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-5` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-6` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `override-default-7` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `real` | 752 | 0 | 752 | 0 | none |
| linux | `none` | `sanity` | 752 | 752 | 0 | 0 | none |

Platform-absent:

- `fetch`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures
- `none`: not-evaluated (hosts: linux) — captures from one host only; platform-absent needs the other hosts' captures

## Notes

- linux/biscuit-file/fetch: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/biscuit-file/fetch: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/biscuit-file/none: override override-ci-8 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
- linux/biscuit-file/none: override override-default-6 was rewritten ('test(=level2_render_tree_style_in_wezterm)' → 'test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)')
