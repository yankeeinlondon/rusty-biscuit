# Formalizing Documentation

## Status

Draft — not yet clarified or ratified. Open questions for Ken are collected in
[Open Questions](#open-questions); everything else is written as a normative
proposal so a review pass can accept or amend each section directly.

## Purpose

Feature and fix specifications (`{area}/features/{YYYY}-{MM}-{DD}-{name}/`,
`{area}/fixes/...`) are how this monorepo creates change. They are excellent
history but expensive "current truth": reconstructing the present state of a
capability means replaying every spec that touched it. This feature formalizes
the documentation that hangs off the back of a completed spec so that:

1. Every package area maintains a small, predictable set of **blessed
   documents** — feature docs, getting-started docs, research docs, READMEs,
   and the area agent skill — with machine-checkable frontmatter contracts.
2. A repeatable Claudine sequence, `prompts/document.md`, walks an agent (and
   the user) through producing/updating those documents after a spec is
   implemented, so documentation stops being a best-effort afterthought.
3. The recently completed schema and DMLS work (suggest-constraint,
   single-sourcing-schema, schema-improvement, schema-coercion,
   compose-schemas, schemas, modal-and-autocomplete, dmls) is used as the
   first exercise of that sequence, both to document that work and to iterate
   the prompt until it is fit for purpose.

The documentation contracts deliberately dogfood Darkmatter's own machinery:
SimplifiedSchema standalone documents define the frontmatter contracts, DMLS
extension baselines give authors completion/diagnostics while editing docs,
and `md hash` keeps generated/blessed artifacts drift-detectable.

## Terminology

- **Spec** — a feature or fix specification directory
  (`{area}/features/2026-07-09-suggest-constraint/`). Specs are immutable
  change records; their directory name is the stable identifier used by the
  `features:` / `fixes:` frontmatter lists (unique per package area, stable
  across moves to `_completed/` / `_unscheduled/`).
- **Docs root** — the `docs/` directory a set of blessed documents hangs off.
  Default: the package-area `docs/` (e.g. `darkmatter/docs/`). Packages that
  own their own `docs/` directory (e.g. a divergent CLI) may use that as their
  docs root instead.
- **Feature doc** — the blessed current-truth document for one user-facing
  capability (distinct from a spec, which records one change to it).
- **Docs kind** — the value of the `kind:` frontmatter property that
  classifies a blessed document (`feature`, `getting-started`, `research`,
  `catalog`).

## Document Taxonomy

| Kind | Location (under docs root) | Audience | Cardinality |
|------|---------------------------|----------|-------------|
| Feature | `features/{name}.md` or `features/{name}/index.md` | agents + humans | one per user-facing capability |
| Getting started | `getting-started/{name}.md` | humans | one per feature doc |
| Research | `research/{topic}/…` | agents (compose) | ad hoc |
| README | area root + each package root | humans first | one per area, one per package |
| Area skill | `.claude/skills/{area}/` | agents | one per `ctx.area` (existing convention) |

A spec has a 0:M relationship to feature docs it **updates** and a 0:M
relationship to feature docs it **creates**. The `document.md` sequence exists
to discover, confirm, and then execute those relationships.

### Naming collision note

`{area}/features/` (specs) and `{docs root}/features/` (feature docs) share
the word "features" intentionally: specs are the change ledger, feature docs
are the current truth the ledger folds into. The `kind:` frontmatter and the
docs-root prefix disambiguate for tooling; humans get a `docs/features/README.md`
one-liner explaining the split.

## Frontmatter Contracts

### Shared conventions

- All blessed documents carry `kind:` — the discriminator every other tool
  (catalog generation, DMLS baselines, staleness checks) keys off.
- Entries in `features:` / `fixes:` lists are **spec directory names** (e.g.
  `2026-07-09-suggest-constraint`), never paths — paths change when specs move
  to `_completed/`.
- File-pointer properties (`feature:`) are declared `file` in the schema so
  compose/DMLS resolve and validate them via `FileReference` semantics.
- Documents produced or heavily rewritten by automation should carry `hash:`
  maintained via `md hash <file> --save` so drift is detectable. Hand-edited
  docs may omit it.
- `content_policy:` (already in use in biscuit-terminal research docs, backed
  by `research/lib/src/metadata/content_policy`) is **accepted everywhere but
  enforced nowhere** in this feature; staleness evaluation is explicitly
  future work.

### Feature documents

Location: `{docs root}/features/{feature-name}.md`, or — once a feature needs
more than one file — `{docs root}/features/{feature-name}/index.md` plus
sibling files whose names have a clear semantic relationship to their content.
A feature that spans both a library surface and a CLI surface should default
to the directory form, with at least one CLI-focused file tagged `cli`.

Required frontmatter:

| Property | Type | Meaning |
|----------|------|---------|
| `kind` | `"feature"` | discriminator |
| `area` | `string` | package area (`ctx.area` value, e.g. `darkmatter`) |
| `packages` | `string[]` | packages that **provide** the functionality (not merely related) |
| `features` | `string[]` | spec directory names that defined/extended this feature |
| `fixes` | `string[]` | fix directory names that refined it (may be empty) |
| `symbols` | `string[]` | key structs/enums/functions (the load-bearing ones, not exhaustive) |
| `description` | `string` | one-to-two sentence summary; feeds the catalog |

Optional: `tags: string[]` (e.g. `cli` on CLI-focused files inside a feature
directory), `hash`, `content_policy`.

Required content (body):

- **WHAT** the functionality is and **WHY** it exists (the why is the part a
  spec archaeology dig can't cheaply recover — it is mandatory).
- At least a few worked usage examples.
- Written for two audiences at once: an agent needing current-truth context
  and a human reader who is not steeped in symbol names. Where holding that
  balance strains a single file, split into the directory form rather than
  compress.

Non-`index.md` files inside a feature directory carry only `kind: feature`,
`description`, and optional `tags`; the `index.md` owns the full contract
(single source for `area`/`packages`/`features`/`fixes`/`symbols`).

### Getting-started documents

Location: `{docs root}/getting-started/{feature-name}.md` — same docs root as
the feature doc it fronts.

Required frontmatter:

| Property | Type | Meaning |
|----------|------|---------|
| `kind` | `"getting-started"` | discriminator |
| `feature` | `file` | reference to the feature doc (`index.md` for directory-form features) |
| `description` | `string` | one-liner |

Required content: clear-language WHAT and WHY; 2–3 usage examples; a short
"advanced features" teaser list where each item links into the relevant
section of the feature doc — short, honest, and utility-forward, no
overselling. Every getting-started doc links to its feature doc at least once.

### Research documents

Location: `{docs root}/research/{topic}/…` (existing convention). Two shapes:

- **One-off research** — an `inline-compose` document carrying the research
  prompt.
- **Fleet research** — a `_fleet.md` document at the topic root: a headed
  Claudine sequence whose body is the per-item research prompt and whose
  `sequence:` iterates an external YAML enumeration (the pattern already live
  at `claudine/docs/research/agent-cli/_agent-cli.md` +
  `claudine/docs/providers.yaml`). Existing `_{topic}.md` / `_summary.md`
  fleet drivers are grandfathered; new fleet drivers use the `_fleet.md`
  name so tooling can find them.

Required frontmatter: `kind: research` on the driver **and** on every document
the fleet produces (the fleet template must stamp it). Optional:
`content_policy` (future-enforced), `prompt`, `last_updated`, `hash` (already
in use on darkmatter research docs).

### README documents

No `kind:` frontmatter (READMEs are landing pages, not pipeline inputs), but a
normative structure:

- **Package-area README** (`{area}/README.md`): the scope boundary all
  packages in the area serve; a list of the area's packages, each with a
  one-sentence description and a Markdown link to that package's README.
- **Package README** (`{area}/{pkg}/README.md`): the package's utility; one
  simple example; a list of provided functionality where **each listed feature
  links to its feature doc**; a backlink to the area README.

The current `darkmatter/README.md` (short + package table) already matches the
area shape; `darkmatter/lib/README.md`'s deep nested feature tree becomes a
linked list into `docs/features/` rather than an ever-growing inline outline.

### Area skill documents

The existing 1:1 `ctx.area` ↔ `.claude/skills/{area}/` convention continues
unchanged in structure. This spec adds two obligations:

1. **Drift coupling** — the `document.md` sequence's final state updates the
   skill wherever its prose intersects features the spec changed, and
   regenerates `hash:` via `md hash` after any edit (existing repo rule).
2. **Token economy** — skills link to feature docs for detail instead of
   inlining it; fleet research reaches the skill only through summarization
   (see `publish-summary-research` below).

## The Documentation Schema

A single standalone SimplifiedSchema document is the machine-readable
authority for the contracts above:

- Location: `docs/schemas/documentation.yaml` at the **repo root** (the
  contracts are monorepo-wide, not darkmatter-specific).
- Shape: the tagged standalone envelope (`kind: schema` + `types:` mapping)
  shipped by the schemas/compose-schemas work, defining named types
  `FeatureDoc`, `FeatureFile`, `GettingStartedDoc`, `ResearchDoc`,
  `FeatureCatalog`.
- Authoring uses the new grammar where it earns its keep: `suggest(...)` for
  `area` (package-area names) and common `tags`, `file` for the
  `feature` pointer, dictionaries for the catalog map.

Binding documents to the schema, in order of preference:

1. **DMLS extension baseline** — a `[schema.extensions.documentation]` entry
   in the repo `.dmls.toml` applying the named types by glob
   (`**/docs/features/**`, `**/docs/getting-started/**`, `**/docs/research/**`),
   so authors get completion, hover, and `dm.*` diagnostics with zero
   per-document boilerplate.
2. **Explicit `$schema`** (`FeatureDoc@/docs/schemas/documentation.yaml`) for
   documents outside the glob or run through `md schema validate` in CI/just
   recipes.

Validation posture: advisory in editors (DMLS), enforced by the `document.md`
sequence for every document it writes (a lifecycle stack runs
`md schema validate` per produced file and fails the step on violations).

## The Feature Catalog

Per docs root, a generated cache at `{docs root}/features/catalog.yaml`:

```yaml
kind: catalog
area: darkmatter
generated: 2026-07-10T00:00:00Z
features:
  simplified-schema:
    doc: features/simplified-schema/index.md
    description: Single-line YAML schema grammar compiled to JSON Schema…
    packages: [darkmatter]
  dmls-autocomplete:
    doc: features/dmls-autocomplete.md
    description: …
    packages: [dmls]
```

Rules:

- The catalog is a **cache, never an authority** — source of truth is the
  frontmatter of the feature docs themselves. Any consumer may regenerate it
  by scanning `features/**` for `kind: feature` index documents.
- The `document.md` Gather state refreshes it opportunistically (regenerate,
  diff, write if changed) so the lookup service the sequence needs is also
  what keeps the file honest.
- It exists to make "what features already exist and what do they claim to
  cover" a single cheap read for both the sequence and ad-hoc agents.

## The `document.md` Sequence

### Invocation and parameters

```bash
claudine sequence prompts/document.md spec={filepath}
```

Lives at repo-root `prompts/document.md` (alongside the existing prompt
library). Parameters:

| Param | Required | Type | Meaning |
|-------|----------|------|---------|
| `spec` | yes | `file` | path to the completed spec's `spec.md` (or its directory) |
| `docs_root` | no | `file` | override the derived docs root (for package-owned `docs/`) |
| `skip_states` | no | `string[]` | states to skip (e.g. rerun only `skill`) |
| `non_interactive` | no | `bool` | accept Gather results without the Associate confirmation (default `false`) |

Derived, not passed: `area` (from the spec path), affected packages (from the
spec's `plan.md` frontmatter `packages:` / `source_code:` lists when present).

The `$schema` block of `document.md` declares these so Claudine's existing
schema-driven missing-required collection prompts for `spec` when omitted.

### States

1. **Gather** — read the spec (and `plan.md`/reviews if present); refresh the
   feature catalog; produce two lists: feature docs **updated** and feature
   docs **created** by this spec, each entry carrying name, description, and
   evidence (which spec sections imply it).
2. **Associate** — interactive checkpoint. Present the two lists; the user
   accepts all or a subset; then: "were features missed?" → if yes, offer the
   remaining catalog entries via a choose-many TUI; then: "name a new feature
   not captured?" → if yes, collect name + description via text inputs.
3. **Refine** — only entered when Associate changed the picture. A
   user-added feature triggers an interactive clarify session (correct
   representation, boundary adjustments in neighboring features). A
   user-associated existing feature triggers a spec-writer validation pass:
   is the spec's interaction with that feature clear? If not, clarify
   interactively.
4. **Feature updates** — orchestrator fans out one subagent per confirmed
   feature doc (parallel); each subagent owns exactly one feature doc
   (create or update), honors the frontmatter contract, and appends the spec
   directory name to `features:`/`fixes:`.
5. **Getting-started updates** — same fan-out shape over the affected
   getting-started docs.
6. **README updates** — update package and area READMEs whose feature lists
   changed (new feature → new linked entry; renamed/split feature → fixed
   links).
7. **Skill updates** — publish pending fleet-research summaries
   (`publish-summary-research`), reconcile skill prose that intersects the
   changed features, regenerate `hash:` frontmatter via `md hash`.

### Determinism and lifecycle usage

Lifecycle hooks are used to pin the deterministic parts so the model only
handles judgment:

- `initialize`: validate `spec` resolves and is a spec document; derive
  `area`/docs root into frontmatter via `set_frontmatter`.
- `start` (per state): announce the state (`message`/`stderr`).
- `success` stacks (per producing state): run `md schema validate` over every
  written document and `md hash --save` where the contract requires `hash:`;
  fail the step on violations rather than trusting the agent's self-report.
- `finalize`: aggregate a summary of documents created/updated per state.
- `fail_fast: true` — a failed state must stop the sequence; documentation
  states build on each other's outputs.

### Interactivity contract (v0 vs target)

Claudine sequences currently **reject** `interactive: true` and have no
general TUI step verb; the interactive Associate/Refine states therefore
cannot be expressed as pure sequence steps today. This spec defines the
target contract and mandates a working v0:

- **v0 (must work now):** `document.md` runs the interactive states through
  the agent conversation itself (the agent asks, the user answers in-session),
  with Gather/Update states as ordinary sequence steps. Where a hard TUI is
  genuinely better (choose-many over dozens of catalog entries), the step may
  shell out to the `question` CLI (`biscuit-tui`) from a lifecycle `shell`
  action or the prompt body.
- **Target:** first-class interactive sequence steps (below) replace the
  workaround; `document.md` is updated when they land.

## Claudine Sequence Capability Requirements

Requirements harvested for the upcoming sequences spec (per the prompt, draft
`document.md` **as if** these exist where that clarifies the design, but keep
the v0 fallback runnable). Note that
`claudine/docs/topics/flow-control/sequences.md` already documents several of
these (groups, `parameters:`, headless `prompt:`/`shell:` steps, step `id`s)
**without an implementation** — the sequences spec should reconcile that doc
with reality rather than invent a third model.

1. **Interactive TUI steps** — step verbs wrapping the six `biscuit-tui`
   components (`choose-one`, `choose-many`, `boolean-switch`, `text-input`,
   `text-area-input`, `input-table`), writing the captured value into
   sequence state / frontmatter for later steps. Needed by Associate/Refine.
2. **Loopable groups** — a named group of steps that cycles under a
   `while`/`until` condition with between-iteration mutations, i.e. the
   existing `loop:` engine (conditions, `LoopAction` mutations, `_loop_*`
   ambient variables) generalized from "one document" to "a group of steps".
   Needed for "iterate on a feature doc until validation passes" and for
   fan-out over the confirmed feature list.
3. **Dynamic enumeration** — a step's output (e.g. Gather's confirmed feature
   list) becoming the iteration list for a later group, not just static
   external YAML.
4. **`parameters:` block** — typed declared params
   (`spec: Filepath`, `non_interactive: Option<bool>`) rather than untyped
   setter conventions; already sketched in sequences.md.
5. **Headless step verbs** — `prompt:` (compose a referenced document as the
   step) and `shell:` steps, so `document.md` can mix agent states with
   deterministic shell states (catalog refresh, `md hash`).
6. **Conditional state skip** — skip a step when a condition holds (e.g. skip
   Refine when Associate accepted everything unchanged) without faking it
   through prompt instructions.

## `publish-summary-research` Recipe

A shared `just` recipe (in `just/`, imported by area justfiles) with the
contract:

```bash
just publish-summary-research <topic>
```

1. Locates `{docs root}/research/<topic>/` and its `_fleet.md` /
   summary driver.
2. Runs a summarizing compose over the topic's research documents producing
   (or refreshing) `_summary.md` (`kind: research`, `hash:` stamped).
3. Publishes the summary into the area skill between managed markers
   (`<!-- research:<topic> --> … <!-- /research:<topic> -->`), replacing any
   previous published block, and regenerates the skill's `hash:` via
   `md hash --save`.

The recipe is idempotent: rerunning with unchanged research is a no-op
(hash-compare before write).

## Scope Boundaries

- **`content_policy` enforcement** — the property is accepted and documented;
  the staleness ruleset/evaluator is explicitly out of scope (near-future
  feature).
- **Sequence engine changes** — this feature *specifies requirements* for the
  sequences spec; it does not implement Claudine engine changes. `document.md`
  v0 must run on today's engine.
- **Retroactive migration** — no bulk rewrite of existing `docs/topics/`,
  component docs, or other areas' docs. Existing documents are grandfathered;
  feature docs may link to them. Migration happens feature-by-feature as
  `document.md` runs touch them.
- **Brainstorming schemas/LSP v1** — the downstream activity that consumes
  the documentation this feature produces. It is sequenced in `plan.md`
  (its deliverable is `darkmatter/features/2026-07-10-v1-schemas-and-lsp/spec.md`)
  but is not part of this spec's normative surface.

## Acceptance Criteria

1. `docs/schemas/documentation.yaml` exists at the repo root as a tagged
   standalone SimplifiedSchema defining `FeatureDoc`, `FeatureFile`,
   `GettingStartedDoc`, `ResearchDoc`, and `FeatureCatalog`, and
   `md schema validate` passes on every blessed document this feature
   produces.
2. The repo `.dmls.toml` applies the documentation types by glob so an editor
   session on a new feature doc gets key completion and `dm.*` diagnostics
   with no per-document `$schema`.
3. `darkmatter/docs/features/` and `darkmatter/docs/getting-started/` exist
   with a generated `catalog.yaml`, and every catalog entry round-trips from
   feature-doc frontmatter (regenerating the catalog is byte-stable).
4. `prompts/document.md` exists, runs via
   `claudine sequence prompts/document.md spec={filepath}` on today's engine,
   and implements the seven states with lifecycle-enforced validation
   (`md schema validate` + `md hash`) on every document it writes.
5. The sequence has been exercised against at least two of the recently
   completed schema/DMLS specs, and the resulting feature docs,
   getting-started docs, README updates, and skill updates all satisfy their
   frontmatter contracts.
6. Each exercise run ends with a recorded fitness review; prompt iterations
   are captured until a run completes with no manual rescue.
7. Darkmatter package READMEs list their features as links into
   `docs/features/`, and the area README links to each package README.
8. `just publish-summary-research <topic>` exists as a shared recipe, is
   idempotent, and the darkmatter skill's `hash:` is valid after a publish.
9. A "sequence capability requirements" document exists (harvested gaps from
   the exercise runs) ready to seed the Claudine sequences spec.
10. `darkmatter/features/2026-07-10-v1-schemas-and-lsp/spec.md` exists as a
    draft produced by the brainstorming session (plan-level deliverable).

## Definition of Done

All acceptance criteria hold; the spec and plan reflect any rulings made
during clarification (recorded inline as blockquotes, per repo convention);
the darkmatter skill documents the new documentation conventions; and Ken has
signed off that `document.md` is fit for purpose as the standing
post-implementation documentation process.

## Open Questions

1. **Feature-doc root naming** — `docs/features/` collides verbally with the
   spec ledger `{area}/features/`. Keep the symmetry (proposed) or pick a
   distinct name (`docs/functionality/`, `docs/capabilities/`)?
2. **Schema home** — repo-root `docs/schemas/documentation.yaml` (proposed,
   monorepo-wide) vs `darkmatter/docs/schemas/` (area-local, nearer its
   machinery)?
3. **Catalog scope** — per docs root (proposed) or one repo-wide catalog?
   Per-root keeps `features:` names area-unique and regeneration cheap.
4. **`feature:` pointer form** — file reference to the feature doc (proposed,
   navigable by DMLS) vs bare feature name resolved through the catalog?
5. **Getting-started cardinality** — strictly one per feature (prompt's
   wording) or allowed to cover a small cluster of tightly related features
   with multiple `feature:` entries?
6. **README enforcement** — should READMEs eventually get a schema-checkable
   frontmatter contract too, or stay structure-by-convention (proposed)?
7. **Where the sequence-gaps deliverable lands** — a document inside this
   feature directory (proposed: `sequence-requirements.md`) or directly as a
   draft `claudine/features/…-sequence-enhancements/spec.md`?
