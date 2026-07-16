---
phase: 1
captured: 2026-07-11
packages:
  - darkmatter
  - dmls
canonical_registrations: 85
overload_descriptors: 88
---

# Phase 1 Baseline: Authored Expression-Function Schemas

This file records the exact post-Godless-Beauty expression-function catalog
state before the authored-schema implementation. The JSON inventory is ordered
by the current `registrations()` iterator; overload arrays preserve descriptor
order.

## Pre-change validation

- `darkmatter/`: `just test` passed. Darkmatter ran 5,443 tests with zero
  failures (111 skipped), Darkmatter CLI ran 552 tests with zero failures (71
  skipped), and DMLS ran 412 tests with 412 passed and zero skipped.
- `darkmatter/`: `just lint` passed for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `claudine/`: `just test` passed. Relevant consumer packages included
  `claudine` (3,378 passed; 7 skipped) and `claudine-cli` (1,900 passed; 152
  skipped). The area recipe also passed `claudine-catalog-types` (21 passed),
  `claudine-contract` (47 selected; 5 skipped), and `claudine-gen` (90 passed;
  1 skipped).
- `claudine/cli/` has no local `justfile`; its inherited area recipe is
  `claudine/justfile`. The CLI baseline above is the `claudine-cli` package run
  performed by that recipe.

## Registration inventory

