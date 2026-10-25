# Spike S2: Schemars 1.2.1 output fixture

Scratch crate: `/tmp/tc-spike-s2/` (`src/main.rs` = Fixture, `src/bin/edge.rs` = edge cases,
`src/bin/ints.rs` = implicit primitive bounds). Raw outputs: `out-openapi3.json`,
`out-draft.json`, `out-tree.json`, `out-x.json`, `out-edge.txt`.

## 1. Settings the repo uses

| Site | Call | Effect |
|---|---|---|
| `schematic/define/src/registration.rs:231` | `SchemaSettings::openapi3().into_generator().into_root_schema_for::<T>()` | **The** component schema for every registered type. |
| `schematic/define/src/models/type_ref.rs:126` | same, for primitives (`inline::<T>()`), then strips `$schema`/`title` | Primitive wire schemas. |
| `schematic/define/src/models/type_ref.rs:206-208` | `{"anyOf":[inner, {"type":"null"}]}` then every `openapi3().transforms` | `TypeRef::Optional` wire schema. |
| `schematic/define/src/components.rs:134-135` (`wire_shape`) | re-runs `openapi3().transforms` over **nested** definitions | Only for agreement comparison of nested components. |
| `schematic/define/src/builders.rs:207` | `schemars::schema_for!(T)` (default = draft 2020-12) | Only reads top-level `type == "object"`. |
| `schematic/define/src/openapi/export/conversion.rs:1-12` | converts stored components to OpenAPI 3.0.3 | Handles both dialects (no separate generation). |

`openapi3()` (schemars `src/generate.rs:121`): `definitions_path = "/components/schemas"`,
`meta_schema = https://spec.openapis.org/oas/3.0/schema/2024-10-18#/definitions/Schema`,
transforms `ReplaceUnevaluatedProperties, ReplaceBoolSchemas{skip_additional_properties},
AddNullable{remove_null_type, add_const_null}, RemoveRefSiblings, SetSingleExample,
ReplaceConstValue, ReplacePrefixItems`, `contract = Deserialize`, `inline_subschemas = false`.

**Key fact:** `into_root_schema_for` appends definitions under `components.schemas` and then
applies transforms; transforms recurse only through schema keywords, so nested definitions stay
**draft 2020-12 shaped** (`type: [T,"null"]`, `{"type":"null"}`, `const`) while the root is
OpenAPI 3.0 shaped (`nullable: true`, `{"enum":[null],"nullable":true}`, `enum:[c]`). Verified
(see Tree.parent in §4, Inner in §4b). No separate OpenAPI 3.0 generation exists; the
exporter's `conversion.rs` accepts both dialects. The "alternate setting" output
(`draft2020_12()`) differs from openapi3 only in: `$defs` path, `type` arrays instead of
`nullable`, `{"type":"null"}` null branch, and `const` kept.

## 2. Keyword × attribute expressibility

| Keyword | How | Exact syntax |
|---|---|---|
| minLength / maxLength | native | `#[schemars(length(min = 1, max = 100))]`, `length(equal = 3)` (sets both) |
| minItems / maxItems | native (same attr; applied when `type` has `array`) | `#[schemars(length(min = 1, max = 100))]` on `Vec<T>`/sets |
| pattern | native | `#[schemars(regex(pattern = r"^[a-z]+$"))]` **or** `#[schemars(pattern(r"^..$"))]` (mutually exclusive); `contains("x")` emits an escaped `pattern` |
| format | native for `email`, `url` (→ `"uri"`), `ip`; else extend | `#[schemars(email)]`, `#[schemars(url)]`, `#[schemars(ip)]`; `extend("format" = "uuid")` |
| minimum / maximum | native | `#[schemars(range(min = 1, max = 100))]` (ints and floats; any const expr e.g. `1u64 << 40`; `range(equal = n)`) |
| exclusiveMinimum / exclusiveMaximum | extend only | `extend("exclusiveMinimum" = 0, "exclusiveMaximum" = 10)` (emitted as 2020-12 numeric form) |
| multipleOf | extend only | `extend("multipleOf" = 0.1)` |
| uniqueItems | implicit for `HashSet`/`BTreeSet`; else extend | `extend("uniqueItems" = true)` |
| enum | extend only (or a Rust unit enum → `$ref` to `{type:string, enum:[..]}`) | `extend("enum" = ["a","b"])` |
| const | extend only | `extend("const" = "fixed")` — root becomes `enum:["fixed"]` (ReplaceConstValue); nested def keeps `const` |
| items-level (array element) | native `inner(...)` for length/range/regex/pattern/format | `#[schemars(inner(length(max = 5)))]`, `inner(range(min=-1,max=1))`, `inner(regex(pattern = r"^x"))`; works on `Vec<T>`, sets, `Option<Vec<T>>`, `Vec<Option<T>>` |
| additionalProperties-level (map value) | **not expressible natively** — `inner` only targets `items`, silently dropped on `HashMap` | `extend("additionalProperties" = {"type":"integer","minimum":0})` (replaces the generated value schema) |
| minProperties etc. on a struct | extend on the container | `#[schemars(extend("minProperties" = 1))]` |
| rule on a newtype | native on the tuple field | `struct Slug(#[schemars(length(min=1,max=8), regex(pattern=..))] String)` → component `Slug` carries them |

