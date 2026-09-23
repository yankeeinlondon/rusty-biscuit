---
description: |-
    Interactive session that turns a caller's rough description of one or more new Darkmatter
    context variables (`ctx.*`, and therefore `current.*`) into clarified requirements, a
    feature spec under `darkmatter/features/`, an implementation across Darkmatter, Sniff, and
    Claudine, updated documentation, and verified parity with the `claudine context` report.

    Pass the requirements on the command line:

        claudine compose prompts/_add/add-context-variables.md requirements='add ctx.iii returning the static phrase "I am the king"'

    Omit `requirements` in a terminal and Claudine collects it interactively before launch.
$schema:
    requirements: string(required; not-empty) -> the caller's description of the context variable(s) to add; rough prose is fine, the session clarifies it
    name: string -> short kebab-case name for the feature directory; derived from the requirements when omitted
kind: "context variable"
kinds: "context variables"
catalog_command: "claudine context"
spec_dir: "{{ ctx.repo_root + '/darkmatter/features/' + ctx.today + '-' + (name || 'add-context-variables') }}"
interactive: true
initialize:
    stack:
        - when: "!file_exists(ctx.repo_root + '/darkmatter/docs/schemas/darkmatter.yaml')"
          action:
              - error: "The **add-context-variables** prompt must run inside the rusty-biscuit checkout: it edits `darkmatter/docs/schemas/darkmatter.yaml`, which was not found under `{{ctx.repo_root}}`."
start:
    stderr: "Starting an interactive session to add context variables; the agent will clarify the requirements, write a spec to `{{spec_dir}}`, then implement it."
    say: "Starting the add context variables session."
success:
    stderr: "The context-variable session finished; spec and implementation are under `{{spec_dir}}`."
    say: "The context variables have been added and are ready for review."
    message: "✅  context variables added in **{{ctx.repo}}** (spec: `{{spec_dir}}`, {{ctx.agent}}/{{ctx.model}})"
failure:
    say: "Adding the context variables failed."
    message: "❌  the add-context-variables session failed in **{{ctx.repo}}** ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
---

# Add Context Variables to Darkmatter

## Context

You are a senior Rust engineer who owns Darkmatter's runtime context: the `ctx.*` namespace
that composed Markdown reads, its lazy twin `current.*`, and the `claudine context` report
that documents both. Load the `darkmatter` skill (its `compose.md` topic in particular), the
`claudine` skill (`composition.md` and `lifecycle.md`), and the `sniff` skill before you
start; load `rust` and `rust-testing` when you write code and tests.

::file ../_repo-context.md

## The Caller's Requirements

{{requirements}}

## Background: What a Context Variable Is

The Claudine topic below is the user-facing contract. Its implementation sections are older
than the code; the "Wiring Recipe" that follows it is what the code does today and wins on
conflict.

::file {{ctx.repo_root}}/claudine/docs/topics/state-management/context-variables.md exclude="## How the type system works" exclude="## How values are captured" exclude="## How to add a context variable" exclude="## Drift control for context variables"

::file {{ctx.repo_root}}/darkmatter/docs/topics/context-variables.md exclude="## Information Provided" exclude="## Overcoming*"

## Definition Checklist

Every candidate variable needs an answer to each of these before it is specified. This is
also the clarification agenda in Phase 2 below.

1. **Name** — `ctx.<snake_case>`; check `claudine context` for collisions and near-duplicates.
2. **Fact** — one sentence saying exactly what the value reports and from which authority
   (the request's retained root document, the captured repository observation, Sniff host
   discovery, the invocation environment, …). If the fact needs a probe Sniff does not have,
   the probe lands in Sniff first and Darkmatter consumes it.
3. **Type** — a SimplifiedSchema type: `string`, `string[]`, `boolean`, `number(integer)`,
   `date`, `datetime`, `object[]` with the object shape spelled out. Arrays are real arrays
   (a bare `{{{ ctx.x }}}` renders compact JSON; authors use `as_unordered_list(...)` and
   friends).
4. **Absence spelling** — decide between `required` (never null; use `""`, `[]`, or `false`
   for "observed nothing") and optional (`null` when unavailable). Rule of thumb from the
   more-context feature: a scope-like string is `required` and `""` so it works directly as
   a truthiness test; a value that can genuinely be unknown is optional and `null`. Never a
   sentinel string such as `"root"` or `"unknown"` unless the caller rules it.
5. **Group** — which `ContextGroup` captures it. Prefer an existing group when the value
   costs nothing beyond what that group already pays for; add a group when the I/O is new
   or expensive (the `GitHistory` split exists so `ctx.branch` never pays for a commit walk).
