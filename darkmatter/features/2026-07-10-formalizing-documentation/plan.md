---
created: 2026-07-10
total_phases: 8
phase: 0
packages:
  - darkmatter
  - claudine
---

# Formalizing Documentation — Execution Plan

Success means the documentation taxonomy in `spec.md` is ratified and
machine-checkable, `prompts/document.md` runs end-to-end on today's Claudine
sequence engine, two exercise runs over the recently completed schema/DMLS
specs have produced contract-satisfying documentation for darkmatter, the
harvested sequence gaps are ready to seed the Claudine sequences spec, and the
brainstorming session has produced the draft
`darkmatter/features/2026-07-10-v1-schemas-and-lsp/spec.md`.

Assumptions:

- Phases 1, 4, 5, and 8 contain **interactive checkpoints** — they require Ken
  in the loop and cannot be completed autonomously. Everything else is
  agent-executable once its predecessor checkpoint has passed.
- `document.md` v0 must run on the sequence engine as implemented in
  `claudine/lib/src/composition/sequence.rs` (no groups, no `parameters:`, no
  interactive steps). Desired-but-missing capabilities are logged, not built.
- The exercise specs are drawn from: suggest-constraint,
  single-sourcing-schema, schema-improvement, schema-coercion,
  compose-schemas, schemas (schema grammar cluster) and dmls,
  modal-and-autocomplete (DMLS cluster).
- No git commits from this plan's execution; committing is a separate
  operation Ken drives.

## Phase 1 — Clarify and Ratify the Specification

> Interactive: resolve the spec's Open Questions with Ken before anything is
> built on top of them.

- [ ] Walk the seven Open Questions in `spec.md` with Ken (feature-doc root
      naming, schema home, catalog scope, `feature:` pointer form,
      getting-started cardinality, README enforcement, sequence-gaps home);
      record each ruling inline in the spec as a blockquote under the relevant
      section.
- [ ] Review the frontmatter contracts (property names, required vs optional)
      and the seven-state sequence design; amend the spec where Ken overrules.
- [ ] Confirm the two exercise-run groupings (schema cluster, DMLS cluster) or
      adjust which specs each run covers.
- [ ] Validation checkpoint: spec contains zero unresolved Open Questions and
      Ken has said the taxonomy is ratified.

## Phase 2 — Documentation Schema and Scaffolding

- [ ] Author the standalone tagged SimplifiedSchema (per the Phase 1 ruling on
      its home; default `docs/schemas/documentation.yaml`) defining
      `FeatureDoc`, `FeatureFile`, `GettingStartedDoc`, `ResearchDoc`, and
      `FeatureCatalog`, using `suggest(...)` for `area`/common `tags` and
      `file` for pointer properties.
- [ ] Verify each type with `md schema validate` against hand-written positive
      and negative fixture documents (keep fixtures in this feature directory
      under `fixtures/`).
- [ ] Add the `[schema.extensions.documentation]` entry to the repo
      `.dmls.toml` with globs for `**/docs/features/**`,
      `**/docs/getting-started/**`, `**/docs/research/**`; open a fixture in a
      DMLS session (L2 fixture or manual editor check) and confirm completion
      and `dm.*` diagnostics fire with no per-document `$schema`.
- [ ] Scaffold `darkmatter/docs/features/` (with the disambiguating
      `README.md` one-liner) and `darkmatter/docs/getting-started/`.
- [ ] Implement catalog generation (scan `features/**` for `kind: feature`
      index documents → `catalog.yaml`) as a deterministic script or just
      recipe, and prove regeneration is byte-stable on an unchanged tree.
- [ ] Validation checkpoint: fixtures validate/reject as expected, DMLS
      surfaces the contracts, catalog regeneration is stable.

## Phase 3 — Author `prompts/document.md` v0

- [ ] Draft `prompts/document.md` as a headed sequence implementing the seven
      states (Gather, Associate, Refine, Feature Updates, Getting-Started
      Updates, README Updates, Skill Updates) with the parameter surface from
      the spec (`spec` required; `docs_root`, `skip_states`,
      `non_interactive` optional) declared via `$schema` so missing-required
      collection prompts for `spec`.
- [ ] Wire lifecycle hooks per the spec: `initialize` (spec resolution + area
      derivation via `set_frontmatter`), per-state `start` announcements,
      `success` stacks running `md schema validate` and `md hash --save` over
      written documents, `finalize` summary, `fail_fast: true`.
- [ ] Implement the v0 interactivity fallback: Associate/Refine run through
      the agent conversation, with `question` CLI (`choose-many`,
      `text-input`, `boolean-switch`) shell-outs where a hard TUI is better.
- [ ] Create `sequence-requirements.md` in this feature directory (or the
      Phase 1 ruled location) seeded with the six capability gaps from the
      spec; every workaround written in this phase gets a corresponding entry.
