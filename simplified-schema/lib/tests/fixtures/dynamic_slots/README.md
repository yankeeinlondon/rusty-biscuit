# Dynamic slot fixtures

Schema templates holding `{{ … }}` slots: complete elements of `enum(...)` /
`suggest(...)` lists, and whole arguments of the value positions
`literal(...)`, `default(...)`, `min(...)`, `max(...)`, and `pattern(...)`.

| File | Shape | Resolved control value |
| --- | --- | --- |
| `schema-with-slot.yaml` | `enum` slot after a literal the data repeats | `os`: `macOS, Linux, Windows` (first occurrence kept) |
| `suggest-slot.yaml` | `suggest` slot between two literals | `FreeBSD, macOS, Linux, Windows, Plan9` |
| `imported-types.yaml` | slot reached through `Name@file` | `platform`: `FreeBSD, Haiku` (from `nested/os.yaml`, the defining file's folder) |
| `value-positions.yaml` | one slot in each value position | `revision`: `literal(5)`; `retries`: `min(1)`, `max(10)`, `default(3)`; `slug`: `pattern(^[a-z][a-z0-9-]*$)` |
| `os.yaml`, `nested/os.yaml`, `values/*.yaml` | slot data | — |

Each robustness-matrix cell copies one fixture and makes exactly one edit.
