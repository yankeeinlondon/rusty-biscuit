# Dynamic slot fixtures

Schema templates whose `enum(...)` / `suggest(...)` lists hold `{{ … }}` slots.

| File | Shape | Resolved control value |
| --- | --- | --- |
| `schema-with-slot.yaml` | `enum` slot after a literal the data repeats | `os`: `macOS, Linux, Windows` (first occurrence kept) |
| `suggest-slot.yaml` | `suggest` slot between two literals | `FreeBSD, macOS, Linux, Windows, Plan9` |
| `imported-types.yaml` | slot reached through `Name@file` | `platform`: `FreeBSD, Haiku` (from `nested/os.yaml`, the defining file's folder) |
| `os.yaml`, `nested/os.yaml` | slot data | — |

Each robustness-matrix cell copies one fixture and makes exactly one edit.
