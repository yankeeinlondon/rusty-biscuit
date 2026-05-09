# Expression Syntax

The document @darkmatter/docs/title/boolean-conditional-logic.md has been used for a while to document a set of expressions which Darkmatter allows for in:

- conditional clauses (such as the `when` property of several directives)
- interpolation (e.g., `{{ foo || bar }}` or `{{ foo > 5 }}`)

However, naming these expressions as being always "boolean expressions" is no longer true and so in the feature we will:

1. correct the nomenclature to be just `Darkmatter Expressions`
2. add some operations to this current set to round out our capabilities

## New Expressions

- somehow when we first implemented this we left out the `<=` operator! We have `==`, `!=`, `>`, `<`, and even `>=` already. In this feature we'll add `<=`
- we need some type predicates to test the types of Frontmatter properties:
    - `IsString(x)`, `IsNumber(x)`, `IsArray(x)`, `IsNull(x)`, `IsObject(x)` will all be added to expression syntax Darkmatter supports
    - we'll add `IsEmpty(x)` which evaluates to **true** when the value is null or an empty string
    - `IsDate(x)` will validate that `x` is a ISO Date of the format YYYY-DD-MM
        - we will also have validations of particular dates such as:
            - `IsToday(x)`, `IsYesterday(x)`, `IsTomorrow(x)`, `IsThisMonth(x)`, `IsThisYear(x)`
    - `IsDateTime(x)` will validate that `x` is a valid ISO Datetime string
- we also need some basic arithmetic operations:
    - `+` for addition and the `++` unary operator for incrementing by 1
    - `-` for subtraction and the `--` unary operator for decrementing by 1
    - `*` for multiplication
    - `/` for division
    - `%` for modulus operation
- a few math helpers:
    - `round(x)` already exists but we'll add ...
    - `min(a,b)` the minimum numeric value from two sources (static value or Frontmatter reference)
    - `max(a,b)` the maximum numeric value from two sources
    - `abs(x)`
- we also need to provide element access to both objects and lists:
    - we will allow "dot-syntax" to reach into properties of a dictionary: `foo.bar.baz`
        - if an non-existent path is referenced it will resolve to `null` not cause an error
    - for lists we will allow access via a bracketed syntax like `foo[0]` or with the dot syntax of `foo.0`
    - lists will also get `first(x)`, `last(x)`
-we also need a few more string validations:
    - we already have `Contains(items, find)`
    - but now we will add:
        - `StartsWith(x, find)`
        - `EndsWith(x, find)`
    - we'll also add a few string mutations to help with comparisons like:
        - `Lower(x)`, `Upper(x)`, `Capitalize(x)`, 
        - `KebabCase(x)`, `CamelCase(x)`, `PascalCase(x)`, `SnakeCase(x)`, `TitleCase(x)`