Compatibility attributes: `#[validate(...)]` (validator) and `#[garde(...)]` are registered
helper attributes and are read: `#[validate(length(min = 1))]` → `minLength: 1`;
`#[garde(length(min = 2))]` → `minLength: 2`. `inner` and `pattern` are not accepted in
`validate`; `regex` is not accepted in `garde`; `validate(regex(path = ...))` uses `path`.

**Type gating (silent drop):** every native rule is inserted only if the field schema's `type`
contains the required kind (`_private::insert_validation_property`: string for length/pattern,
array for length→Items, number|integer for range). Therefore silently dropped, no compile error:
`length` on `Option<Struct>` (anyOf/$ref has no `type`), `range` on `String`, `length` on `i32`,
`range` on `Vec<Option<i32>>`, `inner` on `HashMap` or `Option<String>`. `extend` is never gated
and is applied after native rules, so it overrides them (`range(min=1)` + `extend("minimum"=2)` → `2`).

## 3. Nullable-shape catalogue (literal output)

Option fields are omitted from `required` (and `#[serde(default)]` adds `"default": null`).

| Rust | openapi3 root (repo setting) | nested def under `components.schemas` / draft2020_12 |
|---|---|---|
| `#[schemars(length(min=1,max=10))] Option<String>` | `{"type":"string","maxLength":10,"minLength":1,"nullable":true}` | `{"type":["string","null"],"maxLength":10,"minLength":1}` |
| `#[schemars(range(min=1,max=10))] Option<i32>` | `{"type":"integer","format":"int32","maximum":10,"minimum":1,"nullable":true}` | `{"type":["integer","null"],"format":"int32","maximum":10,"minimum":1}` |
| `#[schemars(length(max=4), inner(length(min=2)))] Option<Vec<String>>` | `{"type":"array","items":{"type":"string","minLength":2},"maxItems":4,"nullable":true}` | `{"type":["array","null"],"items":{"type":"string","minLength":2},"maxItems":4}` |
| `Option<Nested>` | `{"anyOf":[{"$ref":"#/components/schemas/Nested"},{"enum":[null],"nullable":true}]}` | `{"anyOf":[{"$ref":"#/$defs/Nested"},{"type":"null"}]}` (nested def in openapi3: `{"$ref":"#/components/schemas/Tree"},{"type":"null"}`) |
| `#[schemars(length(min=1))] Option<Nested>` | identical to `Option<Nested>` — **rule dropped** | same — dropped |
| `#[schemars(extend("description"="x"))] Option<Nested>` | `{"description":"x","anyOf":[{"$ref":"…/Nested"},{"enum":[null],"nullable":true}]}` | `{"description":"x","anyOf":[{"$ref":"#/$defs/Nested"},{"type":"null"}]}` |
| `Option<Color>` (unit enum) / `Option<StrOrNum>` (untagged) / `Option<Slug>` (newtype) | `{"anyOf":[{"$ref":"#/components/schemas/<T>"},{"enum":[null],"nullable":true}]}` | `{"anyOf":[{"$ref":"#/$defs/<T>"},{"type":"null"}]}` |
| `Option<Box<Tree>>` (recursive root) | `{"anyOf":[{"$ref":"#"},{"enum":[null],"nullable":true}]}` | `{"anyOf":[{"$ref":"#/components/schemas/Tree"},{"type":"null"}]}` |
| `Vec<Option<i32>>` + `inner(range(min=1))` | `{"type":"array","items":{"type":"integer","format":"int32","minimum":1,"nullable":true}}` | `items: {"type":["integer","null"],"format":"int32","minimum":1}` |
| `#[serde(default)] Option<String>` | `{"type":"string","default":null,"nullable":true}` | `{"type":["string","null"],"default":null}` |

Where constraints land: for inline types (`String`, ints, `Vec`) constraints are **siblings of
`type`** on the same object as `nullable`/the `"null"` type entry — the null alternative carries
no branch of its own. For `$ref` targets the field is a 2-member `anyOf` with the referent's
constraints living in the referenced component; field-level native rules cannot attach (dropped);
`extend` keys land as siblings of `anyOf` (outside both branches).