```json
[
  {
    "aliases": [
      "isstring"
    ],
    "canonical_name": "is_string",
    "catalog_order": 0,
    "category": "Type Predicates",
    "description": "Returns true when the value is a string.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_string(\"hello\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_string(x)"
      }
    ]
  },
  {
    "aliases": [
      "isnumber"
    ],
    "canonical_name": "is_number",
    "catalog_order": 1,
    "category": "Type Predicates",
    "description": "Returns true when the value is a number.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_number(42)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_number(x)"
      }
    ]
  },
  {
    "aliases": [
      "isarray"
    ],
    "canonical_name": "is_array",
    "catalog_order": 2,
    "category": "Type Predicates",
    "description": "Returns true when the value is an array.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_array(items)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_array(x)"
      }
    ]
  },
  {
    "aliases": [
      "isnull"
    ],
    "canonical_name": "is_null",
    "catalog_order": 3,
    "category": "Type Predicates",
    "description": "Returns true when the value is null.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_null(null)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_null(x)"
      }
    ]
  },
  {
    "aliases": [
      "isobject"
    ],
    "canonical_name": "is_object",
    "catalog_order": 4,
    "category": "Type Predicates",
    "description": "Returns true when the value is an object.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_object(obj)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 5,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_object(x)"
      }
    ]
  },
  {
    "aliases": [
      "isempty"
    ],
    "canonical_name": "is_empty",
    "catalog_order": 5,
    "category": "Type Predicates",
    "description": "Returns true when the value is null, empty string, empty array, or empty object.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_empty(\"\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 6,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_empty(x)"
      }
    ]
  },
  {
    "aliases": [
      "ispositive"
    ],
    "canonical_name": "is_positive",
    "catalog_order": 6,
    "category": "Type Predicates",
    "description": "Returns true when the coerced value is greater than zero.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_positive(5)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 7,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "is_positive(val)"
      }
    ]
  },
  {
    "aliases": [
      "isnegative"
    ],
    "canonical_name": "is_negative",
    "catalog_order": 7,
    "category": "Type Predicates",
    "description": "Returns true when the coerced value is less than zero.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_negative(-3)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 8,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "is_negative(val)"
      }
    ]
  },
  {
    "aliases": [
      "isinteger"
    ],
    "canonical_name": "is_integer",
    "catalog_order": 8,
    "category": "Type Predicates",
    "description": "Returns true when the value is a JSON number with no fractional component.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_integer(7)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 9,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_integer(val)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "min",
    "catalog_order": 9,
    "category": "Math",
    "description": "Returns the smaller of two numbers.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "min(2, 5)",
          "reason": null,
          "result": "2",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "min(a, b)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "max",
    "catalog_order": 10,
    "category": "Math",
    "description": "Returns the larger of two numbers.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "max(2, 5)",
          "reason": null,
          "result": "5",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "max(a, b)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "abs",
    "catalog_order": 11,
    "category": "Math",
    "description": "Returns the absolute value of a number.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "abs(-3)",
          "reason": null,
          "result": "3",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "abs(x)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "round",
    "catalog_order": 55,
    "category": "Math",
    "description": "Rounds a value to the nearest integer, with an optional default.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "round(3.7)",
          "reason": null,
          "result": "4",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "number",
            "variadic": false
          },
          {
            "array": false,
            "optional": true,
            "type": "number",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "number"
        },
        "signature": "round(x, [default])"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "first",
    "catalog_order": 12,
    "category": "Collection",
    "description": "Returns the first element of an array, or null when empty.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "first(items)",
          "reason": null,
          "result": "1",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "any"
        },
        "signature": "first(x)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "last",
    "catalog_order": 13,
    "category": "Collection",
    "description": "Returns the last element of an array, or null when empty.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "last(items)",
          "reason": null,
          "result": "3",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "any"
        },
        "signature": "last(x)"
      }
    ]
  },
  {
    "aliases": [
      "haskey"
    ],
    "canonical_name": "has_key",
    "catalog_order": 51,
    "category": "Collection",
    "description": "Returns true when the object contains the given key.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "has_key(obj, \"a\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "object",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "has_key(obj, key)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "contains",
    "catalog_order": 52,
    "category": "Collection",
    "description": "Returns true when haystack contains needle (array, object, or string).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "contains(\"hello\", \"ell\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "contains(haystack, needle)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "length",
    "catalog_order": 53,
    "category": "Collection",
    "description": "Returns the length of a string, array, or object.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "length(\"hello\")",
          "reason": null,
          "result": "5",
          "verification": "Executable"
        },
        "order": 5,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "length(x)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "number",
    "catalog_order": 54,
    "category": "Type Conversion",
    "description": "Converts a value to a number, with an optional default.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "number(\"42\")",
          "reason": null,
          "result": "42",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          },
          {
            "array": false,
            "optional": true,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "number(x, [default])"
      }
    ]
  },
  {
    "aliases": [
      "aslineseparated"
    ],
    "canonical_name": "as_line_separated",
    "catalog_order": 82,
    "category": "List Formatting",
    "description": "Joins a list into a newline-separated string (the default bare-array rendering).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_line_separated(items)",
          "reason": "multi-line output; verified via example file",
          "result": "1\n2\n3",
          "verification": "DisplayOnly"
        },
        "order": 1,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_line_separated(list)"
      }
    ]
  },
  {
    "aliases": [
      "ascsv"
    ],
    "canonical_name": "as_csv",
    "catalog_order": 83,
    "category": "List Formatting",
    "description": "Joins a list into a comma-separated string.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_csv(items)",
          "reason": null,
          "result": "1, 2, 3",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_csv(list)"
      }
    ]
  },
  {
    "aliases": [
      "astsv"
    ],
    "canonical_name": "as_tsv",
    "catalog_order": 84,
    "category": "List Formatting",
    "description": "Joins a list into a tab-separated string.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_tsv(items)",
          "reason": "tab-delimited output; verified via example file",
          "result": "1\t2\t3",
          "verification": "DisplayOnly"
        },
        "order": 3,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_tsv(list)"
      }
    ]
  },
  {
    "aliases": [
      "asspaceseparated"
    ],
    "canonical_name": "as_space_separated",
    "catalog_order": 85,
    "category": "List Formatting",
    "description": "Joins a list into a space-separated string.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_space_separated(items)",
          "reason": null,
          "result": "1 2 3",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_space_separated(list)"
      }
    ]
  },
  {
    "aliases": [
      "asunorderedlist"
    ],
    "canonical_name": "as_unordered_list",
    "catalog_order": 86,
    "category": "List Formatting",
    "description": "Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_unordered_list(items)",
          "reason": "multi-line Markdown list; verified via example file",
          "result": "- 1\n- 2\n- 3",
          "verification": "DisplayOnly"
        },
        "order": 5,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_unordered_list(list)"
      }
    ]
  },
  {
    "aliases": [
      "asorderedlist"
    ],
    "canonical_name": "as_ordered_list",
    "catalog_order": 87,
    "category": "List Formatting",
    "description": "Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "as_ordered_list(items)",
          "reason": "multi-line Markdown list; verified via example file",
          "result": "1. 1\n2. 2\n3. 3",
          "verification": "DisplayOnly"
        },
        "order": 6,
        "parameters": [
          {
            "array": true,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "as_ordered_list(list)"
      }
    ]
  },
  {
    "aliases": [
      "startswith"
    ],
    "canonical_name": "starts_with",
    "catalog_order": 14,
    "category": "String Predicates",
    "description": "Returns true when the string starts with the given prefix (case-sensitive).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "starts_with(\"hello\", \"he\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "starts_with(x, find)"
      }
    ]
  },
  {
    "aliases": [
      "endswith"
    ],
    "canonical_name": "ends_with",
    "catalog_order": 15,
    "category": "String Predicates",
    "description": "Returns true when the string ends with the given suffix (case-sensitive).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "ends_with(\"hello\", \"lo\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "ends_with(x, find)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "lower",
    "catalog_order": 16,
    "category": "String Mutations",
    "description": "Converts a string to lowercase.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "lower(\"HELLO\")",
          "reason": null,
          "result": "hello",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "lower(x)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "upper",
    "catalog_order": 17,
    "category": "String Mutations",
    "description": "Converts a string to uppercase.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "upper(\"hello\")",
          "reason": null,
          "result": "HELLO",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "upper(x)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "capitalize",
    "catalog_order": 18,
    "category": "String Mutations",
    "description": "Capitalizes the first character of a string.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "capitalize(\"hello\")",
          "reason": null,
          "result": "Hello",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "capitalize(x)"
      }
    ]
  },
  {
    "aliases": [
      "kebabcase"
    ],
    "canonical_name": "kebab_case",
    "catalog_order": 19,
    "category": "String Mutations",
    "description": "Converts a string to kebab-case.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "kebab_case(\"Hello World\")",
          "reason": null,
          "result": "hello-world",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "kebab_case(x)"
      }
    ]
  },
  {
    "aliases": [
      "snakecase"
    ],
    "canonical_name": "snake_case",
    "catalog_order": 20,
    "category": "String Mutations",
    "description": "Converts a string to snake_case.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "snake_case(\"Hello World\")",
          "reason": null,
          "result": "hello_world",
          "verification": "Executable"
        },
        "order": 5,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "snake_case(x)"
      }
    ]
  },
  {
    "aliases": [
      "camelcase"
    ],
    "canonical_name": "camel_case",
    "catalog_order": 21,
    "category": "String Mutations",
    "description": "Converts a string to camelCase.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "camel_case(\"hello world\")",
          "reason": null,
          "result": "helloWorld",
          "verification": "Executable"
        },
        "order": 6,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "camel_case(x)"
      }
    ]
  },
  {
    "aliases": [
      "pascalcase"
    ],
    "canonical_name": "pascal_case",
    "catalog_order": 22,
    "category": "String Mutations",
    "description": "Converts a string to PascalCase.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "pascal_case(\"hello world\")",
          "reason": null,
          "result": "HelloWorld",
          "verification": "Executable"
        },
        "order": 7,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "pascal_case(x)"
      }
    ]
  },
  {
    "aliases": [
      "titlecase"
    ],
    "canonical_name": "title_case",
    "catalog_order": 23,
    "category": "String Mutations",
    "description": "Converts a string to Title Case.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "title_case(\"hello world\")",
          "reason": null,
          "result": "Hello World",
          "verification": "Executable"
        },
        "order": 8,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "title_case(x)"
      }
    ]
  },
  {
    "aliases": [
      "withoutdate"
    ],
    "canonical_name": "without_date",
    "catalog_order": 24,
    "category": "String Mutations",
    "description": "Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "without_date(\"Note 2024-06-15\")",
          "reason": null,
          "result": "Note ",
          "verification": "Executable"
        },
        "order": 9,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "without_date(string)"
      }
    ]
  },
  {
    "aliases": [
      "ensureleading"
    ],
    "canonical_name": "ensure_leading",
    "catalog_order": 25,
    "category": "String Mutations",
    "description": "Ensures the string form of a value starts with a prefix.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "ensure_leading(\"world\", \"hello \")",
          "reason": null,
          "result": "hello world",
          "verification": "Executable"
        },
        "order": 10,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "ensure_leading(var, prefix)"
      }
    ]
  },
  {
    "aliases": [
      "ensuretrailing"
    ],
    "canonical_name": "ensure_trailing",
    "catalog_order": 26,
    "category": "String Mutations",
    "description": "Ensures the string form of a value ends with a postfix.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "ensure_trailing(\"hello\", \" world\")",
          "reason": null,
          "result": "hello world",
          "verification": "Executable"
        },
        "order": 11,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "ensure_trailing(var, postfix)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "replace",
    "catalog_order": 27,
    "category": "String Mutations",
    "description": "Replaces every literal occurrence of a substring; empty find is a no-op.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "replace(\"a.b.c\", \".\", \"/\")",
          "reason": null,
          "result": "a/b/c",
          "verification": "Executable"
        },
        "order": 12,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "replace(x, find, replacement)"
      }
    ]
  },
  {
    "aliases": [
      "replacefirst"
    ],
    "canonical_name": "replace_first",
    "catalog_order": 28,
    "category": "String Mutations",
    "description": "Replaces the first literal occurrence of a substring; empty find is a no-op.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "replace_first(\"a.b.c\", \".\", \"/\")",
          "reason": null,
          "result": "a/b.c",
          "verification": "Executable"
        },
        "order": 13,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "replace_first(x, find, replacement)"
      }
    ]
  },
  {
    "aliases": [
      "replacelast"
    ],
    "canonical_name": "replace_last",
    "catalog_order": 29,
    "category": "String Mutations",
    "description": "Replaces the last literal occurrence of a substring; empty find is a no-op.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "replace_last(\"a.b.c\", \".\", \"/\")",
          "reason": null,
          "result": "a.b/c",
          "verification": "Executable"
        },
        "order": 14,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "replace_last(x, find, replacement)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "terminal",
    "catalog_order": 30,
    "category": "Rendering",
    "description": "Renders Prose markup to a terminal string with ANSI SGR sequences.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "terminal(\"hello\")",
          "reason": null,
          "result": "hello",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "terminal(string)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "date",
    "catalog_order": 31,
    "category": "Date Formatting",
    "description": "Reformats an ISO date/datetime string into a named human format.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "date(\"2024-06-15\", \"long\")",
          "reason": null,
          "result": "Sat, June 15th, 2024",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "date(iso, fmt)"
      }
    ]
  },
  {
    "aliases": [
      "isdate"
    ],
    "canonical_name": "is_date",
    "catalog_order": 32,
    "category": "Date Validators",
    "description": "Returns true when the string is a valid ISO date (YYYY-MM-DD).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_date(\"2024-06-15\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_date(x)"
      }
    ]
  },
  {
    "aliases": [
      "isdateutc"
    ],
    "canonical_name": "is_date_utc",
    "catalog_order": 33,
    "category": "Date Validators",
    "description": "Same as is_date (the format itself is timezone-agnostic).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_date_utc(\"2024-06-15\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_date_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "isdatetime",
      "is_datetime"
    ],
    "canonical_name": "is_date_time",
    "catalog_order": 34,
    "category": "Date Validators",
    "description": "Returns true when the string is a valid ISO datetime.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_date_time(\"2024-06-15T12:30:00\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_date_time(x)"
      }
    ]
  },
  {
    "aliases": [
      "isdatetimeutc",
      "is_datetime_utc"
    ],
    "canonical_name": "is_date_time_utc",
    "catalog_order": 35,
    "category": "Date Validators",
    "description": "Same parse contract as is_date_time.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_date_time_utc(\"2024-06-15T12:30:00Z\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_date_time_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "istoday"
    ],
    "canonical_name": "is_today",
    "catalog_order": 36,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is today (local).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_today(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 5,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_today(x)"
      }
    ]
  },
  {
    "aliases": [
      "istodayutc"
    ],
    "canonical_name": "is_today_utc",
    "catalog_order": 37,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is today (UTC).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_today_utc(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 6,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_today_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "isyesterday"
    ],
    "canonical_name": "is_yesterday",
    "catalog_order": 38,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is yesterday (local).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_yesterday(\"2024-06-14\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 7,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_yesterday(x)"
      }
    ]
  },
  {
    "aliases": [
      "isyesterdayutc"
    ],
    "canonical_name": "is_yesterday_utc",
    "catalog_order": 39,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is yesterday (UTC).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_yesterday_utc(\"2024-06-14\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 8,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_yesterday_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "istomorrow"
    ],
    "canonical_name": "is_tomorrow",
    "catalog_order": 40,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is tomorrow (local).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_tomorrow(\"2024-06-16\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 9,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_tomorrow(x)"
      }
    ]
  },
  {
    "aliases": [
      "istomorrowutc"
    ],
    "canonical_name": "is_tomorrow_utc",
    "catalog_order": 41,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is tomorrow (UTC).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_tomorrow_utc(\"2024-06-16\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 10,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_tomorrow_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "isthismonth"
    ],
    "canonical_name": "is_this_month",
    "catalog_order": 42,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is in the current month (local).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_this_month(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 11,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_this_month(x)"
      }
    ]
  },
  {
    "aliases": [
      "isthismonthutc"
    ],
    "canonical_name": "is_this_month_utc",
    "catalog_order": 43,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is in the current month (UTC).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_this_month_utc(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 12,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_this_month_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "isthisyear"
    ],
    "canonical_name": "is_this_year",
    "catalog_order": 44,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is in the current year (local).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_this_year(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 13,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_this_year(x)"
      }
    ]
  },
  {
    "aliases": [
      "isthisyearutc"
    ],
    "canonical_name": "is_this_year_utc",
    "catalog_order": 45,
    "category": "Date Validators",
    "description": "Returns true when the date/datetime is in the current year (UTC).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "is_this_year_utc(\"2024-06-15\")",
          "reason": "wall-clock dependent",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 14,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "is_this_year_utc(x)"
      }
    ]
  },
  {
    "aliases": [
      "datedelta"
    ],
    "canonical_name": "date_delta",
    "catalog_order": 46,
    "category": "Date Arithmetic",
    "description": "Returns true when the two dates are at least the given duration apart, ignoring order (duration like 14d, 2mo, 1 hour).",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "date_delta(\"2024-06-01\", \"2024-06-20\", \"14d\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "date_delta(date1, date2, diff)"
      }
    ]
  },
  {
    "aliases": [
      "olderthan"
    ],
    "canonical_name": "older_than",
    "catalog_order": 47,
    "category": "Date Arithmetic",
    "description": "Returns true when date1 is at least the given duration older (earlier) than date2.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "older_than(\"2024-06-01\", \"2024-06-20\", \"14d\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "older_than(date1, date2, diff)"
      }
    ]
  },
  {
    "aliases": [
      "newerthan"
    ],
    "canonical_name": "newer_than",
    "catalog_order": 48,
    "category": "Date Arithmetic",
    "description": "Returns true when date1 is at least the given duration newer (later) than date2.",
    "handler_kind": "Pure",
    "overloads": [
      {
        "example": {
          "invocation": "newer_than(\"2024-06-20\", \"2024-06-01\", \"14d\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "newer_than(date1, date2, diff)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "absolute",
    "catalog_order": 56,
    "category": "Filesystem",
    "description": "Resolves a file path to an absolute path.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "absolute(\"fixture.md\")",
          "reason": "resolves to an absolute path of the resolution context, which is not portable",
          "result": "/path/to/fixture.md",
          "verification": "DisplayOnly"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "file"
        },
        "signature": "absolute(file)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "relative",
    "catalog_order": 57,
    "category": "Filesystem",
    "description": "Returns a best-effort relative path from the document base directory.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "relative(\"fixture.md\")",
          "reason": null,
          "result": "fixture.md",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "file"
        },
        "signature": "relative(file)"
      }
    ]
  },
  {
    "aliases": [
      "fileexists"
    ],
    "canonical_name": "file_exists",
    "catalog_order": 58,
    "category": "Filesystem",
    "description": "Returns true when the file exists (local or remote URL).",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "file_exists(\"fixture.md\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 3,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "file_exists(file)"
      }
    ]
  },
  {
    "aliases": [
      "hascommand"
    ],
    "canonical_name": "has_command",
    "catalog_order": 79,
    "category": "Filesystem",
    "description": "Returns true when the command is found on PATH or is an existing executable absolute path.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": null,
        "order": 24,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "has_command(cmd)"
      }
    ]
  },
  {
    "aliases": [
      "isindexedfile"
    ],
    "canonical_name": "is_indexed_file",
    "catalog_order": 65,
    "category": "Filesystem",
    "description": "Returns true when the filename stem matches the indexed grammar (base-NNN).",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "is_indexed_file(\"review-1.md\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 10,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "is_indexed_file(file)"
      }
    ]
  },
  {
    "aliases": [
      "fileindex"
    ],
    "canonical_name": "file_index",
    "catalog_order": 66,
    "category": "Filesystem",
    "description": "Returns the parsed index suffix, or -1 when non-indexed.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "file_index(\"review-1.md\")",
          "reason": null,
          "result": "1",
          "verification": "Executable"
        },
        "order": 11,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "number"
        },
        "signature": "file_index(file)"
      }
    ]
  },
  {
    "aliases": [
      "incrementfileindex"
    ],
    "canonical_name": "increment_file_index",
    "catalog_order": 67,
    "category": "Filesystem",
    "description": "Increments the numeric index suffix, preserving zero-padding width.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "increment_file_index(\"review-1.md\")",
          "reason": null,
          "result": "review-2.md",
          "verification": "Executable"
        },
        "order": 12,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "file"
        },
        "signature": "increment_file_index(file)"
      }
    ]
  },
  {
    "aliases": [
      "decrementfileindex"
    ],
    "canonical_name": "decrement_file_index",
    "catalog_order": 68,
    "category": "Filesystem",
    "description": "Decrements the numeric index suffix, clamped at 0.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "decrement_file_index(\"review-2.md\")",
          "reason": null,
          "result": "review-1.md",
          "verification": "Executable"
        },
        "order": 13,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "file"
        },
        "signature": "decrement_file_index(file)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "basename",
    "catalog_order": 69,
    "category": "Filesystem",
    "description": "Returns the final path component including extension.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "basename(\"sub/note.md\")",
          "reason": null,
          "result": "note.md",
          "verification": "Executable"
        },
        "order": 14,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "basename(file)"
      }
    ]
  },
  {
    "aliases": [
      "basenamewithoutindex"
    ],
    "canonical_name": "basename_without_index",
    "catalog_order": 70,
    "category": "Filesystem",
    "description": "Returns the basename with any indexed suffix removed from the stem.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "basename_without_index(\"review-1.md\")",
          "reason": null,
          "result": "review.md",
          "verification": "Executable"
        },
        "order": 15,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "basename_without_index(file)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "dirname",
    "catalog_order": 71,
    "category": "Filesystem",
    "description": "Returns the directory portion of the display path.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "dirname(\"sub/note.md\")",
          "reason": null,
          "result": "sub",
          "verification": "Executable"
        },
        "order": 16,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "dirname(file)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "ext",
    "catalog_order": 72,
    "category": "Filesystem",
    "description": "Returns the final extension without the leading dot.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "ext(\"sub/note.md\")",
          "reason": null,
          "result": "md",
          "verification": "Executable"
        },
        "order": 17,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "ext(file)"
      }
    ]
  },
  {
    "aliases": [
      "parentdir"
    ],
    "canonical_name": "parent_dir",
    "catalog_order": 73,
    "category": "Filesystem",
    "description": "Returns the directory segment immediately above the basename.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "parent_dir(\"sub/note.md\")",
          "reason": null,
          "result": "sub",
          "verification": "Executable"
        },
        "order": 18,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "parent_dir(file)"
      }
    ]
  },
  {
    "aliases": [
      "filetrailing"
    ],
    "canonical_name": "file_trailing",
    "catalog_order": 74,
    "category": "Filesystem",
    "description": "Returns the last directory segment plus the basename.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "file_trailing(\"sub/note.md\")",
          "reason": null,
          "result": "sub/note.md",
          "verification": "Executable"
        },
        "order": 19,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "file_trailing(file)"
      }
    ]
  },
  {
    "aliases": [
      "dirleading"
    ],
    "canonical_name": "dir_leading",
    "catalog_order": 75,
    "category": "Filesystem",
    "description": "Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing).",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "dir_leading(\"sub/note.md\")",
          "reason": null,
          "result": "",
          "verification": "Executable"
        },
        "order": 20,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "dir_leading(file)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "join",
    "catalog_order": 76,
    "category": "Filesystem",
    "description": "Joins two path strings with normalized separators.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "join(\"sub\", \"note.md\")",
          "reason": null,
          "result": "sub/note.md",
          "verification": "Executable"
        },
        "order": 21,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "join(left, right)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "link",
    "catalog_order": 77,
    "category": "Filesystem",
    "description": "Creates a Markdown link to a local file, using its relative path as the link text.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "link(\"fixture.md\")",
          "reason": "result includes an absolute path, which is not portable",
          "result": "[fixture.md](/path/to/fixture.md)",
          "verification": "DisplayOnly"
        },
        "order": 22,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "link(file)"
      },
      {
        "example": {
          "invocation": "link(\"fixture.md\", \"Fixture\")",
          "reason": "result includes an absolute destination path, which is not portable",
          "result": "[Fixture](/path/to/fixture.md)",
          "verification": "DisplayOnly"
        },
        "order": 23,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "link(target, desc)"
      }
    ]
  },
  {
    "aliases": [
      "hasskill"
    ],
    "canonical_name": "has_skill",
    "catalog_order": 80,
    "category": "Context",
    "description": "Returns true when a skill directory exists in a user-scoped or local-scoped skill root.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "has_skill(\"darkmatter\")",
          "reason": "depends on agent-specific skill roots outside the tempdir fixture",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "has_skill(name)"
      }
    ]
  },
  {
    "aliases": [
      "haslocalskill"
    ],
    "canonical_name": "has_local_skill",
    "catalog_order": 81,
    "category": "Context",
    "description": "Returns true when a skill directory exists in a local-scoped skill root.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "has_local_skill(\"darkmatter\")",
          "reason": "depends on agent-specific skill roots outside the tempdir fixture",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "has_local_skill(name)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "frontmatter",
    "catalog_order": 59,
    "category": "Filesystem",
    "description": "Reads the frontmatter of a Markdown file as an object.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "frontmatter(\"fixture.md\")",
          "reason": null,
          "result": "{\"title\":\"Fixture Title\"}",
          "verification": "Executable"
        },
        "order": 4,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "object"
        },
        "signature": "frontmatter(file)"
      },
      {
        "example": {
          "invocation": "frontmatter(\"fixture.md\", \"title\")",
          "reason": null,
          "result": "Fixture Title",
          "verification": "Executable"
        },
        "order": 5,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "string",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "any"
        },
        "signature": "frontmatter(file, prop)"
      }
    ]
  },
  {
    "aliases": [
      "markdownbodyempty"
    ],
    "canonical_name": "markdown_body_empty",
    "catalog_order": 61,
    "category": "Filesystem",
    "description": "Returns true when the Markdown body has only whitespace.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "markdown_body_empty(\"fixture.md\")",
          "reason": null,
          "result": "false",
          "verification": "Executable"
        },
        "order": 6,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "markdown_body_empty(file)"
      }
    ]
  },
  {
    "aliases": [
      "markdowntitle"
    ],
    "canonical_name": "markdown_title",
    "catalog_order": 62,
    "category": "Filesystem",
    "description": "Returns the title from frontmatter or the first H1 heading.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "markdown_title(\"fixture.md\")",
          "reason": null,
          "result": "Fixture Title",
          "verification": "Executable"
        },
        "order": 7,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "string"
        },
        "signature": "markdown_title(file)"
      }
    ]
  },
  {
    "aliases": [
      "validateschema"
    ],
    "canonical_name": "validate_schema",
    "catalog_order": 63,
    "category": "Filesystem",
    "description": "Validates a Markdown document against its declared schema.",
    "handler_kind": "Context",
    "overloads": [
      {
        "example": {
          "invocation": "validate_schema(\"fixture.md\")",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 8,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "validate_schema(file)"
      },
      {
        "example": {
          "invocation": "validate_schema(\"fixture.md\", {})",
          "reason": "forward-compatible overload with no evaluable behavior yet",
          "result": "true",
          "verification": "DisplayOnly"
        },
        "order": 9,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "file",
            "variadic": false
          },
          {
            "array": false,
            "optional": false,
            "type": "object",
            "variadic": false
          }
        ],
        "return": {
          "array": false,
          "fallible": true,
          "type": "boolean"
        },
        "signature": "validate_schema(file, obj)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "and",
    "catalog_order": 49,
    "category": "Logical",
    "description": "Logical AND of all arguments. Short-circuits on first falsy value.",
    "handler_kind": "Lazy",
    "overloads": [
      {
        "example": {
          "invocation": "and(true, true)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 1,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": true
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "and(...)"
      }
    ]
  },
  {
    "aliases": [],
    "canonical_name": "or",
    "catalog_order": 50,
    "category": "Logical",
    "description": "Logical OR of all arguments. Short-circuits on first truthy value.",
    "handler_kind": "Lazy",
    "overloads": [
      {
        "example": {
          "invocation": "or(false, true)",
          "reason": null,
          "result": "true",
          "verification": "Executable"
        },
        "order": 2,
        "parameters": [
          {
            "array": false,
            "optional": false,
            "type": "any",
            "variadic": true
          }
        ],
        "return": {
          "array": false,
          "fallible": false,
          "type": "boolean"
        },
        "signature": "or(...)"
      }
    ]
  }
]
```

