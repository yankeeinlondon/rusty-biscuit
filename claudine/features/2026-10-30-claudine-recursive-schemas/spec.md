---
area: claudine
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 20
depends-on:
    - 2026-10-21-claudine-schemas
    - 2026-10-29-recursive-schema-types
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-28-schema-enhancements
---

# Claudine Lifecycle Schemas Use Recursive Types

`2026-10-21-claudine-schemas` ships Claudine's lifecycle schemas in an **interim shape**, because SimplifiedSchema could not express recursion: the `when`/`then`/`else` item is unrolled by hand to three fully typed levels, the bodies below that are a loose list-or-map accepted without deep checking, and the generated side-effect verbs and expression functions are listed as literal keys with untyped values instead of being typed one by one. With recursive named types in Darkmatter (`2026-10-29-recursive-schema-types`), this spec switches those schemas to their target shape. An author gains editor completion, value typing, and typo diagnostics for every verb at every nesting depth, and a schema an order of magnitude smaller and faster to resolve.

## Decided

- **One recursive item type.** Claudine's `lifecycle.yaml` defines the stack item once, and its `then` and `else` bodies (and a loop's `gate:`) reference the same item type recursively, by bare local name. The unrolled per-level types, and the loose list-or-map below the third level, are removed. Everything else about the item shape (which keys it carries, how events are typed as unions of action types, the bundle form) stays exactly as `2026-10-21-claudine-schemas` defined it.

  ```yaml
  kind: schema
  types:
      item: |-
          { <verb keys>, when: expression, then: stack, else: stack, no_error: boolean }
      stack:
          - item[]
          - bundle@./action.yaml
  ```

  (Illustrative: the verb keys and the bundle come from `action.yaml`; the exact spelling follows the interim schema it replaces.)
- **Every generated verb is fully typed.** The generator emits each side-effect verb and each expression function as its own literal key with its short and long forms, replacing the untyped literal keys, so a generated verb's value is checked and a misspelled generated verb is diagnosed by name at any depth. It reads the expression functions from Darkmatter's catalog at `darkmatter/schemas/partials/functions.yaml` (the catalog `2026-10-28-schema-enhancements` installed, through Darkmatter's library API or the file itself) and the side-effect verbs from `darkmatter/lib/src/effects/catalog.rs`, and derives long-form parameter names from the catalog's parameter names. Variadic functions (`and`, `or`) stay positional-only with no long form.
- **The nesting limit is parser-only.** Claudine's parser keeps its limit of five conditional levels and stays the only place a sixth level is rejected (`LifecycleStackInvalidShape`). The schema encodes no depth.
- **Fixture expectations are re-derived.** Every lifecycle schema fixture's recorded verdict is regenerated against the recursive schema. The one that changes meaning is `depth6.md`: the schema now types all six levels fully (a typo at any level is a schema error naming its path) and accepts the document's shape, while the parser rejects it as a sixth level. Whatever verdict the interim fixture set recorded for `depth6.md` is replaced by: `md schema validate` and DMLS accept it; Claudine's parser rejects it with `LifecycleStackInvalidShape` naming the sixth level. A new fixture `depth6-wrong-verb.md`, with a typo'd verb at the sixth level, is rejected by `md schema validate` and DMLS with the verb named, where the interim shape accepted it.

## Location

The files are Claudine's `action.yaml` and `lifecycle.yaml` as `2026-10-21-claudine-schemas` shipped them (under `claudine/schemas/`, embedded into the Claudine binary from YAML). If `2026-10-27-darkmatter-lifecycle-ownership` has since moved or renamed them, this spec applies to them at their current home, and every reference below follows them.

## Out of scope

| Work | Owner |
|---|---|
| Recursive named types in Darkmatter's resolver | `2026-10-29-recursive-schema-types` |
| The lifecycle grammar, the action inventory, and the parser's nesting limit | `2026-10-20-lifecycle-grammar` |
| The interim schemas, their embedding, and the event typing | `2026-10-21-claudine-schemas` |
| Lifecycle event ownership and host catalog files | `2026-10-27-darkmatter-lifecycle-ownership` |

## Estimate

**2–3 engineer-days, likeliest 2.5:** generator switch to the Phase B catalog and full verb typing (0.5–0.75), recursive item type and removal of the unroll (0.25–0.5), fixture re-derivation and parser parity (0.5–0.75), DMLS parity tests (0.25–0.5), docs and skill (0.5).

## Acceptance criteria

All criteria are L1 tests run with nextest through `just test` in `claudine/` (and `darkmatter/` for DMLS tests), with `just lint` passing in both; no test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; no CI cell is added.

1. **Recursive shape.** `lifecycle.yaml` declares one item type referenced recursively from `then`, `else`, and `gate:`, with no per-level types and no loose list-or-map body; a passive test over the embedded schema asserts this, and its resolved JSON Schema contains a `$defs` entry for the item and is at most 2 MB.
2. **Full verb typing.** `action.yaml` contains a literal key for every expression function in `darkmatter/schemas/partials/functions.yaml` and every side-effect verb in Darkmatter's effects catalog, each with its short and long forms typed (no untyped value), and no `<string>` catch-all; a test compares the key set with both catalogs and finds no gap either way.
3. **Fixtures.** The fixture set from `2026-10-21-claudine-schemas` passes `md schema validate` and `md schema triggers` with re-derived verdicts: depth 1, 3, and 5 valid; `depth6.md` accepted by the schema and rejected by the parser with `LifecycleStackInvalidShape`; `depth6-wrong-verb.md` rejected by the schema naming the verb and its path; a typo'd generated verb (for example a misspelled expression function) inside a third-level `then:` rejected by name; root dictionary, bundle in `then`, loop gate, wrong hand verb, and long forms with their existing verdicts.
4. **DMLS parity.** DMLS reports the same verdicts as `md schema validate` for every fixture, and completion offers generated verbs inside a `then:` at depth 4.
5. **Corpus.** `md schema validate` over every shipped Claudine prompt and lifecycle document passes with the recursive schema, and the existing DMLS schema parity tests pass.
6. **Docs.** Claudine's `docs/` schema pages and the `claudine` skill describe the recursive shape and the parser-only nesting limit, and no longer mention the interim unroll or the untyped generated-verb keys.

## Open questions

None.