Non-nullable composition for contrast (untagged enums, stored as components):
```json
"StrOrNum":     {"anyOf":[{"type":"string"},{"type":"integer","format":"int64"}]}
"NestedOrNull": {"anyOf":[{"$ref":"#/components/schemas/Nested"},{"$ref":"#/components/schemas/Extra"}]}
```
**Discriminator for traversal:** an `anyOf` is "Option" iff it has exactly 2 members and one
member admits only null — `{"type":"null"}` (2020-12 / nested defs) or `{"enum":[null],"nullable":true}`
(openapi3 root; the exporter's own null branch is `{"type":"object","nullable":true,"enum":[null]}`,
`conversion.rs:51`). The existing `admits_only_null` (`conversion.rs:312`) is the right predicate.
Never treat a bare `anyOf`/`oneOf` as nullable; never treat `nullable` alone as an anyOf marker.
Inline nullability is `nullable: true` OR `"null"` inside a `type` array — both dialects can
appear in the same stored component.

## 5. Surprises

1. **Silent drops**: native rules on a mismatched/`$ref` type vanish with no diagnostic (§2). A
   rule on `Option<Struct>` or a map value needs `extend` or a newtype.
2. **Large integers via extend**: an unsuffixed literal is typed `i32` → `9223372036854775808`
   is a compile error (`overflowing_literals`). `9223372036854775808u64` and
   `-9223372036854775808i64` emit exactly. `18446744073709551616u128` compiles but **panics at
   schema-generation time** (`serde_json` "number out of range"). A float literal emits
   `1.8446744073709552e+19` (precision lost). `range(min = expr)` accepts any const expr.
3. **extend is unchecked**: `extend("minimum" = "1.5")` emits `"minimum":"1.5"` (string — invalid
   JSON Schema, but a usable decimal-text carrier only via an `x-` key such as
   `extend("x-decimal-max" = "9223372036854775808")`, which survives verbatim).
4. **`const` rewritten** to `enum:[v]` at the root only (ReplaceConstValue); nested defs keep `const`.
5. **Implicit primitive bounds** (not user constraints): `i8/i16/u8/u16` emit min+max
   (e.g. u8 `0..255`), `u32/u64/usize` emit `minimum: 0`, `NonZeroU32` `minimum: 1`,
   `char` `minLength=maxLength=1`, u128 `format: "uint128"`. A user `range(min=1)` on u8 keeps the
   implicit `maximum: 255`. Traversal must not report these as authored constraints.
6. Exclusive bounds come out as 2020-12 numeric form (`"exclusiveMinimum": 0`), which OpenAPI 3.0
   expresses as `minimum` + `exclusiveMinimum: true` — the exporter must convert (conversion.rs
   currently copies `exclusiveMinimum` unchanged).
7. `#[serde(flatten)]` hoists the flattened struct's properties *and their constraints* into the
   parent (`flat_count: {minimum: 0}`, listed in parent `required`); it does not by itself create a component (`Extra` appears here only because `NestedOrNull` references it).
8. `#[serde(rename = "displayName")]` keys the property by wire name; constraints follow it.
9. Recursive root uses `"$ref": "#"`; the same type nested elsewhere uses `#/components/schemas/Tree`.

## 4. Raw JSON — Fixture under repo settings (`SchemaSettings::openapi3()`)

Source struct: `/tmp/tc-spike-s2/src/main.rs`.

