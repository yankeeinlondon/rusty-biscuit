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

One important attribute of Darkmatter's expression engine is that it is _type aware_ and can not only know what type a variable was when it was first defined but also has the ability to understand when a union type has been "narrowed" because