6. **Freshness under `current.*`** — request-owned (identity, invocation, repository
   topology: `current` reads what `ctx` reads) or mutable (branch, working tree, environment,
   host state: refreshes at reference time through the request's `CurrentProvider`). Say
   which, because the provider and its tests differ.
7. **Claudine evidence** — how Claudine-driven compose supplies the value. Supplied capture
   is fail-closed: a group Claudine does not supply projects `null`/empty plus a
   `PartialRuntimeCapture` diagnostic, never ambient discovery. New groups need an evidence
   builder on `ContextCaptureEvidence` and a Claudine caller that fills it.
8. **Cache volatility** — whether the value changes between identical runs
   (`memory_used`, `id`); volatile keys are excluded from compose-cache hashing.
9. **Function pair** — whether a parameterized lazy twin `<name>(args)` is wanted
   (`ctx.recent_commits` / `recent_commits(count)`). A pair shares one descriptor entry and
   one output format; the function side sets `pair: true` and inherits the description.
10. **Examples** — at least one realistic value per platform where platforms differ, and the
    exact value for every "not available" case.

## Wiring Recipe (verified against the code on {{ctx.today}}; the code wins on conflict)

Work in this order. The parity tests are red between steps; the notes say which red is
expected.

1. **Declare it in the schema.** Add the entry to the `ctx:` block of
   `darkmatter/docs/schemas/darkmatter.yaml`, in the form
   `name: "type(generated; required?) -> description"`. This YAML is the single source of
   truth: the descriptor catalog, `md schema about`, `claudine context`, DMLS hover and
   completion, and the generated doc block are all projected from it. Write the description
   for the report reader; it is the sentence they see.
2. **Place it in the report.** Add `("name", "Category", "Subsection")` to
   `CONTEXT_VARIABLE_GROUPING` in
   `darkmatter/lib/src/markdown/compose/context/catalog.rs`. The grouping map must stay
   total (`grouping_map_is_total` fails otherwise); categories are presentation only.
3. **Own it in a capture group.** In `darkmatter/lib/src/markdown/compose/context/capture/`:
    - existing group: append the key to that module's `KEYS` (or `HISTORY_KEYS`, `OS_KEYS`,
      …) and insert the value in its `populate_*` function, projecting the documented
      absence value explicitly. A captured group must project every key it owns, with
      `null` for absence: a missing key is a `ContextProjectionInvariant` bug, not
      "unavailable";
    - new group: add the `ContextGroup` variant, extend `all()`, `name()`, and
      `projected_keys()` in `groups.rs`, add the capture module, add the ambient populator
      to the snapshot, add the `ContextCaptureEvidence::with_*` builder that distinguishes
      "not supplied" from "observed absent", and make `AnchoredRefresh` (in
      `context/current.rs`) able to refresh the group for `current.*`.
    - `every_descriptor_has_a_captured_runtime_key` in `catalog.rs` is the gate: descriptor
      names and captured keys must match in both directions.
4. **Decide the `current.*` behavior.** Request-owned keys (`Invocation`, `Document`, and the
   repository keys) are answered from the eager capture; everything else must be refreshable
   by one `CurrentProvider::refresh(key)` call that observes only that group. Add the key to
   whichever classification applies and test both memo-scope stability within one
   expression and freshness across expressions.
5. **Supply Claudine's evidence.** `claudine/lib/src/invocation_context.rs` builds the
   launch evidence Claudine hands to Darkmatter and installs the invocation-owned refresh
   provider. Extend it for a new group; for an existing group confirm the evidence already
   carries the new fact (Sniff request tiers matter: `GitRequest::summary()` deliberately
   omits worktree enumeration and commit descriptions). `claudine context --values` performs
   its own ambient capture and must show the value too.
6. **Handle volatility.** Add the key to `VOLATILE_CONTEXT_KEYS` in
   `darkmatter/lib/src/markdown/compose/cache/hashing.rs` when two identical runs may differ.
7. **Regenerate the doc block.** The block between the `BEGIN GENERATED: ctx catalog` markers
   in `darkmatter/docs/topics/context-variables.md` is checked by
   `context_variables_doc_matches_generated_catalog` in
   `darkmatter/cli/src/commands/schema/about.rs`; its failure message prints the block to
   paste back. Update that doc's hand-written "Capture Groups" table when a group changed.
8. **Sniff first when the probe is new.** Cross-platform discovery lives in Sniff
   (`sniff/lib/src/...`) with fixture-driven parsers so a test never needs the host to match
   the parsed OS. Darkmatter then consumes the Sniff API; Claudine forwards Sniff's
   observation as evidence.

## Rules Learned the Hard Way

These came out of the `darkmatter/features/2026-09-09-more-context` feature and its five
review cycles. Treat them as constraints, not suggestions.

- **Capture once, never rediscover.** One `FileResolutionContext` and one repository
  observation per request; no populator, refresh provider, or function re-derives CWD, the
  repository root, or package topology. A child pipeline (transclusion, `as_markdown`) reads
  the root's values.
- **Demand-driven means group-level.** A reference to `ctx.x` captures all of `x`'s group and
  nothing else. If a new value would make an existing group expensive, split the group.
- **Fail closed under Claudine.** Missing supplied evidence is `PartialRuntimeCapture` plus
  the group's empty/null projection. Never fall back to ambient discovery to fill a gap.
- **Absence is a value, not a diagnostic.** `null`, `""`, and `[]` from a captured group
  render normally; only a never-captured group is an error. Keep those three states distinct
  in code and in docs.
- **Root identity is request-owned.** Values about the root document (`self`, `hash`, `id`,
  `sid`, `last_updated`) come from bytes retained at request creation; never reopen the file
  or refetch a URL for them.
- **Variables and functions that share a name share one descriptor.** `pair: true` on the
  function; description authored once, on the `ctx` side.
- **Windows paths.** Directory-valued strings render with `/` separators and a stripped
  verbatim prefix so they survive Markdown; compare paths by component, not by string prefix.
- **Volatile output belongs in no persistent cache.** Nothing composed is persisted today
  (only raw remote-URL bodies under `--cache-root`); do not reattach a store.
- **Every descriptor reader is passive.** Schema parsing, DMLS completion and hover, and
  `claudine context` (without `--values`) do no I/O and capture nothing. A new key must be
  discoverable without observing it.
- **Impact analysis before edits.** The catalog loaders and `capture_at_event` in Claudine
  are HIGH/CRITICAL in GitNexus; stop and tell the caller before touching them.

::file ./_workflow.md

::file ../_no_formatting.md

::file ../_os.md