- [ ] Dry-run the sequence against a small synthetic spec fixture (not a real
      one) to shake out compose errors, lifecycle wiring, and state ordering
      before spending a real run.
- [ ] Validation checkpoint: the dry run completes all seven states, produces
      schema-valid fixture output, and fails fast when a state's validation
      stack is deliberately broken.

## Phase 4 — Exercise Run 1: Schema Grammar Cluster

> Interactive: Associate/Refine states and the fitness review need Ken.

- [ ] Run `claudine sequence prompts/document.md spec=darkmatter/features/2026-07-09-suggest-constraint/spec.md`
      (plus the completed schema-cluster specs per the Phase 1 ruling) to
      produce/update the SimplifiedSchema-related feature docs,
      getting-started docs, README entries, and skill sections.
- [ ] Capture every manual rescue, wrong turn, or prompt ambiguity in a
      `run-1-fitness.md` review in this feature directory; add engine-level
      gaps to `sequence-requirements.md`.
- [ ] Iterate `prompts/document.md` against the findings and re-run the
      affected states (`skip_states` for the ones that held).
- [ ] Validation checkpoint: all produced documents pass
      `md schema validate`, the catalog reflects them, and the fitness review
      concludes the run needed no rescue on its final iteration.

## Phase 5 — Exercise Run 2: DMLS Cluster

> Interactive: same checkpoints as Phase 4.

- [ ] Run the sequence over the DMLS-cluster specs (dmls,
      modal-and-autocomplete) — this run should exercise the directory-form
      feature doc (library + LSP surfaces) and the `cli`/multi-file tagging
      rules.
- [ ] Capture `run-2-fitness.md`; iterate the prompt; append any new gaps to
      `sequence-requirements.md`.
- [ ] Validation checkpoint: run 2's final iteration is rescue-free and the
      DMLS feature documentation satisfies all contracts.

## Phase 6 — Skill Publishing Pipeline

- [ ] Implement `publish-summary-research` as a shared recipe in `just/`
      (imported by area justfiles) per the spec contract: locate topic
      research, produce/refresh `_summary.md`, publish between managed
      markers in the area skill, regenerate the skill `hash:`.
- [ ] Prove idempotence: a second run with unchanged research is a no-op
      (hash-compare before write).
- [ ] Exercise it on one darkmatter research topic (e.g. `research/search/`)
      and confirm the skill's published block and `hash:` are valid.
- [ ] Validation checkpoint: recipe runs from the darkmatter area justfile,
      is idempotent, and `md hash` verifies the skill afterward.

## Phase 7 — Harvest and Closure

- [ ] Consolidate `sequence-requirements.md` into its ratified final form —
      each requirement with motivation, the v0 workaround it replaces, and a
      pointer to where `claudine/docs/topics/flow-control/sequences.md`
      already sketches it (groups, `parameters:`, headless steps) so the
      sequences spec reconciles doc and implementation instead of adding a
      third model.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` (and the repo `CLAUDE.md`
      drift-maintenance list if warranted) to document the new taxonomy,
      the catalog, and the `document.md` process; regenerate hashes.
- [ ] Sweep the spec's acceptance criteria 1–9 and record the result per
      criterion in this plan.
- [ ] Validation checkpoint: Ken signs off that `document.md` is fit for
      purpose as the standing documentation process.

## Phase 8 — Brainstorm: Schemas and LSP v1

> Interactive: this phase is a working session with Ken, grounded in the
> documentation produced in Phases 4–5.

- [ ] Prepare a proposal list from the updated feature docs: candidate
      improvements for the SimplifiedSchema grammar (smaller set) and for
      DMLS (larger set), each described in one or two sentences.
- [ ] Present both lists; let Ken choose which features to pursue and rule on
      priorities/boundaries interactively.
- [ ] Draft `darkmatter/features/2026-07-10-v1-schemas-and-lsp/spec.md`
      capturing the chosen v1 scope, with the not-chosen ideas recorded in an
      `_unscheduled`-style appendix so they are not lost.
- [ ] Validation checkpoint: the draft spec exists, reflects the session's
      rulings, and acceptance criterion 10 holds.

## Sequencing Notes

- Phases 2 and 3 are agent-executable back-to-back after Phase 1; within
  Phase 2 the schema authoring and the directory scaffolding are independent,
  but catalog generation depends on the schema (it validates what it scans).
- Phase 6 is independent of Phases 4–5 except for its exercise step and can
  be built in parallel with them once Phase 3 lands.
- Phases 4 → 5 are ordered deliberately: run 1 debugs the sequence on the
  cluster Ken knows best right now; run 2 then stresses the directory-form
  and multi-package rules with a cleaner prompt.
- Phase 8 must trail Phases 4–5 (it consumes their documentation) but does
  not depend on Phases 6–7.
