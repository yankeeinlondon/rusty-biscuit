---
status: draft-spec
parent: ../2026-09-16-expression-type-system/spec.md
peers: ./coercion-design.md
research:
    - ./current-state-coercion.md
related:
    - 2026-09-16-expression-type-system
    - 2026-09-17-remove-strict-mode
    - 2026-05-28-schema-coercion
---

# Schema Enhancements

The current `SimplifiedSchema` is very helpful but it has some constraints that make it less capable and ergonomic than it could be. This specification tries to address that as well as some impacts to how Darkmatter and DMLS (Darkmatter's language server) build their internal model for function types.

> For reference, here is the most authoritative document we currently have for our schema support:
>
> - [Schema Definition](../../docs/topics/schemas/definition.md)

## Summary and Goals

This is the detail spec for **Phase B** ("function schemas in SimplifiedSchema") of the parent spec [2026-09-16-expression-type-system](../2026-09-16-expression-type-system/spec.md). Where the parent's Phase B prototype and this spec differ, this spec wins.

The goals are:

1. **Richer grammar.** Add `tuple` and `function` types, a compact `|` union usable anywhere a type is written, a `category` constraint, a function `example` form, and a top-level `categories:` list for schema files.
2. **One function catalog.** Replace Darkmatter's current expression-function catalog with a catalog written in the new grammar, housed in Darkmatter.
3. **One coercion engine.** Darkmatter has exactly one set of conversion rules. The same engine converts frontmatter values when a document is validated against its `$schema`, and converts arguments before a function is called. Functions declare the types they need and stop converting their own inputs. **Every** catalog function moves onto the engine in one pass; there is no staged rollout.
4. **Proportionate design-time checking.** Darkmatter classifies every argument into one of five categories, rejects documents containing calls that are proven wrong, and exposes the classification through an API. DMLS uses that API to show the same verdicts as the author types, with a strict-mode setting for authors who want more warnings. Composition and the editor must never disagree about a call. _(Editor work moved into this spec in clarification round 5.)_

Three documents in this directory work together:

| Document | Role |
|---|---|
| `spec.md` (this file) | **Normative.** Its requirements, decisions, and rulings govern. |
| [coercion-design.md](./coercion-design.md) | Supporting design detail for the coercion engine. It must conform to this spec; where they conflict, this spec wins. It is not itself a confirmed contract. |
| [current-state-coercion.md](./current-state-coercion.md) | A record of how functions treat their inputs today. It is a baseline, not a proposal. |

Nothing in this spec is implemented yet.

## Terminology

This spec is careful to separate what is known while a document is being _written_ from what is known while it is being _run_.

- **Design-time type** — what a schema or signature says a value can be, known before anything runs. `number`, `string(min(3))`, and `any` are design-time types. An editor can only ever see these.
- **Runtime value** — the actual value that exists when a document is validated or an expression is evaluated, such as `4`, `"4"`, `"pear"`, or `null`. The same design-time type `string` covers both `"4"` and `"pear"`.
- **`any`, `unknown`, and `null`** — formal SimplifiedSchema types, added by the parent spec's Phase A. `unknown` admits every value, including null; `any` is an accepted alias for it. `null` admits only null. An untyped frontmatter variable has the design-time type `unknown`, and an optional typed property declared `number` has the design-time type `number | null`.
- **Text** — a string value. "Refined text" is a text type with its own validation: `file`, `date`, `url`, `email`, `enum(...)`, string literals, and the content types `json` and `yaml`.
- **Coercion engine** — Darkmatter's single shared set of conversion rules. Given a runtime value and a target type, it either keeps the value, converts it, or reports a type error. It serves two surfaces: frontmatter validation and function calls.
- **The two interfaces of a type** — every parameter type (and every frontmatter property type) describes two separate contracts, detailed in [Two Interfaces per Type](#two-interfaces-per-type):
    1. **Accepted inputs** (caller → coercion engine): which runtime values a caller may supply. These are the conversion rules.
    2. **Delivery guarantee** (coercion engine → function): what the function, or the validated document, is guaranteed to receive.
- **Union option** — one alternative of a union: `string` and `number` are the options of `string | number`. Also called an _arm_.
- **Binding** — the coercion engine's work on a function call: matching the caller's runtime values to the function's parameters. A call "binds" when every argument ends up satisfying its parameter. The function runs only when the call binds.
- **Domain behavior** — what a function does with arguments that have already bound: its computation, its handling of `null` where it declares `| null`, and any error its own work can produce.
- **Fallible** — a function that can return an error even when every argument bound, because its own work can fail (a file is unreadable, a network is down). A type mismatch alone never makes a function fallible; that error belongs to the coercion engine.
- **Argument-type category** — one of five design-time verdicts on "does this argument's type fit this parameter's type?", described in [Argument-Type Categories](#argument-type-categories).
- **Composition** — Darkmatter's processing of a document into its final output. Schema validation runs at the start of composition, before any expression is evaluated.
- **Overload** — one of several alternative signatures for the same function name, such as `pr_list(count)` and `pr_list(query)`.
- **Constraint** — a refinement written inside a type's parentheses, such as `min(1)`, `max(3)`, `pattern(re)`, `integer`, or `required`. Some constraints (`category`, `example`) carry metadata rather than restricting values.
- **Category** — a named group used to organize items in documentation, such as `Math` or `Filesystem`. Not to be confused with an _argument-type_ category.

## Scope

### In scope (definition of done for this spec)

1. **The grammar:** `tuple`, `function` (with `parameters`, `returns`, `category`, `fallible`, and `example`), `|` unions everywhere, the `category` constraint, and the top-level `categories:` list.
2. **The catalog swap:** the new catalog, complete and moved into Darkmatter, replaces `darkmatter/docs/schemas/expression-functions.yaml` everywhere it is consumed.
3. **The shared coercion engine,** with frontmatter validation moved onto it. Today's frontmatter rules (`coerce_frontmatter`, from feature 2026-05-28-schema-coercion) are replaced by the single rule list in [The Coercion Engine](#the-coercion-engine). The intentional behavior changes are listed in [Changes to Frontmatter Coercion](#changes-to-frontmatter-coercion).
4. **The one-pass migration** (_revised in clarification round 4_): **every** catalog function (about 110, including the 14 missing from the draft) moves onto the shared coercion engine in a single pass, with corrected signatures. Every function's own coercion code is removed, the old catalog is deleted, and the [per-function items](#per-function-items-for-the-catalog-migration) are resolved. See [One-pass Migration](#one-pass-migration).
5. **Design-time checking in Darkmatter:** the five-category [compatibility API](#design-time-compatibility-api), and rejection at composition of any document containing a category 5 call to **any** function.
6. **Editor diagnostics in DMLS** (_added in clarification round 5_): DMLS shows the argument-type categories as the author types, using the category-to-diagnostic mapping in [Argument-Type Categories](#argument-type-categories), and gains the strict-mode setting described in [Editor Diagnostics](#editor-diagnostics).

### Out of scope for this spec / handed to parent Phase E

_Revised in clarification round 5: DMLS diagnostics for function calls and the strict-mode setting moved into this spec._

These remain with the parent spec's **Phase E** (diagnostics and CLI) and its later increments:

- retiring the interim null-suppression rule the parent spec already assigns to Phase E;
- `file_exists`'s own always-false warning for provably malformed literals, such as `file_exists("https://")` or the empty string. _(Revised in clarification round 6: the parent's advisory exception for known unsupported types and its opt-in unknown-input warning are retired; see [`file_exists` Has No Exception](#file_exists-has-no-exception).)_
- the CLI's diagnostic policies, shared diagnostic codes, and report format.

### Not expressed in the catalog grammar

Two function facts are deliberately **not** part of the `function(...)` grammar. They are recorded in Darkmatter's Rust function registration instead:

- **Lazy evaluation.** `and` and `or` stop evaluating their arguments as soon as the answer is known. A signature cannot say this.
- **Type-predicate narrowing.** When `is_number(x)` returns `true`, `x` is known to be a number inside the guarded branch. A `returns(boolean)` alone does not express this.

## Requirements

### Grammar: Types Added

> Note:
>
> - one consistency concession I think we need to make for ergonomics is that function parameters and tuple elements should be considered required unless specified as optional.
> - for people more involved in language structure this may seem like an obvious thing but for others it may be a surprise
> - however I think that most of the people who would be writing schemas for functions would be a very limited subset and probably leaning toward more advanced in language design
> - in a _way_ this isn't actually a full consistency break: both functions and tuples are describing _elements_ of a property in their own constraint block versus property definitions which are at a different level

#### Tuples

Today we provide for array types -- and the constraint system _does_ provide the ability to specify a min and max length -- but arrays and tuples are only related cousins, not the same type category.

The **type** will be called `tuple` and through its constraint system we will support not only fixed length tuples but also variadic types.

- the basic syntax for a fixed length tuple would look like:

    ```yaml
    my_tuple: tuple([string,number,boolean])
    ```

- if we wanted to make one or more of the trailing elements _optional_ we would use the `?` operator:

    ```yaml
    my_tuple: tuple([string,number?,boolean?])
    ```

- it's important to understand that the property `my_tuple` is _optional_ because all schema types are optional by default. If I wanted the property to be required I'd use precisely the same grammar as you'd use for any other property:

    ```yaml
    my_tuple: tuple([string,number?,boolean?]; required)
    ```

- an element may be a union, written with `|` (see [The `|` Union Operator](#the--union-operator)):

    ```yaml
    my_tuple: tuple([string, number | boolean])
    ```

- tuple elements cannot be named; a tuple is a positional shape.

##### Variadic Tuples

To allow for a "variadic tuple" (a tuple shape with a variadic component that makes its _length_ indeterminate) we again lean into TypeScript's grammar. Here's a simple example:

```yaml
my_tuple: tuple([string, ...number[]])
```

- the `...` prefix operator indicates that the _type_ it is operating on is _spread_ into the type definition.
- to keep complexity within bounds the grammar only allows _one_ spread per tuple, and it must be the last element.
- the `?` operator must not be applied to a variadic element because it's redundant: every variadic element is optional by nature (it may match zero values).

Element order is therefore fixed: required elements, then optional (`?`) elements, then at most one spread. Any other order is a schema-load error.

##### Tuples in JSON Schema

Tuples are valid anywhere a type is, including as property types in a document's `$schema` (for example `point: tuple([number, number])`). They compile to standard JSON Schema: `prefixItems` for the positional elements, `minItems` for the count of required elements, and `items` for what follows (`false` for a fixed-length tuple, the spread's element schema for a variadic one).

#### Functions

Although functions will not be a strong requirement for many Markdown documents' schemas, we have an immediate need to express function signatures.

The easy part is the generic **type**, which is just called `function`. The complications arise in creating a set of **constraints** that allows ergonomic, understandable syntax while complementing the rest of the grammar.

**A `function` type is description-only.** It describes a callable; it never validates data:

- it may appear as a named type in a schema file's `types:` mapping, and nested inside another function's parameters or return type or inside a tuple element;
- a document schema (`$schema` in a Markdown file's frontmatter) that declares a function-typed property is a **schema-load error**, because no frontmatter value can be a function;
- no JSON Schema output is produced for `function`.

Parsing a `function(...)` type produces a description only. It does not create an executable function; a Darkmatter function also needs a registered Rust implementation.

`parameters(...)` and `returns(...)` are both **required**; leaving either out is a schema-load error.

##### Constraint System

1. `parameters`

    The parameters constraint is a comma-separated list of name/type pairs:

    ```yaml
    doit: function( parameters(name: string, age: number); returns(boolean) )
    ```

    In the above example both `name` and `age` are required parameters. To make a parameter optional we borrow TypeScript's grammar:

    ```yaml
    doit: function( parameters(name: string, age?: number); returns(boolean) )
    ```

    The `?:` symbol marks `age` as an _optional_ parameter. Optional parameters must follow all required parameters.

    > Note: being an "optional parameter" (it may be omitted) is distinct from having a union type that includes `null` (it may be passed an explicit `null`). `age?: number | null` allows both.

    A final **rest parameter** collects any remaining arguments. The type after `:` describes _each_ remaining argument, so `...values: number` means zero or more numbers:

    ```yaml
    and: function( parameters(...values: any); returns(boolean) )
    ```

    When a function takes no parameters it is written with `void`:

    ```yaml
    no_input: function( parameters(void); returns(string) )
    ```

    `parameters(void)` is the only spelling. The draft catalog's `parameters()` and its `name: type(optional)` forms must be normalized to `parameters(void)` and `name?: type` during migration.

    Parameter types should say what the function _needs_, not what it is willing to put up with. Use `number` when the function computes with numbers; the [coercion engine](#the-coercion-engine) takes care of callers who pass `"4"`. Use `numberlike` (or `boolish`) only when the function genuinely needs the caller's original form, for example to return a value of the same type the caller sent (see [`numberlike` and `boolish`](#numberlike-and-boolish)).

2. `returns`

    This types the value a function returns on success. Any valid type/constraint combination is allowed, including a union:

    ```yaml
    doit: function( parameters(name: string, age?: number); returns(boolean | enum("banned","underage")) )
    ```

3. `category`

    Associates the function with a documentation category. This constraint is available to every type and is described in [Categories](#categories).

4. `fallible`

    A flag with no arguments. Fallibility indicates whether a function -- when given valid inputs -- can return an error instead of its declared return type.

    "Valid inputs" means the arguments bound: they either matched the parameter types exactly, or the coercion engine converted them without changing their meaning. A string `"4"` passed to a parameter expecting a number is valid input, because the caller's intent is not in question; this matters especially in a medium where many values arrive as strings (shell command output, values passed on the command line).

    Because invalid inputs are rejected before the function runs, a function whose only failure is "wrong input type" is **not** fallible. `min(a: number, b: number): number` therefore should not be marked `fallible`: it is infallible whenever its type contract is met, and the coercion engine guarantees it is never called otherwise.

    It can help to think of a fallible function's result as `T | error`, where `T` is its `returns` type. This is illustrative only: `error` is not a SimplifiedSchema type and cannot be written in a schema.

5. `example`

    Functions benefit more from _examples_ than other types do, so `function` accepts an inline example form. A function may carry any number of examples (zero or more).

    An inline example is a call, `=>`, and the expected result:

    ```yaml
    example(min(3, 5) => 3)
    ```

    - The result is written as a Darkmatter expression literal: `3`, `"3"`, `[1, 2]`, `{a: 1}`, or `null`.
    - Results are compared **structurally**: `3` is not `"3"`, and the key order of an object does not matter.
    - An example whose expected outcome is an error is written `=> error`, as in `example(min("pear", 5) => error)`.
    - Such an example is **executable**: tests evaluate the call and require the declared result.
    - An example that cannot run reliably in a test (it needs a network, the local clock, or a particular repository history) adds `display-only:` and a short reason. It is shown in documentation but never run:

        ```yaml
        example(ping("192.168.1.1") => true; display-only: needs network)
        ```

    The existing file form, `example(./file.yaml)`, which attaches example _files_ to any property (see [Schema Definition](../../docs/topics/schemas/definition.md#example-constraint)), is also allowed on `function`. The two forms coexist and are told apart by the `=>`. An example whose result spans several lines uses the file form.

    This replaces the old catalog's separate `verification: executable | display-only` and `reason` fields.

##### Overloads

A function with more than one signature is written as a union of `function(...)` types, using either union syntax (a YAML list, or `|`):

```yaml
pr_list:
    - |-
        function(
            parameters(count: number(integer; min(1)));
            returns(string[]);
            category(Devops);
            fallible;
        ) -> Lists the most recent pull or merge requests.
    - |-
        function(
            parameters(query: object);
            returns(string[]);
            category(Devops);
            fallible;
        ) -> Lists pull or merge requests matching a structured query.
```

Which signature a call uses is decided by the order-independent rules in [Overload Selection](#overload-selection); reordering the list never changes the outcome. Two overloads with identical parameter lists are a **catalog-load error**.

##### Example

```yaml
min: |-
    function(
        parameters(a: number, b: number);
        returns(number);
        category(Math);
        example(min(3, 5) => 3);
        example(min(-2, 0.5) => -2);
    ) -> Returns the smaller of two numbers.
```

### The `|` Union Operator

Schema _properties_ already have a syntax for union types (a YAML list), but it is awkward inside a constraint block. The `|` operator separates the options of a union, as in `string | number`, and is allowed **everywhere a type is written**: property types in a document's `$schema` (`id: string | number`), function parameters and returns, tuple elements, and array elements.

- The YAML-list union remains valid and means exactly the same thing as `|`.
- A `|` union compiles to JSON Schema `anyOf`.
- Composition follows TypeScript: `[]` binds tighter than `|`, so `string | number[]` means "a string, or a list of numbers". Parentheses group: `(string | number)[]` means "a list whose elements are strings or numbers".

A function should be as precise about its type needs as possible. Declaring `string | number` just to be accommodating makes the function responsible for converting its own input, which is exactly what the coercion engine exists to prevent; it also admits text such as `"pear"`. A function that needs "a number, or text holding a number, exactly as sent" declares `numberlike` instead.

> Note: just because we have more ways to create union types doesn't mean we _want_ more union types! Union types are expensive to represent compared to non-union types, so use them where you need them but don't overdo it.

### Categories

#### The `category` Constraint

`category(Name)` associates any property in a schema with a category. It is metadata rather than a constraint. While the need came from Darkmatter's function catalog, this kind of metadata has good reuse potential, so it is available on every type.

- The name is the raw text up to the closing `)`, with surrounding whitespace trimmed. It may contain spaces and punctuation (`Date Arithmetic`, `CI/CD`) and is case-sensitive.
- On a non-function property, the category is emitted in JSON Schema as the Darkmatter annotation `x-darkmatter-category`.

#### The `categories:` List

Any schema file may declare a top-level `categories:` list beside `types:`. Each entry is a name, optionally followed by `-> description`. A description may span several lines:

```yaml
kind: schema
categories:
    - Math -> Arithmetic on numbers.
    - Devops
    - |-
        Filesystem ->
        Questions about files and paths, such as whether a file
        exists or what kind of file it is.
types:
    min: |-
        function(
            parameters(a: number, b: number);
            returns(number);
            category(Math);
        ) -> Returns the smaller of two numbers.
```

- When a file declares `categories:`, a `category(X)` that names a category not in the list is a **schema-load error**.
- When a file declares no `categories:`, any category name is accepted.
- Documentation shows categories in **list order**, and items **alphabetically** within each category.

The [Schema Definition](../../docs/topics/schemas/definition.md#malformed-envelopes) rules currently treat unsupported top-level keys beside `types:` as an error. That rule must be relaxed to allow `categories:`.

### Function Catalog Migration

Today Darkmatter embeds `darkmatter/docs/schemas/expression-functions.yaml` at build time and uses it to describe its expression functions. The draft replacement is `claudine/docs/schemas/partials/functions.yaml`.

Requirements:

1. **Home.** The new catalog moves to `darkmatter/schemas/` (for example `darkmatter/schemas/functions.yaml`), in line with [2026-09-17-remove-strict-mode](../../../claudine/fixes/2026-09-17-remove-strict-mode/spec.md). Its companion `claudine/docs/schemas/partials/mutations.yaml` describes Darkmatter's document mutations and moves with it. Darkmatter embeds the catalog; Claudine reads it through Darkmatter's library API rather than from its own copy.
2. **Consumers.** Every current consumer switches to the new catalog: the Darkmatter library, DMLS, and `claudine context --expressions`.
3. **Discovery.** Both files are types-only schema files. Types-only files never auto-apply to documents; 2026-09-17-remove-strict-mode already guarantees this, so no extra exclusion is needed.
4. **Completeness.** The draft is missing 14 functions that are still implemented. Each must be added: `has_binary`, `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli`, `package`, `package_area`, `recent_commits`, `ipv4`, `ipv6`, `ping`, `ping_under`, and `as_markdown`.
5. **Examples carried over.** All 117 examples in the old catalog are carried over: the 72 marked `verification: executable` as executable examples, and the rest as `display-only:` examples with their existing reason, or as file-form examples where the result spans several lines.
6. **Categories.** The draft's `categories:` list gets a real description for every entry (several are placeholders) and gains every category the catalog uses: today `Git`, `CI/CD`, and `Date Arithmetic` are used but not listed, and the missing network functions need a category such as `Network`.
7. **Dropped fields.** The old catalog's `order` field is not needed: documentation groups by category and sorts alphabetically within each category. `mutations.yaml` drops its top-level `name:` key.
8. **Return shapes.** The old `literals` and `nullable` return fields (used only by `ping` and `ping_under`) become `|` unions, for example `returns(boolean | enum("unstable") | null)` for `ping_under`.
9. **Shared types instead of `pair`.** The old `pair` field linked `recent_commits(count)` to the `ctx.recent_commits` variable. It is dropped: the function's return and the variable use one shared named type, and each description cross-references the other.
10. **Grammar normalization.** `parameters()` becomes `parameters(void)`, and `name: type(optional)` becomes `name?: type`. Accommodation-only `any` parameters are narrowed to what the function needs. A parameter keeps (or gains) `numberlike` or `boolish` only where the function genuinely needs the caller's original form, such as to return the same type it was given; otherwise it becomes `number` or `boolean`.
11. **Draft corrections.** Signatures and descriptions must agree:
    - `length` describes strings, arrays, and objects but its signature omits objects; it becomes `length(val: string | any[] | object): number`.
    - Every description must describe its own function. `is_string` was reported as carrying `max`'s description ("Returns the larger of two numbers"); verify each entry against its implementation.
    - Functions marked `fallible` only because of argument rejection (such as `min`) lose the flag.
12. **Per-function items.** The domain-behavior items in [Per-function Items for the Catalog Migration](#per-function-items-for-the-catalog-migration) are resolved within this spec, as part of the [one-pass migration](#one-pass-migration).

### The Coercion Engine

_Confirmed in the use-case review of 2026-09-25; the rule list was revised in clarification round 3 the same day. Round 4 added the two-interface framing, kept `numberlike` and `boolish`, and restated the union rule._

#### The Governing Rule

> Darkmatter has **one** coercion engine and **one** rule list. It serves frontmatter validation and function calls alike, with no per-surface exceptions.
>
> The engine converts a value only when the conversion has **not** changed the meaning of what was supplied. A function is **never called** unless its arguments satisfy the types and constraints it declared.

Today there are two partial rule sets. Frontmatter validation has its own coercion (`coerce_frontmatter`), and function calls have none: some functions share strict type checks, some share conversion helpers, and a few convert their inputs in their own way (see [current-state-coercion.md](./current-state-coercion.md)). A declared type alone does not tell a reader what is accepted. This spec replaces both with the single engine.

#### Two Interfaces per Type

_Added in clarification round 4 (2026-09-25)._

A parameter type answers two different questions, and they must not be confused:

| Interface | Question it answers | Who relies on it |
|---|---|---|
| 1. **Accepted inputs** (caller → engine) | Which runtime values may a caller supply? | Document authors and callers. Governed by the [conversion rules](#conversion-rules). |
| 2. **Delivery guarantee** (engine → function) | What exactly will the function receive? | The function's implementation. |

For most types the two interfaces differ only by conversion. A `number` parameter **accepts** a real number or text that converts to one (`4`, `"4"`, `" 4 "`), and **delivers** a real number, always. The function never sees text, so it cannot tell whether the caller wrote `4` or `"4"`.

Sometimes that difference matters to the function, which is why [`numberlike` and `boolish`](#numberlike-and-boolish) exist: they accept exactly what `number` and `boolean` accept, but deliver the value exactly as the caller sent it.

Frontmatter works the same way: interface 2 describes the value a validated document holds after the engine has run.

#### Three Actions per Value

For each value (a frontmatter property, or a function argument), the engine takes exactly one of three actions:

1. **Keep unchanged** — only when the runtime value already matches the target type and constraints.
2. **Convert** — apply a [conversion](#conversion-rules) into the target type, then check the converted value against the type and constraints.
3. **Report a type error** — when neither of the above succeeds.

The engine checks **every** value before deciding. A failure in one never stops it from checking the others, and the resulting type error reports each value that failed.

The caller's or document's original value is never mutated; the engine produces a converted copy.

#### Two Steps for a Call: Bind, Then Compute

Every function call runs in two separate steps:

| Step | Owner | Input | Output |
|---|---|---|---|
| 1. Bind | the coercion engine | the caller's runtime values, of any type | values of exactly the declared types, **or** a type error |
| 2. Compute | the function | values of exactly the declared types | the declared return type, **or** (only if `fallible`) a domain error |

A type error always comes from step 1. A domain error always comes from step 2: a type error means "this function was not given what it declared"; a domain error means "this function was given what it declared and its own work failed."

#### Domain Behavior Is Not the Engine's Concern

What a function returns for inputs that bound, and whether it is `fallible`, is the function's own contract.

Take `number(val: any, fallback?: number): number` called with the runtime text `"pear"`:

- **Step 1.** `val` is declared `any`, so `"pear"` is kept unchanged. There is nothing to convert to.
- **Step 2.** What `number()` does with `"pear"` is its domain behavior, settled per function in the [catalog migration](#per-function-items-for-the-catalog-migration).

The same reasoning applies to `and(...values: any)` and `or(...values: any)`: there is nothing to convert, their short-circuiting is registered in Rust, and their truthiness rules (whether `"no"` counts as true) are domain behavior.

A parameter declared `any` is right only when inspecting the original value **is** the function's purpose. `is_string(val: any): boolean` must see the original value; converting it first would make the answer meaningless. `add(a: number, b: number): number` should never declare `any`.

##### `file_exists` Has No Exception

_Added in clarification round 6 (2026-09-25)._

`file_exists(value: string | file | null): boolean` binds through the engine like every other function. Given the number `42`, number → text is always safe, so `value` receives the text `"42"` and `file_exists` checks for a file named `42`. Both `string` and `file` are reachable, and both receive the same text, so there is no tie to break. At design time a `number` argument is category 1: no editor diagnostic, in either setting.

This retires the parent spec's advisory exception, under which a known unsupported type such as a number produced an always-false warning and a runtime `false` without a type error. One engine with a special case for one function is two engines; a reader could no longer learn what a function accepts from its declared type. It also retires the parent's separate opt-in warning for `unknown` arguments to `file_exists`: [`expressions.strict`](#editor-diagnostics) already warns for every category 4 argument, `file_exists` included, so no `file_exists`-specific setting exists.

What stays is `file_exists`'s own domain behavior: a provably malformed literal, such as `"https://"` or the empty string, returns `false` before any filesystem or network access, and the parent spec's always-false warning for such literals remains.

#### Null Is a Type Like Any Other

`null` is a formal type. The engine never converts `null` into anything (`0`, `false`, `""`, `"null"`, or omission).

- A function that wants to receive `null` declares it: `min(a: number | null, b: number | null): number | null`. The engine keeps `null`, and the function handles it in its compute step.
- A function that does not declare `| null` on a parameter never sees a `null` there. A null argument to such a parameter is a type error.

**Example.** A document declares `low: number` (optional, so its design-time type is `number | null`) and leaves `high` untyped. The frontmatter omits `low` and sets `high: pear`. The call `min(low, high)` against `min(a: number, b: number): number`:

- `a` receives `null`. `min` does not declare `| null`, so this fails.
- `b` receives the text `"pear"`. It does not convert to a number, so this fails.
- Result: one type error reporting both `a` and `b`. `min` is never called.

#### Conversion Rules

_Revised in clarification round 3 (2026-09-25): this list supersedes the earlier "only text → number and text → boolean" ruling._

These are the only conversions the engine performs. Each is a direct, one-step mapping; conversions are never chained.

| From (runtime value) | To (target type) | Result | Design-time category when the source is typed |
|---|---|---|---|
| number | text | canonical text: `2026` → `"2026"`, `1.2` → `"1.2"` | 1 (always safe) |
| boolean | text | `true` → `"true"` | 1 (always safe) |
| text | number | `"4"`, `" 4 "`, `"+4"`, `"-3.5"`, `"1e3"`, `".5"`, `"1_000"` | 3 (the text may not convert) |
| text | boolean | `true`/`false`, `yes`/`no`, `on`/`off`, `1`/`0`, any letter case | 3 (the text may not convert) |
| object, list, or scalar | `json` or `yaml` | the value serialized as JSON or YAML text | 1 (always safe) |

**Number and boolean → text** never changes meaning: every number and boolean has exactly one text form. `upper(2026)` returns `"2026"`, and frontmatter `version: 1.2` against `version: string` becomes `"1.2"`. The opposite direction is where care is needed.

> Caveat: the engine sees the value YAML produced. `version: 1.10` is already the number `1.1` by the time the engine runs, so it becomes `"1.1"`. Authors who need the exact spelling should quote it.

**Text → number:**

- Surrounding whitespace, a leading sign, an exponent, a leading dot, and underscore digit separators are accepted.
- `NaN`, infinity, hexadecimal, and locale separators such as `"1,000"` are not numbers.
- **Precision.** _Whole-number text_ means text with no decimal point and no exponent. It becomes an exact 64-bit integer when it fits: `"9007199254740993"` becomes exactly `9007199254740993`. Whole-number text that is out of range, or that would lose digits, is a type error; silently changing a number would change its meaning. Everything else (`"0.1"`, `"4.0"`, `"1e3"`) takes the ordinary floating-point path.

**Text → boolean:** the words are matched case-insensitively (`"Yes"`, `"FALSE"`, `"On"`) after trimming surrounding whitespace.

**Serializing into `json` and `yaml`** keeps the value's meaning; it only changes its representation. Text given to a `json` or `yaml` target is already content: it is validated as JSON or YAML and never re-quoted into a string literal.

**Refined text.** A refined text target (`file`, `date`, `url`, `email`, `enum`, string literals) first accepts text, including text produced by number → text or boolean → text, and then runs its own validation. A number sent to a `date` becomes text and then fails the date check in the normal way.

These conversions are explicitly **rejected**:

| Rejected conversion | Example | Why |
|---|---|---|
| boolean → number | `true` → `1` | A boolean is not a count. |
| null → anything | `null` → `0` or `"null"` | See [Null Is a Type Like Any Other](#null-is-a-type-like-any-other). |
| wrapping a single value into a list | `"a"` → `["a"]` | A list target needs a list. |
| parsing text into an object or list | `"[1, 2]"` → `[1, 2]` | Text is not structure. |

Exact spelling edge cases (trailing dot, doubled underscores, whitespace inside boolean words) are detailed in [coercion-design.md](./coercion-design.md#numeric-and-boolean-boundaries).

#### `numberlike` and `boolish`

_Added in clarification round 4 (2026-09-25)._

`numberlike` and `boolish` are kept as types in their own right, distinct from `number` and `boolean`. They differ only in their [delivery guarantee](#two-interfaces-per-type):

| Type | Accepts (caller → engine) | Delivers (engine → function or document) |
|---|---|---|
| `number` | a real number, or text the engine would convert to a number | a real number, always |
| `numberlike` | the same: a real number, or text the engine would convert to a number (`"4"`, `" 4 "`, `"+4"`, `"1e3"`, `".5"`, `"1_000"`, ...) | the value **exactly as sent**: `5` stays `5`, `"5"` stays `"5"`, `"02134"` stays `"02134"` |
| `boolean` | a real boolean, or a boolean word (`true`/`false`, `yes`/`no`, `on`/`off`, `1`/`0`, any letter case) | a real boolean, always |
| `boolish` | the same: a real boolean, or a boolean word | the value **exactly as sent**: `true` stays `true`, `"Yes"` stays `"Yes"` |

- Acceptance is identical on purpose; this is a feature, not a defect. Whatever the engine would convert for `number`, `numberlike` accepts, and whatever it would reject (`"pear"`, `true`, `null`), `numberlike` rejects. The same holds for `boolish` and `boolean`: a native number `1` is not a boolean, so `boolish` rejects it, while the text `"1"` is accepted.
- `numberlike` and `boolish` never take the **convert** action. A value either is kept unchanged or is a type error.
- Inside a union, a value that `numberlike` or `boolish` accepts is an exact match of that option, because it is delivered unchanged (see [Unions: Choosing an Option](#unions-choosing-an-option)).

**Why keep them.** A function may need to know what the caller sent, for example so its return value can match the caller's type: given `5` it returns a number, given `"5"` it returns text. A `number` parameter only ever sees numbers, so that function cannot know. There is also no other ergonomic way to write "text that may hold only a numeric value": `string | number` admits `"pear"`. And both are familiar types that read well in generated documentation. The name `boolish` stays; it is the name most languages choose.

**In frontmatter**, a `numberlike` or `boolish` property is validated but **no longer normalized**: `zip: "02134"` against `zip: numberlike` stays the text `"02134"`. This is an intentional behavior change (see [Changes to Frontmatter Coercion](#changes-to-frontmatter-coercion)).

#### Constraints Are Enforced, Never Repaired

The engine enforces constraints (lengths, `min`/`max`, `pattern`, `integer`) on the final value, and never "repairs" a value to fit. Doing so would change what was meant.

- `"2.5"` given to `number(integer)` is a type error. It is never rounded.
- `"ABCD"` given to `string(min(3); max(3))` is a type error. It is never truncated.

Constraints are checked at runtime. On their own they never produce a design-time warning (see [Argument-Type Categories](#argument-type-categories)).

#### Unions: Choosing an Option

_Restated in clarification round 4 (2026-09-25); this supersedes the earlier numbered priority list. Clarification round 5 added the second tie-breaker and settled object options whose fields disagree._

When the target is a union, the engine chooses an option like this:

1. **Exact match.** If the value already matches any option (its type and constraints), it is kept unchanged. Several exact matches are harmless: the value is the same either way.
2. **One reachable option.** Otherwise, an option is _reachable_ when the [conversion rules](#conversion-rules) can convert the value into it and the converted value satisfies the option's constraints. If exactly one option is reachable, the value converts into it. If none is, it is a type error.
3. **Tie-breaker.** Only when more than one option is reachable, with different results, is a tie-breaker needed. There are exactly two:
    1. **Number over boolean** for text that reads both as a number and as a boolean (`"1"`, `"0"`) given to a union with both a number and a boolean option.
    2. **JSON over YAML** for a native value (an object, list, number, or boolean) given to a union with both a `json` and a `yaml` option. JSON text is also valid YAML, so the JSON reading satisfies both options.

    Any tie these two do not settle is a type error that names each reading. _(Confirmed in clarification round 6.)_ This safety net protects two promises as the list of conversions grows: the engine never guesses, and the order in which options are written never matters. A new conversion that makes two options reachable where only one was before surfaces as a type error rather than as a silent, order-dependent choice.

**Object options.** For structured-object options, the tie-breakers apply field by field. When every tied field points to the same option, that option wins: given `{ id: "1" }`, the union `{ id: number } | { id: boolean }` produces `{ id: 1 }`. When the fields point to **different** options, the value is ambiguous and it is a type error. Given `{ a: "1", b: "1" }`, the union `{ a: number, b: boolean } | { a: boolean, b: number }` can read the value as `{ a: 1, b: true }` (option 1) or `{ a: true, b: 1 }` (option 2); "number wins" picks option 1 for `a` and option 2 for `b`. Choosing either would be guessing, which the [governing rule](#the-governing-rule) forbids, so the type error names both readings. At design time a literal argument with this ambiguity is category 5, like any other literal that cannot bind.

**Text given to `json | yaml`** is not a tie. Text is already content, so it is validated and kept unchanged: an exact match of every option it is valid for (see [Conversion Rules](#conversion-rules)).

The order in which options are written never matters.

| Target | Value | Result | Why |
|---|---|---|---|
| `string \| number` | `"4"` | the text `"4"` | exact match of `string` |
| `string \| string[]` | `2026` | the text `"2026"` | only `string` is reachable |
| `number(min(5)) \| boolean` | `"1"` | `true` | `1` fails `min(5)`, so only `boolean` is reachable |
| `number \| boolean` | `"1"` | the number `1` | both reachable; number wins |
| `{ id: number } \| { id: boolean }` | `{ id: "1" }` | `{ id: 1 }` | both reachable; number wins for `id` |
| `{ a: number, b: boolean } \| { a: boolean, b: number }` | `{ a: "1", b: "1" }` | type error naming both readings | number wins for `a` in option 1 and for `b` in option 2 |
| `json \| yaml` | the object `{ port: 8080, debug: true }` | the text `{"port":8080,"debug":true}` | both reachable; JSON wins |
| `json \| yaml` | the text `"port: 8080"` | the text `"port: 8080"` | exact match of `yaml`; validated only |

This replaces frontmatter's current union rules: root unions no longer take "the first option in index order", and property unions no longer require "exactly one option to validate after coercion".

#### Containers

For lists, tuples, and structured objects, the engine walks into the value following the **declared** structure, at every depth, and applies the same per-value rules to each element:

- all-or-nothing: one failing element fails the whole value, and the error names the failing element's path;
- the original value is never mutated; the result is a converted copy;
- a missing optional tuple slot is left missing, never filled with `null`;
- a missing required tuple slot is a type error at its path;
- a single value is never wrapped into a list;
- a plain `object` or `any` has no declared structure, so its contents are kept as they are.

In frontmatter this means the engine now reaches nested objects too; today it stops at top-level properties, typed arrays, and inline objects.

| Signature | Runtime value | Outcome |
|---|---|---|
| `person(p: tuple([string, number, boolean?])): string` | `["Ada", "36", "yes"]` | Called with `["Ada", 36, true]`. |
| same | `["Ada", "36"]` | Called with `["Ada", 36]`. |
| same | `["Ada"]` | Type error at `p[1]`: required slot missing. |
| `sum(values: number[]): number` | `["1", "2", "x"]` | Type error at `values[2]`: `"x"` is not a number. |
| same | `"1"` | Type error: a list is required. |

#### Overload Selection

An overloaded function is a union of `function(...)` types. The engine selects a signature this way:

1. If exactly one overload accepts all arguments unchanged, it wins over any overload that needs conversion.
2. Otherwise, exactly one overload must bind. Two overloads that both accept the arguments unchanged, or two that both bind after conversion, are an ambiguity error. None binding is a type error that lists every signature and why each failed.
3. The order in which overloads are written never matters.

**Worked example.** `pr_list` has two overloads:

```text
pr_list(count: number(integer; min(1))): string[]
pr_list(query: object): string[]
```

| Call (runtime value of `n`) | Outcome |
|---|---|
| `pr_list(n)`, `n` is the text `"3"` | The `count` overload, called with the number `3`. |
| `pr_list(n)`, `n` is the number `3` | The `count` overload, exact match. |
| `pr_list(n)`, `n` is the text `"0"` | Type error listing both signatures: `count` converted `"0"` to `0`, which fails `min(1)`; `query` needs an object. |
| `pr_list(n)`, `n` is the text `"2.5"` | Type error: `2.5` fails `integer`; `query` needs an object. |

A literal `pr_list(0)` is known to be wrong before anything runs: it is category 5.

#### Type Errors

A type error must be clear and carry as much context as possible:

- for a call: the function name and the signature (or, for overloads, every candidate signature), and the parameter name and position;
- for frontmatter: the property name and its source location;
- the nested path for container elements, such as `values[2]`;
- the expected type, **including its constraints**;
- the received value and its type;
- the reason (no conversion, conversion failed, constraint not met, number out of range, ambiguous union, ambiguous overload);
- for an ambiguous union, each reading the value could have taken;
- for overloads, why each candidate failed.

An illustrative rendering (the wording is not normative):

```text
pr_list: no signature accepts these arguments
  pr_list(count: number(integer; min(1))): string[]
    count (argument 1): received text "0", converted to 0; fails min(1)
  pr_list(query: object): string[]
    query (argument 1): received text "0"; expected object; no conversion from text
```

#### Changes to Frontmatter Coercion

Moving frontmatter onto the shared engine intentionally changes some results. There is no compatibility mode:

| Behavior | Before (feature 2026-05-28-schema-coercion) | After |
|---|---|---|
| number or boolean → `string` (and refined text) | converted | converted (unchanged) |
| native value → `json` / `yaml` | serialized | serialized (unchanged) |
| `"yes"`, `"no"`, `"on"`, `"off"`, `"1"`, `"0"` → `boolean` | rejected as ambiguous | converted |
| boolean words in mixed case (`"tRuE"`) | only `true`/`True`/`TRUE` and `false` equivalents | any letter case |
| number spellings `" 4 "`, `"+4"`, `"1e3"`, `".5"`, `"1_000"` | rejected (only `^-?\d+(\.\d+)?$`) | converted |
| out-of-range or precision-losing whole-number text | not distinguished | type error |
| root-level unions | first option, in index order, that validates after coercion | order-independent [union rule](#unions-choosing-an-option) |
| property-level unions | converted only when exactly one option validates | order-independent union rule |
| nested objects | not converted below inline objects | converted at every declared depth |
| `numberlike` and `boolish` values | numeric text normalized to a real number; `"true"`/`"false"` normalized to a real boolean | validated, then kept **exactly as written** (see [`numberlike` and `boolish`](#numberlike-and-boolish)) |
| text accepted by `numberlike` | only `^-?\d+(\.\d+)?$` | the same lenient spellings as `number` |
| text accepted by `boolish` | only `true`/`True`/`TRUE` and `false` equivalents | the full boolean word list, any letter case |

Validation-only calls stay non-mutating, and compose's write-back of converted values is unchanged in shape. The JSON Schema that `numberlike` and `boolish` compile to (today `anyOf` a number or boolean and a text pattern or enum) must accept the same text as the engine.

#### Performance

This spec sets no numeric performance targets. The parent spec's before/after evidence rule applies to the engine, including the frontmatter path it replaces.

### Design-time Checking

_Confirmed in the use-case review of 2026-09-25; scope settled in clarification round 3 and revised in clarification round 5, which moved DMLS diagnostics into this spec._

#### Design Time and Runtime

An editor sees only design-time information: schemas, signatures, and literals, never runtime values. Darkmatter's schema validation evaluates the same design-time setup at the start of composition, before any expression is evaluated. Unlike TypeScript, declared types are still available at runtime.

So the five categories below drive two consumers:

- **Darkmatter** (this spec) rejects a document at composition, before evaluation begins, when any call is category 5. This applies to every function; because the [one-pass migration](#one-pass-migration) moves every function onto the engine at once, there are no legacy functions to exempt.
- **DMLS** (this spec, since clarification round 5) turns them into editor diagnostics as the author types; see [Editor Diagnostics](#editor-diagnostics).

For everything else, the engine decides per runtime value when the call runs. A call that raises nothing at design time can still fail at runtime; silence means "not proven wrong," not "proven right."

#### Argument-Type Categories

When an argument's design-time type is checked against a parameter's type, exactly one of five categories applies. The DMLS columns are the mapping DMLS implements in this spec (see [Editor Diagnostics](#editor-diagnostics)); the examples assume one parameter of the named type.

| # | Category | Example (argument type → parameter type) | DMLS default | DMLS strict |
|---|---|---|---|---|
| 1 | Perfect type match, or a conversion that is always safe | `number` → `number`; `number` → `string`; literal `"4"` → `number` | — | — |
| 2 | Perfect match, if it exists | `number \| null` → `number` | — | warning |
| 3 | Might be the right type, might not | `string` → `number` (text may or may not convert); `number \| boolean` → `number` | **warning** | warning |
| 4 | Type not known | `unknown` / `any` → `number` | — | warning |
| 5 | Known to be wrong | `boolean` → `number`; `object` → `number`; literal `"pear"` → `number`; literal `pr_list(0)` | **error** | error |

The principle, in the author's words: _"if you don't type at all you are given large latitude and no warnings; as soon as you provide some schematic typing we will warn"_ — except the `T | null` case, which is silent by default because nearly every typed property is optional.

Classification notes:

- **Unions.** A union argument whose non-null options all fit (exactly or by an always-safe conversion) is category 1, or category 2 if it also admits `null`. If every option is known to be wrong, it is category 5. Anything in between is category 3.
- **Conversions.** Typed text to a `number` or `boolean` parameter is category 3. A typed number or boolean to a text parameter is category 1, and so is any value to a `json`/`yaml` parameter.
- **`numberlike` and `boolish`.** A `numberlike` argument to a `number` parameter is category 1 (its text always converts), and so is a `numberlike` argument to a `string` parameter, or a `number` argument to a `numberlike` parameter. Typed text to a `numberlike` parameter is category 3. `boolish` follows the same pattern against `boolean`.
- **Constraints and refinements do not warn.** Categories are judged on types; constraints are enforced at runtime. A typed `string` to `string(min(3); max(3))`, a typed `number` to the same parameter, and a typed `number` to `number(integer)` are all category 1. Refinements (`file`, `date`, `url`, `email`, `enum`) are judged the same way: a typed `string` or `number` to a `file` or `enum` parameter is category 1, and the refinement is checked on the value at runtime.
- **Literals** are the only arguments judged by their actual value, constraints and refinements included: a literal certain to bind (`"4"` to `number`, `2026` to `string`) is category 1; one that cannot (`"pear"` to `number`, `0` to `number(min(1))`, `"purple"` to `enum("red","green")`, `min("pear", 5)`) is category 5.
- **Wrong argument count**, and a definite overload ambiguity, are always category 5.

#### Worked Example: One Schema, Nine Calls

The schema:

```yaml
$schema:
    count: string              # optional  → string | null
    maybe: number              # optional  → number | null
    size: number(required)     #           → number
    flag: boolean(required)    #           → boolean
    foo:                       # required  → string | number
        - string(required)
        - number(required)
    # untyped_var has no schema entry     → any
```

The functions:

```text
min(a: number, b: number): number
length(val: string | any[] | object): number
is_number(val: any): boolean
is_string(val: any): boolean
```

| Row | Expression | Category of the checked argument | DMLS default | DMLS strict |
|---|---|---|---|---|
| a | `min(size, 5)` | 1 | — | — |
| b | `min(maybe, 5)` | 2 | — | warning |
| c | `min(count, 5)` | 3 | **WARNING** | warning |
| d | `min(flag, 5)` | 5 | **ERROR** | error |
| e | `min(untyped_var, 5)` | 4 | — | warning |
| f | `min(length(count), 5)` | 1 — `length` returns `number`. The inner `length(count)` is category 2. | — | warning (inner call) |
| g | `is_number(foo) ? min(foo, 5) : 0` | 1 — `foo` is narrowed to `number` in the true branch. | — | — |
| h | `is_string(foo) ? 0 : min(foo, 5)` | 1 — `foo` is narrowed to `number` in the false branch. | — | — |
| i | `min(maybe \|\| 0, 5)` | 1 — `\|\| 0` removes `null`, leaving `number`. | — | — |

Darkmatter rejects row d at composition. Rows f–i depend on the parent spec's **Phase C** (function return types) and **Phase D** (flow-sensitive narrowing), and are required test cases there. An earlier example, `max(foo || 0, 100)` with `foo: string | number`, was withdrawn: `foo || 0` keeps the `string` arm, so it is category 3.

#### Worked Example: `three_letter_acronym`

_Revised in clarification round 3 (2026-09-25); the earlier category 5 treatment is withdrawn._

```text
three_letter_acronym(tla: string(min(3); max(3))): string
```

```markdown
---
$schema:
    lookup: number(required)
lookup: 123
definition: "{{ three_letter_acronym(lookup) }}"
---
```

**Design time.** A `number` argument to a text parameter is category 1: number → text is always safe, and the length constraint is checked at runtime. There is no warning and no error, and Darkmatter composes the document.

**Runtime.**

| Runtime value of `lookup` | Outcome |
|---|---|
| `123` | Converted to `"123"`, which satisfies `min(3); max(3)`. The function runs. |
| `1234` | Converted to `"1234"`, which fails `max(3)`. Type error; never truncated. The function is not called. |

As the author noted, `"123"` is not a TLA in the everyday sense. Schemas judge form, not meaning: `"123"` is three characters of text.

For comparison, the other ways `lookup` might be typed:

| Declared type of `lookup` | Category | DMLS default |
|---|---|---|
| none (`any`) | 4 | silent |
| `string` (optional → `string \| null`) | 2 | silent (strict: warning) |
| `number(required)` | 1 | silent |

#### Narrowing and Tightening

Type-predicate functions such as `is_number(val: any): boolean` accept any value and, when they return `true`, prove the variable's type inside the guarded branch (rows g and h). That narrowing fact comes from Darkmatter's Rust registration (see [Not expressed in the catalog grammar](#not-expressed-in-the-catalog-grammar)).

Many Darkmatter functions are lenient today and declare `any` parameters. The one-pass migration tightens them all; afterwards a function given the wrong type (a type error from binding) and a function whose own work failed (a fallible function's domain error) are clearly distinct.

#### Design-time Compatibility API

Darkmatter exposes a fast design-time query: "given an argument of design-time type A and a parameter of type B, which [argument-type category](#argument-type-categories) applies?" The answer is one of the five categories (or a classification that maps exactly onto them). Literals are checked by value through the same rules the runtime uses. DMLS is the most important client. A batch form may be offered if measurement shows a benefit.

The rules live in Darkmatter only; DMLS and Claudine call this API rather than reproduce them. See [coercion-design.md](./coercion-design.md#public-compatibility-api-and-dmls).

#### Editor Diagnostics

_Added in clarification round 5 (2026-09-25); this revises the earlier decision to leave editor diagnostics to parent Phase E._

DMLS shows the argument-type categories while the author types, so a mistake is visible before the document is ever composed. The reason this ships with the rest of the spec: Darkmatter now rejects category 5 calls at composition, and the editor must never disagree with that verdict. An editor that stays silent about a call that composition will reject, or flags a call composition will accept as an error, would teach authors to ignore it.

Requirements:

1. **Darkmatter decides; DMLS displays.** DMLS classifies every call argument through the [design-time compatibility API](#design-time-compatibility-api) and maps the category to a severity. It never reproduces a conversion rule, a union rule, or a category rule of its own.
2. **Severity follows the category table.** By default, category 5 is an **error** and category 3 is a **warning**; categories 1, 2, and 4 produce no diagnostic. In strict mode, categories 2 and 4 also become **warnings**. Category 5 is always an error and category 1 is always silent, whatever the setting. This matches DMLS's existing expression ladder: a warning means the call _might_ be wrong, an error means it will never work.
3. **Ranging.** The diagnostic ranges the offending argument. Wrong argument count and overload ambiguity, which belong to the call rather than one argument, range the whole call.
4. **Message.** The message names the function, the parameter, the argument's design-time type, the parameter's type, and why the pair falls in its category. A category 5 message reads like the type error composition would report.
5. **Agreement with composition.** DMLS reports a call as an error exactly when Darkmatter would reject the document at composition because of that call. Both read the same category from the same API.
6. **Existing diagnostics are unaffected.** The new diagnostics join DMLS's `dm.expression.*` family; the existing codes, such as `dm.expression.unknown_identifier`, keep their current behavior.

**The strict-mode setting** _(confirmed in clarification round 6)_: `expressions.strict`, a boolean defaulting to `false`, in DMLS's existing configuration: a `[expressions]` table in `.dmls.toml` at the workspace root, overridable by the editor through LSP `workspace/configuration` under the `dmls` section, and reloadable without a restart.

```toml
# .dmls.toml
[expressions]
strict = true    # categories 2 and 4 also warn
```

The name follows the pattern DMLS already uses, one `strict` flag per surface: `schema.strict` governs frontmatter schema diagnostics and `style.strict` governs `style:` frontmatter. Expression calls are a separate surface, so they get their own flag rather than reusing `schema.strict`, which would tie unrelated severities together. Frontmatter never configures DMLS, and this setting is no exception.

This is the only setting for call diagnostics. No function has a setting of its own; in particular, the parent spec's opt-in unknown-input warning for `file_exists` is replaced by this flag (see [`file_exists` Has No Exception](#file_exists-has-no-exception)).

### One-pass Migration

_Added in clarification round 4 (2026-09-25); replaces the earlier pilot-then-rollout plan._

Every catalog function moves onto the shared coercion engine in **one pass**. There is no pilot stage and no period in which some functions bind through the engine while others still convert their own inputs. The human: _"We convert all functions in one pass!"_

Requirements:

1. **Every function.** All catalog functions (about 110, including the 14 [missing from the draft](#function-catalog-migration)) are registered against their new `function(...)` signatures and called through the engine's binding step.
2. **Corrected signatures.** Each signature says what the function needs ([catalog migration](#function-catalog-migration) items 10 and 11).
3. **No function converts its own inputs.** Every function's own arity checks, type checks, and conversion code are removed, along with the shared conversion helpers that only served them. Conversions that _are_ a function's purpose, such as `number()`, remain as documented domain behavior.
4. **The old catalog is deleted.** `darkmatter/docs/schemas/expression-functions.yaml` has no remaining reader and is removed.
5. **Per-function items resolved.** The [per-function items](#per-function-items-for-the-catalog-migration) are decided within this spec, and the catalog records the outcome.
6. **Behavior-change ledger.** Every function's behavior, measured against the baseline in [current-state-coercion.md](./current-state-coercion.md), is recorded as either **preserved** or **intentionally changed**, with a one-line reason for each change. Nothing is left "to be decided". The ledger is reviewed by the human before merge.

**Suggested order within the pass.** The earlier pilot functions (`min`, `is_number`, `number()`, and both `pr_list` forms, which together exercise conversion, `any` pass-through, a constraint, and overloads) are a sensible place to start building, as described in [coercion-design.md](./coercion-design.md#suggested-implementation-order). This is an order of work, not a stopping point: the spec is not done until every function has moved.

#### Risks

The one-pass migration is the highest-risk part of this spec. Every expression function changes at once, and a behavior change in any one of them reaches every document that calls it. The behavior-change ledger and the catalog-wide acceptance criteria are the main safeguards. A throwaway spike, a harness that records the current behavior of every function over a shared set of inputs so the migrated functions can be compared against it, is being considered as part of risk assessment. This spec does not design it.

### Documentation

Every topic in this spec needs complete, high-quality documentation readable by a human audience who:

- is technical, but
- has no knowledge of this monorepo's capabilities, packages, etc.

The language must avoid jargon, and time should be spent making sure clear language, good examples, and smart structure appear in every document. At minimum:

- [Schema Definition](../../docs/topics/schemas/definition.md) documents `tuple`, `function`, `|` unions (including precedence and grouping), `category`, the function `example(call => result)` form, and `categories:`;
- the "Type Coercion" section of Schema Definition is rewritten to the single rule list: its **Coercion Matrix**, **Never Coerced (Ambiguous)**, **Root Unions**, and **Inline Object and Union Coercion** subsections, and the coercion bullet under **Content-Format Types**, are replaced by the conversion rules, rejected conversions, union rule, and container rules of this spec, and the [frontmatter changes](#changes-to-frontmatter-coercion) are listed for existing users;
- the `numberlike` and `boolish` rows of Schema Definition's type table, which today say their text is **normalized** to a real number or boolean, are rewritten: both accept what `number` and `boolean` accept and keep the value exactly as written;
- the coercion engine is documented once, for frontmatter and function calls alike: the two interfaces of a type, its three actions, conversions, rejected conversions, and type errors;
- the five argument-type categories are documented for document authors, including what Darkmatter rejects at composition;
- DMLS's [diagnostics guide](../../dmls/docs/diagnostics.md) documents the new call diagnostics, their severities, and the strict-mode setting in its **Severity** and **Configuration** sections;
- the generated function reference (including `claudine context --expressions`) groups functions by category in list order, alphabetically within each category, and shows each function's examples.

### Pre-authorized Preparatory Work

The implementation may do the following without pausing for approval:

- move `functions.yaml` and `mutations.yaml` to `darkmatter/schemas/`, and delete `darkmatter/docs/schemas/expression-functions.yaml` once nothing reads it;
- edit other packages to use the new catalog and API, including the `claudine context --expressions` renderer in claudine-cli and DMLS's catalog consumers;
- add the call diagnostics and the strict-mode setting to DMLS (`darkmatter/dmls`), including its configuration model and tests;
- remove every function's own coercion code and the helpers that only served it;
- regenerate generated documentation, update the `darkmatter` and `claudine` agent skills, and add test fixtures and snapshots;
- change existing frontmatter-coercion and catalog tests whose expectations change because of a behavior change this spec approves.

## Decisions Log

Decisions 1–15 were confirmed by the human on 2026-09-25 in a clarification pass; decisions 16–30 the same day in the ten-case use-case review; decisions 31–47 in clarification round 3; decisions 48–56 in clarification round 4; decisions 57–61 in clarification round 5; decisions 62–64 in clarification round 6. Revised decisions keep their number and say what replaced them.

1. **Home.** This spec is the detail spec for the parent's Phase B and lives under Darkmatter — the work is Darkmatter's grammar and runtime.
2. **Done means grammar + catalog swap + engine + pilot** _(revised in round 3; see 31 and 45; revised again in round 4; see 52)_. Was: catalog-wide rollout goes to parent Phase E. Now every function migrates in this spec, in one pass.
3. **Catalog home is `darkmatter/schemas/`.** Darkmatter embeds it; Claudine reads it through the library — one owner, no copies.
4. **This spec supersedes the Phase B prototype.** Compact `function(...)` grammar and one shared engine replace per-parameter records and conversion policies — simpler to write and read.
5. **Laziness and narrowing live in Rust registration.** Neither belongs in a type signature.
6. **`function` is description-only.** It is an error in a document schema and emits no JSON Schema — no frontmatter value is a function.
7. **Overloads are a union of `function(...)` types.** Reuses existing union syntax instead of inventing new syntax.
8. **`example(call => result)` with `display-only:` for unrunnable ones.** Replaces the old executable/display-only fields; all 117 old examples carried over. _(Extended in round 3; see 38.)_
9. **`categories:` is a general schema-file feature.** Unlisted categories fail at load; docs follow list order — keeps categories honest and ordered.
10. **Functions never convert their own inputs; one engine does.** Only conversions that keep meaning; otherwise a type error and no call — removes duplicated, inconsistent per-function logic.
11. **Constraints are enforced, never repaired.** Truncating or clamping would violate caller intent.
12. **Approved conversions** _(revised in the use-case review, then revised again in round 3; see 32)_. Was: text → number and text → boolean only.
13. **DMLS diagnostics follow the five argument-type categories** _(revised in the use-case review)_**:** category 5 is an error and category 3 a warning by default; categories 2 and 4 warn only in strict mode — untyped values get latitude, typed ones get warnings. _(Round 3: implemented in parent Phase E, together with the strict-mode setting; see 45. Revised in round 5: implemented in this spec; see 57.)_
14. **`T | error` is an illustration only.** `error` is not a schema type.
15. **`spec.md` is normative; `coercion-design.md` is supporting detail.** Keeps one source of truth.
16. **Three actions per value: keep, convert, or type error.** A function is never called unless it gets its declared types — makes every function's contract trustworthy.
17. **Golden rule: convert only when meaning is unchanged.** The single test for any conversion.
18. **Domain behavior is not an engine concern.** `number()` failure behavior and the `round` fallback move to the catalog migration — they are per-function contracts. _(Round 4: resolved within this spec; see 53.)_
19. **`and`/`or` take `any`; truthiness is their domain behavior.** Nothing to convert; laziness stays in Rust.
20. **`null`, `any`, and `unknown` are formal types; `null` is never converted.** A non-null parameter given `null` is a type error, and every argument is checked (case 3) — a null never hides another bad argument.
21. **Type errors carry full context.** Function or property, signature, parameter, path, expected type with constraints, received value and type, reason — so authors can fix a document without reading source.
22. **Number → text and boolean → text removed** (case 4) _(revised in round 3; see 32)_. Now always-safe conversions.
23. **Whole-number text is exact or rejected** (case 7). A 64-bit integer when it fits, otherwise a type error — never silently change a number.
24. **Union conversion priority: exact, then number, then boolean** (case 5). `"1"` to `number | boolean` is `1` — "prefer number" beats an ambiguity error. _(Extended in round 3; see 35. Restated in round 4; see 50.)_
25. **Refined text types inherit no other conversions** _(revised in round 3; see 34)_. Was: a number to `file` is an error.
26. **Overload selection** (case 8): unchanged beats converted; otherwise exactly one must bind; order never matters — predictable and reorder-safe.
27. **Containers** (case 9): walk the declared structure, all-or-nothing, report paths, never mutate, never fill or wrap — the same per-value rules at every depth.
28. **Five argument-type categories** (case 10), shared by DMLS and Darkmatter's composition-time check — one vocabulary for editor and runtime.
29. **A `number`-typed argument to a text parameter is category 5** (case 4) _(revised in round 3; see 40)_.
30. **Rows f–i are parent Phase C/D test cases.** Return types and narrowing belong to the parent's phases.
31. **One coercion engine for frontmatter and function calls.** The human: _"We need a single logic engine … ONE set of business logic."_ Frontmatter validation moves onto it in this spec, with no per-surface exceptions.
32. **The conversion rule list.** Number and boolean → text (always safe); text → number (lenient spellings, exact integers); text → boolean (word list, any case); native values → `json`/`yaml` (serialized). Rejected: boolean → number, null → anything, wrapping into a list, parsing text into structure. The human: _"Converting a number to a string is completely fine! … It's only in the opposite direction (string to number) you have to be careful."_
33. **Whole-number text** means no decimal point and no exponent; `"1e3"` and `"4.0"` take the floating-point path.
34. **Refined text** accepts the conversions into text, then runs its own validation.
35. **Union priority is exact, number, boolean, then text** _(revised in round 4; see 50)_, skipping arms whose constraints fail, and replaces frontmatter's first-arm-in-order rule — one order-independent rule for both surfaces.
36. **Constraints never warn at design time on their own.** They are enforced at runtime.
37. **Intentional frontmatter changes ship without a compatibility mode.** Listed in [Changes to Frontmatter Coercion](#changes-to-frontmatter-coercion).
38. **Example results** are Darkmatter expression literals compared structurally; `=> error` expects a failure; the file form is allowed on `function` and holds multi-line results.
39. **Literals certain to convert are category 1.**
40. **`three_letter_acronym(lookup)` with `lookup: number(required)`** is category 1: `123` binds as `"123"`; `1234` fails `max(3)` at runtime.
41. **`tuple` and `|` are allowed everywhere**, including a document's `$schema`; they compile to `prefixItems`/`minItems`/`items` and `anyOf`. The YAML-list union means the same as `|`.
42. **TypeScript-like composition.** `[]` binds tighter than `|`; parentheses group; tuple elements go required, optional, then at most one final spread; tuple elements are unnamed.
43. **`parameters(...)` and `returns(...)` are required; `fallible` is a flag.** Leaving either out is a schema-load error.
44. **Category names** are raw, trimmed, case-sensitive text; `category(...)` on a non-function property is emitted as `x-darkmatter-category`; `categories:` entries may omit the description, and descriptions may span lines.
45. **Design-time scope.** This spec ships the five-category API and composition-time rejection of category 5 calls. DMLS diagnostics and the strict-mode setting move to parent Phase E. _(Round 4: rejection covers every function; see 54. Revised in round 5: DMLS diagnostics and the strict-mode setting are delivered by this spec; see 57.)_
46. **Overload ambiguity and duplicates.** Two overloads both accepting the arguments unchanged is an ambiguity error; duplicate signatures fail at catalog load.
47. **Catalog housekeeping.** `pair` is replaced by a shared named type with cross-referenced descriptions; `mutations.yaml` drops `name:`; types-only catalog files never auto-apply; null pass-through requires `| null`; boolean words are case-insensitive; no numeric performance targets.
48. **Every type has two interfaces:** accepted inputs (caller → engine) and a delivery guarantee (engine → function). Keeps what a caller may send separate from what a function may rely on.
49. **`numberlike` and `boolish` are kept, distinct from `number` and `boolean`.** They accept the same inputs but deliver the value exactly as sent — a function can then return the caller's own type, "text holding only a number" has no other ergonomic spelling, and both read well in generated docs. The name `boolish` stays: _"boolish is the name most languages choose."_ Resolves the former open question 1.
50. **Union rule: exact match, else the one reachable option, else a tie-breaker** (number beats boolean, applied field by field for object options). Replaces the numbered priority list of 24 and 35; resolves the former open question 2. _(Extended in round 5; see 59, 60, and 61.)_
51. **Refinements are judged like constraints at design time.** A typed `string` or `number` to `file`, `date`, `url`, `email`, or `enum` is category 1; only literals are judged by value. Resolves the former open question 3.
52. **One-pass migration.** Every catalog function moves onto the engine in this spec, with corrected signatures, its own coercion removed, and the old catalog deleted; the pilot is only a suggested build order. The human: _"NOT comfortable with the staged rollout."_ Revises 2.
53. **Per-function items are resolved in this spec's migration,** and every function's behavior is recorded as preserved or intentionally changed in a ledger reviewed before merge — a one-pass change needs a complete, reviewable record.
54. **Composition-time rejection of category 5 calls applies to every function.** With no legacy functions left, there is nothing to exempt.
55. **Frontmatter `numberlike` and `boolish` are validated but no longer normalized.** An intentional behavior change, listed with the other frontmatter changes.
56. **Catalog parameters use `numberlike`/`boolish` only where the function needs the caller's original form;** otherwise `number`/`boolean`.
57. **DMLS diagnostics and the strict-mode setting are delivered by this spec.** DMLS shows the five categories as the author types (category 5 an error, category 3 a warning; strict mode adds warnings for 2 and 4), classifying through Darkmatter's compatibility API and never reproducing its rules — composition-time rejection and the editor must not disagree. Revises 45 and the Phase E note on 13; resolves the former open question 1.
58. **The strict-mode setting is `expressions.strict`** _(confirmed in round 6; see 62)_, a `false`-by-default boolean in a `[expressions]` table of `.dmls.toml`, overridable through LSP `workspace/configuration` — it follows DMLS's existing one-`strict`-flag-per-surface pattern (`schema.strict`, `style.strict`).
59. **Object options whose fields point to different options are an ambiguity type error** naming every reading. `{ a: "1", b: "1" }` against `{ a: number, b: boolean } | { a: boolean, b: number }` fails — choosing would be guessing, and option order never matters. Resolves the former open question 2.
60. **`json | yaml` given a native value: JSON wins.** `{ port: 8080, debug: true }` becomes `{"port":8080,"debug":true}` — JSON text is also valid YAML, so it satisfies both options. Text given to `json | yaml` is an exact match, validated only, and not a tie. Resolves the former open question 3.
61. **There are exactly two tie-breakers:** number over boolean for text reading as both (field by field for object options, erroring when fields disagree), and JSON over YAML for native values. Any other tie is a type error — the golden rule forbids guessing. _(Confirmed in round 6; see 64.)_
62. **`expressions.strict` is confirmed** as the name and home of the strict-mode setting (boolean, default `false`, `[expressions]` table in `.dmls.toml`, overridable through the LSP configuration channel) — it follows DMLS's one-`strict`-flag-per-surface pattern. Resolves the former open question 1.
63. **`file_exists` has no exception to the single engine.** `file_exists(42)` binds the text `"42"` and checks for that file, with no DMLS diagnostic by default; the parent's advisory always-false exception for known unsupported types is retired, and its opt-in unknown-input warning is replaced by `expressions.strict` — one engine with a per-function exception would be two engines. Its always-false warning for provably malformed literals stays as domain behavior.
64. **The union safety net is confirmed:** any tie neither tie-breaker settles is an ambiguity type error — it keeps "never guess" and "option order never matters" true as the conversion list grows.

## Acceptance Criteria

Runtime criteria for function calls run through normal dispatch with a handler-invocation counter; functions named "test-registered" exist only in tests. Frontmatter criteria run through Darkmatter's schema validation and compose paths on fixture documents.

### Grammar and Catalog

| # | Criterion | How to validate |
|---|---|---|
| 1 | The grammar parses `tuple` (fixed, optional elements, one final spread), `function` with every constraint, `\|` unions everywhere, and `categories:`. | Parser unit tests over each form in this spec, plus rejection tests for two spreads, a spread that is not last, an optional element before a required one, `?` on a spread, a named tuple element, and a `function` missing `parameters` or `returns`. |
| 2 | Precedence and grouping follow TypeScript. | `string \| number[]` accepts `"a"` and `[1]` and rejects `["a"]`; `(string \| number)[]` accepts `["a", 1]`. |
| 3 | Tuples and `\|` compile to standard JSON Schema. | Snapshot tests of `point: tuple([number, number])`, a variadic tuple, and `id: string \| number` show `prefixItems`/`minItems`/`items` and `anyOf`; `id: string \| number` and the YAML-list form produce identical output. |
| 4 | A function-typed property in a document's `$schema` fails at load. | A fixture document declaring `run: function(parameters(void); returns(string))` under `$schema` produces a schema-load error. |
| 5 | Categories load and emit correctly. | A schema file with `categories: [Math -> ...]` and `category(Text)` fails at load; the same file without `categories:` loads; a non-function property with `category(Math)` emits `x-darkmatter-category: Math`; entries without descriptions and with multi-line descriptions load. |
| 6 | Duplicate overloads fail at load. | A catalog with two `pr_list` overloads with identical parameters produces a catalog-load error. |
| 7 | The new catalog is complete and is the only catalog. | A test compares catalog function names with registered implementations and finds no gaps either way; `expression-functions.yaml` is deleted and nothing reads it. |
| 8 | Executable catalog examples pass, compared structurally. | A test evaluates every example not marked `display-only:`; the count is at least 72. Unit tests show `=> 3` fails against `"3"`, `=> {a: 1, b: 2}` passes against `{b: 2, a: 1}`, and `=> error` passes only when the call fails. |
| 9 | Display-only and file-form examples are shown but display-only ones never run. | The example test reports display-only examples as skipped by design; generated docs include both forms. |

### One-pass Migration

| # | Criterion | How to validate |
|---|---|---|
| 10 | Every registered function binds through the engine. | A test enumerates every registered function and fails unless each has accepted, rejected, and converted-input fixtures run through normal dispatch; on every rejected fixture the counter stays at 0. |
| 11 | No function performs its own coercion. | The per-function arity, type-check, and conversion code is deleted; handlers read arguments only through bound-argument accessors that neither parse nor convert; review confirms no handler inspects an argument's type to convert it, except where conversion is the function's documented purpose (such as `number()`). |
| 12 | The per-function items are resolved. | The catalog signatures of `number()` and `round` carry one settled parameter name and fallback behavior, with executable examples covering unconvertible text and a supplied fallback. |
| 13 | The behavior-change ledger is complete and reviewed. | The ledger lists every registered function as preserved or intentionally changed, each change with a reason; the human's review is recorded before merge. |

### Coercion Engine: Function Calls

| # | Criterion | How to validate |
|---|---|---|
| 14 | Text converts to a number. | `min("4", 5)` returns `4`; the counter records exactly one call. |
| 15 | Number converts to text. | `upper(2026)` returns `"2026"`. |
| 16 | Type errors are complete and invalid input never reaches the function. | `min("pear", 5)` returns a type error naming the function (`min`), parameter (`a`), expected type (`number`), received value and type (text `"pear"`), and reason; the counter stays at 0. |
| 17 | Constraints are enforced, not repaired. | Test-registered `three_letter_acronym(tla: string(min(3); max(3))): string` given `123` runs with `"123"`; given `1234` or `"ABCD"` returns a type error. Test-registered `f(n: number(integer))` given `"2.5"` returns a type error. Failing counters stay at 0. |
| 18 | `any` preserves the original value. | `is_number("4")` returns `false`; `is_number(4)` returns `true`. |
| 19 | Rejected conversions are rejected. | `min(true, 5)`, `min(null, 5)`, and test-registered `sum(values: number[])` given `"1"` all return type errors; counters stay at 0. |
| 20 | The union rule. | Test-registered `describe(val: number \| boolean)` given `"1"` receives `1`; `g(val: number(min(5)) \| boolean)` given `"1"` receives `true`; `s(val: string \| string[])` given `2026` receives `"2026"`; `k(val: { id: number } \| { id: boolean })` given `{ id: "1" }` receives `{ id: 1 }`; `m(val: { a: number, b: boolean } \| { a: boolean, b: number })` given `{ a: "1", b: "1" }` returns an ambiguous-union type error naming both readings, and its counter stays at 0; `c(config: json \| yaml)` given the object `{ port: 8080, debug: true }` receives the text `{"port":8080,"debug":true}`, and given the text `"port: 8080"` receives it unchanged; each holds with the options written in either order. |
| 21 | Whole-number text is exact. | Test-registered `echo(val: number): number` given `"9007199254740993"` returns exactly `9007199254740993`; given `"99999999999999999999"` returns an out-of-range type error; `"1e3"` returns `1000`. |
| 22 | Every argument is checked. | `min(low, high)` with `low` omitted (`low: number` optional) and `high: pear` untyped returns one type error reporting both `a` and `b`; the counter stays at 0. |
| 23 | Overload selection matches the worked example in both orders. | With the `pr_list` signatures in both orders: `"3"` and `3` select `count` with `3`; `"0"` returns a type error listing both signatures and why each failed; `"2.5"` returns a type error; no provider request is made. |
| 24 | Containers follow the declared structure. | Test-registered `person` and `sum` produce every outcome in the [Containers](#containers) table, including the paths `p[1]` and `values[2]`, and the caller's original list is unchanged. |
| 25 | Native values serialize into `json`. | Test-registered `h(doc: json)` given the object `{a: 1}` receives the text `{"a":1}`; given the text `"{\"a\":1}"` receives it unchanged; given the text `"{a"` returns a type error. |
| 26 | `numberlike` delivers the value as sent. | Test-registered `n(val: numberlike)` given `5`, `"5"`, `"02134"`, and `" 4 "` receives each exactly as given; given `"pear"`, `true`, or `null` returns a type error and the counter stays at 0. |
| 27 | `boolish` delivers the value as sent. | Test-registered `b(val: boolish)` given `true` and `"Yes"` receives each exactly as given; given the number `1` or the text `"maybe"` returns a type error and the counter stays at 0. |

### Coercion Engine: Frontmatter

| # | Criterion | How to validate |
|---|---|---|
| 28 | Number → text. | `version: 1.2` against `version: string` validates and composes as `"1.2"`. |
| 29 | Boolean words. | `draft: "yes"` and `draft: "On"` against `draft: boolean` become `true`. |
| 30 | Lenient numbers and precision. | `n: "+4"` and `n: "1_000"` against `n: number` become `4` and `1000`; `n: "99999999999999999999"` fails validation. |
| 31 | Root unions are order-independent. | A root union whose options type `x` as `number` and as `boolean` produces the same result for `x: "1"` with the options in either order, following the [union rule](#unions-choosing-an-option). |
| 32 | Nested objects are converted. | `meta: { inner: { count: number } }` given `meta: { inner: { count: "3" } }` becomes `count: 3`. |
| 33 | Native values serialize into `json`/`yaml`. | `config: json` given a native mapping validates and composes as JSON text; `config: json \| yaml` given `{ port: 8080, debug: true }` composes as `{"port":8080,"debug":true}`; `null` is not serialized. |
| 34 | `numberlike` and `boolish` are validated, not normalized. | `zip: "02134"` against `zip: numberlike` and `flag: "On"` against `flag: boolish` validate and compose unchanged; `zip: "pear"` fails validation. |
| 35 | One engine. | Frontmatter and function-call tests over the same value/type pairs (including every row above) produce identical keep/convert/error outcomes; the old frontmatter-only rule code is gone. |

### Design-time Checking

| # | Criterion | How to validate |
|---|---|---|
| 36 | The compatibility API returns the five categories. | For each argument/parameter pair in rows a–e of the [worked example](#worked-example-one-schema-nine-calls), and for `number` → `string`, `boolean` → `number`, `object` → `number`, `numberlike` → `number`, `string` → `numberlike`, `number` → `numberlike`, `string` → `file`, literal `"4"` → `number`, literal `"purple"` → `enum("red","green")`, literal `pr_list(0)`, and `string` → `string(min(3); max(3))`, the API returns the category this spec lists. |
| 37 | Category 5 is rejected at composition, for every function. | A document calling `min(flag, 5)` with `flag: boolean(required)` is rejected before evaluation; the counter stays at 0 and no expression is evaluated. For every registered function with a parameter that some design-time type cannot reach, a generated document making such a call is rejected the same way. |
| 38 | Category 1 conversions compose. | A document with `lookup: number(required)` and `lookup: 123` calling `three_letter_acronym(lookup)` composes; with `lookup: 1234` it fails at runtime with a type error. |
| 39 | Documentation grouping. | `claudine context --expressions` lists categories in `categories:` order and functions alphabetically within each; a snapshot test pins it. |
| 40 | DMLS shows the categories under both settings. | A DMLS test opens a fixture document with the [worked example's](#worked-example-one-schema-nine-calls) schema, with every variable (including `untyped_var`) set in its frontmatter so no unknown-identifier warning fires, and calls rows a–e. With `expressions.strict` off, only row c has a warning and only row d an error; with it on, rows b, c, and e have warnings and row d an error; row a is silent in both. The same results hold when the setting comes from `.dmls.toml` and from `workspace/configuration`. |
| 41 | A category 5 call is underlined as an error, in agreement with composition. | In the DMLS test, row d's `flag` argument carries an error-severity diagnostic ranged at the argument; composing the same fixture is rejected. A review confirms DMLS obtains every category from Darkmatter's compatibility API and contains no conversion or category rules of its own. |
| 42 | `file_exists` binds like every other function. | `file_exists(42)` binds the text `"42"` and returns `true` when a file named `42` exists at the resolution root and `false` otherwise, with no type error; the compatibility API returns category 1 for `number` → `string \| file \| null`, and DMLS shows no diagnostic for the call with default settings. A review confirms no `file_exists`-specific setting exists in DMLS's configuration model. |

## Open Questions

Clarification round 5 resolved the earlier questions on where DMLS diagnostics live, object options whose fields disagree, and `json | yaml` given a native value (decisions 57, 59, and 60). Clarification round 6 confirmed the strict-mode setting's name (decision 62).

No open questions remain.

## Per-function Items for the Catalog Migration

These concern a single function's domain behavior rather than the engine. Under the [one-pass migration](#one-pass-migration) they are resolved within this spec, before merge, and each outcome is recorded in the behavior-change ledger:

1. **`number(val: any, fallback?: number): number` — failure behavior.** `val` is kept unchanged, so what `number("pear")` and `number("pear", 7)` return (the fallback, a domain error that makes `number` `fallible`, or something else) is `number()`'s contract. Today they return `0` and `7`. A supplied `fallback` that is not a number, as in `number("pear", "x")`, is already a type error from binding. The draft catalog names this parameter `def_val?`; the migration settles one name (`def_val?` or `fallback?`).
2. **`round(val: number, fallback?: number): number` — the fallback.** The draft signature is inconsistent: `val: number` means binding already rejects `"pear"`, so the fallback can never be used. The migration must either drop the fallback or declare `val: any` and give `round` its own documented conversion, as `number()` has. The draft catalog names this parameter `default: number(optional)`; it is normalized with the decision.
