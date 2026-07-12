---
$schema:
    review: file
description: |-
    Performs a review on the current package area for cyclometric risk.
review: {{ ctx.area }}/reviews/{{ ctx.today }}-cyclometric-risk/review.md
start:
    message: "👀  performing a cyclometric risk analysis on the **{{ctx.area}}** package area"
success:
    message: "🔁  the cyclometric risk analysis on **{{ctx.area}}** completed successfully"
    info: "the cyclometric risk analysis on **{{ctx.area}}** completed successfully"
---
# Cyclomatic Risk Audit

Audit the **{{ ctx.area }}** package area for structural quality risks:
dependency cycles, cyclomatic/cognitive complexity, and coupling hotspots.
This is an **assessment** — report findings and proposed fixes; do not change
any source code.

## 1. Dependency cycles and coupling

The module-graph scan for every crate in this package area (import cycles as
SCCs with their intra-cycle edges, plus fan-in/fan-out hubs):

::shell tsx .claude/scripts/module-cycles.ts {{ ctx.package_area_root }} --md

For every reported cycle (SCC):

- Classify it: an **intentional self-referential data structure** (e.g. a
  graph arena whose node/edge/index modules reference each other by design)
  versus an **accidental cycle** (a layering violation).
- For accidental cycles, identify the **minimal back-edge** — usually one
  import going against the crate's dependency direction (data/leaf modules →
  domain logic → orchestration/registration). Name the exact `use` or
  `crate::` path and file:line that closes the loop.
- Propose the smallest fix, preferring in order:
  1. **Leaf extraction** — move the shared item (constants, a legend, a pure
     data type) into a new leaf module both sides import. Precedent:
     `darkmatter/dmls/src/semantic_legend.rs`.
  2. **Dependency inversion** — define the trait upstream, implement it
     downstream; or pass the needed value/closure in as a parameter instead
     of importing downward.
  3. **Module merge** — if two modules cycle because they are really one
     concept, say so.

From the fan-in/fan-out summary, flag any module that is high in **both**
directions — that is god-module risk. High fan-in alone (a shared
vocabulary/types module) is usually healthy; note but don't flag it.

## 2. Function complexity

For each crate in the area, surface the allow-by-default complexity lints
without touching any config:

```sh
cargo clippy -p <pkg> --all-targets -- \
  -W clippy::cognitive_complexity \
  -W clippy::too_many_lines \
  -W clippy::type_complexity \
  -W clippy::too_many_arguments 2>&1
```

Rank the offenders (worst first). For each of the top ~10, read the function
and judge it before recommending anything: a broad `match` over an enum or
token stream is often idiomatic Rust that trips these lints while being the
clearest possible shape — say so and move on. Recommend refactoring only where
complexity comes from *tangle*, not *breadth*, using these idioms:

- extract long match arms into named functions
- `let-else` / early-return to flatten nesting
- split parse-from-validate (or compute-from-render) phases
- table/`HashMap`-driven dispatch for long if-else chains
- a params struct or builder for argument-heavy signatures

## 3. New vs. pre-existing

Distinguish organic growth from legacy debt: extract the same crates at the
merge-base with `main` (`git archive $(git merge-base HEAD main) <crate>/src |
tar -x -C <scratch-dir>`), run the analyzer there, and diff the cycle sets.
Findings introduced on this branch get priority.

## Report

Write a severity-ranked report to `{{ review }}` (create parent directories
as needed — this review file is the one deliverable you create). Each
finding: file:line, what the risk is in one sentence, why it matters here,
the proposed minimal fix, and its blast radius (callers affected, tests
touching it). Close with a short verdict: is structural quality trending
better or worse in this area, and what single fix buys the most.

Success criteria: every cycle in the area is either explained as intentional
or has a named back-edge with a concrete fix; every flagged function has been
read (not just lint-counted); the report exists at the path above; no source
files were modified.
