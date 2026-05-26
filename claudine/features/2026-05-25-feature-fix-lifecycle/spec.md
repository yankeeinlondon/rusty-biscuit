# Feature / Fix Lifecycle and Topic-Doc Convention

**Date:** 2026-05-25
**Status:** Draft
**Scope:** Repo-wide policy. Initial implementation lives in
`just/lifecycle.just`; documentation lives at repo root.

## Goal

Formalize the relationship between change vehicles (features and fixes)
and durable subsystem documentation (topic docs), and define what must
happen at the moment a change is marked complete. Capture enough
metadata at closure that future tooling (drift detection, episodic
synthesis, blast-radius cross-referencing) can be built on top without
backfilling.

## Motivation

The convention you described is already partially in place but
undocumented:

- `{area}/docs/topics/`, `{area}/docs/cli/`, etc. hold 36+ topic docs in
  `claudine/` alone; they are treated as authoritative source-of-truth.
- `{area}/features/YYYY-MM-DD-<name>/` and `{area}/fixes/YYYY-MM-DD-<name>/`
  hold in-flight change work.
- `_completed/` archives preserve the full artifact set (spec, plan,
  reviews, drift notes) of finished work.
- `sniff docs --blast-radius` already filters for the `blast_radius`
  frontmatter key, and several docs declare it.
- `just complete` already moves a finished feature/fix to `_completed/`.

What is missing is the policy connecting these pieces. Today:

- No rule says a feature must update its topic docs at closure; some do,
  some do not.
- `blast_radius` semantics are ambiguous (missing vs empty vs populated
  carry no agreed meaning; `agent-prompt.md`'s empty value reads as
  TODO under one interpretation and "no sensitivity" under another).
- No structured record exists that ties a completed change back to its
  commits and to the topic docs it affected.
- `just complete` does not capture any of this — it only renames the
  directory.

Without this connective tissue, every future tool (drift detection,
episodic synthesis, blast-radius checking) has to scrape and infer the
relationships from heterogeneous sources. Capturing them at closure,
when the author still has full context, is dramatically cheaper than
reconstructing them later.

## Non-goals

- **TreeHugger-based drift detection.** The blast_radius semantics
  defined here enable cross-referencing source changes against topic
  docs, but the actual drift-detection CLI is a separate spec.
- **Backfilling historical `_completed/` entries.** This spec applies
  going forward. Existing archives keep their current shape.
- **Agent skill publishing.** Deferred until Darkmatter ships its
  `publish` feature, which will unify human-facing topic docs and
  agent-facing skills.
- **Claudine daemon closure events.** The closure flow could one day
  emit a structured event into a claudine-managed database; this is
  deferred until the claudine logging refactor lands.
- **Episodic synthesis / narrative generation.** The data captured at
  closure (commits, design_docs, episodic.jsonl entries) is the
  *input* to future synthesis; the synthesis itself is its own spec.
- **Migration of existing `blast_radius:` entries from file refs to
  symbol refs.** Mixed file + symbol refs are valid going forward;
  migration is opportunistic.

## Concepts

### Functionality

The atomic unit of capability. A coherent named thing with a clear
contract — for example "the `system_prompt` resolution path,"
"non-interactive sessions," or "the MCP catalog."

Operational definition: **functionality is what a topic doc covers.**
One topic doc, one functionality. New functionality means a new topic
doc; modified functionality means an updated topic doc.

### Topic doc

A subsystem-scoped document with a 1:1 relationship to a functionality.

- Lives at `{area}/docs/{category}/{name}.md` where `category` is the
  conventional bucket for that area (`topics/`, `cli/`, `getting-started/`,
  etc.). Categories are per-area and conventional; this spec does not
  legislate them.
- May be a HEAD doc that links to sub-docs when depth warrants. The
  HEAD doc is itself a topic doc; its sub-docs are also topic docs
  with their own (narrower) functionalities.
- Long-lived. Touched whenever the functionality it covers changes —
  potentially across many features and fixes.
- Declares `blast_radius` in frontmatter (see below).

### Change vehicle: feature vs fix

A **feature** introduces or substantially extends functionality. Cardinality
with functionality: 0..M new, 0..M modified, 0..M realized-without-design-
change. Most commonly 0..1 new and 0..M modified.

A **fix** is an incremental change to an existing design. Cardinality with
functionality: rarely 0..M new (a fix that introduces new functionality is
usually a sign it should have been a feature), commonly 0..M modified,
most often 0..M realized-without-design-change (bug fixes that make an
existing design more fully realized without altering it).