## Generated function table

Exact `generate_expression_function_table()` output follows. It is 11,921 bytes
across 90 lines with SHA-256
`ec53916ef1f9d90856555bcd5929f40faf126e9ebd4c18b288ded5c10728e809`.

```markdown
| Category | Function | Description | Example |
| --- | --- | --- | --- |
| Type Predicates | `is_string(x)` | Returns true when the value is a string. | `is_string("hello")` ⇒ `true` |
| Type Predicates | `is_number(x)` | Returns true when the value is a number. | `is_number(42)` ⇒ `true` |
| Type Predicates | `is_array(x)` | Returns true when the value is an array. | `is_array(items)` ⇒ `true` |
| Type Predicates | `is_null(x)` | Returns true when the value is null. | `is_null(null)` ⇒ `true` |
| Type Predicates | `is_object(x)` | Returns true when the value is an object. | `is_object(obj)` ⇒ `true` |
| Type Predicates | `is_empty(x)` | Returns true when the value is null, empty string, empty array, or empty object. | `is_empty("")` ⇒ `true` |
| Type Predicates | `is_positive(val)` | Returns true when the coerced value is greater than zero. | `is_positive(5)` ⇒ `true` |
| Type Predicates | `is_negative(val)` | Returns true when the coerced value is less than zero. | `is_negative(-3)` ⇒ `true` |
| Type Predicates | `is_integer(val)` | Returns true when the value is a JSON number with no fractional component. | `is_integer(7)` ⇒ `true` |
| Math | `min(a, b)` | Returns the smaller of two numbers. | `min(2, 5)` ⇒ `2` |
| Math | `max(a, b)` | Returns the larger of two numbers. | `max(2, 5)` ⇒ `5` |
| Math | `abs(x)` | Returns the absolute value of a number. | `abs(-3)` ⇒ `3` |
| Collection | `first(x)` | Returns the first element of an array, or null when empty. | `first(items)` ⇒ `1` |
| Collection | `last(x)` | Returns the last element of an array, or null when empty. | `last(items)` ⇒ `3` |
| String Predicates | `starts_with(x, find)` | Returns true when the string starts with the given prefix (case-sensitive). | `starts_with("hello", "he")` ⇒ `true` |
| String Predicates | `ends_with(x, find)` | Returns true when the string ends with the given suffix (case-sensitive). | `ends_with("hello", "lo")` ⇒ `true` |
| String Mutations | `lower(x)` | Converts a string to lowercase. | `lower("HELLO")` ⇒ `hello` |
| String Mutations | `upper(x)` | Converts a string to uppercase. | `upper("hello")` ⇒ `HELLO` |
| String Mutations | `capitalize(x)` | Capitalizes the first character of a string. | `capitalize("hello")` ⇒ `Hello` |
| String Mutations | `kebab_case(x)` | Converts a string to kebab-case. | `kebab_case("Hello World")` ⇒ `hello-world` |
| String Mutations | `snake_case(x)` | Converts a string to snake_case. | `snake_case("Hello World")` ⇒ `hello_world` |
| String Mutations | `camel_case(x)` | Converts a string to camelCase. | `camel_case("hello world")` ⇒ `helloWorld` |
| String Mutations | `pascal_case(x)` | Converts a string to PascalCase. | `pascal_case("hello world")` ⇒ `HelloWorld` |
| String Mutations | `title_case(x)` | Converts a string to Title Case. | `title_case("hello world")` ⇒ `Hello World` |
| String Mutations | `without_date(string)` | Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched. | `without_date("Note 2024-06-15")` ⇒ `Note ` |
| String Mutations | `ensure_leading(var, prefix)` | Ensures the string form of a value starts with a prefix. | `ensure_leading("world", "hello ")` ⇒ `hello world` |
| String Mutations | `ensure_trailing(var, postfix)` | Ensures the string form of a value ends with a postfix. | `ensure_trailing("hello", " world")` ⇒ `hello world` |
| String Mutations | `replace(x, find, replacement)` | Replaces every literal occurrence of a substring; empty find is a no-op. | `replace("a.b.c", ".", "/")` ⇒ `a/b/c` |
| String Mutations | `replace_first(x, find, replacement)` | Replaces the first literal occurrence of a substring; empty find is a no-op. | `replace_first("a.b.c", ".", "/")` ⇒ `a/b.c` |
| String Mutations | `replace_last(x, find, replacement)` | Replaces the last literal occurrence of a substring; empty find is a no-op. | `replace_last("a.b.c", ".", "/")` ⇒ `a.b/c` |
| Rendering | `terminal(string)` | Renders Prose markup to a terminal string with ANSI SGR sequences. | `terminal("hello")` ⇒ `hello` |
| Date Formatting | `date(iso, fmt)` | Reformats an ISO date/datetime string into a named human format. | `date("2024-06-15", "long")` ⇒ `Sat, June 15th, 2024` |
| Date Validators | `is_date(x)` | Returns true when the string is a valid ISO date (YYYY-MM-DD). | `is_date("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_utc(x)` | Same as is_date (the format itself is timezone-agnostic). | `is_date_utc("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_time(x)` | Returns true when the string is a valid ISO datetime. | `is_date_time("2024-06-15T12:30:00")` ⇒ `true` |
| Date Validators | `is_date_time_utc(x)` | Same parse contract as is_date_time. | `is_date_time_utc("2024-06-15T12:30:00Z")` ⇒ `true` |
| Date Validators | `is_today(x)` | Returns true when the date/datetime is today (local). |  |
| Date Validators | `is_today_utc(x)` | Returns true when the date/datetime is today (UTC). |  |
| Date Validators | `is_yesterday(x)` | Returns true when the date/datetime is yesterday (local). |  |
| Date Validators | `is_yesterday_utc(x)` | Returns true when the date/datetime is yesterday (UTC). |  |
| Date Validators | `is_tomorrow(x)` | Returns true when the date/datetime is tomorrow (local). |  |
| Date Validators | `is_tomorrow_utc(x)` | Returns true when the date/datetime is tomorrow (UTC). |  |
| Date Validators | `is_this_month(x)` | Returns true when the date/datetime is in the current month (local). |  |
| Date Validators | `is_this_month_utc(x)` | Returns true when the date/datetime is in the current month (UTC). |  |
| Date Validators | `is_this_year(x)` | Returns true when the date/datetime is in the current year (local). |  |
| Date Validators | `is_this_year_utc(x)` | Returns true when the date/datetime is in the current year (UTC). |  |
| Date Arithmetic | `date_delta(date1, date2, diff)` | Returns true when the two dates are at least the given duration apart, ignoring order (duration like 14d, 2mo, 1 hour). | `date_delta("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `older_than(date1, date2, diff)` | Returns true when date1 is at least the given duration older (earlier) than date2. | `older_than("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `newer_than(date1, date2, diff)` | Returns true when date1 is at least the given duration newer (later) than date2. | `newer_than("2024-06-20", "2024-06-01", "14d")` ⇒ `true` |
| Logical | `and(...)` | Logical AND of all arguments. Short-circuits on first falsy value. | `and(true, true)` ⇒ `true` |
| Logical | `or(...)` | Logical OR of all arguments. Short-circuits on first truthy value. | `or(false, true)` ⇒ `true` |
| Collection | `has_key(obj, key)` | Returns true when the object contains the given key. | `has_key(obj, "a")` ⇒ `true` |
| Collection | `contains(haystack, needle)` | Returns true when haystack contains needle (array, object, or string). | `contains("hello", "ell")` ⇒ `true` |
| Collection | `length(x)` | Returns the length of a string, array, or object. | `length("hello")` ⇒ `5` |
| Type Conversion | `number(x, [default])` | Converts a value to a number, with an optional default. | `number("42")` ⇒ `42` |
| Math | `round(x, [default])` | Rounds a value to the nearest integer, with an optional default. | `round(3.7)` ⇒ `4` |
| Filesystem | `absolute(file)` | Resolves a file path to an absolute path. |  |
| Filesystem | `relative(file)` | Returns a best-effort relative path from the document base directory. | `relative("fixture.md")` ⇒ `fixture.md` |
| Filesystem | `file_exists(file)` | Returns true when the file exists (local or remote URL). | `file_exists("fixture.md")` ⇒ `true` |
| Filesystem | `frontmatter(file)` | Reads the frontmatter of a Markdown file as an object. | `frontmatter("fixture.md")` ⇒ `{"title":"Fixture Title"}` |
| Filesystem | `frontmatter(file, prop)` | Reads a single frontmatter property from a Markdown file. | `frontmatter("fixture.md", "title")` ⇒ `Fixture Title` |
| Filesystem | `markdown_body_empty(file)` | Returns true when the Markdown body has only whitespace. | `markdown_body_empty("fixture.md")` ⇒ `false` |
| Filesystem | `markdown_title(file)` | Returns the title from frontmatter or the first H1 heading. | `markdown_title("fixture.md")` ⇒ `Fixture Title` |
| Filesystem | `validate_schema(file)` | Validates a Markdown document against its declared schema. | `validate_schema("fixture.md")` ⇒ `true` |
| Filesystem | `validate_schema(file, obj)` | Two-argument form accepted for forward compatibility. |  |
| Filesystem | `is_indexed_file(file)` | Returns true when the filename stem matches the indexed grammar (base-NNN). | `is_indexed_file("review-1.md")` ⇒ `true` |
| Filesystem | `file_index(file)` | Returns the parsed index suffix, or -1 when non-indexed. | `file_index("review-1.md")` ⇒ `1` |
| Filesystem | `increment_file_index(file)` | Increments the numeric index suffix, preserving zero-padding width. | `increment_file_index("review-1.md")` ⇒ `review-2.md` |
| Filesystem | `decrement_file_index(file)` | Decrements the numeric index suffix, clamped at 0. | `decrement_file_index("review-2.md")` ⇒ `review-1.md` |
| Filesystem | `basename(file)` | Returns the final path component including extension. | `basename("sub/note.md")` ⇒ `note.md` |
| Filesystem | `basename_without_index(file)` | Returns the basename with any indexed suffix removed from the stem. | `basename_without_index("review-1.md")` ⇒ `review.md` |
| Filesystem | `dirname(file)` | Returns the directory portion of the display path. | `dirname("sub/note.md")` ⇒ `sub` |
| Filesystem | `ext(file)` | Returns the final extension without the leading dot. | `ext("sub/note.md")` ⇒ `md` |
| Filesystem | `parent_dir(file)` | Returns the directory segment immediately above the basename. | `parent_dir("sub/note.md")` ⇒ `sub` |
| Filesystem | `file_trailing(file)` | Returns the last directory segment plus the basename. | `file_trailing("sub/note.md")` ⇒ `sub/note.md` |
| Filesystem | `dir_leading(file)` | Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing). | `dir_leading("sub/note.md")` ⇒ `` |
| Filesystem | `join(left, right)` | Joins two path strings with normalized separators. | `join("sub", "note.md")` ⇒ `sub/note.md` |
| Filesystem | `link(file)` | Creates a Markdown link to a local file, using its relative path as the link text. |  |
| Filesystem | `link(target, desc)` | Creates a Markdown link to a local file or HTTP(S) URL with the given description. |  |
| Filesystem | `has_command(cmd)` | Returns true when the command is found on PATH or is an existing executable absolute path. |  |
| Context | `has_skill(name)` | Returns true when a skill directory exists in a user-scoped or local-scoped skill root. |  |
| Context | `has_local_skill(name)` | Returns true when a skill directory exists in a local-scoped skill root. |  |
| List Formatting | `as_line_separated(list)` | Joins a list into a newline-separated string (the default bare-array rendering). |  |
| List Formatting | `as_csv(list)` | Joins a list into a comma-separated string. | `as_csv(items)` ⇒ `1, 2, 3` |
| List Formatting | `as_tsv(list)` | Joins a list into a tab-separated string. |  |
| List Formatting | `as_space_separated(list)` | Joins a list into a space-separated string. | `as_space_separated(items)` ⇒ `1 2 3` |
| List Formatting | `as_unordered_list(list)` | Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
| List Formatting | `as_ordered_list(list)` | Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
```

## Checked-in function table

Exact bytes between the generated-table markers in
`darkmatter/docs/topics/darkmatter-expressions.md` follow. This capture is also
11,921 bytes across 90 lines with SHA-256
`ec53916ef1f9d90856555bcd5929f40faf126e9ebd4c18b288ded5c10728e809`.
`cmp` returned zero against the generated output.

```markdown
| Category | Function | Description | Example |
| --- | --- | --- | --- |
| Type Predicates | `is_string(x)` | Returns true when the value is a string. | `is_string("hello")` ⇒ `true` |
| Type Predicates | `is_number(x)` | Returns true when the value is a number. | `is_number(42)` ⇒ `true` |
| Type Predicates | `is_array(x)` | Returns true when the value is an array. | `is_array(items)` ⇒ `true` |
| Type Predicates | `is_null(x)` | Returns true when the value is null. | `is_null(null)` ⇒ `true` |
| Type Predicates | `is_object(x)` | Returns true when the value is an object. | `is_object(obj)` ⇒ `true` |
| Type Predicates | `is_empty(x)` | Returns true when the value is null, empty string, empty array, or empty object. | `is_empty("")` ⇒ `true` |
| Type Predicates | `is_positive(val)` | Returns true when the coerced value is greater than zero. | `is_positive(5)` ⇒ `true` |
| Type Predicates | `is_negative(val)` | Returns true when the coerced value is less than zero. | `is_negative(-3)` ⇒ `true` |
| Type Predicates | `is_integer(val)` | Returns true when the value is a JSON number with no fractional component. | `is_integer(7)` ⇒ `true` |
| Math | `min(a, b)` | Returns the smaller of two numbers. | `min(2, 5)` ⇒ `2` |
| Math | `max(a, b)` | Returns the larger of two numbers. | `max(2, 5)` ⇒ `5` |
| Math | `abs(x)` | Returns the absolute value of a number. | `abs(-3)` ⇒ `3` |
| Collection | `first(x)` | Returns the first element of an array, or null when empty. | `first(items)` ⇒ `1` |
| Collection | `last(x)` | Returns the last element of an array, or null when empty. | `last(items)` ⇒ `3` |
| String Predicates | `starts_with(x, find)` | Returns true when the string starts with the given prefix (case-sensitive). | `starts_with("hello", "he")` ⇒ `true` |
| String Predicates | `ends_with(x, find)` | Returns true when the string ends with the given suffix (case-sensitive). | `ends_with("hello", "lo")` ⇒ `true` |
| String Mutations | `lower(x)` | Converts a string to lowercase. | `lower("HELLO")` ⇒ `hello` |
| String Mutations | `upper(x)` | Converts a string to uppercase. | `upper("hello")` ⇒ `HELLO` |
| String Mutations | `capitalize(x)` | Capitalizes the first character of a string. | `capitalize("hello")` ⇒ `Hello` |
| String Mutations | `kebab_case(x)` | Converts a string to kebab-case. | `kebab_case("Hello World")` ⇒ `hello-world` |
| String Mutations | `snake_case(x)` | Converts a string to snake_case. | `snake_case("Hello World")` ⇒ `hello_world` |
| String Mutations | `camel_case(x)` | Converts a string to camelCase. | `camel_case("hello world")` ⇒ `helloWorld` |
| String Mutations | `pascal_case(x)` | Converts a string to PascalCase. | `pascal_case("hello world")` ⇒ `HelloWorld` |
| String Mutations | `title_case(x)` | Converts a string to Title Case. | `title_case("hello world")` ⇒ `Hello World` |
| String Mutations | `without_date(string)` | Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched. | `without_date("Note 2024-06-15")` ⇒ `Note ` |
| String Mutations | `ensure_leading(var, prefix)` | Ensures the string form of a value starts with a prefix. | `ensure_leading("world", "hello ")` ⇒ `hello world` |
| String Mutations | `ensure_trailing(var, postfix)` | Ensures the string form of a value ends with a postfix. | `ensure_trailing("hello", " world")` ⇒ `hello world` |
| String Mutations | `replace(x, find, replacement)` | Replaces every literal occurrence of a substring; empty find is a no-op. | `replace("a.b.c", ".", "/")` ⇒ `a/b/c` |
| String Mutations | `replace_first(x, find, replacement)` | Replaces the first literal occurrence of a substring; empty find is a no-op. | `replace_first("a.b.c", ".", "/")` ⇒ `a/b.c` |
| String Mutations | `replace_last(x, find, replacement)` | Replaces the last literal occurrence of a substring; empty find is a no-op. | `replace_last("a.b.c", ".", "/")` ⇒ `a.b/c` |
| Rendering | `terminal(string)` | Renders Prose markup to a terminal string with ANSI SGR sequences. | `terminal("hello")` ⇒ `hello` |
| Date Formatting | `date(iso, fmt)` | Reformats an ISO date/datetime string into a named human format. | `date("2024-06-15", "long")` ⇒ `Sat, June 15th, 2024` |
| Date Validators | `is_date(x)` | Returns true when the string is a valid ISO date (YYYY-MM-DD). | `is_date("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_utc(x)` | Same as is_date (the format itself is timezone-agnostic). | `is_date_utc("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_time(x)` | Returns true when the string is a valid ISO datetime. | `is_date_time("2024-06-15T12:30:00")` ⇒ `true` |
| Date Validators | `is_date_time_utc(x)` | Same parse contract as is_date_time. | `is_date_time_utc("2024-06-15T12:30:00Z")` ⇒ `true` |
| Date Validators | `is_today(x)` | Returns true when the date/datetime is today (local). |  |
| Date Validators | `is_today_utc(x)` | Returns true when the date/datetime is today (UTC). |  |
| Date Validators | `is_yesterday(x)` | Returns true when the date/datetime is yesterday (local). |  |
| Date Validators | `is_yesterday_utc(x)` | Returns true when the date/datetime is yesterday (UTC). |  |
| Date Validators | `is_tomorrow(x)` | Returns true when the date/datetime is tomorrow (local). |  |
| Date Validators | `is_tomorrow_utc(x)` | Returns true when the date/datetime is tomorrow (UTC). |  |
| Date Validators | `is_this_month(x)` | Returns true when the date/datetime is in the current month (local). |  |
| Date Validators | `is_this_month_utc(x)` | Returns true when the date/datetime is in the current month (UTC). |  |
| Date Validators | `is_this_year(x)` | Returns true when the date/datetime is in the current year (local). |  |
| Date Validators | `is_this_year_utc(x)` | Returns true when the date/datetime is in the current year (UTC). |  |
| Date Arithmetic | `date_delta(date1, date2, diff)` | Returns true when the two dates are at least the given duration apart, ignoring order (duration like 14d, 2mo, 1 hour). | `date_delta("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `older_than(date1, date2, diff)` | Returns true when date1 is at least the given duration older (earlier) than date2. | `older_than("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `newer_than(date1, date2, diff)` | Returns true when date1 is at least the given duration newer (later) than date2. | `newer_than("2024-06-20", "2024-06-01", "14d")` ⇒ `true` |
| Logical | `and(...)` | Logical AND of all arguments. Short-circuits on first falsy value. | `and(true, true)` ⇒ `true` |
| Logical | `or(...)` | Logical OR of all arguments. Short-circuits on first truthy value. | `or(false, true)` ⇒ `true` |
| Collection | `has_key(obj, key)` | Returns true when the object contains the given key. | `has_key(obj, "a")` ⇒ `true` |
| Collection | `contains(haystack, needle)` | Returns true when haystack contains needle (array, object, or string). | `contains("hello", "ell")` ⇒ `true` |
| Collection | `length(x)` | Returns the length of a string, array, or object. | `length("hello")` ⇒ `5` |
| Type Conversion | `number(x, [default])` | Converts a value to a number, with an optional default. | `number("42")` ⇒ `42` |
| Math | `round(x, [default])` | Rounds a value to the nearest integer, with an optional default. | `round(3.7)` ⇒ `4` |
| Filesystem | `absolute(file)` | Resolves a file path to an absolute path. |  |
| Filesystem | `relative(file)` | Returns a best-effort relative path from the document base directory. | `relative("fixture.md")` ⇒ `fixture.md` |
| Filesystem | `file_exists(file)` | Returns true when the file exists (local or remote URL). | `file_exists("fixture.md")` ⇒ `true` |
| Filesystem | `frontmatter(file)` | Reads the frontmatter of a Markdown file as an object. | `frontmatter("fixture.md")` ⇒ `{"title":"Fixture Title"}` |
| Filesystem | `frontmatter(file, prop)` | Reads a single frontmatter property from a Markdown file. | `frontmatter("fixture.md", "title")` ⇒ `Fixture Title` |
| Filesystem | `markdown_body_empty(file)` | Returns true when the Markdown body has only whitespace. | `markdown_body_empty("fixture.md")` ⇒ `false` |
| Filesystem | `markdown_title(file)` | Returns the title from frontmatter or the first H1 heading. | `markdown_title("fixture.md")` ⇒ `Fixture Title` |
| Filesystem | `validate_schema(file)` | Validates a Markdown document against its declared schema. | `validate_schema("fixture.md")` ⇒ `true` |
| Filesystem | `validate_schema(file, obj)` | Two-argument form accepted for forward compatibility. |  |
| Filesystem | `is_indexed_file(file)` | Returns true when the filename stem matches the indexed grammar (base-NNN). | `is_indexed_file("review-1.md")` ⇒ `true` |
| Filesystem | `file_index(file)` | Returns the parsed index suffix, or -1 when non-indexed. | `file_index("review-1.md")` ⇒ `1` |
| Filesystem | `increment_file_index(file)` | Increments the numeric index suffix, preserving zero-padding width. | `increment_file_index("review-1.md")` ⇒ `review-2.md` |
| Filesystem | `decrement_file_index(file)` | Decrements the numeric index suffix, clamped at 0. | `decrement_file_index("review-2.md")` ⇒ `review-1.md` |
| Filesystem | `basename(file)` | Returns the final path component including extension. | `basename("sub/note.md")` ⇒ `note.md` |
| Filesystem | `basename_without_index(file)` | Returns the basename with any indexed suffix removed from the stem. | `basename_without_index("review-1.md")` ⇒ `review.md` |
| Filesystem | `dirname(file)` | Returns the directory portion of the display path. | `dirname("sub/note.md")` ⇒ `sub` |
| Filesystem | `ext(file)` | Returns the final extension without the leading dot. | `ext("sub/note.md")` ⇒ `md` |
| Filesystem | `parent_dir(file)` | Returns the directory segment immediately above the basename. | `parent_dir("sub/note.md")` ⇒ `sub` |
| Filesystem | `file_trailing(file)` | Returns the last directory segment plus the basename. | `file_trailing("sub/note.md")` ⇒ `sub/note.md` |
| Filesystem | `dir_leading(file)` | Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing). | `dir_leading("sub/note.md")` ⇒ `` |
| Filesystem | `join(left, right)` | Joins two path strings with normalized separators. | `join("sub", "note.md")` ⇒ `sub/note.md` |
| Filesystem | `link(file)` | Creates a Markdown link to a local file, using its relative path as the link text. |  |
| Filesystem | `link(target, desc)` | Creates a Markdown link to a local file or HTTP(S) URL with the given description. |  |
| Filesystem | `has_command(cmd)` | Returns true when the command is found on PATH or is an existing executable absolute path. |  |
| Context | `has_skill(name)` | Returns true when a skill directory exists in a user-scoped or local-scoped skill root. |  |
| Context | `has_local_skill(name)` | Returns true when a skill directory exists in a local-scoped skill root. |  |
| List Formatting | `as_line_separated(list)` | Joins a list into a newline-separated string (the default bare-array rendering). |  |
| List Formatting | `as_csv(list)` | Joins a list into a comma-separated string. | `as_csv(items)` ⇒ `1, 2, 3` |
| List Formatting | `as_tsv(list)` | Joins a list into a tab-separated string. |  |
| List Formatting | `as_space_separated(list)` | Joins a list into a space-separated string. | `as_space_separated(items)` ⇒ `1 2 3` |
| List Formatting | `as_unordered_list(list)` | Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
| List Formatting | `as_ordered_list(list)` | Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
```

## Verbose expression-function signatures

Exact `expression_function_signatures_markdown()` output used by
`md schema about --verbose` follows. It is 10,083 bytes across 130 lines with
SHA-256
`07e6b498e4b2516d695a21f616dee7d62ac5cc2480c9636d884ee520404059d4`.

```markdown

**Type Predicates**

- `is_string(x: any) -> boolean` — Returns true when the value is a string.
- `is_number(x: any) -> boolean` — Returns true when the value is a number.
- `is_array(x: any) -> boolean` — Returns true when the value is an array.
- `is_null(x: any) -> boolean` — Returns true when the value is null.
- `is_object(x: any) -> boolean` — Returns true when the value is an object.
- `is_empty(x: any) -> boolean` — Returns true when the value is null, empty string, empty array, or empty object.
- `is_positive(val: any) -> boolean | error` — Returns true when the coerced value is greater than zero.
- `is_negative(val: any) -> boolean | error` — Returns true when the coerced value is less than zero.
- `is_integer(val: any) -> boolean` — Returns true when the value is a JSON number with no fractional component.

**Math**

- `min(a: number, b: number) -> number | error` — Returns the smaller of two numbers.
- `max(a: number, b: number) -> number | error` — Returns the larger of two numbers.
- `abs(x: number) -> number | error` — Returns the absolute value of a number.
- `round(x: number, [default: number]) -> number` — Rounds a value to the nearest integer, with an optional default.

**Collection**

- `first(x: any[]) -> any | error` — Returns the first element of an array, or null when empty.
- `last(x: any[]) -> any | error` — Returns the last element of an array, or null when empty.
- `has_key(obj: object, key: string) -> boolean | error` — Returns true when the object contains the given key.
- `contains(haystack: any, needle: any) -> boolean | error` — Returns true when haystack contains needle (array, object, or string).
- `length(x: any) -> number | error` — Returns the length of a string, array, or object.

**String Predicates**

- `starts_with(x: string, find: string) -> boolean | error` — Returns true when the string starts with the given prefix (case-sensitive).
- `ends_with(x: string, find: string) -> boolean | error` — Returns true when the string ends with the given suffix (case-sensitive).

**String Mutations**

- `lower(x: string) -> string | error` — Converts a string to lowercase.
- `upper(x: string) -> string | error` — Converts a string to uppercase.
- `capitalize(x: string) -> string | error` — Capitalizes the first character of a string.
- `kebab_case(x: string) -> string | error` — Converts a string to kebab-case.
- `snake_case(x: string) -> string | error` — Converts a string to snake_case.
- `camel_case(x: string) -> string | error` — Converts a string to camelCase.
- `pascal_case(x: string) -> string | error` — Converts a string to PascalCase.
- `title_case(x: string) -> string | error` — Converts a string to Title Case.
- `without_date(string: string) -> string | error` — Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.
- `ensure_leading(var: any, prefix: any) -> string | error` — Ensures the string form of a value starts with a prefix.
- `ensure_trailing(var: any, postfix: any) -> string | error` — Ensures the string form of a value ends with a postfix.
- `replace(x: string, find: string, replacement: string) -> string | error` — Replaces every literal occurrence of a substring; empty find is a no-op.
- `replace_first(x: string, find: string, replacement: string) -> string | error` — Replaces the first literal occurrence of a substring; empty find is a no-op.
- `replace_last(x: string, find: string, replacement: string) -> string | error` — Replaces the last literal occurrence of a substring; empty find is a no-op.

**Rendering**

- `terminal(string: string) -> string | error` — Renders Prose markup to a terminal string with ANSI SGR sequences.

**Date Formatting**

- `date(iso: string, fmt: string) -> string | error` — Reformats an ISO date/datetime string into a named human format.

**Date Validators**

- `is_date(x: any) -> boolean` — Returns true when the string is a valid ISO date (YYYY-MM-DD).
- `is_date_utc(x: any) -> boolean` — Same as is_date (the format itself is timezone-agnostic).
- `is_date_time(x: any) -> boolean` — Returns true when the string is a valid ISO datetime.
- `is_date_time_utc(x: any) -> boolean` — Same parse contract as is_date_time.
- `is_today(x: any) -> boolean` — Returns true when the date/datetime is today (local).
- `is_today_utc(x: any) -> boolean` — Returns true when the date/datetime is today (UTC).
- `is_yesterday(x: any) -> boolean` — Returns true when the date/datetime is yesterday (local).
- `is_yesterday_utc(x: any) -> boolean` — Returns true when the date/datetime is yesterday (UTC).
- `is_tomorrow(x: any) -> boolean` — Returns true when the date/datetime is tomorrow (local).
- `is_tomorrow_utc(x: any) -> boolean` — Returns true when the date/datetime is tomorrow (UTC).
- `is_this_month(x: any) -> boolean` — Returns true when the date/datetime is in the current month (local).
- `is_this_month_utc(x: any) -> boolean` — Returns true when the date/datetime is in the current month (UTC).
- `is_this_year(x: any) -> boolean` — Returns true when the date/datetime is in the current year (local).
- `is_this_year_utc(x: any) -> boolean` — Returns true when the date/datetime is in the current year (UTC).

**Date Arithmetic**

- `date_delta(date1: string, date2: string, diff: string) -> boolean | error` — Returns true when the two dates are at least the given duration apart, ignoring order (duration like 14d, 2mo, 1 hour).
- `older_than(date1: string, date2: string, diff: string) -> boolean | error` — Returns true when date1 is at least the given duration older (earlier) than date2.
- `newer_than(date1: string, date2: string, diff: string) -> boolean | error` — Returns true when date1 is at least the given duration newer (later) than date2.

**Logical**

- `and(...any) -> boolean` — Logical AND of all arguments. Short-circuits on first falsy value.
- `or(...any) -> boolean` — Logical OR of all arguments. Short-circuits on first truthy value.

**Type Conversion**

- `number(x: any, [default: any]) -> number | error` — Converts a value to a number, with an optional default.

**Filesystem**

- `absolute(file: file) -> file | error` — Resolves a file path to an absolute path.
- `relative(file: file) -> file | error` — Returns a best-effort relative path from the document base directory.
- `file_exists(file: file) -> boolean | error` — Returns true when the file exists (local or remote URL).
- `frontmatter(file: file) -> object | error` — Reads the frontmatter of a Markdown file as an object.
- `frontmatter(file: file, prop: string) -> any | error` — Reads a single frontmatter property from a Markdown file.
- `markdown_body_empty(file: file) -> boolean | error` — Returns true when the Markdown body has only whitespace.
- `markdown_title(file: file) -> string | error` — Returns the title from frontmatter or the first H1 heading.
- `validate_schema(file: file) -> boolean | error` — Validates a Markdown document against its declared schema.
- `validate_schema(file: file, obj: object) -> boolean | error` — Two-argument form accepted for forward compatibility.
- `is_indexed_file(file: file) -> boolean | error` — Returns true when the filename stem matches the indexed grammar (base-NNN).
- `file_index(file: file) -> number | error` — Returns the parsed index suffix, or -1 when non-indexed.
- `increment_file_index(file: file) -> file | error` — Increments the numeric index suffix, preserving zero-padding width.
- `decrement_file_index(file: file) -> file | error` — Decrements the numeric index suffix, clamped at 0.
- `basename(file: file) -> string | error` — Returns the final path component including extension.
- `basename_without_index(file: file) -> string | error` — Returns the basename with any indexed suffix removed from the stem.
- `dirname(file: file) -> string | error` — Returns the directory portion of the display path.
- `ext(file: file) -> string | error` — Returns the final extension without the leading dot.
- `parent_dir(file: file) -> string | error` — Returns the directory segment immediately above the basename.
- `file_trailing(file: file) -> string | error` — Returns the last directory segment plus the basename.
- `dir_leading(file: file) -> string | error` — Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing).
- `join(left: string, right: string) -> string | error` — Joins two path strings with normalized separators.
- `link(file: file) -> string | error` — Creates a Markdown link to a local file, using its relative path as the link text.
- `link(target: file, desc: string) -> string | error` — Creates a Markdown link to a local file or HTTP(S) URL with the given description.
- `has_command(cmd: string) -> boolean` — Returns true when the command is found on PATH or is an existing executable absolute path.

**Context**

- `has_skill(name: string) -> boolean` — Returns true when a skill directory exists in a user-scoped or local-scoped skill root.
- `has_local_skill(name: string) -> boolean` — Returns true when a skill directory exists in a local-scoped skill root.

**List Formatting**

- `as_line_separated(list: any[]) -> string | error` — Joins a list into a newline-separated string (the default bare-array rendering).
- `as_csv(list: any[]) -> string | error` — Joins a list into a comma-separated string.
- `as_tsv(list: any[]) -> string | error` — Joins a list into a tab-separated string.
- `as_space_separated(list: any[]) -> string | error` — Joins a list into a space-separated string.
- `as_unordered_list(list: any[]) -> string | error` — Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists.
- `as_ordered_list(list: any[]) -> string | error` — Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists.
```
