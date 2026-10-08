# Release Notes

## Automatic package sections in stats

`hug functions|types|symbols|classes --stats` now prints overall totals followed
by package sections. Counts and documentation coverage keep their meanings.

Before:

```text
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 3 documented (33.3%); 0 unavailable
```

After (tables abbreviated):

```text
All selected declarations
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 3 documented (33.3%); 0 unavailable

utils (crates/utils)
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 2 documented (50.0%); 0 unavailable

Unassigned
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 0 of 1 documented (0.0%); 0 unavailable
```

JSON retains overall `stats` and report version 1, adding `package_root_dir`
and `groups`. Group paths resolve against the discovery root, which can differ
from the scan root. See [Symbol Reports](./symbol-reports.md#--stats).

The intentional snapshot changes are `symbols.stats` (overall heading and
Unassigned section) and `symbols-stats.json` (discovery root and one null-package
group with the same stats as the overall totals). Both cover the existing
16-language fixture corpus; no declaration counts changed.