### Realized-without-design-change

A change that makes an existing topic doc's description more accurately
reflect the code, without altering the topic doc's content. The doc was
already correct; the code is the thing that moved. This category exists
because not every change needs to touch a doc — most bug fixes do not.

## `blast_radius` semantics

Three states. Authoritative.

| State | YAML | Meaning |
|---|---|---|
| **Unanalyzed** | `blast_radius:` key absent, or value is `null` / missing | This doc has not yet been classified. Drift tooling treats it as needing classification. Default for new docs. |
| **No sensitivity** | `blast_radius: ""` (explicit empty string) | This doc has been reviewed and intentionally has no source-code blast radius (pure tutorial, overview, naming-conventions, etc.). Drift tooling treats it as a no-op. |
| **Populated** | `blast_radius:` is a list of strings | Each entry is either a file ref (relative path) or a symbol ref (`"symbol::{name}"`). Drift tooling cross-references against source changes. |

### Symbol reference format

```yaml
blast_radius:
  - "claudine/cli/src/commands/sequence.rs"       # file ref (existing form)
  - "symbol::SystemPromptReport"                  # bare symbol name
  - "symbol::prompt_reporting::AgentPromptReport" # path-qualified symbol
```

- **Bare symbol names** are resolved by TreeHugger across the doc's
  package area. If unique within the area, the bare name suffices.
- **Path-qualified symbols** disambiguate when the bare name is not
  unique within the area, or when referencing across areas. The path
  uses Rust module-path syntax (`::`-separated).
- **Cross-area references** use full repo-relative paths in the symbol
  qualification or in a file ref.
- **Mixed lists** of file and symbol refs in the same doc are allowed.
  Migration from file refs to symbol refs is opportunistic, not
  required by this spec.

### What `blast_radius` does NOT define

- It does not list every file/symbol the doc *mentions* — only those
  whose semantic change would invalidate or require revision of the
  doc.
- It does not capture call-graph reach. A symbol may be in blast_radius
  even when it is not used by anything the doc talks about, if changing
  it would invalidate the doc's claims.

## `spec.md` frontmatter additions

Two new frontmatter properties are populated at closure (not before):

```yaml
---
# … existing frontmatter …
design_docs:
  - "claudine/docs/topics/agent-prompt.md"
  - "claudine/docs/topics/system-prompt.md"
commits:
  - "e79282bf4"
  - "dc2217794"
  - "e960cf205"
---
```

- **`design_docs`** — list of topic docs (by repo-relative path) that this
  feature/fix created or modified. The list is the authoritative record
  of which durable docs the change affected. Empty list (`design_docs: []`)
  is valid and means "this change had no impact on topic docs"
  (typically a bug fix that fully-realizes an existing design).

- **`commits`** — list of commit hashes implementing this feature/fix.
  Aggregated from the branch's git history at closure time, filtered to
  commits that are actually relevant (sniff supplies the candidate list;
  the closure agent filters). Order is git-log order (newest first or
  oldest first — implementation decides).

Existing frontmatter on `spec.md` is preserved. These two properties
are *additive*; they MUST NOT replace or overwrite existing keys.

## `episodic.jsonl` shape

Each area's `features/` and `fixes/` directory gains an `episodic.jsonl`
file. JSONL = newline-delimited JSON, one entry per completed change,
appended at closure.

```
claudine/features/episodic.jsonl
claudine/fixes/episodic.jsonl
biscuit-terminal/features/episodic.jsonl
biscuit-terminal/fixes/episodic.jsonl
…
```

### Entry schema (worked example)

```json
{
  "id": "2026-05-25-prompt-reporting-encapsulation",
  "kind": "feature",
  "area": "claudine",
  "title": "Prompt Reporting Encapsulation",
  "completed_at": "2026-05-25",
  "spec_path": "claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/spec.md",
  "commits": ["e79282bf4", "dc2217794", "e960cf205"],
  "design_docs": [
    "claudine/docs/topics/agent-prompt.md",
    "claudine/docs/topics/system-prompt.md"
  ],
  "functionality": {
    "created": [],
    "modified": ["agent-prompt", "system-prompt"],
    "realized": []
  },
  "summary": "Replaces 21 free functions and two boolean-bag config structs with SystemPromptReport and AgentPromptReport; collapses PromptVerbosity + PromptReportFormat into a single ReportMode enum; renames EffectiveSystemPrompt to ResolvedSystemPrompt."
}
```

