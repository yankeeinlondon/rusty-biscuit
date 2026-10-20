---
area: darkmatter
status: finalized-spec
created: 2026-09-21
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 0
human_review: false
pipeline_step: 18
formerly: 2026-09-21-schema-enhancements
depends-on:
    - 2026-10-16-core-schema-types
    - 2026-10-16-type-narrowing
related:
    - 2026-10-14-union-type-constraints
    - 2026-10-29-recursive-schema-types
    - 2026-10-31-type-aware-evaluation
    - 2026-10-23-shell-template-approval
    - 2026-09-17-remove-strict-mode
    - 2026-05-28-schema-coercion
---

# Schema Enhancements

Darkmatter authors will be able to describe tuples and function signatures with SimplifiedSchema, use the same conversion rules for frontmatter and function arguments, and see proven call errors in DMLS (Darkmatter's language server) before composition runs. This specification defines the grammar, catalog migration, shared conversion engine, and editor checks needed for that behavior.

> For reference, here is the most authoritative document we currently have for our schema support:
>
> - [Schema Definition](../../../darkmatter/docs/topics/schemas/definition.md)

## Summary and Goals

This is **Phase B** ("function schemas in SimplifiedSchema") of Darkmatter's expression type system. The type-system charter that used to hold all five phases was split and retired on 2026-10-10: Phase A (`unknown`, `null`, `alphanumeric`, `shell`) is `2026-10-16-core-schema-types`, this spec is Phase B, and Phases C–E (type-aware evaluation, flow-sensitive narrowing, diagnostics, and CLI policy) are `2026-10-31-type-aware-evaluation`. The charter's Phase B material — its merge decisions M1–M16 and its function-contracts, number-contract, and catalog-audit annexes — now sits in this directory as reconciliation input and is reconciled below; this spec is the only normative Phase B text.

An author gains a schema language that can describe functions and tuples, one documented set of conversion rules that applies identically to frontmatter and to function arguments, and an editor that flags a call composition will reject before the document is ever run.

The goals are:

1. **Richer grammar.** Add `tuple` and `function` types, a compact `|` union usable anywhere a type is written, a `category` constraint, a function `example` form, and a top-level `categories:` list for schema files.
2. **One function catalog.** Replace Darkmatter's current expression-function catalog with a catalog written in the new grammar, housed in Darkmatter.
3. **One coercion engine.** Darkmatter has exactly one set of conversion rules. The same engine converts frontmatter values when a document is validated against its `$schema`, and converts arguments before a function is called. Functions declare the types they need and stop converting their own inputs. **Every** catalog function moves onto the engine in one pass; there is no staged rollout.
4. **Proportionate design-time checking.** Darkmatter classifies every argument into one of five categories, rejects documents containing calls that are proven wrong, and exposes the classification through an API. DMLS uses that API to show the same verdicts as the author types, with a strict-mode setting for authors who want more warnings. Composition and the editor must never disagree about a call. _(Editor work moved into this spec in clarification round 5.)_ The checks read variable types from a minimal [expression-variable type projection](#expression-variable-type-projection) built in this spec.
5. **One numeric model.** Every Darkmatter number is an `f64`; whole-number admission preserves the input exactly, fractional input takes the nearest `f64`, and non-finite or non-zero-underflow results error (see [Numbers](#numbers)).

The documents in this directory work together:

| Document | Role |
|---|---|
| `spec.md` (this file) | **Normative.** Its requirements, decisions, and rulings govern. |
| [coercion-design.md](./coercion-design.md) | Supporting design detail for the coercion engine. It must conform to this spec; where they conflict, this spec wins. It is not itself a confirmed contract. |
| [current-state-coercion.md](./current-state-coercion.md) | A record of how functions treat their inputs today. It is a baseline, not a proposal. |
| [merge-decisions.md](./merge-decisions.md), [function-contracts.md](./function-contracts.md), [number-contract.md](./number-contract.md), [catalog-audit.md](./catalog-audit.md) | The retired charter's Phase B material (2026-09-26). Reconciliation input only: each is marked superseded where this spec decides. The [Reconciliation](#reconciliation-with-the-charter-decided-2026-10-10) section records every outcome. |
| `spikes/coercion-baseline/`, `spikes/single-engine/` | Throwaway spikes (2026-09-26) whose rulings are recorded in [Spike Rulings](#spike-rulings-decided-2026-10-10). |
| `drafts/` | Draft schema files; `kind-document.yaml` and `schema-target.yaml` are transcluded by shipped docs, so this directory and its file names are fixed. |
| [strict-mode-integration-design.md](./strict-mode-integration-design.md) | Historical integration proposal; its remaining scope items were dropped or moved in the 2026-10-10 restructure. |

Nothing in this spec is implemented yet.

## Reconciliation with the Charter (Decided 2026-10-10)

The charter's 2026-09-26 consolidation absorbed an early draft of this spec without seeing its six 2026-09-25 clarification rounds. The owner settled every resulting conflict on 2026-10-10. The outcomes, all reflected in the body below:

| Topic | Outcome |
|---|---|
| Numbers | One representation, `f64`, plus a whole-number exactness check; whole numbers must be exact whatever their spelling, decimals such as `0.1` take the nearest `f64`, and a non-zero value that underflows to zero is an error (see [Numbers](#numbers)). This replaces this spec's earlier "exact 64-bit integer" rule and the charter's "accept ordinary rounding" rule at the point a value enters the number model. |
| Runtime `null` | Decision 20 stands: `null` is never converted, and a function receives `null` only on a parameter that declares `\| null`. Which parameters declare it follows the [migration-ledger null rule](#which-functions-accept-null). The charter's M2 ("ordinary functions reject null") is replaced by that rule. |
| Nullable-argument warnings | Silent by default (category 2); `expressions.strict` warns. The charter's "a union including null warns" is withdrawn. |
| Conversion spellings | This spec's lenient spellings (`" 4 "`, `"+4"`, `"1e3"`, `".5"`, `"1_000"`, boolean words in any case) win. |
| Union selection | This spec's two tie-breakers (number over boolean; JSON over YAML) win. |
| Refined text | `enum` and string-literal targets accept number → text and boolean → text, then check the value, like every refined text type. |
| Rollout | One-pass catalog migration wins over the charter's first-slice-then-rollout plan. |
| `file_exists` | No special case: it declares `any` like every inspecting predicate, so `file_exists(42)` is `false` without conversion (decision 82, revising 63). The charter's known-unsupported-type advisory and its opt-in unknown-input warning are retired. |
| `length` | `length(true)` and `length(false)` are rejected before the boolean → text conversion: a deliberate per-function exception, recorded in registration metadata (charter M11). |
| `number()` / `round()` | The charter contract: `number(x: numberlike, fallback?: number)` and `round(x: numberlike, fallback?: number)`; any failure to use `x` as a number selects a supplied fallback, and without one the call errors; `round` rounds half away from zero (charter M12–M14). |
| Dead branches | A provably wrong (category 5) call in a branch that never runs still rejects the document at composition; the editor and composition agree. |
| Catalog home | `darkmatter/schemas/partials/functions.yaml` (charter M8), with `mutations.yaml` beside it. |
| Charter-only function contracts | Owned here: numeric presentation (M6), `validate_schema` merging (M9), text-based `contains` (M10), and operator consistency (M15). |
| Variable types | This spec builds the minimal [expression-variable type projection](#expression-variable-type-projection) its checks need; `2026-10-31-type-aware-evaluation` extends it and never builds a second one. |
| Strict-root validation | Struck from the type system entirely (no `is_known_variable_root` hook, no unknown-root error). |

## Terminology

This spec is careful to separate what is known while a document is being _written_ from what is known while it is being _run_.

- **Design-time type** — what a schema or signature says a value can be, known before anything runs. `number`, `string(min(3))`, and `any` are design-time types. An editor can only ever see these.
- **Runtime value** — the actual value that exists when a document is validated or an expression is evaluated, such as `4`, `"4"`, `"pear"`, or `null`. The same design-time type `string` covers both `"4"` and `"pear"`.
- **`any`, `unknown`, and `null`** — formal SimplifiedSchema types, added by `2026-10-16-core-schema-types` (Phase A). `unknown` admits every value, including null; `any` is an accepted alias for it. `null` admits only null. As a function parameter type, `unknown` does not admit null; only an explicit `| null` does (see [Null Is a Type Like Any Other](#null-is-a-type-like-any-other)). An untyped frontmatter variable has the design-time type `unknown`, and an optional typed property declared `number` has the design-time type `number | null`.
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
- **Composition** — Darkmatter's processing of a document into its final output. The new passive call check runs before expression evaluation; frontmatter schema validation retains its established position after initial frontmatter interpolation.
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
7. **The minimal expression-variable type projection** that the design-time checks read: see [Expression-variable Type Projection](#expression-variable-type-projection).
8. **The numeric model:** `f64` everywhere with an exactness check, numeric presentation, and consistent numeric operators: see [Numbers](#numbers).
9. **Charter-only function contracts** carried into the one-pass migration: `number()` and `round()` recovery, `length`'s boolean exclusion, `validate_schema` merging, and text-based `contains` (see [Function Contracts Carried from the Charter](#function-contracts-carried-from-the-charter)), plus the new map functions `keys()` and `values()`.
10. **Spike rulings:** raw JSON Schema conversion, the completed frontmatter Changes table, and `serde_json`'s `preserve_order` feature (see [Spike Rulings](#spike-rulings-decided-2026-10-10)).

### Out of scope

| Work | Owner |
|---|---|
| The `unknown`, `null`, `alphanumeric`, and `shell` types | `2026-10-16-core-schema-types` (this spec depends on it) |
| How `[]`, constraints, and `required` apply to a reference to a union type, and the list-of-union representation (`items: { anyOf }`) | `2026-10-14-union-type-constraints`; this spec's `(A \| B)[]` lowers onto that representation |
| Recursive named types | `2026-10-29-recursive-schema-types` |
| Extending the variable-type projection (root unions, imported and merged layers, raw JSON Schema, nested paths, host globals), and function return types and operator typing in argument checks (worked-example rows f and i) | `2026-10-31-type-aware-evaluation` |
| Flow-sensitive narrowing (`TypeScope`) and the single subtype relation (`is_subtype`) | `2026-10-16-type-narrowing` |
| `is_file_type`, guarded file arguments, `file_exists`'s structural rejection of malformed strings and its always-false warning for provably malformed literals, and the host function-extension contract | `2026-10-31-type-aware-evaluation` |
| Retiring the interim absence-predicate suppression of `dm.expression.undeclared_property`, shared CLI/DMLS diagnostic codes, the CLI policy flags, and the report output format | `2026-10-31-type-aware-evaluation` |
| Positional CLI arguments converted through this engine | `2026-09-30-positional-args`, a later, separate spec that is not part of this pipeline |

### Not expressed in the catalog grammar

Four function facts are deliberately **not** part of the `function(...)` grammar. They are recorded in Darkmatter's Rust function registration instead, and every consumer (runtime binding, the compatibility API, DMLS) reads the same registration:

- **Lazy evaluation.** `and` and `or` stop evaluating their arguments as soon as the answer is known. A signature cannot say this.
- **Type-predicate narrowing.** When `is_number(x)` returns `true`, `x` is known to be a number inside the guarded branch. A `returns(boolean)` alone does not express this. `2026-10-16-type-narrowing` creates the fact registry that `TypeScope` reads and registers the predicates that exist today; the one-pass migration keeps every predicate's fact in that registry, declares each as true-iff or true-implies, and defines no second fact store.
- **Conversion recovery.** `number()` and `round()` select a supplied fallback when their first argument fails to bind (see [Function Contracts Carried from the Charter](#function-contracts-carried-from-the-charter)).
- **Source-type exclusion.** `length` rejects a boolean argument before the boolean → text conversion could reach its `string` option. The parameter union alone cannot say this.

## Requirements

### Grammar: Types Added

Document properties remain optional by default. Function parameters and tuple positions are required by default; `?` permits omission. This distinction makes `parameters(x: number)` mean one argument and `tuple([number, number])` mean two positions without adding `required` to each.

Inside parameters and tuple positions, a type describes the value itself: the document-property optional-null wrapper is not applied. `x: number | null` still requires an argument, but accepts explicit null; `x?: number` permits omission but rejects explicit null. Likewise, `tuple([number?])` accepts `[]` and `[1]`, but not `[null]`. Phase A's null-admitting **property** presence rules do not make function arguments or tuple positions optional.

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

Element order is therefore fixed: required elements, then optional (`?`) elements, then at most one spread. Any other order is a schema-load error. The spread must name an array type (`...number[]`, including a grouped union array); a scalar spread is invalid. `tuple([])` is the empty tuple and accepts only an empty array. A supplied optional position is checked normally; omission never shifts a later supplied value to a different position.

##### Tuples in JSON Schema

Tuples are valid anywhere a type is, including as property types in a document's `$schema` (for example `point: tuple([number, number])`). They compile to standard JSON Schema: `prefixItems` for the positional elements, `minItems` for the count of required elements, and `items` for what follows (`false` for a fixed-length tuple, the spread's element schema for a variadic one).

#### Functions

Although functions will not be a strong requirement for many Markdown documents' schemas, we have an immediate need to express function signatures.

The easy part is the generic **type**, which is just called `function`. The complications arise in creating a set of **constraints** that allows ergonomic, understandable syntax while complementing the rest of the grammar.

**A `function` type is description-only.** It describes a callable; it never validates data:

- it may appear as a named type in a schema file's `types:` mapping, and nested inside another function's parameters or return type or inside a tuple element;
- a document schema (`$schema` in a Markdown file's frontmatter) that reaches a function type is a **schema-load error**, including through a named reference, union, array, tuple, or nested object; checking only direct properties would leave invalid shapes loadable;
- no JSON Schema output is produced for `function`.

Parsing a `function(...)` type produces a description only. It does not create an executable function; a Darkmatter function also needs a registered Rust implementation. Higher-order signatures may be described, but this spec adds no function runtime values, callbacks, or callable serialization. An executable registration whose parameter or return requires a function value fails catalog registration; unused description-only named types remain legal.

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

    The `?:` symbol marks `age` as an _optional_ parameter. Optional parameters must follow all required parameters. Names must be unique. At most one rest parameter is allowed, it must be last, and it cannot carry `?`. `void` cannot be mixed with other parameters. Unknown constraints and duplicate singleton constraints (`parameters`, `returns`, `category`, `fallible`) fail schema loading; repeated `example` constraints are allowed.

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

    The existing file form, `example(./file.yaml)`, which attaches example _files_ to any property (see [Schema Definition](../../../darkmatter/docs/topics/schemas/definition.md#example-constraint)), is also allowed on `function`. The two forms coexist and are told apart by the `=>`. An example whose result spans several lines uses the file form.

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

Which signature a call uses is decided by the order-independent rules in [Overload Selection](#overload-selection); reordering the list never changes the outcome. Two overloads with identical binding contracts are a **catalog-load error**. Compare resolved parameter types, constraints, optional/rest positions, and registered source exclusions; parameter names, descriptions, examples, categories, return types, aliases such as `any`/`unknown`, and union ordering do not distinguish overloads. This prevents spelling differences from creating a call that can only be ambiguous.

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
- `(A | B)[]` uses the list-of-union representation that `2026-10-14-union-type-constraints` introduces: one array whose `items` is `{ anyOf: [A, B] }`. It is the same value as a list of a union-typed named type (`t: [A, B]` used as `t[]`), lowers to the same JSON Schema, and is walked by the same union-aware consumers. This spec adds no second representation of a list of a union.

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

The [Schema Definition](../../../darkmatter/docs/topics/schemas/definition.md#malformed-envelopes) rules currently treat unsupported top-level keys beside `types:` as an error. That rule must be relaxed to allow `categories:`.

### Function Catalog Migration

Today Darkmatter embeds `darkmatter/docs/schemas/expression-functions.yaml` at build time and uses it to describe its expression functions. The draft replacement is `claudine/docs/schemas/partials/functions.yaml`.

Requirements:

1. **Home.** The new catalog is `darkmatter/schemas/partials/functions.yaml` (decided 2026-10-10; charter M8), in line with `2026-09-17-remove-strict-mode`. Its companion `claudine/docs/schemas/partials/mutations.yaml` describes Darkmatter's document mutations and moves with it to `darkmatter/schemas/partials/mutations.yaml`. Darkmatter embeds the catalog; Claudine reads it through Darkmatter's library API rather than from its own copy, and Darkmatter never depends on Claudine for its built-ins.
2. **Consumers.** Every current consumer switches to the new catalog: the Darkmatter library, DMLS, and `claudine context --expressions`. Preserve registered aliases and case-insensitive lookup, but list each canonical function only once. Unknown-function suggestions use the same registry. Schema changes must update public catalog descriptors and generated references together; consumers must not retain the old reduced type vocabulary or a second hand-maintained signature list.
3. **Discovery.** Both files are types-only schema files. Types-only files never auto-apply to documents; 2026-09-17-remove-strict-mode already guarantees this, so no extra exclusion is needed.
4. **Completeness.** The draft is missing 14 functions that are still implemented. Each must be added: `has_binary`, `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli`, `package`, `package_area`, `recent_commits`, `ipv4`, `ipv6`, `ping`, `ping_under`, and `as_markdown`. Two new functions join the catalog: `keys(obj: object): string[]` and `values(obj: object): any[]` (see [Map Functions](#map-functions-keys-and-values)). `is_file_type`, which some drafts already list, is added by `2026-10-31-type-aware-evaluation`, not here.
5. **Examples carried over.** All 117 examples in the old catalog are carried over: the 72 marked `verification: executable` as executable examples, and the rest as `display-only:` examples with their existing reason, or as file-form examples where the result spans several lines.
6. **Categories.** The draft's `categories:` list gets a real description for every entry (several are placeholders) and gains every category the catalog uses: today `Git`, `CI/CD`, and `Date Arithmetic` are used but not listed, and the missing network functions need a category such as `Network`.
7. **Dropped fields.** The old catalog's `order` field is not needed: documentation groups by category and sorts alphabetically within each category. `mutations.yaml` drops its top-level `name:` key. The draft's top-level `behavior:` block (the charter's metadata form) is dropped too: a schema file holds only `kind`, `categories:`, and `types:`, and the facts it carried live in Rust registration (see [Not expressed in the catalog grammar](#not-expressed-in-the-catalog-grammar)). Its `file_exists` `advisory_file` entry has no successor ([`file_exists` Has No Exception](#file_exists-has-no-exception)), and its `is_file_type` entry moves with that function to `2026-10-31-type-aware-evaluation`.
8. **Return shapes.** The old `literals` and `nullable` return fields (used only by `ping` and `ping_under`) become `|` unions, for example `returns(boolean | enum("unstable") | null)` for `ping_under`.
9. **Shared types instead of `pair`.** The old `pair` field linked `recent_commits(count)` to the `ctx.recent_commits` variable. It is dropped: the function's return and the variable use one shared named type, and each description cross-references the other.
10. **Grammar normalization.** `parameters()` becomes `parameters(void)`, and `name: type(optional)` becomes `name?: type`. Accommodation-only `any` parameters are narrowed to what the function needs. A parameter keeps (or gains) `numberlike` or `boolish` only where the function genuinely needs the caller's original form, such as to return the same type it was given; otherwise it becomes `number` or `boolean`.
11. **Draft corrections.** Signatures and descriptions must agree:
    - `length` describes strings, arrays, and objects but its signature omits objects; it becomes `length(val: string | any[] | object): number`, with the registered boolean exclusion (charter M11): `length(true)` is a type error, `length(2.5)` counts the characters of `"2.5"` through number → text, and `length("true")` is `4`.
    - `pr_list` and `cicd_list` take `count: number(integer; min(1); max(100))`, matching their handlers; `recent_commits` takes `count: number(integer; min(1))`. Wherever a handler validates content, the parameter uses the refined type or constraint that expresses it (`date`, `ip-address`, `enum(...)`, ranges), so bad input stays a type error at binding rather than a domain error.
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

Take `is_string(val: any): boolean` called with the runtime number `4`:

- **Step 1.** `val` is declared `any`, so `4` is kept unchanged. There is nothing to convert to.
- **Step 2.** Answering "is this a string?" is the function's domain behavior; it returns `false`.

The same reasoning applies to `and(...values: any)` and `or(...values: any)`: there is nothing to convert, their short-circuiting is registered in Rust, and their truthiness rules (whether `"no"` counts as true) are domain behavior.

A parameter declared `any` is right only when inspecting the original value **is** the function's purpose. `is_string(val: any): boolean` must see the original value; converting it first would make the answer meaningless. `add(a: number, b: number): number` should never declare `any`.

##### `file_exists` Has No Exception

_Added in clarification round 6 (2026-09-25); revised 2026-10-20 (decision 82): the argument is inspected, not converted._

`file_exists(value: any): boolean` binds through the engine like every other function. Its purpose is to inspect the original value, so it declares `any` under the rule above and receives the value unconverted. A `string` or `file` value is checked as a path. Every other value (null, a number, a boolean, an array, an object) returns `false`: `file_exists(42)` does not look for a file named `42`. At design time any argument is category 1: no editor diagnostic, in either setting.

Not converting keeps the call's answer a fact about the author's original value. Only text can make the call `true`, so `2026-10-16-type-narrowing` can register a sound true-side fact (`value` is a present `string | file`) without conversion metadata. The 2026-09-25 ruling, under which `42` bound the text `"42"`, would have left a successful check proving only presence.

This retires the retired charter's advisory exception, under which a known unsupported type such as a number produced an always-false warning and a runtime `false` without a type error. One engine with a special case for one function is two engines; a reader could no longer learn what a function accepts from its declared type. It also retires the charter's separate opt-in warning for `unknown` arguments to `file_exists`: [`expressions.strict`](#editor-diagnostics) already warns for every category 4 argument, `file_exists` included, so no `file_exists`-specific setting exists.

What stays is `file_exists`'s own domain behavior: a provably malformed literal, such as `"https://"` or the empty string, returns `false`. Rejecting malformed strings before any filesystem or network access, and the editor's always-false warning for a provably malformed literal, are delivered by `2026-10-31-type-aware-evaluation`.

#### Null Is a Type Like Any Other

`null` is a formal type. The engine never converts `null` into anything (`0`, `false`, `""`, `"null"`, or omission).

- A function that wants to receive `null` declares a null-admitting type: `min(a: number | null, b: number | null): number | null`. The engine keeps `null`, and the function handles it in its compute step.
- A function that does not declare `| null` on a parameter never sees a `null` there. A null argument to such a parameter is a type error. This includes an `unknown` (`any`) parameter: only an explicit `| null` admits null, so a function that accepts anything including null declares `unknown | null`.

**Example.** A document declares `low: number` (optional, so its design-time type is `number | null`) and leaves `high` untyped. The frontmatter omits `low` and sets `high: pear`. The call `min(low, high)` against `min(a: number, b: number): number`:

- `a` receives `null`. `min` does not declare `| null`, so this fails.
- `b` receives the text `"pear"`. It does not convert to a number, so this fails.
- Result: one type error reporting both `a` and `b`. `min` is never called.

##### Which Functions Accept Null

_Decided 2026-10-10 (the migration-ledger null rule)._ Every catalog signature is assigned by one rule, and the [behavior-change ledger](#one-pass-migration) records each outcome:

| Function family | Parameter | Result for `null` | Example |
|---|---|---|---|
| Text transforms (`upper`, `lower`, `trim`, `capitalize`, …) | declares `\| null` | returns `null` | `upper(null)` → `null` |
| Predicates (`is_*`, `has_*`) | declares `\| null` (or `any`) | returns `false` | `has_key(null, "a")` → `false`; `is_null(null)` → `true` because inspecting `null` is its purpose |
| Arithmetic and comparison (`min`, `max`, `abs`, `add`, …) | does not declare `\| null` | type error | `min(null, 5)` → type error |
| Any other function | follows the nearest of the three analogies above; the ledger records the choice and its reason | | `file_exists(null)` → `false` (a predicate) |

Booleans and numbers reaching a text parameter convert to text through the one engine like everywhere else, so `has_command(true)` looks for a command named `true`. `length` is the one function that rejects booleans (see [Function Contracts Carried from the Charter](#function-contracts-carried-from-the-charter)).

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
- **Precision.** The result is an `f64`, subject to the exactness check in [Numbers](#numbers): `"9007199254740992"` converts, `"9007199254740993"` is a type error because `f64` cannot hold it, and `"0.1"`, `"4.0"`, and `"1e3"` convert to the nearest `f64` in the ordinary way.

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

Exact spelling edge cases (trailing dot, doubled underscores, whitespace inside boolean words) are detailed in [coercion-design.md](./coercion-design.md#numeric-and-boolean-boundaries). Darkmatter has one canonical numeric-text parser (charter M13): the engine, `numberlike` validation, `number()`, `round()`, and `numberlike`/`boolish` JSON Schema formats all call it, so they agree on every spelling and boundary.

#### Numbers

_Decided 2026-10-10; replaces decisions 23 and 33 and the charter's M4–M7, M15, and M16 where they differ._

**One representation.** Every Darkmatter number is an `f64`: literals, frontmatter values, converted text, function arguments and results, calculations, and comparisons. `number(integer)` is a constraint on the value (no fractional part, and below `2^64`), not a separate storage type. There is no exact-integer or arbitrary-precision subsystem.

**The exactness check.** A whole number must be represented exactly, whatever its spelling: `9007199254740993`, `9007199254740993.0`, and `9.007199254740993e15` are all the same whole number and all error. A decimal (a value with a non-zero fractional part) takes the nearest `f64`, so `0.1` is accepted. A non-zero value that underflows to zero is an error. Otherwise the engine reports a type error with the reason "cannot be represented exactly". Whether a value is whole is decided on the mathematical input before conversion, so YAML/JSON loaders, expression literals, and numeric schema metadata keep the numeric spelling (or an equivalent lossless parsed value) until admission; `f64` stays the only runtime representation. A caller that supplies an existing `f64` gets a finiteness check only, since Darkmatter cannot recover digits lost before the value reached it. It never rounds, truncates, saturates, or clamps a whole number to make it fit. The check applies wherever a value becomes a number: text converted by the engine or by `number()`/`round()`, native YAML or JSON numbers in frontmatter and function arguments, and numeric literals in expressions.

| Input | Result | Why |
|---|---|---|
| `"9007199254740992"` (2^53) | `9007199254740992` | exactly representable |
| `"9007199254740993"` | type error | `f64` would store `9007199254740992` |
| YAML `count: 9007199254740993` against `count: number` | validation error | the same check on a native integer |
| `"18446744073709551615"` (`u64::MAX`) | type error | `f64` would store `2^64` |
| `"0.1"`, `"1.2"`, `"1e3"` | the nearest `f64` (`0.1`, `1.2`, `1000`) | decimals take the nearest `f64`; `1e3` denotes the exactly representable whole number 1000 |
| `"9007199254740993.0"`, `"9.007199254740993e15"` | type error | a whole number in decimal or exponent spelling gets the same whole-number check |
| `"1e400"` | type error | not finite |
| `"1e-400"` | type error | a non-zero value would become `0` |

**Arithmetic and comparison** (charter M15). Expression operators and numeric functions compute in `f64` with ordinary binary rounding. A non-finite result (overflow, division by zero) is an error, never a saturated, wrapped, or infinite success. Numeric equality and ordering agree on the represented values; every saturating integer cast in operators and handlers is removed. String concatenation, non-numeric comparisons, truthiness, and short-circuit evaluation keep their current behavior.

**Presentation** (charter M6). A number with no fractional part displays without a decimal suffix (`4.0` displays as `4`); meaningful fractional digits are preserved; formatting never narrows a value through an integer cast. This governs how a number becomes text everywhere, including number → text conversion, `contains`, and rendered output.

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

In frontmatter this preserves existing inline-object conversion and adds the missing paths through unions, preservation types (`numberlike`/`boolish`), and pattern-key dictionaries. Missing optional object properties remain missing inside the engine; defaults and top-level optional-property materialization stay with their existing schema stages.

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
| text or native numbers `f64` cannot hold exactly (`"9007199254740993"`, `"1e400"`) | silently rounded or not distinguished | type error ([Numbers](#numbers)) |
| root-level unions | first option, in index order, that validates after coercion | order-independent [union rule](#unions-choosing-an-option) |
| property-level unions | converted only when exactly one option validates | order-independent union rule |
| nested objects | nested inline objects already convert, but unions, `numberlike`/`boolish`, and dictionary (pattern-key) values inside a nested object do not | converted at every declared depth, including those three |
| a number or boolean given to `enum(...)` or a string literal | rejected | converted to text, then checked against the allowed values (`2` against `enum("1","2")` becomes `"2"`) |
| values under a pattern key (`<string>: number`) | not converted | converted like any declared property |
| raw JSON Schema (`$ref`, `$defs`, type arrays, `oneOf`, mixed-type `enum`, `patternProperties`, `additionalProperties`, `exclusiveMinimum`/`exclusiveMaximum`) | converted only through the few keywords the old walker reached | converted through every keyword the engine's JSON Schema translator models; keywords it does not model (`allOf`, `not`, `if`/`then`/`else`, `dependent*`, `propertyNames`, `contains`, `unevaluated*`, `multipleOf`, remote `$ref`) are validated on the final value but drive no conversion |
| key order of a value serialized into `json`/`yaml` | sorted alphabetically | the order the value was written in |
| `numberlike` and `boolish` values | numeric text normalized to a real number; `"true"`/`"false"` normalized to a real boolean | validated, then kept **exactly as written** (see [`numberlike` and `boolish`](#numberlike-and-boolish)) |
| text accepted by `numberlike` | only `^-?\d+(\.\d+)?$` | the same lenient spellings as `number` |
| text accepted by `boolish` | only `true`/`True`/`TRUE` and `false` equivalents | the full boolean word list, any letter case |

Validation-only calls stay non-mutating, and compose's write-back of converted values is unchanged in shape. The JSON Schema that `numberlike` and `boolish` compile to (today `anyOf` a number or boolean and a text pattern or enum) must accept the same text as the engine: both compile to the custom formats `darkmatter-numberlike` and `darkmatter-boolish`, registered with the validator and backed by the canonical parser, because a regex cannot express the exactness check. The resulting change to `md schema detect --format json` output and its snapshots is intentional; external consumers that ignore unknown formats become looser, which is acceptable.

#### Raw JSON Schema Conversion Boundaries

Reader's note: a conversion translator is not a replacement validator. Raw schemas retain Draft 2020-12 behavior: `oneOf` requires exactly one matching alternative, `$ref` siblings still apply, and a key matching both `properties` and `patternProperties` must satisfy both. `additionalProperties` applies only to keys covered by neither. These rules differ from the SimplifiedSchema dictionary convenience rule and must not be erased during translation. See [JSON Schema's applicator rules](https://json-schema.org/draft/2020-12/json-schema-core#section-10) and [reference rules](https://json-schema.org/draft/2020-12/json-schema-core#section-8.2.3).

Candidate conversions are built from the original value, independently; never feed one candidate's changes into the next. Discard any candidate that fails the complete applicable schema, including unmodeled assertions, before choosing among valid candidates with the union rules. If the original value fully validates, keep it. For overlapping property declarations, commit only a conversion satisfying every applicable declaration; conflicting converted readings error instead of depending on traversal order. No partial write-back occurs after failure. For example, `{oneOf: [{type: number}, {type: integer}]}` rejects `"3"`: the converted `3` matches both alternatives.

Local references use the validator's resolved identity and dialect; translation must not treat every `#...` as a root JSON pointer when `$id` or anchors change its meaning. Unmodeled or cyclic shapes remain validation-only with a bounded traversal; they never become unconstrained success. Unresolved references retain existing schema errors and never trigger a new network fetch. Referenced schema resolution stays in the captured request context.

Unsupported conversion keywords still constrain the final value. For `{type: number, multipleOf: 2}`, `"3"` fails and `"4"` may convert to `4`. Raw `integer` retains its standard whole-number meaning; the SimplifiedSchema `number(integer)` ceiling is not applied to a raw schema unless explicitly authored.

#### Spike Rulings (Decided 2026-10-10)

Two throwaway spikes ran on 2026-09-26 (`spikes/single-engine/`, `spikes/coercion-baseline/`). Their open questions are settled:

1. **Engine structure.** One rule core over a neutral shape type, fed by two translators (compiled or raw JSON Schema → shape, used by frontmatter; grammar descriptors → shape, used by function calls and design-time checks). Translators map syntax only and never decide keep/convert/error; a conformance test binds every grammar case through both translators and requires identical outcomes. The callers that pick union options themselves today (`compose/schema_validation.rs::root_schema_arm_applies`, `schemas/rewrite.rs`, `schemas/example.rs`) call the engine instead.
2. **Raw JSON Schema** converts through every keyword the translator models (the Changes table row above).
3. **The Changes table** includes enum/literal refined text and pattern-key values (rows above).
4. **Key order.** The workspace enables `serde_json`'s `preserve_order` feature, so a native mapping serialized into `json` keeps its written order (criterion 33). Because Cargo unifies features across the workspace, every crate's `serde_json::Map` becomes insertion-ordered; audit existing consumers that rely on sorted serialized output before updating expectations. Preserve an established stable-order contract by sorting explicitly at that consumer; update snapshots only where insertion order is the intended change. Include recursively nested maps and maps built by functions: their order is insertion order, which need not be source order. `docs/dependencies.md` records the feature.
5. **Nested objects.** The row is restated as above: the gaps were unions, `numberlike`/`boolish`, and dictionary values inside nested objects, not nested objects as such.
6. **Null.** The [migration-ledger null rule](#which-functions-accept-null) replaces the spike's suggested `propagates-null` flag.
7. **Conversion functions.** `number()` and `round()` take the charter contract ([below](#function-contracts-carried-from-the-charter)).
8. **Exact integers.** Superseded by the exactness check: `"9007199254740993"` is rejected at binding, so no handler can lose its digits.
9. **Boolean → text in lookup functions.** Accepted as intentional: `has_command(true)` looks for a command named `true`.
10. **Refined types over `string`** wherever a handler validates content (catalog migration item 11).

#### Function Contracts Carried from the Charter

_Decided 2026-10-10: these charter-only contracts are owned by this spec and land in the one-pass migration._

- **`number(x: numberlike, fallback?: number): number`** (charter M12–M13). If `x` binds, the result is its numeric value (`number("4")` → `4`, `number(" 4 ")` → `4`). If `x` fails to bind for any reason — malformed text, `null`, a boolean, a list, an object, a value that fails the exactness check — a supplied fallback is returned (`number("pear", 7)`, `number(null, 7)`, `number(true, 7)`, and `number([], 7)` all return `7`), and without one the call returns the engine's type error (`number("pear")` errors; it no longer returns `0`). A supplied fallback is always validated on its own: `number("4", "x")` is a type error even though `x` is usable. Omitting `x` is an arity error. The parameter is named `fallback` (the draft's `def_val` and the old catalog's `default` are retired).
- **`round(x: numberlike, fallback?: number): number`** (charter M14). The same binding and recovery as `number()`, then the selected value is rounded to the nearest whole number, halfway values away from zero: `round("3.7")` → `4`, `round(-2.5)` → `-3`, `round("pear", 2.7)` → `3` (the fallback is rounded too), `round("pear")` → type error. The result is `number`, not the narrower `number(integer)`, and no `i64` cast is involved.
- **`length`** (charter M11). Strings count Unicode characters, arrays count elements, objects count keys, and numbers count the characters of their displayed text; empty values give `0`. Booleans are rejected before conversion (`length(true)` is a type error at runtime and category 5 at design time). Null follows the [ledger rule](#which-functions-accept-null).
- **`validate_schema(file, obj?)`** (charter M9). The two-argument form merges `obj` into the page's frontmatter using the existing explicit-override merge (nested objects merge, supplied values win, arrays and explicit `null` replace), validates the combined page in memory, and returns `false` when it is invalid. The file and both inputs are never modified; file-loading and schema-processing failures stay errors; a combined page that declares no schema still succeeds. The one-argument form validates the page's own frontmatter. Today's implementation ignores `obj`; that is corrected. DMLS never invokes it.
- **`contains(haystack, needle: any)`** (charter M10). Text-based containment is preserved: a string haystack is a case-sensitive substring search; array elements and object values (not keys) are compared to the needle by rendered text, so `contains([4], "4")` is `true`. Rendering keeps strings, renders `null` as empty text, numbers per [presentation](#numbers), and arrays and objects as compact JSON. A numeric or boolean haystack converts to the string option.
- **Operators** (charter M15): see [Numbers](#numbers).

#### Map Functions: `keys()` and `values()`

_Decided 2026-10-10._ Two functions join the catalog for iterating over key/value maps such as a concurrent group's `group.legs`:

```yaml
keys: |-
    function(
        parameters(obj: object);
        returns(string[]);
        category(Collection);
        example(keys({b: 1, a: 2}) => ["b", "a"]);
    ) -> Returns the keys of a map, in the order they were written.
values: |-
    function(
        parameters(obj: object);
        returns(any[]);
        category(Collection);
        example(values({b: 1, a: 2}) => [1, 2]);
    ) -> Returns the values of a map, in the order their keys were written.
```

Order is the map's stored order, which `preserve_order` makes the written (insertion) order. Both belong to the existing `Collection` category. Their null handling follows the [ledger rule](#which-functions-accept-null), recorded in the ledger like every other function.

#### Performance

This spec sets no numeric performance targets. If the existing spike results do not already answer the implementation's cost question, record one quick before/after sample on one host using an existing representative fixture, with checking cost separated from unrelated I/O. Explain a visible regression in the implementation log. No additional hosts, workload matrix, repetitions, statistical threshold, or CI job are required. Existing expression depth and resource bounds are preserved; compile descriptors once and reuse them rather than parsing the catalog for each call or editor diagnostic.

### Design-time Checking

_Confirmed in the use-case review of 2026-09-25; scope settled in clarification round 3 and revised in clarification round 5, which moved DMLS diagnostics into this spec._

#### Design Time and Runtime

An editor sees only design-time information: schemas, signatures, and literals, never runtime values. Darkmatter's passive call check reads the same design-time setup before any expression is evaluated; schema validation and conversion retain their later position after initial frontmatter interpolation. Unlike TypeScript, declared types are still available at runtime.

So the five categories below drive two consumers:

- **Darkmatter** (this spec) rejects a document at composition, before evaluation begins, when any call is category 5. This applies to every function; because the [one-pass migration](#one-pass-migration) moves every function onto the engine at once, there are no legacy functions to exempt.
- **DMLS** (this spec, since clarification round 5) turns them into editor diagnostics as the author types; see [Editor Diagnostics](#editor-diagnostics).

For everything else, the engine decides per runtime value when the call runs. A call that raises nothing at design time can still fail at runtime; silence means "not proven wrong," not "proven right."

**Check before evaluation.** The current Darkmatter [compose pipeline](../../../darkmatter/lib/src/markdown/compose/mod.rs) evaluates frontmatter interpolation before schema validation. Therefore the new call check is a passive preparation step before that interpolation, using a resolved schema and declared types, without executing expressions to discover types. Reuse Darkmatter's [prepared-expression validation](../../../darkmatter/lib/src/markdown/compose/expression/prepared.rs), which already walks every branch without evaluation. Preserve the established interpolation → schema conversion → shell expansion order after this check. The check must reject a bad frontmatter call before a sibling expression can perform a side effect. DMLS checks the same authored expressions and types.

```mermaid
flowchart LR
    A[Resolve schema and declared types] --> B[Check authored calls without evaluation]
    B --> C[Interpolate frontmatter]
    C --> D[Validate and convert frontmatter]
    D --> E[Expand approved shells and compose body]
```

Check only executable authored syntax: frontmatter expressions, body expressions, and conditional block expressions within the requested composition surface. Examples in fenced code, literal tokens, inserted data, and a body excluded by `only_frontmatter_surface` are not executable syntax. A transcluded document is checked in its own schema and origin before its evaluation; this spec does not require fetching unseen documents solely to reject the parent before any parent evaluation.

**Branches that do not run** _(decided 2026-10-10)_. The check covers every call in every expression of the document, including both branches of a ternary, both sides of `||` and `&&`, and the body of a conditional block whose condition is false. A category 5 call in a branch that will never run still rejects the document at composition, exactly as DMLS reports it as an error. `true ? 'ok' : min(flag, 5)` with `flag: boolean(required)` is rejected even though `min` would never be called.

#### Expression-variable Type Projection

_Decided 2026-10-10._ The categories need the design-time type of each argument. This spec builds the minimal projection that supplies it; `2026-10-31-type-aware-evaluation` extends this same projection and never builds a second one.

One Darkmatter library function derives, from a document's `EffectiveSchema`, a map from each top-level variable name to its design-time type. Composition and DMLS call the same function; neither walks schema syntax itself, and Claudine does not reproduce it.

That function is the declared-type layer of `TypeScope`, which `2026-10-16-type-narrowing` lands first; this spec completes it to the table below. The categories read each argument's type from the `TypeScope` at the call site, never from the projection directly, so every guard's narrowing (including literal narrowing from `==`) reaches classification.

| What the argument is | Design-time type in this spec |
|---|---|
| A top-level property declared `A(required)` in the document's typed projection (`EffectiveSchema.simplified`, single shape) | `A`, constraints included |
| A top-level property declared `A` without `required` | `A \| null` |
| A top-level property-level union (`[string(required), number]`) | the union of its options, using the shared effective presence rule: a required marker on any arm requires the property unless its whole-value type explicitly admits null or `unknown` |
| A top-level property that the schema does not declare, or any variable when the effective schema has no typed single-shape projection (raw JSON Schema, a mixed root union) | `unknown` |
| A static frontmatter access (`fm.size` or `fm["size"]`) | the same projected type as the top-level property `size` |
| A path below a top-level property (`meta.count` or `fm.meta.count`), bare `fm`, a dynamic index (`fm[key]`), any other reserved root (`ctx`, `env`, `err`, `timing`, `current`, `seq`, `loop`, `group`), or a host global | `unknown` |
| A literal (`5`, `"pear"`, `[1, 2]`) | judged by its value, as the [classification notes](#argument-type-categories) say |
| Any other expression (a call, an operator such as `maybe \|\| 0`, a ternary) | `unknown` |

Rules:

- The projection is passive: it reads no file, runs no expression, and never inserts or replaces a property to supply a name or a type.
- It is deterministic and independent of mapping order or validator error order.
- Frontmatter namespace access follows `2026-10-18-fm-namespace`: `fm.size` and `fm["size"]` are static aliases of the document property, not host globals. A bare reserved name still resolves to its registered root, so a document property named `env` is typed through `fm.env`, never by assigning its type to the bare `env` root. This prevents a namespace spelling from bypassing a proven type error without adding nested-path inference.
- Use the same resolved presence calculation as schema lowering and validation, including inherited and hoisted markers, rather than requiring every union arm to carry `required`. An optional `A` includes `| null`; a concrete required whole value does not. An explicitly null-admitting whole value stays optional even with `required`, as Phase A specifies. `unknown(required)` remains `unknown`. This corrects accidental disagreement with the prerequisite specs; it is not a new presence policy.

Under this projection rows a–e of the worked example below are exact, and row d (`min(flag, 5)`) is rejected at composition. Rows g and h are exact through `TypeScope`'s predicate narrowing. Rows f and i need return types and operator typing, which `2026-10-31-type-aware-evaluation` adds.

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

- **Unions.** A union argument whose non-null options all fit (exactly or by an always-safe conversion) is category 1, or category 2 if it also admits `null` that the target rejects. If every option is known to be wrong, it is category 5. Anything in between is category 3.
- **Conversions.** Typed text to a `number` or `boolean` parameter is category 3. A typed number or boolean to a text parameter is category 1. A native non-null object, list, number, or boolean to `json`/`yaml` is category 1; arbitrary text is category 3 because it must parse as the target content format. Null is accepted only by a null-admitting target, and `unknown` follows category 4. For example, literal `"{a"` to `json` is category 5, while typed `string` to `json` is category 3.
- **`numberlike` and `boolish`.** A `numberlike` argument to a `number` parameter is category 1 (its text always converts), and so is a `numberlike` argument to a `string` parameter, or a `number` argument to a `numberlike` parameter. Typed text to a `numberlike` parameter is category 3. `boolish` follows the same pattern against `boolean`.
- **An exact fit is `is_subtype`.** Whether an argument type fits a parameter type without conversion is decided by `is_subtype` (`schemas/simplified/subtype.rs`), the single type-acceptance relation `2026-10-16-type-narrowing` establishes; a narrowed `literal('release')` therefore fits `string` and an `enum` with that member, and `literal(4)` fits `number`. The conversion rules in these notes apply only where `is_subtype` does not, and no second acceptance relation is built.
- **Constraints and refinements do not warn.** Categories are judged on types; constraints are enforced at runtime. A typed `string` to `string(min(3); max(3))`, a typed `number` to the same parameter, and a typed `number` to `number(integer)` are all category 1. Refinements (`file`, `date`, `url`, `email`, `enum`) are judged the same way: a typed `string` or `number` to a `file` or `enum` parameter is category 1, and the refinement is checked on the value at runtime.
- **Literals** are the only arguments judged by their actual value, constraints and refinements included: a literal certain to bind (`"4"` to `number`, `2026` to `string`) is category 1; one that cannot (`"pear"` to `number`, `0` to `number(min(1))`, `"purple"` to `enum("red","green")`, `min("pear", 5)`) is category 5.
- **Wrong argument count**, and a definite overload ambiguity, are always category 5.
- **Null accepted by the target.** `number | null` to `number | null` is category 1; category 2 is only for an otherwise compatible source that can be null when the target rejects null. Only an explicit `| null` accepts null: any non-null source to `unknown` is category 1, a source that can be null to a plain `unknown` parameter is category 2, and any source to `unknown | null` is category 1.
- **Registered exceptions.** Source exclusions and conversion recovery are part of whole-call classification, not just the runtime binder. `length(true)` is category 5 even though boolean → text normally works. `number("pear", 7)` and `round(true, 7)` are category 1 because their valid fallback recovers the first-argument binding failure; their first argument must not independently cause a category 5 rejection. `number("4", "x")` remains category 5 because the supplied fallback must bind. A typed incompatible first argument with a valid literal fallback is likewise recoverable.
- **Passive refinements.** Literal checks perform only validation that needs no I/O. File existence, remote access, command lookup, and repository state are domain or composition concerns; they cannot make a literal category 5 in an editor. A structurally valid file reference remains runtime-checked against the captured request context.

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
    # untyped_var has no schema entry     → unknown
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

Darkmatter rejects row d at composition. Rows g and h are exact in this spec: `TypeScope` (`2026-10-16-type-narrowing`) narrows `foo` in each guarded branch, and they are required test cases here. Rows f and i depend on function return types and operator typing, delivered by `2026-10-31-type-aware-evaluation`, and are required test cases there; under this spec the outer argument in each is `unknown` (category 4) and the composition verdict is "not rejected". An earlier example, `max(foo || 0, 100)` with `foo: string | number`, was withdrawn: `foo || 0` keeps the `string` arm, so it is category 3.

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

Type-predicate functions such as `is_number(val: any): boolean` accept any value and prove the variable's type on both sides of a guard (rows g and h): a true-iff predicate narrows its true side to the tested family and removes that family on its false side. That narrowing fact comes from Darkmatter's Rust registration (see [Not expressed in the catalog grammar](#not-expressed-in-the-catalog-grammar)) and is applied by `TypeScope` (`2026-10-16-type-narrowing`), not by the call check.

Many Darkmatter functions are lenient today and declare `any` parameters. The one-pass migration tightens them all; afterwards a function given the wrong type (a type error from binding) and a function whose own work failed (a fallible function's domain error) are clearly distinct.

#### Design-time Compatibility API

Darkmatter exposes a fast design-time query: "given an argument of design-time type A and a parameter of type B, which [argument-type category](#argument-type-categories) applies?" The answer is one of the five categories (or a classification that maps exactly onto them). Literals are checked by value through the same rules the runtime uses. DMLS is the most important client. It also exposes whole-call classification with arity, overload selection, source exclusions, and recovery metadata, so callers cannot misclassify a recoverable argument in isolation. A batch form may be offered if measurement shows a benefit.

The rules live in Darkmatter only; DMLS and Claudine call this API rather than reproduce them. See [coercion-design.md](./coercion-design.md#public-compatibility-api-and-dmls).

#### Editor Diagnostics

_Added in clarification round 5 (2026-09-25); this revises the earlier decision to leave editor diagnostics to the charter's Phase E._

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

This is the only setting for call diagnostics. No function has a setting of its own; in particular, the retired charter's opt-in unknown-input warning for `file_exists` is replaced by this flag (see [`file_exists` Has No Exception](#file_exists-has-no-exception)).

### One-pass Migration

_Added in clarification round 4 (2026-09-25); replaces the earlier pilot-then-rollout plan._

Every catalog function moves onto the shared coercion engine in **one pass**. There is no pilot stage and no period in which some functions bind through the engine while others still convert their own inputs. The human: _"We convert all functions in one pass!"_

Requirements:

1. **Every function.** All catalog functions (about 110, including the 14 [missing from the draft](#function-catalog-migration)) are registered against their new `function(...)` signatures and called through the engine's binding step.
2. **Corrected signatures.** Each signature says what the function needs ([catalog migration](#function-catalog-migration) items 10 and 11).
3. **No function converts its own inputs.** Every function's own arity checks, type checks, and conversion code are removed, along with the shared conversion helpers that only served them. Conversions that _are_ a function's purpose, such as `number()`, remain as documented domain behavior.
4. **The old catalog is deleted.** `darkmatter/docs/schemas/expression-functions.yaml` has no remaining reader and is removed.
5. **Per-function items resolved.** The [per-function items](#per-function-items-for-the-catalog-migration) are decided within this spec, and the catalog records the outcome.
6. **Lazy calls.** `and` and `or` retain short-circuit evaluation. The passive check visits all authored arguments, but the runtime binder evaluates and binds only an argument the function actually needs. A skipped argument never runs a function, filesystem probe, or provider call. Do not route lazy calls through an eager argument-array builder merely to reuse binding.
7. **Return contracts.** Returned values must already satisfy the declared return type; never coerce a handler result to conceal a wrong implementation. Validate pure value constraints without probing files or remotes. A wrong return shape is a typed implementation-contract error, distinct from argument rejection and a declared domain failure. Cover it with a deliberately faulty test handler.
8. **Behavior-change ledger.** Every function's behavior, measured against the baseline in [current-state-coercion.md](./current-state-coercion.md), is recorded as either **preserved** or **intentionally changed**, with a one-line reason for each change. Nothing is left "to be decided". The ledger is reviewed by the human before merge.

**Suggested order within the pass.** The earlier pilot functions (`min`, `is_number`, `number()`, and both `pr_list` forms, which together exercise conversion, `any` pass-through, a constraint, and overloads) are a sensible place to start building, as described in [coercion-design.md](./coercion-design.md#suggested-implementation-order). This is an order of work, not a stopping point: the spec is not done until every function has moved.

#### Risks

The one-pass migration is the highest-risk part of this spec. Every expression function changes at once, and a behavior change in any one of them reaches every document that calls it. The behavior-change ledger and the catalog-wide acceptance criteria are the main safeguards. The coercion-baseline spike (`spikes/coercion-baseline/`) recorded the current behavior of every function over 25 shared inputs (3,575 outcomes in `baseline.json`); the ledger is built by comparing the migrated functions against that baseline, and `spikes/coercion-baseline/ledger-draft.md` is its starting point. The spike found that most regressions come from `null`; the [ledger null rule](#which-functions-accept-null) addresses them. Shipped documents, examples, and tests that rely on a changed result (for example `number("pear")` returning `0`) are updated in the same change.

### Documentation

Every topic in this spec needs complete, high-quality documentation readable by a human audience who:

- is technical, but
- has no knowledge of this monorepo's capabilities, packages, etc.

The language must avoid jargon, and time should be spent making sure clear language, good examples, and smart structure appear in every document. At minimum:

- [Schema Definition](../../../darkmatter/docs/topics/schemas/definition.md) documents `tuple`, `function`, `|` unions (including precedence and grouping), `category`, the function `example(call => result)` form, and `categories:`;
- the "Type Coercion" section of Schema Definition is rewritten to the single rule list: its **Coercion Matrix**, **Never Coerced (Ambiguous)**, **Root Unions**, and **Inline Object and Union Coercion** subsections, and the coercion bullet under **Content-Format Types**, are replaced by the conversion rules, rejected conversions, union rule, and container rules of this spec, and the [frontmatter changes](#changes-to-frontmatter-coercion) are listed for existing users;
- the `numberlike` and `boolish` rows of Schema Definition's type table, which today say their text is **normalized** to a real number or boolean, are rewritten: both accept what `number` and `boolean` accept and keep the value exactly as written;
- the coercion engine is documented once, for frontmatter and function calls alike: the two interfaces of a type, its three actions, conversions, rejected conversions, and type errors;
- the five argument-type categories are documented for document authors, including what Darkmatter rejects at composition (calls in branches that do not run included);
- the numeric model (`f64`, the exactness check, presentation) is documented once, with the table in [Numbers](#numbers);
- DMLS's [diagnostics guide](../../../darkmatter/dmls/docs/diagnostics.md) documents the new call diagnostics, their severities, and the strict-mode setting in its **Severity** and **Configuration** sections;
- the generated function reference (including `claudine context --expressions`) groups functions by category in list order, alphabetically within each category, and shows each function's examples.

### Pre-authorized Preparatory Work

The implementation may do the following without pausing for approval:

- move `functions.yaml` and `mutations.yaml` to `darkmatter/schemas/partials/`, and delete `darkmatter/docs/schemas/expression-functions.yaml` once nothing reads it;
- edit other packages to use the new catalog and API, including the `claudine context --expressions` renderer in claudine-cli and DMLS's catalog consumers;
- add the call diagnostics and the strict-mode setting to DMLS (`darkmatter/dmls`), including its configuration model and tests;
- remove every function's own coercion code and the helpers that only served it;
- regenerate generated documentation, update the `darkmatter` and `claudine` agent skills, and add test fixtures and snapshots;
- change existing frontmatter-coercion and catalog tests whose expectations change because of a behavior change this spec approves;
- enable `serde_json`'s `preserve_order` feature at the workspace level and update the snapshots and tests whose key order changes, recording the feature in `docs/dependencies.md` and `darkmatter/docs/dependencies.md`;
- update shipped documents and examples that rely on a result this spec intentionally changes.

## Decisions Log

Decisions 1–15 were confirmed by the human on 2026-09-25 in a clarification pass; decisions 16–30 the same day in the ten-case use-case review; decisions 31–47 in clarification round 3; decisions 48–56 in clarification round 4; decisions 57–61 in clarification round 5; decisions 62–64 in clarification round 6; decisions 65–76 by the owner on 2026-10-10 in the charter reconciliation. Revised decisions keep their number and say what replaced them. "The parent" in decisions 1–64 means the retired type-system charter.

1. **Home.** This spec is the detail spec for the parent's Phase B; its area is Darkmatter — the work is Darkmatter's grammar and runtime. _(2026-10-10: the charter is retired; this spec is Phase B in its own right, filed with the rest of the rollout pipeline; see 65.)_
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
23. **Whole-number text is exact or rejected** (case 7). A 64-bit integer when it fits, otherwise a type error — never silently change a number. _(Revised 2026-10-10; see 66: one `f64` representation plus an exactness check.)_
24. **Union conversion priority: exact, then number, then boolean** (case 5). `"1"` to `number | boolean` is `1` — "prefer number" beats an ambiguity error. _(Extended in round 3; see 35. Restated in round 4; see 50.)_
25. **Refined text types inherit no other conversions** _(revised in round 3; see 34)_. Was: a number to `file` is an error.
26. **Overload selection** (case 8): unchanged beats converted; otherwise exactly one must bind; order never matters — predictable and reorder-safe.
27. **Containers** (case 9): walk the declared structure, all-or-nothing, report paths, never mutate, never fill or wrap — the same per-value rules at every depth.
28. **Five argument-type categories** (case 10), shared by DMLS and Darkmatter's composition-time check — one vocabulary for editor and runtime.
29. **A `number`-typed argument to a text parameter is category 5** (case 4) _(revised in round 3; see 40)_.
30. **Rows f–i are parent Phase C/D test cases.** Return types and narrowing belong to the parent's phases. _(2026-10-10: those phases are `2026-10-31-type-aware-evaluation`.)_
31. **One coercion engine for frontmatter and function calls.** The human: _"We need a single logic engine … ONE set of business logic."_ Frontmatter validation moves onto it in this spec, with no per-surface exceptions.
32. **The conversion rule list.** Number and boolean → text (always safe); text → number (lenient spellings, exact integers — revised by 66); text → boolean (word list, any case); native values → `json`/`yaml` (serialized). Rejected: boolean → number, null → anything, wrapping into a list, parsing text into structure. The human: _"Converting a number to a string is completely fine! … It's only in the opposite direction (string to number) you have to be careful."_
33. **Whole-number text** means no decimal point and no exponent; `"1e3"` and `"4.0"` take the floating-point path. _(Superseded 2026-10-10 by 66: every number takes the `f64` path and the exactness check.)_
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
63. **`file_exists` has no exception to the single engine.** _(Binding revised by 82: `file_exists(42)` is now `false` without conversion.)_ `file_exists(42)` binds the text `"42"` and checks for that file, with no DMLS diagnostic by default; the parent's advisory always-false exception for known unsupported types is retired, and its opt-in unknown-input warning is replaced by `expressions.strict` — one engine with a per-function exception would be two engines. Its always-false warning for provably malformed literals stays as domain behavior.
64. **The union safety net is confirmed:** any tie neither tie-breaker settles is an ambiguity type error — it keeps "never guess" and "option order never matters" true as the conversion list grows.
65. **The charter is split and retired** (2026-10-10). Phase A is `2026-10-16-core-schema-types`, this spec is Phase B, Phases C–E are `2026-10-31-type-aware-evaluation`. The charter's Phase B annexes moved here as reconciliation input; this spec is the only normative Phase B text.
66. **Numbers are `f64` with an exactness check.** A value `f64` cannot hold exactly is a type error, never silently changed. Replaces 23 and 33 and the charter's integer-storage rules.
67. **The migration-ledger null rule.** Text transforms accept and return `null`; predicates accept `null` and return `false`; arithmetic and comparison reject it; every other function follows the nearest analogy, recorded in the ledger. Replaces the charter's M2.
68. **Where the charter and this spec disagreed on conversion details, this spec wins:** nullable-argument warnings silent by default, lenient spellings, the two tie-breakers, enums and string literals accepting number/boolean → text, and the one-pass migration.
69. **`length(true)` is rejected** — a deliberate per-function exception recorded in registration metadata (charter M11).
70. **`number()` and `round()` take the charter contract:** `x: numberlike, fallback?: number`; any binding failure selects a supplied fallback, otherwise the call errors; `round` rounds half away from zero. Resolves per-function items 1 and 2.
71. **A provably wrong call in a branch that does not run still rejects the document at composition.** Editor and composition agree.
72. **This spec builds the minimal expression-variable type projection** (top-level variables; optional `A` → `A | null`); `2026-10-31-type-aware-evaluation` extends it.
73. **The catalog lives at `darkmatter/schemas/partials/functions.yaml`** (charter M8), with `mutations.yaml` beside it.
74. **Charter-only function contracts are owned here:** numeric presentation (M6), `validate_schema` merging (M9), text-based `contains` (M10), and operator consistency (M15). Strict-root validation and the `is_known_variable_root` hook are struck from the type system.
75. **Spike rulings:** raw JSON Schema converts through every keyword the translator models; the Changes table adds enum/literal text and pattern-key values; `serde_json` `preserve_order` is enabled; the nested-objects row is restated; `pr_list` and `cicd_list` keep `max(100)`, matching their handlers.
76. **`keys()` and `values()` join the catalog** for iterating over key/value maps such as `group.legs`.
77. **Inline review, 2026-10-11: presence follows the prerequisites.** The variable projection uses the shared effective-property presence calculation; parameters and tuple positions keep their separate omission rules.
78. **Inline review: call checks honor registration metadata.** Valid conversion fallbacks are not rejected at composition, lazy calls keep short-circuit behavior, and literal refinements remain passive. The passive document check precedes frontmatter evaluation.
79. **Inline review: raw schemas keep their original assertions.** Conversion candidates must pass the complete applicable schema; `oneOf`, reference siblings, and overlapping key declarations cannot be reduced to permissive unions or first-match traversal.
80. **Inline review: numeric wording distinguishes fractions from whole values.** Exact binary preservation cannot be required for `0.1`. Settled 2026-10-11: whole numbers must be exact in every spelling, decimals take the nearest `f64`, and a non-zero value that underflows to zero is an error.
81. **2026-10-16: one type scope and one subtype relation.** Decided with the owner while writing `2026-10-16-type-narrowing`. The variable projection is `TypeScope`'s declared-type layer; argument types are read from the scope at the call site; predicate facts live in the scope's fact registry; exact fit is `is_subtype`. Rows g and h move into this spec.
82. **2026-10-20: `file_exists` inspects its argument.** Decided with the owner while resolving the `file_exists` Open Question in `2026-10-16-type-narrowing`. `file_exists(value: any): boolean` is an inspecting predicate: a `string` or `file` value is checked as a path, and null, numbers, booleans, arrays, and objects return `false` without conversion or type error. This revises 63's binding (`42` no longer becomes `"42"`) and keeps its other rulings: no per-function exception, no `file_exists`-specific setting, and the always-false warning for provably malformed literals. The handler and catalog entry already behave this way (changed 2026-10-20); the migration keeps the behavior when it moves `file_exists` onto bound arguments.

## Acceptance Criteria

Runtime criteria for function calls run through normal dispatch with a handler-invocation counter; functions named "test-registered" exist only in tests. Frontmatter criteria run through Darkmatter's schema validation and compose paths on fixture documents. Every automated criterion is an L1 test run with nextest through `just test` in `darkmatter/` (including DMLS) and in `claudine/` for the `claudine context --expressions` renderer; the ledger review remains a human review requirement. `just lint` passes in both areas. Grammar, schema, and catalog changes also get a passive shipped-artifact corpus test and an end-to-end test through `md compose`. No test focuses a terminal or browser window, every test is portable to macOS, Linux, native Windows, and WSL2, and no CI cell is added.

### Grammar and Catalog

| # | Criterion | How to validate |
|---|---|---|
| 1 | The grammar parses `tuple` (fixed, optional elements, one final spread), `function` with every constraint, `\|` unions everywhere, and `categories:`. | Parser unit tests over each form in this spec, plus rejection tests for two spreads, a spread that is not last, an optional element before a required one, `?` on a spread, a named tuple element, and a `function` missing `parameters` or `returns`. |
| 2 | Precedence and grouping follow TypeScript. | `string \| number[]` accepts `"a"` and `[1]` and rejects `["a"]`; `(string \| number)[]` accepts `["a", 1]`. |
| 3 | Tuples and `\|` compile to standard JSON Schema. | Snapshot tests of `point: tuple([number, number])`, a variadic tuple, and `id: string \| number` show `prefixItems`/`minItems`/`items` and `anyOf`; `id: string \| number` and the YAML-list form produce identical output. |
| 4 | A function-typed property in a document's `$schema` fails at load. | A fixture document declaring `run: function(parameters(void); returns(string))` under `$schema` produces a schema-load error. |
| 5 | Categories load and emit correctly. | A schema file with `categories: [Math -> ...]` and `category(Text)` fails at load; the same file without `categories:` loads; a non-function property with `category(Math)` emits `x-darkmatter-category: Math`; entries without descriptions and with multi-line descriptions load. |
| 6 | Duplicate overloads fail at load. | A catalog with two `pr_list` overloads with identical parameters produces a catalog-load error. |
| 7 | The new catalog is complete and is the only catalog. | A test compares the names in `darkmatter/schemas/partials/functions.yaml` with registered implementations and finds no gaps either way (`keys` and `values` included); `expression-functions.yaml` is deleted and nothing reads it; `claudine context --expressions` and DMLS read the catalog through Darkmatter's library API. |
| 8 | Executable catalog examples pass, compared structurally. | A test evaluates every example not marked `display-only:`; the count is at least 72. Unit tests show `=> 3` fails against `"3"`, `=> {a: 1, b: 2}` passes against `{b: 2, a: 1}`, and `=> error` passes only when the call fails. |
| 9 | Display-only and file-form examples are shown but display-only ones never run. | The example test reports display-only examples as skipped by design; generated docs include both forms. |

### One-pass Migration

| # | Criterion | How to validate |
|---|---|---|
| 10 | Every registered function binds through the engine. | A test enumerates every registered function and fails unless each has accepted, rejected, and converted-input fixtures run through normal dispatch; on every rejected fixture the counter stays at 0. |
| 11 | No function performs its own coercion. | The per-function arity, type-check, and conversion code is deleted; handlers read arguments only through bound-argument accessors that neither parse nor convert; review confirms no handler inspects an argument's type to convert it, except where conversion is the function's documented purpose (such as `number()`). |
| 12 | `number()` and `round()` follow the charter contract. | Through normal dispatch: `number("4")` → `4`; `number(" 4 ")` → `4`; `number("pear", 7)`, `number(null, 7)`, `number(true, 7)`, `number([], 7)`, and `number("9007199254740993", 7)` → `7`; `number("pear")` → type error; `number("4", "x")` → type error; `number()` → arity error; `round("3.7")` → `4`; `round(-2.5)` → `-3`; `round("pear", 2.7)` → `3`; `round("pear")` → type error. The catalog names the parameter `fallback` and carries executable examples for these cases. |
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
| 21 | The exactness check. | Test-registered `echo(val: number): number` given `"9007199254740992"` returns `9007199254740992`; given `"9007199254740993"`, `"18446744073709551615"`, `"99999999999999999999"`, `"1e400"`, or `"1e-400"` returns a type error whose reason is "cannot be represented exactly" (or "not finite"), and its counter stays at 0; given `"1e3"` returns `1000`; given `"0.1"` returns `0.1`. The expression literal `9007199254740993` is rejected the same way. |
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
| 30 | Lenient numbers and precision. | `n: "+4"` and `n: "1_000"` against `n: number` become `4` and `1000`; `n: "99999999999999999999"` and the native YAML integer `n: 9007199254740993` fail validation. |
| 31 | Root unions are order-independent. | A root union whose options type `x` as `number` and as `boolean` produces the same result for `x: "1"` with the options in either order, following the [union rule](#unions-choosing-an-option). |
| 32 | Nested objects are converted. | `meta: { inner: { count: number } }` given `meta: { inner: { count: "3" } }` becomes `count: 3`. |
| 33 | Native values serialize into `json`/`yaml`. | `config: json` given a native mapping validates and composes as JSON text; `config: json \| yaml` given `{ port: 8080, debug: true }` composes as `{"port":8080,"debug":true}` (written key order, through `preserve_order`); `null` is not serialized. |
| 33a | The completed Changes table. | `n: 2` against `n: enum("1","2")` composes as `"2"`; a value under `<string>: number` given `"3"` becomes `3`; a raw JSON Schema fixture using `$ref`/`$defs`, a type array, and `oneOf` converts `"3"` to `3` at the referenced number property; a nested object holding a `number \| boolean` union, a `numberlike` field, and a pattern-key dictionary converts each per the rules (the `numberlike` field kept as written). |
| 33b | `numberlike`/`boolish` JSON Schema agrees with the engine. | The compiled schema uses the `darkmatter-numberlike` and `darkmatter-boolish` formats; for a shared probe list (including `"+4"`, `"yes"`, `"On"`, `"99999999999999999999"`, `"1e-400"`) the validator and the engine agree on acceptance; the `md schema detect --format json` snapshot is updated. |
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
| 42 | `file_exists` binds like every other function, inspecting its argument. | With a file named `42` present at the resolution root, `file_exists(42)` and `file_exists(true)` return `false` with no type error, and `file_exists("42")` returns `true`; the compatibility API returns category 1 for `number` → `any`, and DMLS shows no diagnostic for the call with default settings. A review confirms no `file_exists`-specific setting exists in DMLS's configuration model. |
| 43 | The ledger null rule. | Through normal dispatch: `upper(null)` and `trim(null)` return `null`; `has_key(null, "a")` returns `false`; `min(null, 5)` returns a type error with the counter at 0; `file_exists(null)` returns `false`. The ledger records every function's null outcome with its family or analogy. |
| 44 | Charter-only contracts. | `length(true)` is a type error at runtime and category 5 from the compatibility API, while `length("true")` is `4`, `length(2.5)` is `3`, and `length({a: 1})` is `1`. `validate_schema` passes M9's required examples (a supplied required field repairs an incomplete page; an invalid override fails a valid page; an empty object equals the one-argument form; the file and inputs are unchanged after repeated calls). `contains([4], "4")` is `true` and `contains({a: "x"}, "a")` is `false`. `has_command(true)` looks for a command named `true`. |
| 45 | Map functions. | `keys({b: 1, a: 2})` returns `["b", "a"]` and `values({b: 1, a: 2})` returns `[1, 2]`; both have executable catalog examples. |
| 46 | Numeric operators and presentation. | `1e308 * 10` and `1 / 0` are errors, not infinities; `2 == 2.0` and `2 < 2.5` agree with ordering; no saturating cast remains (`9007199254740992 + 0` stays `9007199254740992`); `{{ 4.0 }}` renders `4` and `{{ 2.5 }}` renders `2.5`. |
| 47 | `(A \| B)[]` uses the list-of-union representation. | `tags: (string \| number)[]` and the named-type form `t: [string, number]` used as `tags: t[]` lower to identical JSON Schema (`items: { anyOf }`); both accept `["a", 1]` and reject `[true]`; `["2", "yes"]` against `(boolean \| number)[]` converts element by element to `[2, true]` by the union rule. |
| 48 | The variable-type projection. | A library test over the worked-example schema returns `size: number`, `maybe: number \| null`, `count: string \| null`, `flag: boolean`, `foo: string \| number`, and no entry (`unknown`) for `untyped_var`; a raw JSON Schema document and a mixed root union project every variable as `unknown`; projection performs no I/O (failing access stubs) and leaves the frontmatter unchanged. |
| 49 | Calls in branches that do not run are checked. | Documents containing `true ? 'ok' : min(flag, 5)`, `size \|\| min(flag, 5)`, and a `::block when="false"` whose body calls `min(flag, 5)` (with `flag: boolean(required)`) are each rejected at composition before any expression is evaluated (counter at 0); DMLS reports an error on `flag` in each. |

### Review-added Contract Checks

| # | Criterion | How to validate |
|---|---|---|
| 50 | Projection agrees with property presence. | A union `[string(required), number]` projects without an implicit null arm; `[string(required), null]` remains optional; inherited required markers and `unknown(required)` agree with lowering. `min(fm.flag, 5)` and `min(fm["flag"], 5)` reject exactly like `min(flag, 5)` for a required boolean property, while bare reserved roots keep their registered meaning. `number | null` to a null-admitting parameter is category 1. |
| 51 | Recovery agrees between composition and editor. | Compose and DMLS accept `number("pear", 7)`, `number(true, 7)`, `number(null, 7)`, and `round("pear", 2.7)` without an argument error; reject `number("4", "x")`, `number("pear")`, and missing required arguments. Repeat with a typed boolean first argument and valid literal fallback. |
| 52 | Content-format classification matches binding. | Typed `string` to `json` is category 3; a valid literal JSON string is category 1; malformed literal JSON is category 5; a native object is category 1; null to a non-null target is category 5. |
| 53 | Lazy evaluation is preserved. | `and(false, effect())` and `or(true, effect())` do not invoke a test-registered effect counter, while passive checking still rejects a proven bad call inside either skipped argument. |
| 54 | Checking precedes effects and respects the requested surface. | A bad frontmatter call rejects before a sibling effect counter runs. Fenced examples, literal tokens, and inserted data are not checked as executable expressions. Frontmatter-only composition leaves an invalid body untouched; full composition checks it. |
| 55 | Raw schema assertions survive conversion. | Cover overlapping `oneOf` number/integer alternatives; `$ref` siblings; a referenced number constrained by a sibling; overlapping literal/pattern keys; typed additional keys; unmodeled `multipleOf`; and a cyclic local reference. Failure leaves the input unchanged, and reordering alternatives or patterns cannot change the result. |
| 56 | Grammar closes indirect invalid forms. | Reject scalar tuple spreads, duplicate parameter names, two/nonfinal/optional rest parameters, and `void` mixed with parameters. Reject a document schema reaching a function through each container/reference form. Accept `tuple([])` only for `[]`; optional numeric slots reject supplied null. |
| 57 | Catalog identity and return contracts survive migration. | Canonical names and registered aliases resolve as before, documentation lists canonicals once, semantically duplicate overloads fail loading, and a deliberately wrong-return test handler reports an implementation-contract error without converting its result. |
| 58 | Map ordering does not silently alter existing contracts. | Cover nested insertion order and function-built maps; keep existing stable-order consumers explicitly sorted. Review changed serialized-output expectations for intentional behavior rather than mechanically replacing snapshots. |
| 59 | Numeric admission checks whole numbers in every spelling. | `"9007199254740993.0"`, `"9.007199254740993e15"`, and the native YAML `n: 9007199254740993.0` against `n: number` are rejected as "cannot be represented exactly"; `"-9007199254740992"` is accepted and `"-9007199254740993"` rejected; numeric defaults, literals, enum members, and constraints get the same check; an externally supplied finite `f64` is accepted and a non-finite one rejected. Keep the existing `0.1`, overflow, and underflow outcomes. |

## Open Questions

None.

## Per-function Items for the Catalog Migration

Both items are resolved (decision 70) and recorded in the behavior-change ledger:

1. **`number()` — failure behavior and parameter name.** `number(x: numberlike, fallback?: number): number`. Any failure of `x` to bind selects a supplied fallback; without one, the call returns the engine's type error (`number("pear")` errors; `number("pear", 7)` returns `7`). The parameter is `fallback`. Today's `0` results for unconvertible input are an intentional change.
2. **`round()` — the fallback.** `round(x: numberlike, fallback?: number): number`, with the same binding and recovery as `number()`, then rounding half away from zero; the fallback is kept and is rounded too. The draft's contradictory `round(val: number, default?)` is retired.
