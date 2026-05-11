---
phases: 6
created: 2026-05-09
start_phase: 5
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-05-08-expression-syntax/design-notes.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - darkmatter/lib/README.md
  - darkmatter/features/2026-05-08-expression-syntax/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/tests/expression_regression.rs
docs_updated_during_phase_6:
  - darkmatter/features/2026-05-08-expression-syntax/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter
  - darkmatter-cli
---

# Darkmatter Expression Syntax Execution Plan

## Phase 1: Baseline And Semantic Decisions

- [x] Confirm the source-of-truth package list with `cargo metadata --no-deps --format-version 1` and identify the exact `darkmatter` package target names used by local test commands.
- [x] Read `darkmatter/lib/src/markdown/compose/expression/{ast.rs,lexer.rs,parser.rs,mod.rs}` and record the current AST, token, precedence, and evaluator shapes before editing.
- [x] Read the interpolation and condition call sites in `compose/interpolation`, `compose/conditions.rs`, `compose/page_blocks`, and `compose/transclusion` to confirm all expression users share the same parser/evaluator path.
- [x] Decide whether `||` remains represented as separate `Fallback` and condition-mode `Or(...)` nodes, or is unified in the AST while preserving current interpolation and condition behavior.
- [x] Decide the internal value contract for expression evaluation: keep returning `serde_json::Value`, preserve arrays/objects for type predicates and indexing, and convert to strings only at interpolation boundaries.
- [x] Decide the concrete local timezone helper shape using the existing `sniff` and `chrono` dependencies, including how tests will inject stable reference dates.
- [x] Validation checkpoint: add or update a small design note in the implementation PR description covering precedence, associativity, null propagation, timezone behavior, and any intentional deviations from the spec.

## Phase 2: Lexer, AST, And Parser Grammar

- [x] Add lexer tokens and tests for `<=`, `+`, `-`, `*`, `/`, `%`, `[`, `]`, and bracket string/index operands without regressing existing string, number, and dotted variable tokenization.
- [x] Add or update AST variants for binary operators and bracket/member access so arithmetic, comparison, fallback/logical operators, ternary, dot access, and bracket access can be represented without lossy function-call lowering.
- [x] Implement parser precedence in this order: primary/access, unary `!`, multiplicative, additive, comparison, logical `&&`, logical/fallback `||`, and right-associative ternary.
- [x] Preserve left associativity for all binary operators and add parser tests for `a - b - c`, `a / b / c`, `a || b || c`, and mixed-precedence expressions such as `a + b * c <= d || e`.
- [x] Preserve right associativity for ternary and add parser tests for `a ? b : c ? d : e`.
- [x] Implement dot access semantics so named property paths continue to work while numeric dot access such as `foo.0` is rejected or parsed as unsupported according to the final grammar decision.
- [x] Implement bracket access parsing for array indexes, negative indexes, object string keys, and nested forms such as `items[-1].name` and `config["key"][0]`.
- [x] Validation checkpoint: run focused parser and lexer tests with `cargo test -p darkmatter expression::lexer expression::parser`.

## Phase 3: Core Evaluator Operators And Access Semantics

- [x] Add evaluator support for `<=` and update comparison tests to cover all six comparison operators with numeric and string-backed numeric operands.
- [x] Implement arithmetic evaluation for `+`, `-`, `*`, `/`, and `%`, including string concatenation for `+` when either operand is a string.
- [x] Return clear evaluation errors for division by zero, remainder by zero, and non-numeric arithmetic operands where the spec requires hard errors.
- [x] Implement C-style remainder semantics where the sign of `a % b` follows the left operand, including negative dividend test cases.
- [x] Implement null-propagating dot and bracket access: missing paths, null bases, out-of-bounds indexes, invalid indexes, and key access on non-collections all evaluate to `null`.
- [x] Implement negative array indexing from the end and return `null` for empty arrays or indexes outside the valid range.
- [x] Update truthiness tests to explicitly cover `null`, `false`, `0`, `0.0`, `""`, `[]`, `{}`, and representative truthy values.
- [x] Validation checkpoint: run focused evaluator tests with `cargo test -p darkmatter markdown::compose::expression`.

## Phase 4: Function Library Expansion