### Field definitions

| Field | Type | Source |
|---|---|---|
| `id` | string | The directory basename (e.g. `2026-05-25-foo`). |
| `kind` | `"feature"` \| `"fix"` | From the parent directory (`features/` vs `fixes/`). |
| `area` | string | Package area (e.g. `claudine`, `biscuit-terminal`). |
| `title` | string | From spec.md frontmatter `title:` if present, else derived from `id`. |
| `completed_at` | ISO date | Date `just complete` was run. |
| `spec_path` | repo-relative path | Final location of `spec.md` after the move to `_completed/`. |
| `commits` | array of strings | Same list written to `spec.md::commits`. |
| `design_docs` | array of strings | Same list written to `spec.md::design_docs`. |
| `functionality.created` | array of strings | Topic doc base-names (without `.md`) for newly-created docs. |
| `functionality.modified` | array of strings | Topic doc base-names for docs whose content changed. |
| `functionality.realized` | array of strings | Topic doc base-names for docs whose content did NOT change, where the change made existing claims more accurate. |
| `summary` | string | 1–2 sentence summary produced by the closure agent. |

### JSON Schema deliverable

A canonical JSON Schema for the entry shape lives at
`docs/schemas/episodic-entry.schema.json` so tooling (including a
future `claudine` daemon) can validate entries deterministically. The
schema is the authoritative spec; the worked example above is
illustrative.

## Enhanced `just complete` workflow

The shared recipe at `just/lifecycle.just::complete` is extended. Its
selection logic (the existing fzf/auto-match flow) is preserved; new
steps are inserted between *selection* and *move-to-archive*.

### New flow

1. **Select** the feature/fix dir (unchanged from today).
2. **Verify** the dir is non-empty (unchanged).
3. **Gather closure inputs:**
   - The selected dir's `spec.md` content.
   - The branch's commit candidates from sniff (sniff supplies the
     full candidate list; the agent will filter to relevant commits).
   - The set of existing topic docs in the affected area
     (`{area}/docs/**/*.md`).
4. **Invoke claudine** with a closure prompt (template lives in
   `compositions/closure.md` or similar — exact location is an
   implementation choice). The agent's responsibilities:
   - Filter commit candidates down to those that actually implement
     this feature/fix.
   - Identify topic docs created or modified for this change.
   - Update topic docs in place where needed (the user reviews diffs
     before they land).
   - Produce the `design_docs`, `commits`, and `functionality.*`
     values.
   - Produce the 1–2 sentence summary.
5. **Write closure metadata:**
   - Update `spec.md` frontmatter (add `design_docs:` and `commits:`).
   - Append the new entry to `{area}/{features|fixes}/episodic.jsonl`.
6. **Move** the dir to `_completed/` (existing logic).
7. **Print** the existing confirmation message.

### Failure modes

- **Claudine invocation fails or times out** — the recipe stops before
  the move. The user can re-run `just complete` after addressing the
  failure. Partial state is acceptable because the spec.md frontmatter
  edits are idempotent and the JSONL append is the last write before
  the move.
- **Agent produces obviously wrong output** — the user reviews the
  diffs/proposed JSONL entry before the move proceeds. The recipe
  surfaces a confirmation prompt for the proposed metadata. (The
  details of the confirmation UI are an implementation choice.)
- **No spec.md in the selected dir** — the recipe falls back to a
  metadata-light path: it still aggregates commits via sniff and
  prompts the user for `design_docs`/`functionality` interactively,
  then writes the JSONL entry and moves the dir. This covers older
  dirs and emergency fixes that did not start with a spec.

### Sniff's role

Sniff supplies the candidate commit list for the agent to filter. The
expected interface is something like
`sniff git commits --since-branch-from=main` (exact flag set is an
implementation detail) producing a structured list the agent can
consume. Sniff already has the git infrastructure; this spec just
formalizes that the closure recipe is sniff's primary consumer for
this use case.

## Where the policy lives

1. **Repo-root `CLAUDE.md`** gains a new section titled "Feature / Fix
   Lifecycle." Contents: the three states of `blast_radius`, the
   `design_docs` / `commits` frontmatter additions, and a one-line
   pointer to the detail doc and the `just complete` recipe. Target
   length: 25 lines or fewer.