```json
{
  "$schema": "https://spec.openapis.org/oas/3.0/schema/2024-10-18#/definitions/Schema",
  "title": "Fixture",
  "type": "object",
  "properties": {
    "big_float": {
      "type": "number",
      "format": "double",
      "maximum": 1e+30
    },
    "big_int": {
      "type": "integer",
      "format": "uint64",
      "maximum": 9223372036854775808,
      "minimum": 0
    },
    "btree_set": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 3
      },
      "minItems": 1,
      "uniqueItems": true
    },
    "code": {
      "type": "string",
      "maxLength": 3,
      "minLength": 3
    },
    "color": {
      "$ref": "#/components/schemas/Color"
    },
    "constant": {
      "type": "string",
      "enum": [
        "fixed"
      ]
    },
    "contains_value": {
      "type": "string",
      "pattern": "needle"
    },
    "count": {
      "type": "integer",
      "format": "int32",
      "maximum": 100,
      "minimum": 1
    },
    "decimal_string": {
      "type": "integer",
      "format": "uint64",
      "minimum": 0,
      "x-decimal-max": "9223372036854775808"
    },
    "defaulted": {
      "type": "string",
      "default": null,
      "nullable": true
    },
    "displayName": {
      "type": "string",
      "minLength": 2
    },
    "email": {
      "type": "string",
      "format": "email"
    },
    "enum_extend": {
      "type": "string",
      "enum": [
        "a",
        "b"
      ]
    },
    "exclusive": {
      "type": "integer",
      "format": "int32",
      "exclusiveMaximum": 10,
      "exclusiveMinimum": 0
    },
    "flat_count": {
      "type": "integer",
      "format": "uint32",
      "minimum": 0
    },
    "garde_attr": {
      "type": "string",
      "minLength": 2
    },
    "hash_set": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "uniqueItems": true
    },
    "homepage": {
      "type": "string",
      "format": "uri"
    },
    "id": {
      "type": "string",
      "format": "uuid"
    },
    "inner_ints": {
      "type": "array",
      "items": {
        "type": "integer",
        "format": "int32",
        "maximum": 1,
        "minimum": -1
      }
    },
    "inner_regex": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^x"
      }
    },
    "inner_strings": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 5
      }
    },
    "ip": {
      "type": "string",
      "format": "ip"
    },
    "map_extend": {
      "type": "object",
      "additionalProperties": {
        "type": "integer",
        "minimum": 0
      }
    },
    "map_inner": {
      "type": "object",
      "additionalProperties": {
        "type": "integer",
        "format": "int32"
      }
    },
    "multiple": {
      "type": "number",
      "format": "double",
      "multipleOf": 0.1
    },
    "name": {
      "type": "string",
      "maxLength": 100,
      "minLength": 1
    },
    "opt_color": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Color"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "opt_inner": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 2
      },
      "maxItems": 4,
      "nullable": true
    },
    "opt_int": {
      "type": "integer",
      "format": "int32",
      "maximum": 10,
      "minimum": 1,
      "nullable": true
    },
    "opt_nested": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Nested"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "opt_nested_extend": {
      "description": "x",
      "anyOf": [
        {
          "$ref": "#/components/schemas/Nested"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "opt_nested_with_rule": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Nested"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "opt_string": {
      "type": "string",
      "maxLength": 10,
      "minLength": 1,
      "nullable": true
    },
    "opt_string_extend": {
      "type": "string",
      "minLength": 1,
      "nullable": true
    },
    "opt_untagged": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/StrOrNum"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "ratio": {
      "type": "number",
      "format": "double",
      "maximum": 99.5,
      "minimum": 0.5
    },
    "slug_pattern": {
      "type": "string",
      "pattern": "^[A-Z]+$"
    },
    "slug_regex": {
      "type": "string",
      "pattern": "^[a-z]+$"
    },
    "tags": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 100,
      "minItems": 1
    },
    "tree": {
      "$ref": "#/components/schemas/Tree"
    },
    "unique_vec": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "uniqueItems": true
    },
    "unsigned_with_min": {
      "type": "integer",
      "format": "uint8",
      "maximum": 255,
      "minimum": 1
    },
    "untagged": {
      "$ref": "#/components/schemas/StrOrNum"
    },
    "untagged_structs": {
      "$ref": "#/components/schemas/NestedOrNull"
    },
    "validate_attr": {
      "type": "string",
      "minLength": 1
    }
  },
  "components": {
    "schemas": {
      "Color": {
        "type": "string",
        "enum": [
          "Red",
          "Green"
        ]
      },
      "Extra": {
        "type": "object",
        "properties": {
          "flat_count": {
            "type": "integer",
            "format": "uint32",
            "minimum": 0
          }
        },
        "required": [
          "flat_count"
        ]
      },
      "Nested": {
        "type": "object",
        "properties": {
          "label": {
            "type": "string",
            "minLength": 1
          }
        },
        "required": [
          "label"
        ]
      },
      "NestedOrNull": {
        "anyOf": [
          {
            "$ref": "#/components/schemas/Nested"
          },
          {
            "$ref": "#/components/schemas/Extra"
          }
        ]
      },
      "StrOrNum": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "integer",
            "format": "int64"
          }
        ]
      },
      "Tree": {
        "type": "object",
        "properties": {
          "children": {
            "type": "array",
            "items": {
              "$ref": "#/components/schemas/Tree"
            }
          },
          "parent": {
            "anyOf": [
              {
                "$ref": "#/components/schemas/Tree"
              },
              {
                "type": "null"
              }
            ]
          },
          "value": {
            "type": "integer",
            "format": "int32"
          }
        },
        "required": [
          "value",
          "children"
        ]
      }
    }
  },
  "required": [
    "name",
    "tags",
    "code",
    "count",
    "ratio",
    "unsigned_with_min",
    "slug_regex",
    "slug_pattern",
    "contains_value",
    "inner_strings",
    "inner_ints",
    "inner_regex",
    "map_inner",
    "map_extend",
    "multiple",
    "exclusive",
    "unique_vec",
    "constant",
    "enum_extend",
    "id",
    "big_int",
    "big_float",
    "decimal_string",
    "email",
    "homepage",
    "ip",
    "validate_attr",
    "garde_attr",
    "displayName",
    "flat_count",
    "tree",
    "color",
    "untagged",
    "untagged_structs",
    "hash_set",
    "btree_set"
  ]
}
```

