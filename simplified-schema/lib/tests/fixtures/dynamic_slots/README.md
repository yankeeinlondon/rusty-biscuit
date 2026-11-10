# Dynamic slot fixtures

Schema templates holding `{{ … }}` slots: complete elements of `enum(...)` /
`suggest(...)` lists, and whole arguments of the template constraints
`literal(...)`, `default(...)`, `min(...)`, `max(...)`, and `pattern(...)`.

| File | Shape | Resolved control value |
| --- | --- | --- |
| `schema-with-slot.yaml` | `enum` slot after a literal the data repeats | `os`: `macOS, Linux, Windows` (first occurrence kept) |
| `suggest-slot.yaml` | `suggest` slot between two literals | `FreeBSD, macOS, Linux, Windows, Plan9` |
| `imported-types.yaml` | slot reached through `Name@file` | `platform`: `FreeBSD, Haiku` (from `nested/os.yaml`, the defining file's folder) |
| `template-constraints.yaml` | one slot in each template constraint | `revision`: `literal(5)`; `retries`: `min(1)`, `max(10)`, `default(3)`; `slug`: `pattern(^[a-z][a-z0-9-]*$)` |
| `os.yaml`, `nested/os.yaml`, `values/*.yaml` | slot data | — |
| `template-types/properties.yaml` | a template type (a slot standing for a property's whole type) at each allowed position: property type, each suffix (`[]`, `(required)`, `[](required)`, `(required)[](min(1))`, `-> description`), `\|` and YAML-list union arms, grouped union, tuple elements, inline and nested object properties, and a named type body used bare and as `State@this`; each slot has its own expression | — (passive tests only; nothing resolves it) |
| `template-types/status-type.yaml` | template-type data: a `type-definition` value in list form | — |

A template type at the root, in a root-union arm, a key, a pattern key, an
import path, part of a type name, a tuple spread, a `function(...)`
signature, or a `property(...)` declaration is refused at parse time; those
inputs are written inline in `tests/l1/template_types_passive.rs`.

Each robustness-matrix cell copies one fixture and makes exactly one edit.
