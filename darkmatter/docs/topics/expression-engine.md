---
kind: canonical-doc
---
# Darkmatter's Expression Engine

## Introduction

An incredibly important part of Darkmatter's functionality is it's inclusion of an **Expression Engine** which allows for a rich set of _safe functions_ and _comparison operators_ to be used to produce rich logical expressions as well as mutate variables to the author's intended shape.

This expression engine also allows _callers_ of the library to use this expression engine -- and as we'll see at the end of this document -- to _extend_ the expressions which are provided.

Let's start with a few simple examples of how you might leverage the expression engine:

### 

## Advanced Features

### Type Aware Parsing

One important attribute of Darkmatter's expression engine is that it is _type aware_ and can not only know what type a variable was when it was first defined but also has the ability to understand when a union type has been "narrowed" because a guard such as `::block when="is_string(x)"` encloses the place it is read. See [What a Guard Proves](./darkmatter-expressions.md#what-a-guard-proves).

Equality takes part in that narrowing. `==` and `!=` are strict: they compare evaluated values by runtime type and value, and a missing value is `null`, which equals only `null`. So `::block when="x != null"` proves `x` is present inside the block, `::block when="x == 'debug'"` narrows `x` to that string, and `::block when="x != ''"` proves nothing about presence, because a missing `x` is `null` and `null != ''` holds. The analysis reads the guard without running it, using the same comparison the evaluator uses. See [Equality and Inequality](./darkmatter-expressions.md#equality-and-inequality).

### Parsed-once Interpolation

A host that checks one piece of authored text several ways can scan and parse its `{{ … }}` occurrences once with `PreparedInterpolation` and read the retained syntax (including located failures) through views, rather than parsing per check. The collection holds syntax only; evaluation always reads current values. The text APIs remain and share the same analysis bodies.

### Callbacks and Local Scopes

Use `map(people, p => p.name)` to produce an array of names, or
`filter(people, p -> p.active)` to retain active people. Both functions take an
inline callback as their second argument. The [`::map`](../inline/map.md) and
[`::filter`](../inline/filter.md) directives use the same callback grammar and
lexical bindings. See [Inline collection callbacks](./darkmatter-expressions.md#inline-collection-callbacks)
for syntax, null handling, budgets, and errors.
The parameter, or a shallow `{ a, b } =>` pattern, becomes an immutable binding
in the reserved `local` namespace, visible by bare name or as `local.item`. A
local shadows a frontmatter property of the same name even when it is null;
`fm.item` still reads the document. Guards narrow the types of a local in the
same way they narrow document values.

Every callback of one directive names the same original item, so a fact one
selection predicate proves holds in the later callbacks and the body whatever
spelling they use: `filter="p => is_array(p.members)"` proves that a
destructured `{members}` binding is an array, and
`filter="{members} => is_array(members)"` proves `p.members`. A map body binds
the mapped result, which is a new binding typed by its mapping expression:
`"{members} => {members}"` keeps the proven type, while a result field that
only shares an input field's name, as in `"{members, other} => {members: other}"`,
does not.


### Callback execution and passive analysis

Ordinary calls bind evaluated value arguments before computing a result.
`and` and `or` short-circuit; `map` and `filter` instead bind the iterable once
and evaluate the callback serially for each item. A callback is an argument,
never a stored or returned value. Neither callback function is a lifecycle verb;
use it inside an expression operand or condition.

The shared spanned parser represents these calls with `CallArgument::Value` or
`CallArgument::Callback`; value-only calls retain their ordinary AST shape.
`parse_callback` and `parse_callback_spanned` also serve directive callbacks.
`LexicalScope` retains immutable runtime frames, while `ItemFrame` and
`branch_scope` provide their passive type projection. Registered callback
signatures drive both call checking and execution; there is no second editor
checker. A schema-declared array, tuple, or object supplies item types; map
projects its body result and filter preserves the retained item type.

Validation and DMLS inspect every callback body, including bodies with empty or
null sources, without invoking it or resolving file contents. Guards on an
outer document property do not narrow a shadowing callback local. `fm.name`
continues to denote that document property. Evaluation inherits the request's
context, permissions, and shared iteration counter; failures keep each original
index/key and the underlying typed cause.
