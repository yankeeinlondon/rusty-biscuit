# Expression Syntax

The document @darkmatter/docs/title/boolean-conditional-logic.md has been used for a while to document a set of expressions which Darkmatter allows for in:

- conditional clauses (such as the `when` property of several directives)
- interpolation (e.g., `{{ foo || bar }}` or `{{ foo > 5 }}`)

However, naming these expressions as being always "boolean expressions" is no longer true and so in the feature we will:

1. correct the nomenclature to be just `Darkmatter Expressions`
2. add some operations to this current set to round out our capabilities

## Operator Precedence

Operators are evaluated in the following precedence order, from highest to lowest:

1. **Function calls / primary** — `foo[0]`, `foo.bar`
2. **Unary** — `!`
3. **Multiplicative** — `*`, `/`, `%`
4. **Additive** — `+`, `-`
5. **Comparison** — `==`, `!=`, `>`, `>=`, `<`, `<=`
6. **Logical AND** — `&&`
7. **Logical OR / Fallback** — `||`
8. **Ternary** — `? :`

## Operator Associativity

- All **binary operators** are **left-associative**. For example, `a - b - c` is evaluated as `(a - b) - c`, and `a || b || c` is evaluated as `(a || b) || c`.
- The **ternary operator** `? :` is **right-associative**. For example, `a ? b : c ? d : e` is evaluated as `a ? b : (c ? d : e)`.

## Truthiness

The following values are considered **falsy**:

- `null`
- `false`
- `0`
- `0.0`
- `""` (empty string)
- `[]` (empty array)
- `{}` (empty object)

All other values are **truthy**. This makes `||` a true fallback operator: `a || b` evaluates to `a` when `a` is truthy, and `b` otherwise.

## New Expressions

- somehow when we first implemented this we left out the `<=` operator! We have `==`, `!=`, `>`, `<`, and even `>=` already. In this feature we'll add `<=`
- we need some type predicates to test the types of Frontmatter properties:
    - `IsString(x)`, `IsNumber(x)`, `IsArray(x)`, `IsNull(x)`, `IsObject(x)` will all be added to expression syntax Darkmatter supports
    - `IsEmpty(x)` evaluates to **true** for `null`, `""` (empty string), `[]` (empty array), and `{}` (empty object). It evaluates to **false** for all numbers (including `0` and `0.0`), booleans (including `false`), and all non-empty containers. Scalars are not considered containers.
    - `IsDate(x)` will validate that `x` is a ISO Date of the format `YYYY-MM-DD` using the **local timezone**
        - `IsDateUtc(x)` validates the same format using **UTC**
        - we will also have validations of particular dates such as:
            - `IsToday(x)`, `IsYesterday(x)`, `IsTomorrow(x)`, `IsThisMonth(x)`, `IsThisYear(x)` — all evaluated in the **local timezone**
            - `IsTodayUtc(x)`, `IsYesterdayUtc(x)`, `IsTomorrowUtc(x)`, `IsThisMonthUtc(x)`, `IsThisYearUtc(x)` — all evaluated in **UTC**
    - `IsDateTime(x)` will validate that `x` is a valid ISO Datetime string using the **local timezone**
        - `IsDateTimeUtc(x)` validates the same using **UTC**
- we also need some basic arithmetic operations:
    - `+` for addition (or string concatenation if either operand is a string)
    - `-` for subtraction
    - `*` for multiplication
    - `/` for division
    - `%` for remainder operation (C-style semantics: the sign of the result follows the dividend / left operand; e.g., `-5 % 3 == -2`)
- a few math helpers:
    - `round(x)` already exists but we'll add ...
    - `min(a,b)` the minimum numeric value from two sources (static value or Frontmatter reference)
    - `max(a,b)` the maximum numeric value from two sources
    - `abs(x)`
- we also need to provide element access to both objects and lists:
    - dot-syntax is used to reach into named properties of a dictionary: `foo.bar.baz`
        - if a non-existent path is referenced it will resolve to `null` not cause an error
        - dot-access on a `null` base (e.g., `null.foo`) returns `null` and does not error
        - numeric property access via dot (e.g., `foo.0`) is **not supported**
    - bracket syntax is used for all indexed/collection access:
        - lists: `foo[0]`, `foo[-1]` (negative indexing from the end)
        - objects: `foo["key"]` (string keys)
        - bracket access follows a **null-propagation philosophy**: any invalid bracket access returns `null` and never errors
            - out-of-bounds index (e.g., `items[-1]` on an empty array) → `null`
            - index on a null base (e.g., `items[0]` where `items` is `null`) → `null`
            - key access on a non-collection (e.g., `config["key"]` where `config` is a string) → `null`
    - lists will also get `first(x)`, `last(x)`
        - `first(x)` returns the first element of array `x`, or `null` if `x` is empty
        - `last(x)` returns the last element of array `x`, or `null` if `x` is empty
-we also need a few more string validations:
    - we already have `Contains(items, find)`
    - but now we will add:
        - `StartsWith(x, find)`
        - `EndsWith(x, find)`
    - we'll also add a few string mutations to help with comparisons like:
        - `Lower(x)`, `Upper(x)`, `Capitalize(x)`, 
        - `KebabCase(x)`, `CamelCase(x)`, `PascalCase(x)`, `SnakeCase(x)`, `TitleCase(x)`

## Timezone & Date/DateTime Behavior

- The local system timezone is detected using the `sniff` library (already a dependency).
- The default behavior for all date and datetime operators uses the **local timezone**.
- UTC variants (suffix `Utc`) are provided for all date/datetime operators.
- Datetime values with **no offset**:
    - Treated as **local time** when using the base (non-UTC) variant
    - Treated as **UTC** when using the UTC variant

### Date Validator Input Contracts (Tiered)

Date validators are divided into two tiers with different input contracts:

**Strict format validators:**
- `IsDate(x)` and `IsDateTime(x)` (and their UTC variants) accept **strings only**
- Return `false` for non-string inputs, including `null`
- Return `false` for strings that do not match the expected exact format

**Relative validators:**
- `IsToday(x)`, `IsYesterday(x)`, `IsTomorrow(x)`, `IsThisMonth(x)`, `IsThisYear(x)` and their UTC variants accept **both date and datetime strings**
- When given a datetime string, they extract the date portion for comparison
- Return `false` on `null` or any invalid input
- Use the operator's timezone semantics (local or UTC) for the reference date

## Error Handling

The following arithmetic error conditions produce clearly articulated errors:

- **Division by zero** — evaluating `x / 0` or `x % 0` results in a division by zero error
- **Non-numeric operands** — attempting arithmetic with `+` (when neither operand is a string), `-`, `*`, `/`, or `%` on values that are null, boolean, object, or array results in an error
- All arithmetic operators (`+`, `-`, `*`, `/`, `%`) require numeric operands, with the sole exception that `+` performs string concatenation when either operand is a string

### Null-Safety and Type-Mismatch Handling for Functions

All functions listed below follow a consistent null-safety and type-mismatch contract:
- **Null argument propagation**: If any argument to the function is `null`, the function returns `null`
- **Type-mismatch error**: If any argument has the wrong type for the function's domain, the function returns an error

This contract applies to:
- Math functions: `min(a, b)`, `max(a, b)`, `abs(x)`
- Collection functions: `first(x)`, `last(x)`
- String predicates: `StartsWith(x, find)`, `EndsWith(x, find)`
- String mutations: `Lower(x)`, `Upper(x)`, `Capitalize(x)`, `KebabCase(x)`, `CamelCase(x)`, `PascalCase(x)`, `SnakeCase(x)`, `TitleCase(x)`
