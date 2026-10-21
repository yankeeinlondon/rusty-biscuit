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
