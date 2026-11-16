---
kind: implementation-record
spec: spec.md
plan: plan.md
log: implementation-log.md
status: implementation complete, ready for review
---

# Implementation: sequence `$schema` describes the document, `$state` types the steps

This note summarizes the implementation as a whole. The detailed per-phase
record, test names, gate runs, and every judgment call
(`decisions_for_review`) are in [implementation-log.md](implementation-log.md).
The specification is left as the design snapshot; the departures it does not
describe are listed below.

## What changed

- **Declarations.** A sequence document's root `$schema` is now its document
  schema in YAML and Markdown alike. `$state` (new) types each step's
  normalized state; `template:` keeps filling steps. Both are read by one
  declarations module (`composition/sequence/declarations.rs`) that judges
  placement and shape by key presence before any row is read, and resolves
  `$state` once per owner in the declaring file's context.
- **One normalizer.** Every authored inline list (YAML root, Markdown
  frontmatter, inline nested container, referenced whole document) goes
  through the shared formal normalization pipeline with the owner's own
  declarations. Review correction (2026-11-15, iteration 2): root and nested
  templates now receive prepared owner values; the earlier nested-value gap
  was repaired. Declarations are removed
  from ordinary composition in both formats, preventing early interpolation.
- **Owner values.** A root template reads its document's prepared values
  under setters and root initialization writes, then the item's fields.
  Review correction (2026-11-15, iteration 2): the earlier statement that
  inline nested templates receive raw enclosing values is no longer true.
  `Loader::owner_template_values` uses discovery's prepared owner values,
  or prepares the owner once. The library's
  `every_inline_template_reads_its_owners_prepared_values` test verified
  literal, absent-default, null-default, and derived values at all six
  placements through three graph builders; CLI template-fill tests also pass.
  Referenced document preparation resolves authored file
  values from its own folder and setters from the caller's.
- **Input collection.** Root and referenced sequence documents join the
  combined missing-input pass (`preflight/inputs.rs`), including empty
  lists, nested references, and group legs. Required `$state` fields and
  inline `$schema` checks never prompt.
- **Inline container checks.** After `when:`, admission checks current values
  against the reread declaration, and `start` checks it again after initialization.
  Review correction (2026-11-15, iteration 2): the earlier statement that
  an edited check allows initialization before rejection is no longer true.
  `admit_inline_container` rereads before opening the run; the library's
  `stale_admission` tests and CLI tightened-check tests verify all four
  placements produce no initialization effects on refusal. A dry run names
  the check instead of evaluating it.
- **Reserved writes.** Setters, `with:`, runtime `set`, and template keys may
  not write `$state` or `template` (template keys may not write `$schema`
  either); each refusal is typed and located.
- **Frozen graph.** Templates are filled when the graph is built. Rereads,
  retries (`reread_preflight_graph_with_invocation`), and loop passes keep
  the planned ids and template state; `Loader::refresh` applies at every
  depth of a reread.
- **Schemas.** `schemas/sequence.yaml` declares `$state`;
  `schemas/sequence-types.yaml` declares the inline task options; the shipped
  artifact was regenerated.
- **Docs.** `docs/topics/flow-control/sequences.md` (new "Sequence documents"
  section with a Mermaid flow; input collection, dry run, reserved keys),
  `docs/topics/schemas.md`, `schemas/README.md`, `docs/cli/sequence.md`,
  `docs/topics/execution-flow.md`, `docs/topics/frontmatter-properties.md`,
  the `use-claudine` caller skill (published), and the repo `claudine` skill.

## Tests

All new tests are L1. Library coverage lives in the existing
`composition::sequence` test modules (declarations, document, formal
normalization, loader matrix, scoped graph, inputs, inline checks,
declaration writes, work counters). CLI coverage spawns `claudine` through
`CliProcessFixture` with portable launch counting:
`sequence_sources_cli.rs` (including the frozen `complete.yaml`
reproduction), `sequence_referenced_state.rs`, `sequence_frozen_graph.rs`,
and `sequence_dry_run.rs`. The frozen reproduction lives under
`cli/tests/fixtures/frozen_prompts/sequence-state-schema/`; no test reads the
internal `prompts/` tree.

Local evidence is macOS (`just test`, `just lint`,
`just check-tier-coverage claudine`). Targeted Windows runs on the build rig
are recorded in Phase 5 of the log; other OS coverage is left to the normal
CI scope.

## Departures from the specification

- **Inventory before templates.** Inputs are discovered before any template
  is filled, because a template can read an input that is still missing. The
  CLI no longer builds a plan before `initialize`: it judges root
  declarations and inline item grammar, runs `initialize`, then discovers,
  collects, and fills. A root `sequence: child.yaml` and a `$( … )` source
  are therefore loaded after `initialize`.
- **Root empty-list validation.** The spec requires an empty sequence's
  document to be validated. An empty root is judged by input discovery as a
  `Document` entry and rechecked strictly, since it has no step composition
  to do it.
- **Referrer values for referenced templates.** Review correction (2026-11-15,
  iteration 2): the earlier statement that a referrer's properties remain a
  lowest layer is no longer true. Both fallback merges were removed. The
  referenced-template route tests and input-discovery test verify the child
  uses its own fallback; fleets now supply `research_prompt` through root
  initialization writes. Dry runs need the equivalent caller setter because
  they do not execute initialization.
- **Root reports.** When only the root is missing inputs and it has composing
  steps, the report keeps its per-step headings; otherwise each owner is a
  `Document` entry.

## Known practical limits

- Graph diagnostics for root rows still read `tasks[i]` while the inventory
  and normalizer use `sequence[i]`.
- At the first review (2026-11-15), passive approval judged a relative
  task `with:` file input from the launch directory and could reject a valid
  author-relative file. Runtime preparation is correct: when a second file
  in the launch folder lets approval finish, direct and nested CLI dry runs
  both render the author-adjacent file. See `review-1.md`. Repaired in the
  first review-fix cycle (R1-F2): approval and the step barrier now record
  `with:` values with their authoring origin, as runtime does.
  Iteration 3 verification note (2026-11-15): the previous present-tense
  description was historical, not a remaining limit. The relative, absolute,
  and competing-folder `with_file_provenance` cases passed in this review's
  2,103-test sequence run.
- Review correction (2026-11-15): the former claim that root-reference
  `set` reads `seq.<key>` as null was false. The side effect prints the prior
  value; a following shell verified the new value is `ada` for direct, root,
  and nested references. There is no established defect from that probe.
- Interactive (PTY) collection of sequence-document inputs has no dedicated
  new test; it shares the non-TTY collection path.