2. **`docs/feature-lifecycle.md`** (new, at repo root). Contents: full
   guidance with worked examples — at least one per concept (a feature
   that creates new functionality, a feature that modifies existing
   functionality, a fix that realizes-without-design-change), the
   `blast_radius` examples, and a walked-through `just complete`
   transcript.
3. **`docs/schemas/episodic-entry.schema.json`** (new). The
   authoritative JSON Schema for the `episodic.jsonl` entry shape.

The existing `claudine/docs/topics/lifecycle.md` is the claudine
*runtime* lifecycle (events / hooks); it is unrelated and stays where
it is.

## Acceptance criteria

- `just/lifecycle.just::complete` performs the new flow end-to-end on
  a representative feature dir and on a representative fix dir
  (claudine area).
- `docs/feature-lifecycle.md` exists at repo root with the three
  worked examples named above.
- `docs/schemas/episodic-entry.schema.json` exists and validates the
  example entry in this spec.
- `CLAUDE.md` contains the new "Feature / Fix Lifecycle" section
  (≤ 25 lines) with the pointer to the detail doc.
- A dry-run of `just complete` on a no-op test feature produces the
  expected `episodic.jsonl` entry and `spec.md` frontmatter without
  moving the dir.
- The first real `just complete` invocation after this lands (likely
  the prompt-reporting encapsulation feature or the comment-quality
  feature) produces a valid entry and updates the relevant topic
  docs.
- All existing `_completed/` archives remain untouched (no backfill).

## Risks

- **Closure agent makes wrong associations.** The agent could mislabel
  a commit as relevant when it is not, or miss a relevant topic doc.
  Mitigation: the user reviews the proposed metadata and edits before
  the move proceeds. The agent's output is a draft, not authoritative.
- **`blast_radius` symbol resolution ambiguity.** Bare symbol names
  may not be unique within an area. Mitigation: TreeHugger error
  surfaces the ambiguity; author re-declares with path qualification.
  Drift tool documents the disambiguation rule.
- **`episodic.jsonl` becomes too large to read manually.** Likely over
  years. Mitigation: format is designed for tool consumption first;
  human-readable rollups are a future spec (episodic synthesis).
- **`spec.md` frontmatter additions break existing tooling that
  expects a fixed frontmatter shape.** Mitigation: the additions are
  *additive* with no key reuse; consumers using `deny_unknown_fields`
  semantics would need a one-line update.
- **Re-running `just complete` on a partially-closed dir produces
  duplicate JSONL entries.** Mitigation: the recipe checks for an
  existing entry with the same `id` and either updates in place or
  refuses, with the chosen behavior documented in the recipe.
- **Bigger blast radius than expected.** This is a repo-wide policy
  change that affects 10+ areas. The recipe lives once in
  `just/lifecycle.just` so the rollout is centralized, but every area
  becomes subject to the policy at the same moment.

## Out of scope (future specs)

- **Drift-detection CLI** that cross-references `blast_radius`
  declarations against TreeHugger-detected semantic changes and
  surfaces docs whose blast-radius symbols changed since the doc was
  last touched.
- **Episodic synthesis** that consumes `episodic.jsonl` files (and,
  later, the claudine daemon's event database) to produce narrative
  rollups, monthly digests, or queryable history.
- **Claudine logging-system integration** that emits a structured
  closure event into the claudine daemon's database. Waits for the
  claudine logging refactor.
- **Agent skill publishing** that mirrors human-facing topic docs into
  agent-facing skills with progressive disclosure. Waits for
  Darkmatter's `publish` feature.
- **Backfill** of `blast_radius` across existing topic docs.
  Opportunistic; happens as docs are touched.
- **Migration** of existing file-ref `blast_radius` entries to symbol
  refs. Opportunistic.
- **Schema-driven validation** of frontmatter at `just complete` time.
  Soft validation only in v1; hard validation is a future hardening
  pass.

## Trajectory

This is the first of three related specs. The other two are
acknowledged here so this spec does not preclude them.

| Spec | Scope |
|---|---|
| **This spec — Lifecycle and topic-doc convention** | Concepts, `blast_radius` semantics, frontmatter additions, `episodic.jsonl`, enhanced `just complete`. |
| **Drift detection** | TreeHugger semantic-change detection cross-referenced against `blast_radius`. Produces a `just check-drift` (or similar) recipe that surfaces docs needing review after recent code changes. |
| **Episodic synthesis** | Periodic synthesis of `episodic.jsonl` (and, later, claudine daemon events) into narrative summaries. Waits until claudine has a queryable event database. |
