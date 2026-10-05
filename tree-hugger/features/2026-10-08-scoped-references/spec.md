---
kind: feature
name: scoped-references
date: 2026-10-08
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-improvements
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-05-deferred-kind-precision
---

# Spec: Scope-Aware References and `hug refs`

## Problem

The tree-hugger library binds each reference to the first same-named
declaration in the file (`find_reference_target` in `lib/src/analysis/mod.rs`).
A shadowed local, a same-named method on another type, or `other.method()` can
all bind to the wrong declaration. Because of that, `semantics.is_recursive`
cannot be trusted, and `2026-10-03-symbol-improvements` makes it report
`Unknown` whenever semantic analysis runs.

## What was proposed

Proposed in the pre-split draft of `2026-10-03-symbol-improvements` (items B6
and D9), and moved here by the author on 2026-10-03.

### Scoped resolution

Replace first-match binding with innermost lexical scope first, then outward,
honoring each language's declaration order, hoisting and import aliases.
Inventory effective `@local.scope` captures; add language-specific scopes or
explicitly report limited resolution where syntax is insufficient, and update
query provenance for modified vendored queries.

Lexical scope alone cannot identify `other.method()` without knowing the
receiver's type. Resolve a qualified target only when its namespace, container
or receiver is established locally; never redirect it to the caller merely
because the name matches. Ambiguous targets keep an absent ID and their written
qualifier rather than choosing the first candidate.

### Occurrence records

Add an occurrence record to `FileSymbolIndex`: owner reference, target
reference, reference span, relation kind (`Call` or `Usage`) and confidence
(`Scoped`, `NameOnly` or `Unresolved`). `Scoped` means syntax and lexical
visibility establish one target; `NameOnly` is explicitly a candidate. Missing
scope support is not proof of resolution. Existing per-symbol relation lists
become deterministic summaries derived from occurrences. Occurrences become a
`SymbolQuery` capability and are included in `--json-full`; sort them
deterministically.

### Honest recursion

`is_recursive` is `Yes` only for a `Call` occurrence resolved to the function's
own ID with `Scoped` confidence. A self-value reference is not recursion. Use
`Unknown` when any potentially recursive call is unresolved, and `No` only when
call extraction/resolution for the body is complete and excludes self-calls.
Nested callable bodies have their own owner and cannot establish recursion for
the enclosing function. No analysis requested remains `None`.

### `hug refs <symbol>`

Lists reference occurrence paths and spans, grouped by owning declaration. It
requests binding even if ordinary listings do not. `--callers` / `--callees`
are mutually exclusive and show one hop of `Call` occurrences only. Every row
reports confidence; unresolved/name-only candidates stay visibly separate from
proven targets. It is a per-file view: a directory scan cannot discover callers
in another file, and help and reports say so. The selector is an exact short or
qualified name using smart case; multiple matches are an ambiguity error that
lists locations and IDs and accepts `--id`. No matches succeeds with an empty
report. Filter the target after analyzing full lexical context.

## Scheduling

Scheduled 2026-10-08, after `2026-10-05-deferred-kind-precision` (constructors
and other newly precise kinds are call and usage targets) and the two metric
features, so the record shape is settled before occurrences reference it.

It was first deferred because nothing in the monorepo consumed reference data.
The author ruled on 2026-10-05 that a missing consumer is not a reason to
defer: callers cannot adopt reference data that does not exist. The cost is
still uncertain, since it touches every language's resolution rules and
possibly vendored queries; the spike below sizes it.

## Prerequisites

- `2026-10-03-symbol-improvements` (candidate-name index, `SymbolQuery`
  capabilities, `is_recursive` reporting `Unknown`).
- `2026-10-03-symbol-filters-and-reports` for qualified names (B5) and
  smart-case matching, which `hug refs` selection uses.
- `2026-10-05-deferred-kind-precision` (precise kinds for reference targets).

## Notes

- **Scope captures already exist but are unused.** 15 of the 16 supported
  languages have `@local.scope` captures in their vendored `locals.scm`
  (JavaScript and TypeScript through the shared `ecma` query); Perl has none.
  No Rust code reads them today; `lib/src/queries/inventory.rs` only lists the
  capture name.
- **Suggested first step:** a half-day spike that builds a scope tree for Rust
  and TypeScript from those captures and counts how many references change
  target compared with first-match binding. The result shapes the plan (which
  languages the captures cover well and where language-specific scopes are
  needed); it does not decide whether the feature goes ahead.
- **Testing (from the pre-split draft):** per-language matrix asserting that
  shadowing follows language scope/order rules, unrelated receiver calls are
  unresolved and not recursion, and occurrences retain spans, owner,
  `Call`/`Usage` and confidence. Fixtures add a shadowed local and same-named
  methods on two types. Unresolved receivers, overload ambiguity, nested
  callable owners and self-value uses never become definite recursion;
  callers/callees exclude non-call occurrences.
- Cross-file resolution stays out of scope; `lib/src/resolver/` covers the
  project level.
