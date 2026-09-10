# Claudine compatibility replay — frozen era inputs

`tests/*-claudine-compat.test.ts` prove one thing: **this shared tool
reproduces the numbers the first-generation Claudine scripts recorded, from the
inputs those scripts consumed.** That is a statement about a fixed past, so
every side of it has to be fixed.

The inputs are already frozen and live in the consuming fix directory, because
they are that fix's evidence and are far too large to duplicate:

| Input | Path |
|---|---|
| nextest listings, 16 selections | `claudine/fixes/2026-09-07-faster-claudine-tests/enumeration/9fc5151a0/` |
| `just test` / `just test-l2` gate logs | `…/baseline/local-gates/` |
| `just test-rendezvous` gate log | `…/enumeration/recipes/` |

This directory holds the two remaining sides, which used to be read live:

| File | Was read from | Why it is frozen here |
|---|---|---|
| `families.json` | the fix's live `families.json` | the classifier that turns an identity into a family |
| `expectations.json` | the fix's live `attribution.md` and `inventory.md`, plus literals in the test bodies | the recorded numbers being reproduced |

## Why this is not a snapshot for its own sake

A replay of frozen inputs against a *live* classifier is not a proof, and it is
a fault injector for every consuming package area. Review 1 of
`2026-09-07-faster-claudine-tests` reclassified nineteen PTY tests, which
renamed one family in Claudine's `families.json` — an entirely correct change
in a fix directory that this package does not own — and three tests here went
red. Nothing in `tools/test-audit` was wrong. A shared package whose suite any
consumer can break by renaming one of its own things has the dependency
backwards.

So the replays read this directory, and `claudine-compat-inputs.test.ts` pins
that: a `*-claudine-compat.test.ts` that reaches for the live `families.json`,
`inventory.md`, `attribution.md`, or the live top-level `enumeration/` listings
fails the suite. Live-consumer health is not this package's suite to assert; it
is what `test-audit reconcile` does when the consumer runs its own gate.

`expectations.json` records `renames`, because the numbers were published in
`attribution.md` under the family's *current* id while the frozen
`families.json` still calls it by its era id. Both names are stated rather than
one being quietly rewritten.

## Refreshing this bundle

Do not. A new era means a new directory: capture the listings under a new
revision-named subdirectory in the consuming area, snapshot the classifier and
expectations of *that* era alongside, and add a replay for it. Editing these
files to make a test pass destroys the only thing they are for.
