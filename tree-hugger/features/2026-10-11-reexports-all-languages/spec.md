---
kind: feature
name: reexports-all-languages
area: tree-hugger
status: draft-spec
scheduled: true
origin: ruling on `2026-10-10-docs-audit-code-defects` (F3), which removed the placeholder `TreeFile::reexported_symbols()` and moved the real capability here
packages:
  - tree-hugger
  - tree-hugger-cli
---

# Spec: Re-exports across all supported languages

This is a draft. It records the goal, the starting questions and the first
phase; it still needs its own clarification pass before planning. Nothing here is decided beyond
what the Goal and Phase 1 sections say.

## Problem

A *re-export* makes a name that a file imported from elsewhere available to
that file's own importers, so a package can present one public surface while
its definitions live in many files. Tree Hugger detects re-exports for one
language only: Rust `pub use`, reported as `ImportSymbol::is_reexport` on the
file's imports. Every other language always reports `false`. The symbol
record field `visibility.is_reexported` is never filled by extraction (it is
`true` only for the synthetic prelude records the CLI creates).

A placeholder method, `TreeFile::reexported_symbols()`, always returned an
empty list and was removed by the fix that moved this work here.

## Goal

Re-exports are detected, and reported through one consistent shape, for
**every** supported language that has the concept, and each language that
does not is documented as having none. Adding the feature for a subset of
languages is explicitly not wanted: a library that answers "what does this
file re-export?" for Rust and silently answers "nothing" for the rest misleads
its callers.

The 16 supported languages are Rust, JavaScript, TypeScript, Go, Python,
Java, PHP, Perl, Bash, Zsh, C, C++, C#, Swift, Scala and Lua.

## Phase 1: per-language audit

Before any design, give every one of the 16 languages an explicit answer,
verified against that language's Tree-sitter grammar (parse a real sample and
read the node types), not against memory. Each language is classified as one
of:

- **Real syntax**: the language has a dedicated re-export construct.
- **Convention**: re-exporting is done by an idiom that is not dedicated
  syntax.
- **No such concept.**

The audit's output is a table of language, classification, the exact syntax
or idiom, the grammar node types that match it, and a sample file.

### Starting notes (unverified)

These come from the reviewer's recollection and have not been checked against
any grammar. Treat each as a hypothesis for Phase 1 to confirm or reject.

| Language | Possible re-export form | Likely class |
|----------|-------------------------|--------------|
| Rust | `pub use a::B;` (already detected) | real syntax |
| JavaScript | `export { a } from "./x"`, `export * from "./x"`, `export * as ns from "./x"` | real syntax |
| TypeScript | the JavaScript forms, plus `export type { T } from "./x"` | real syntax |
| Python | `from .x import y` in an `__init__.py`, together with `__all__` | convention |
| Scala 3 | `export a.b` | real syntax |
| Swift | `@_exported import Module` | real syntax (underscored attribute) |
| C++ | namespace-level `using` declarations; C++20 `export import` | real syntax / convention |
| Go | `type X = pkg.X` alias | approximation |
| Perl | `@EXPORT` / `@EXPORT_OK` | convention |
| Lua | `M.foo = require("x").foo` | convention |
| C | `#include` in a public header | convention |
| Java, C#, PHP, Bash, Zsh | probably none | no such concept |

## Open questions

- Do **conventions** count as re-exports, or only dedicated syntax? A
  convention-based answer is a guess about intent (for example, an
  `__init__.py` import may be an internal convenience). If they count, should
  the result carry a confidence marker?
- How is `export * from "./x"` represented, given that it names no symbol?
- Does the indirect form `import { a } from "./x"; export { a };` count, or
  only a single-statement re-export?
- What is the API shape? A replacement for the removed method, a filter over
  imports, a new record kind, or only a field on existing records?
- Should extraction fill the symbol record field `visibility.is_reexported`,
  and does that need a record schema version bump?
- Which docs and agent-skill pages change: the Tree Hugger library page, the
  symbol-record field reference, the agent skill's API reference, and the
  diagnostics rule `unused-import` (which today exempts Rust `pub use`).

## Out of scope

Following a re-export chain across files to find the original definition.
That is cross-file resolution; this feature only reports what a single file
re-exports.

## Acceptance (provisional)

- Phase 1's table exists, and every one of the 16 languages has a verified
  classification.
- Each language classified as real syntax or (if conventions are accepted)
  convention has extraction and an L1 test with a sample file.
- Each language classified as having no such concept is documented as such,
  and a test asserts it reports no re-exports.
- The docs and the agent skill describe the final shape.
