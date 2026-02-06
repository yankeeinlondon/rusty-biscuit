# Improving Schematic package

Here is a list of problems I've found while reviewing the schematic library. We'll start out with a list of problem with the existing functionality and then move onto functionality not yet implemented.

## Overview

### Known Problems with Existing Functionality

- Ergonomic and Metadata
    - A critical design feature for the Schematic package is that it produces type safe ergonomic API Clients which are self-documenting.
    - We need to ensure that not only are public functions, structs, and enums very well documented in code (so that when we run `cargo docs` the resulting documentation is first class)
    - we also need to make sure

### Missing Features

- Pagination support
    - we need to have an ergonomic solution for defining pagination support in an API
- Variant refactor
    - there is a variant() function exposed to the generated API's but it's not fit for purpose, it was designed to meet an immediate need but is brittle and too limiting

## Context

This plan will focus on improving code in the `schematic/define` and `schematic/gen` modules which will then likely result in some downstream changes necessary in the `schematic/definitions` sub-package.

The most important "product" of this plan is an improved API Client in `schematic/schema` and while this code is generated it should be evaluated, tested, and analyzed to make sure we've made the right design decisions in upstream sub-packages.

### Sub-packages

The schematic package consists of four sub-packages:

1. `schematic/define` - provides us the primitives we will use to _define_ an API surface
2. `schematic/definitions` - the client definitions we have so far (which leverage `schematic/define`)
3. `schematic/gen` - the meta programming which converts our definitions in `schematic/definitions` into API Client which reside in `schematic/schema`
4. `schematic/schema` - the generated API Clients

## Resources to be used in this plan and follow on Implementation

**Important:** there is a `schematic` skill which should be used in all planning activity to help context-efficient knowledge of the Schematic package and it's sub-packages.

1. REVIEW: Ergonomics and Documentation

    - we have just recently conducted a review of the Schematic code base
    - all suggestions found in the [review](@schematic/docs/reviews/2026-02.06.review.md) should be added as tasks to the plan

2. Variant Refactor Design

    - we have created an initial design document for refactoring the **variant** functionality which the generated API Client exposes
    - you can find this design document at @schematic/docs/reviews/variant-api-design.md

3. Pagination Design

    - we have just recently written up an initial design document for the **Pagination** functionality which we want to be part of this plan
    - you can find this design document at @schematic/docs/reviews/schematic-pagination-design.md

## Task

Build a multi-phase plan to implement all the functionality and fixes we've identified. Make use of sub-agents during planning and implementation as much as possible and use skills where ever possible.

Skills that should be used in most if not all tasks/sub-tasks are:

- `rust` : best practices and insight into using the Rust programming language
- `rust-testing` : best practices and insight into Rust based testing
- `schematic` : for details about the Schematic package
- `syn` : for insights and knowledge on the `syn` crate used for code generation