- [x] Add type predicates `IsString`, `IsNumber`, `IsArray`, `IsNull`, `IsObject`, and `IsEmpty` with tests for correct true/false behavior across every JSON value kind.
- [x] Add math helpers `min(a, b)`, `max(a, b)`, and `abs(x)` using the spec's null-propagation and type-mismatch error contract.
- [x] Add collection helpers `first(x)` and `last(x)` with tests for arrays, empty arrays, `null`, and non-array type errors.
- [x] Add string predicates `StartsWith(x, find)` and `EndsWith(x, find)` with null propagation, type mismatch errors, and case-sensitive behavior tests.
- [x] Add string mutation helpers `Lower`, `Upper`, `Capitalize`, `KebabCase`, `CamelCase`, `PascalCase`, `SnakeCase`, and `TitleCase` with tests for whitespace, punctuation, existing separators, empty strings, and `null`.
- [x] Add strict date validators `IsDate`, `IsDateUtc`, `IsDateTime`, and `IsDateTimeUtc` that accept strings only and reject invalid formats and non-string inputs.
- [x] Add relative date validators `IsToday`, `IsYesterday`, `IsTomorrow`, `IsThisMonth`, `IsThisYear` and UTC variants, accepting date and datetime strings and returning `false` for invalid inputs.
- [x] Add deterministic tests for local and UTC date behavior using injectable clocks/timezones or isolated helpers so tests do not depend on the machine's current date.
- [x] Parallelizable: implement math, collection, string, and date helper groups in separate patches after Phase 3's evaluator value contract is merged.
- [x] Validation checkpoint: run function-focused tests with `cargo test -p darkmatter fn_ isdate istoday startswith first last min max abs`.

## Phase 5: Integration, Diagnostics, And Documentation

- [x] Update interpolation tests to prove new arithmetic, access, predicates, and string/date helpers work inside `{{ ... }}` and produce expected rendered strings.
- [x] Update condition tests to prove the same expression syntax works in `when` clauses for page blocks, transclusion directives, and the `evaluate_condition_against` shortcut API.
- [x] Update expression parse/evaluation error messages and condition syntax hints so `<=`, arithmetic operators, bracket access, and the expanded helper list are visible in diagnostics.
- [x] Rename user-facing docs from "Boolean Conditional Logic" to "Darkmatter Expressions", including the primary topic file, links from page blocks and transclusion docs, and any README references.
- [x] Document operator precedence, associativity, truthiness, null propagation, arithmetic errors, function contracts, bracket access, and timezone behavior in the renamed expression docs.
- [x] Update rustdoc in `compose::expression`, `compose::conditions`, and interpolation modules, keeping the repo's rustdoc section-order convention.
- [x] Update `docs/dependencies.md` and the darkmatter package dependency docs if any crates or features are added or removed; otherwise explicitly note that existing `chrono` and `sniff` dependencies were reused.
- [x] Parallelizable: documentation and diagnostic hint updates can proceed alongside integration tests once the public syntax and helper names are stable.
- [x] Validation checkpoint: run doc-sensitive checks with `cargo test -p darkmatter --doc` and a targeted search confirming no stale "boolean expression" terminology remains where it now refers to general Darkmatter expressions.

## Phase 6: Full Verification And Release Readiness

- [x] Run formatting for the touched Rust code with `cargo fmt --package darkmatter` or the repository's established formatting command.
- [x] Run focused library tests for expression, interpolation, conditions, page blocks, and transclusion behavior.
- [x] Run `cargo test -p darkmatter` and investigate any failures before broader workspace validation.
- [x] Run the relevant root or area lint command, preferring `just lint` or the darkmatter area `justfile` recipe if available.
- [x] Run the relevant root or area build command, preferring `just build` or the darkmatter area `justfile` recipe if available.
- [x] Add regression examples or fixtures for representative end-to-end documents using arithmetic, type predicates, bracket access, date helpers, and string mutation helpers together.
- [x] Review changed public docs, rustdocs, and error snapshots for consistency with the spec and with the final implemented behavior.
- [x] Validation checkpoint: final acceptance requires green formatting, focused tests, `cargo test -p darkmatter`, lint/build commands, and a reviewer-readable summary of any skipped commands or unresolved semantic tradeoffs.