### 4a. draft2020_12 (alternate) — full
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Fixture",
  "type": "object",
  "properties": {
    "big_float": {
      "type": "number",
      "format": "double",
      "maximum": 1e+30
    },
    "big_int": {
      "type": "integer",
      "format": "uint64",
      "maximum": 9223372036854775808,
      "minimum": 0
    },
    "btree_set": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 3
      },
      "minItems": 1,
      "uniqueItems": true
    },
    "code": {
      "type": "string",
      "maxLength": 3,
      "minLength": 3
    },
    "color": {
      "$ref": "#/$defs/Color"
    },
    "constant": {
      "type": "string",
      "const": "fixed"
    },
    "contains_value": {
      "type": "string",
      "pattern": "needle"
    },
    "count": {
      "type": "integer",
      "format": "int32",
      "maximum": 100,
      "minimum": 1
    },
    "decimal_string": {
      "type": "integer",
      "format": "uint64",
      "minimum": 0,
      "x-decimal-max": "9223372036854775808"
    },
    "defaulted": {
      "type": [
        "string",
        "null"
      ],
      "default": null
    },
    "displayName": {
      "type": "string",
      "minLength": 2
    },
    "email": {
      "type": "string",
      "format": "email"
    },
    "enum_extend": {
      "type": "string",
      "enum": [
        "a",
        "b"
      ]
    },
    "exclusive": {
      "type": "integer",
      "format": "int32",
      "exclusiveMaximum": 10,
      "exclusiveMinimum": 0
    },
    "flat_count": {
      "type": "integer",
      "format": "uint32",
      "minimum": 0
    },
    "garde_attr": {
      "type": "string",
      "minLength": 2
    },
    "hash_set": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "uniqueItems": true
    },
    "homepage": {
      "type": "string",
      "format": "uri"
    },
    "id": {
      "type": "string",
      "format": "uuid"
    },
    "inner_ints": {
      "type": "array",
      "items": {
        "type": "integer",
        "format": "int32",
        "maximum": 1,
        "minimum": -1
      }
    },
    "inner_regex": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^x"
      }
    },
    "inner_strings": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 5
      }
    },
    "ip": {
      "type": "string",
      "format": "ip"
    },
    "map_extend": {
      "type": "object",
      "additionalProperties": {
        "type": "integer",
        "minimum": 0
      }
    },
    "map_inner": {
      "type": "object",
      "additionalProperties": {
        "type": "integer",
        "format": "int32"
      }
    },
    "multiple": {
      "type": "number",
      "format": "double",
      "multipleOf": 0.1
    },
    "name": {
      "type": "string",
      "maxLength": 100,
      "minLength": 1
    },
    "opt_color": {
      "anyOf": [
        {
          "$ref": "#/$defs/Color"
        },
        {
          "type": "null"
        }
      ]
    },
    "opt_inner": {
      "type": [
        "array",
        "null"
      ],
      "items": {
        "type": "string",
        "minLength": 2
      },
      "maxItems": 4
    },
    "opt_int": {
      "type": [
        "integer",
        "null"
      ],
      "format": "int32",
      "maximum": 10,
      "minimum": 1
    },
    "opt_nested": {
      "anyOf": [
        {
          "$ref": "#/$defs/Nested"
        },
        {
          "type": "null"
        }
      ]
    },
    "opt_nested_extend": {
      "description": "x",
      "anyOf": [
        {
          "$ref": "#/$defs/Nested"
        },
        {
          "type": "null"
        }
      ]
    },
    "opt_nested_with_rule": {
      "anyOf": [
        {
          "$ref": "#/$defs/Nested"
        },
        {
          "type": "null"
        }
      ]
    },
    "opt_string": {
      "type": [
        "string",
        "null"
      ],
      "maxLength": 10,
      "minLength": 1
    },
    "opt_string_extend": {
      "type": [
        "string",
        "null"
      ],
      "minLength": 1
    },
    "opt_untagged": {
      "anyOf": [
        {
          "$ref": "#/$defs/StrOrNum"
        },
        {
          "type": "null"
        }
      ]
    },
    "ratio": {
      "type": "number",
      "format": "double",
      "maximum": 99.5,
      "minimum": 0.5
    },
    "slug_pattern": {
      "type": "string",
      "pattern": "^[A-Z]+$"
    },
    "slug_regex": {
      "type": "string",
      "pattern": "^[a-z]+$"
    },
    "tags": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 100,
      "minItems": 1
    },
    "tree": {
      "$ref": "#/$defs/Tree"
    },
    "unique_vec": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "uniqueItems": true
    },
    "unsigned_with_min": {
      "type": "integer",
      "format": "uint8",
      "maximum": 255,
      "minimum": 1
    },
    "untagged": {
      "$ref": "#/$defs/StrOrNum"
    },
    "untagged_structs": {
      "$ref": "#/$defs/NestedOrNull"
    },
    "validate_attr": {
      "type": "string",
      "minLength": 1
    }
  },
  "required": [
    "name",
    "tags",
    "code",
    "count",
    "ratio",
    "unsigned_with_min",
    "slug_regex",
    "slug_pattern",
    "contains_value",
    "inner_strings",
    "inner_ints",
    "inner_regex",
    "map_inner",
    "map_extend",
    "multiple",
    "exclusive",
    "unique_vec",
    "constant",
    "enum_extend",
    "id",
    "big_int",
    "big_float",
    "decimal_string",
    "email",
    "homepage",
    "ip",
    "validate_attr",
    "garde_attr",
    "displayName",
    "flat_count",
    "tree",
    "color",
    "untagged",
    "untagged_structs",
    "hash_set",
    "btree_set"
  ],
  "$defs": {
    "Color": {
      "type": "string",
      "enum": [
        "Red",
        "Green"
      ]
    },
    "Extra": {
      "type": "object",
      "properties": {
        "flat_count": {
          "type": "integer",
          "format": "uint32",
          "minimum": 0
        }
      },
      "required": [
        "flat_count"
      ]
    },
    "Nested": {
      "type": "object",
      "properties": {
        "label": {
          "type": "string",
          "minLength": 1
        }
      },
      "required": [
        "label"
      ]
    },
    "NestedOrNull": {
      "anyOf": [
        {
          "$ref": "#/$defs/Nested"
        },
        {
          "$ref": "#/$defs/Extra"
        }
      ]
    },
    "StrOrNum": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "integer",
          "format": "int64"
        }
      ]
    },
    "Tree": {
      "type": "object",
      "properties": {
        "children": {
          "type": "array",
          "items": {
            "$ref": "#/$defs/Tree"
          }
        },
        "parent": {
          "anyOf": [
            {
              "$ref": "#/$defs/Tree"
            },
            {
              "type": "null"
            }
          ]
        },
        "value": {
          "type": "integer",
          "format": "int32"
        }
      },
      "required": [
        "value",
        "children"
      ]
    }
  }
}
```

### 4b. Edge cases (openapi3 then draft2020_12) — source `src/bin/edge.rs`
```
=== openapi3 Edge ===
{
  "$schema": "https://spec.openapis.org/oas/3.0/schema/2024-10-18#/definitions/Schema",
  "title": "Edge",
  "type": "object",
  "properties": {
    "big_negative": {
      "type": "integer",
      "format": "int64",
      "minimum": -9223372036854775808
    },
    "big_unsuffixed": {
      "type": "integer",
      "format": "uint128",
      "maximum": 1.8446744073709552e+19,
      "minimum": 0
    },
    "expr_range": {
      "type": "integer",
      "format": "uint64",
      "minimum": 1099511627776
    },
    "inner": {
      "$ref": "#/components/schemas/Inner"
    },
    "int_with_length": {
      "type": "integer",
      "format": "int32"
    },
    "opt_scalar_inner": {
      "type": "string",
      "nullable": true
    },
    "opt_slug": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Slug"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "override_min": {
      "type": "integer",
      "format": "int32",
      "minimum": 2
    },
    "slug": {
      "$ref": "#/components/schemas/Slug"
    },
    "string_min": {
      "type": "number",
      "format": "double",
      "minimum": "1.5"
    },
    "string_with_range": {
      "type": "string"
    },
    "vec_nested_opt": {
      "type": "array",
      "items": {
        "type": "integer",
        "format": "int32",
        "nullable": true
      }
    },
    "vec_of_opt": {
      "type": "array",
      "items": {
        "type": "integer",
        "format": "int32",
        "minimum": 1,
        "nullable": true
      }
    }
  },
  "components": {
    "schemas": {
      "Inner": {
        "type": "object",
        "properties": {
          "fixed": {
            "type": "integer",
            "format": "int32",
            "const": 5
          },
          "maybe": {
            "type": [
              "string",
              "null"
            ],
            "maxLength": 10,
            "minLength": 1
          },
          "maybe_n": {
            "type": [
              "integer",
              "null"
            ],
            "format": "uint16",
            "maximum": 65535,
            "minimum": 1
          }
        },
        "required": [
          "fixed"
        ]
      },
      "Slug": {
        "type": "string",
        "maxLength": 8,
        "minLength": 1,
        "pattern": "^[a-z]+$"
      }
    }
  },
  "minProperties": 1,
  "required": [
    "inner",
    "slug",
    "override_min",
    "big_unsuffixed",
    "big_negative",
    "string_min",
    "string_with_range",
    "int_with_length",
    "vec_nested_opt",
    "vec_of_opt",
    "expr_range"
  ]
}
=== draft2020_12 Edge ===
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Edge",
  "type": "object",
  "properties": {
    "big_negative": {
      "type": "integer",
      "format": "int64",
      "minimum": -9223372036854775808
    },
    "big_unsuffixed": {
      "type": "integer",
      "format": "uint128",
      "maximum": 1.8446744073709552e+19,
      "minimum": 0
    },
    "expr_range": {
      "type": "integer",
      "format": "uint64",
      "minimum": 1099511627776
    },
    "inner": {
      "$ref": "#/$defs/Inner"
    },
    "int_with_length": {
      "type": "integer",
      "format": "int32"
    },
    "opt_scalar_inner": {
      "type": [
        "string",
        "null"
      ]
    },
    "opt_slug": {
      "anyOf": [
        {
          "$ref": "#/$defs/Slug"
        },
        {
          "type": "null"
        }
      ]
    },
    "override_min": {
      "type": "integer",
      "format": "int32",
      "minimum": 2
    },
    "slug": {
      "$ref": "#/$defs/Slug"
    },
    "string_min": {
      "type": "number",
      "format": "double",
      "minimum": "1.5"
    },
    "string_with_range": {
      "type": "string"
    },
    "vec_nested_opt": {
      "type": "array",
      "items": {
        "type": [
          "integer",
          "null"
        ],
        "format": "int32"
      }
    },
    "vec_of_opt": {
      "type": "array",
      "items": {
        "type": [
          "integer",
          "null"
        ],
        "format": "int32",
        "minimum": 1
      }
    }
  },
  "minProperties": 1,
  "required": [
    "inner",
    "slug",
    "override_min",
    "big_unsuffixed",
    "big_negative",
    "string_min",
    "string_with_range",
    "int_with_length",
    "vec_nested_opt",
    "vec_of_opt",
    "expr_range"
  ],
  "$defs": {
    "Inner": {
      "type": "object",
      "properties": {
        "fixed": {
          "type": "integer",
          "format": "int32",
          "const": 5
        },
        "maybe": {
          "type": [
            "string",
            "null"
          ],
          "maxLength": 10,
          "minLength": 1
        },
        "maybe_n": {
          "type": [
            "integer",
            "null"
          ],
          "format": "uint16",
          "maximum": 65535,
          "minimum": 1
        }
      },
      "required": [
        "fixed"
      ]
    },
    "Slug": {
      "type": "string",
      "maxLength": 8,
      "minLength": 1,
      "pattern": "^[a-z]+$"
    }
  }
}
```

### 4c. Tree as root, and as nested def after wire_shape transforms
```
=== openapi3 Tree ===
{
  "$schema": "https://spec.openapis.org/oas/3.0/schema/2024-10-18#/definitions/Schema",
  "title": "Tree",
  "type": "object",
  "properties": {
    "children": {
      "type": "array",
      "items": {
        "$ref": "#"
      }
    },
    "parent": {
      "anyOf": [
        {
          "$ref": "#"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "value": {
      "type": "integer",
      "format": "int32"
    }
  },
  "required": [
    "value",
    "children"
  ]
}
=== Tree def after wire_shape transforms ===
{
  "type": "object",
  "properties": {
    "children": {
      "type": "array",
      "items": {
        "$ref": "#/components/schemas/Tree"
      }
    },
    "parent": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Tree"
        },
        {
          "enum": [
            null
          ],
          "nullable": true
        }
      ]
    },
    "value": {
      "type": "integer",
      "format": "int32"
    }
  },
  "required": [
    "value",
    "children"
  ]
}
```

## Correction (orchestrator, 2026-10-23)

Surprise 6 ("conversion.rs copies exclusiveMinimum unchanged") is wrong: `define/src/openapi/export/conversion.rs:151-159` already lowers a numeric 2020-12 `exclusiveMinimum`/`exclusiveMaximum` to the bound plus `true`, and returns an unsupported-feature error when the same-side inclusive bound is also present.

## Fixture source (`/tmp/tc-spike-s2/src/main.rs`)

```rust
#![allow(dead_code)]
use std::collections::{BTreeSet, HashMap, HashSet};

