---
title: "Spike 2 — should plan-structure problems have their own vocabulary?"
created: 2026-09-15
status: complete
results: reviews/2026-09-15-python-test-code/spike-2-results.md
timebox: 1 day
source: reviews/2026-09-15-python-test-code/review.md §5.1
decides: whether to add PLAN_REJECTIONS and subdivide malformed-receipt
---

# Spike 2 — Should plan-structure problems have their own vocabulary?

## Question

The suites carry 309 substring assertions, ~100 of them against
`validate_resolved_plan` output. Would a finer rejection vocabulary make those
assertions less brittle without making failures less useful to a human reading a CI
log?

## What is actually wrong (corrected from the first reading)

The codes are **not missing**. All 53 problem literals in `schema.py` carry one.
They are **too coarse**:

| Count | Code |
|---:|---|
| **45** | `malformed-receipt` |
| 3 | `unknown-package` |
| 2 | `scope-malformed` |
| 1 each | `unknown-environment`, `missing-receipt`, `conflicting-evidence` |

`malformed-receipt` currently spans "area has no selection reason", "build key has
two owners", "consumers must be sorted", "counts.total disagrees with its parts",
and 41 others. A test that wants to pin one of those rules has no choice but to
match the prose tail — which is why `test_schema.py:594` and `:613` both assert
`"only an executing L1, L2, or browser cell"` for two *different* rules, and neither
can tell which fired.

## The complication that makes this a spike, not a chore

`malformed-receipt` is a member of `schema.REJECTIONS` — the frozen cross-language
vocabulary published in `.github/ci/schemas/contract.json` and mirrored on the Rust
side. Its docstring says:

> Why a receipt, or one of its cells, was not accepted. Shared by the verifier, the
> planner, and `just ci-local --plan` so a rejection reads the same everywhere.
> **Adding a code is a contract change.**

So `validate_resolved_plan` is borrowing a *receipt-rejection* code to report
*plan-structure* problems. That is arguably a category error already — and
`schema.py` itself gives the reasoning for why such things are kept apart, twice:

> `SCOPE_REJECTIONS` … Kept apart from `REJECTIONS`: those refuse a *cell's outcome*;
> these refuse a whole document.
>
> `BUILD_REJECTIONS` … These refuse the *inputs* to a cell that has not run yet.

A plan-structure problem refuses neither a cell's outcome, nor a scope document, nor
a cell's inputs. It refuses the plan. By the module's own stated logic that is a
fourth vocabulary.

## Preconditions

- Spike runs read-only until step 3; steps 0–2 change no code.
- `validate_resolved_plan` is called from production paths (`apply_accepted_cells`,
  `record_cells`, `record_cross_check`), so any change is a contract change.

## Steps

### 0. Inventory (~1h, read-only)

For each of the 53 literals record: code, the rule it reports, and every test
assertion that pins it.

```
python3 - <<'PY'   # starting point; extend to map literal -> asserting test
import re, pathlib, collections
src = pathlib.Path('scripts/ci/schema.py').read_text()
lits = re.findall(r'problems\.append\(\s*(?:f?")([^"]{5,140})', src)
print(collections.Counter(l.split(':')[0] for l in lits).most_common())
PY
```

### 1. Count collisions (~2h, read-only)

A collision is one asserted substring that more than one rule can produce. One is
confirmed (`test_schema.py:594` / `:613`). Find the rest by, for each asserted
substring, grepping the 53 literals for other rules that could emit it.

**This number is the decision input.** Every collision is a test that cannot
distinguish the rule it names from another.

### 2. Map the downstream consumers (~1h, read-only)

Before subdividing anything, establish who branches on these strings versus who only
renders them. Known so far:

| Consumer | Uses | Branches on the code? |
|---|---|---|
| `scripts/ci-plan.rs:337` | `evidence_rejections` | no — renders as an `UnorderedList` |
| `scripts/ci-plan-tests.rs:54` | fixture `"gate-inputs-changed: …"` | fixture string only |
| `scripts/ci-build-archive-tests.rs:285` | `contract["vocabulary"]["build_rejections"]` | **yes — asserts the frozen set** |
| `.github/ci/schemas/contract.json` | publishes `REJECTIONS`, `SCOPE_REJECTIONS`, build rejections | frozen |
| `just ci-local --plan` | renders | verify |
| area `coverage-audit` | ? | verify |

**Gate:** if anything branches on the *literal* `malformed-receipt` for a
plan-structure problem, subdividing is a breaking change and the spike must say so.

### 3. Prototype on the build-record rules only (~3h)

Narrowest useful slice: the ~20 refusals behind `BuildRecordValidationTests`.

- Add `PLAN_REJECTIONS` to `schema.py` as a fourth vocabulary, with a docstring in
  the same register as its three siblings, saying what it refuses and why it is kept
  apart.
- Re-code those ~20 literals. **Do not touch `REJECTIONS`** — the frozen vocabulary
  must be byte-identical afterwards, and `contract.json` must not change except to
  gain the new list.
- Keep **one** test per rule asserting the exact human text, so the prose stays
  specified and the fixtures keep documenting what a reader sees. Move the rest to
  code assertions.

### 4. Measure (~1h)

- Lines changed in `schema.py`, `test_schema.py`, `contract.json`.
- Did any previously-passing test now fail? A failure here means a collision was
  masking a *wrong rule firing* — the highest-value outcome the spike can produce.
- Does `test_shipped_contract_matches_this_module` (`test_schema.py:240`) still pass
  after regeneration?

## Decision rule

- **≥3 collisions in step 1** → roll `PLAN_REJECTIONS` across all 45 literals.
- **1–2 collisions** → apply only to the colliding rules; record that the prose is
  precise enough elsewhere and stop. (Fixing the two known build-reference tests is
  worth doing regardless.)
- **0 collisions** → the prose assertions are precise. Record the finding, close the
  spike, change nothing.
- **Anything branches on the literal code (step 2)** → stop; write it up as a
  contract change needing its own fix cycle, not a test-quality improvement.

## Ships regardless of the outcome

- A written inventory of what `validate_resolved_plan` can refuse and which test
  pins each rule — the closest thing to a specification of the validator that
  currently exists.
- The two colliding build-reference tests disambiguated.

## Risks

- **Contract creep.** `REJECTIONS` is frozen and cross-language. The spike's
  discipline is that it adds a vocabulary and never edits an existing one.
- **Losing the prose as documentation.** The fixtures' assertions on human text are
  a genuine asset — several read as the specification of what a reviewer sees. Step
  3's "keep one test per rule asserting the exact text" is not optional.
- **Rust-side drift.** `contract.json` is asserted byte-for-byte from both sides
  (`test_schema.py:249`, `ci-build-archive-tests.rs:285`). Regenerate, do not
  hand-edit.