use schemars::{JsonSchema, generate::SchemaSettings};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Nested {
    #[schemars(length(min = 1))]
    pub label: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Extra {
    #[schemars(range(min = 0))]
    pub flat_count: u32,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub enum Color { Red, Green }

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StrOrNum { S(String), N(i64) }

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum NestedOrNull { A(Nested), B(Extra) }

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Tree {
    pub value: i32,
    pub children: Vec<Tree>,
    pub parent: Option<Box<Tree>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Fixture {
    #[schemars(length(min = 1, max = 100))]
    pub name: String,
    #[schemars(length(min = 1, max = 100))]
    pub tags: Vec<String>,
    #[schemars(length(equal = 3))]
    pub code: String,
    #[schemars(range(min = 1, max = 100))]
    pub count: i32,
    #[schemars(range(min = 0.5, max = 99.5))]
    pub ratio: f64,
    #[schemars(range(min = 1))]
    pub unsigned_with_min: u8,
    #[schemars(regex(pattern = r"^[a-z]+$"))]
    pub slug_regex: String,
    #[schemars(pattern(r"^[A-Z]+$"))]
    pub slug_pattern: String,
    #[schemars(contains("needle"))]
    pub contains_value: String,
    #[schemars(inner(length(max = 5)))]
    pub inner_strings: Vec<String>,
    #[schemars(inner(range(min = -1, max = 1)))]
    pub inner_ints: Vec<i32>,
    #[schemars(inner(regex(pattern = r"^x")))]
    pub inner_regex: Vec<String>,
    #[schemars(length(max = 4), inner(length(min = 2)))]
    pub opt_inner: Option<Vec<String>>,
    #[schemars(inner(range(max = 10)))]
    pub map_inner: HashMap<String, i32>,
    #[schemars(range(min = 0), extend("additionalProperties" = {"type": "integer", "minimum": 0}))]
    pub map_extend: HashMap<String, i32>,
    #[schemars(extend("multipleOf" = 0.1))]
    pub multiple: f64,
    #[schemars(extend("exclusiveMinimum" = 0, "exclusiveMaximum" = 10))]
    pub exclusive: i32,
    #[schemars(extend("uniqueItems" = true))]
    pub unique_vec: Vec<String>,
    #[schemars(extend("const" = "fixed"))]
    pub constant: String,
    #[schemars(extend("enum" = ["a", "b"]))]
    pub enum_extend: String,
    #[schemars(extend("format" = "uuid"))]
    pub id: String,
    #[schemars(extend("maximum" = 9223372036854775808u64))]
    pub big_int: u64,
    #[schemars(extend("maximum" = 1e30))]
    pub big_float: f64,
    #[schemars(extend("x-decimal-max" = "9223372036854775808"))]
    pub decimal_string: u64,
    #[schemars(email)]
    pub email: String,
    #[schemars(url)]
    pub homepage: String,
    #[schemars(ip)]
    pub ip: String,
    #[validate(length(min = 1), range(max = 5))]
    pub validate_attr: String,
    #[garde(length(min = 2))]
    pub garde_attr: String,
    #[schemars(length(min = 1, max = 10))]
    pub opt_string: Option<String>,
    #[schemars(range(min = 1, max = 10))]
    pub opt_int: Option<i32>,
    pub opt_nested: Option<Nested>,
    #[schemars(length(min = 1))]
    pub opt_nested_with_rule: Option<Nested>,
    #[schemars(extend("minLength" = 1))]
    pub opt_string_extend: Option<String>,
    #[schemars(extend("description" = "x"))]
    pub opt_nested_extend: Option<Nested>,
    #[serde(rename = "displayName")]
    #[schemars(length(min = 2))]
    pub display_name: String,
    #[serde(flatten)]
    pub extra: Extra,
    pub tree: Tree,
    pub color: Color,
    pub opt_color: Option<Color>,
    pub untagged: StrOrNum,
    pub opt_untagged: Option<StrOrNum>,
    pub untagged_structs: NestedOrNull,
    pub hash_set: HashSet<String>,
    #[schemars(length(min = 1), inner(length(max = 3)))]
    pub btree_set: BTreeSet<String>,
    #[serde(default)]
    pub defaulted: Option<String>,
}

fn print<T: JsonSchema>(label: &str, settings: SchemaSettings) {
    let schema = settings.into_generator().into_root_schema_for::<T>();
    println!("=== {label} ===\n{}", serde_json::to_string_pretty(&schema).unwrap());
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    match which.as_str() {
        "openapi3" => print::<Fixture>("openapi3 Fixture", SchemaSettings::openapi3()),
        "draft" => print::<Fixture>("draft2020_12 Fixture", SchemaSettings::draft2020_12()),
        "tree" => print::<Tree>("openapi3 Tree", SchemaSettings::openapi3()),
        _ => {
            // Nested definitions transformed as components.rs::wire_shape does.
            let schema = SchemaSettings::openapi3().into_generator().into_root_schema_for::<Fixture>();
            let tree = schema.get("components").and_then(|c| c.get("schemas")).and_then(|s| s.get("Tree")).cloned().unwrap();
            let mut tree = schemars::Schema::try_from(tree).unwrap();
            for t in &mut SchemaSettings::openapi3().transforms { t.transform(&mut tree); }
            println!("=== Tree def after wire_shape transforms ===\n{}", serde_json::to_string_pretty(&tree).unwrap());
        }
    }
}
```
