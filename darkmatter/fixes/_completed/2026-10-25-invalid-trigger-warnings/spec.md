---
kind: fix
created: 2026-10-25
status: finalized-spec
area: darkmatter
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
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-25
clarified: false
implemented: true
review_iterations: 3
completed: true
human_review: false
needs_rulings: false
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
os_failing: windows
implemented_by: "claude/opus"
---

# Warn and skip invalid schema triggers

An invalid automatically discovered schema trigger must produce a warning
and be ignored. Under the default warning policy, it must never prevent
otherwise valid composition, prompt launch, or schema discovery from
completing. Ordinary CLI output writes the warning to stderr and keeps
stdout available for its normal output. Structured report modes include
the warning in their report instead.

This review is complete and the specification is ready to plan. No author
decision is outstanding; implementation has not started as part of this review.

A schema trigger is a YAML rule that selects documents and optionally
supplies a schema, called its payload. Discovery builds a registry: the
ordered set of usable triggers. A document's effective schema combines the
default baseline, matching trigger payloads, and its own authored `$schema`.

## Problem

Claudine built from `feat/pipeline` was invoked on
`prompts/merge-conflicts.md` in another worktree. That prompt declares no
`$schema`, but automatic discovery found this repository-level
`schemas/memory.yaml`:

```yaml
kind: schema-trigger
name: Memory Trigger
description: TODO
match:
```

The envelope rejects `name` and `description`; its null `match` would also
be invalid. Discovery aborts the entire scan before evaluating whether a
trigger matches the prompt. Claudine then renders the failure as:

```text
CompositionError: schema load failed
Could not load the $schema referenced by .../prompts/merge-conflicts.md.
```

Both the blocking behavior and the attribution are wrong for this case.
An unfinished trigger should not stop unrelated work, and the prompt did
not author a schema reference.

## Required behavior

### Recover per trigger

Automatic discovery loads each trigger independently. When an individual
trigger is invalid, record a warning, exclude that entire trigger and its
payload from the active registry, and continue discovering other triggers.
Valid triggers retain their existing ordering, matching, and precedence.
If every discovered trigger is invalid, continue with the baseline and any
authored document schema.

This applies whether or not the invalid trigger would match the current
document, and covers:

- Malformed YAML that claims a trigger envelope.
- Unsupported envelope keys, missing or invalid `match`, invalid condition
  grammar, vacuous conditions, and rejected renamed spellings.
- Invalid trigger declarations already classified as errors, including a
  null or non-string `kind`.
- Trigger payload resolution, parsing, compilation, import, and
  merge-compatibility failures, including missing files and cycles.
- Invalid discovered embedded triggers, identified by bundle and document.

A trigger is installed only after its envelope and payload have loaded
successfully. Never install a partial payload or discard other valid
triggers because one failed. Keep the existing passive parsers strict;
recovery belongs at the discovery boundary.

Keep filename shadowing unchanged. An invalid nearer file still reserves
its filename; skipping it does not activate a same-named trigger in a later
root or embedded bundle. Ordinary YAML files that do not claim the trigger
envelope remain silently ignored.

Use the existing envelope classifier, including its treatment of malformed
claimed YAML and invalid `kind` values. Do not infer a trigger from its
filename or from an arbitrary YAML parse error. Shadowed files remain
unread and generate no warnings. Between embedded bundles, the first
document classified as a trigger or invalid trigger reserves its name
before payload loading, even when it is skipped. Ordinary embedded import
documents retain their existing behavior.

Root enumeration, unreadable-root failures, filename collisions, and
invalid embedded bundle structure remain discovery errors. Reading an
enumerated file can also fail before its contents can be classified; that
remains an infrastructure error. Once a trigger is identified, a failure
reading its payload or imports is recoverable for that trigger. Do not
replace an infrastructure error with an apparently successful empty registry.

Check every candidate's payload through validator construction before
installing it, including raw JSON Schema without `$defs`. A payload that
passes the simple-object shape check but cannot compile is still skipped,
even if unmatched or if a later schema would replace its bad property.
Reuse the existing validator cache; do not compile every accepted payload
again on every document pass. Failures of an explicitly configured
baseline or the document's authored schema remain errors, as do combined
schema failures that cannot be attributed to one invalid trigger.

### Visible, accurate warnings

Every skipped trigger produces a warning identifying its source, the
underlying reason, and the fact that it was ignored. For example:

```text
warning: ignoring invalid schema trigger /repo/schemas/memory.yaml:
schema-trigger envelope supports only kind, match, and $schema;
found unknown keys: name, description
```

The warning must be visible during ordinary CLI use without enabling
debug logging. It must not claim that the prompt referenced `$schema`,
suggest correcting the prompt's schema path, or masquerade as a fatal
`CompositionError`. Its presence alone does not make the invocation fail
under the default warning policy.

Darkmatter retains typed, source-aware warning records for callers; the
library discovery layer does not print directly. Use the shared diagnostic
code `dm.schema.invalid_trigger`, severity `warning`, and a typed trigger
source: a disk path or an embedded bundle/document identity. Retain the
underlying typed cause and any available error position and related
payload/import source. Emit one warning for the first failure encountered
while loading each trigger; this fix does not add a second parser to
collect every defect within a broken envelope.

The primary warning source is the trigger, not the document being composed.
If the cause points into a payload or import, preserve that location
separately and include its identity in the explanation. Never attach a
payload line number to the trigger file or invent an exact position. Use
the established file-level fallback when a format requires a range, and
the format's source-free representation when no real source location is
available. Embedded identities must never become local file links or watches.

Warnings belong to the discovery result, independently of whether an
effective schema exists. Return them when all triggers are skipped, no
baseline or document schema is configured, or no valid trigger matches.
Retain already collected warnings if later authored-schema resolution,
validation, or composition fails, and deliver them with that failure.
Claudine must carry them through its preliminary schema checks as well as
composition, including checks that return no effective schema or reject
the prompt before a provider launches.

`md` and Claudine render warnings to stderr using their established terminal
components during ordinary output. `md compose --output report-json` uses
its existing diagnostic records, with no repeated warning row on stderr.
`md schema validate --format json` uses its existing `warnings` collection;
extend failure outcomes to retain earlier discovery warnings without
changing existing outcome fields or exit statuses. In a multi-file report,
place a shared trigger warning on the first input that discovers it, once
for the invocation. Do not add report modes to commands that lack them.

`md clean` and DMLS formatting run the same discovery and recover the same
way. Ordinary `md clean` output writes the warning to stderr. `md clean
--json` keeps its version-1 envelope and adds a top-level `warnings` array,
separate from `diagnostics`, whose document-span entries cannot describe a
trigger source; report mode does not repeat the warning on stderr. DMLS
formatting applies its edits and adds no message of its own, because the
registry already publishes the warning on the trigger file.

Register the new code with composition's existing `--deny` policy.
`md compose --deny dm.schema.invalid_trigger` and `--deny all` may explicitly
make this warning blocking; it keeps warning severity and its own source
instead of becoming a schema-load error. Apply the existing rule that stops
composition before its next effect when the warning becomes known, including
during preflight or child composition. No new recovery flag is needed. An existing
`--disable schema-triggers` setting performs no discovery and emits no
warning from it.

Deduplicate by trigger source within one top-level invocation, including
repeated schema passes, preflight, transclusion, Claudine retries, and
multi-file validation. Use the existing normalized file identity for disk
sources and the bundle/document pair for embedded sources; never deduplicate
by filename alone. Separate triggers referencing the same broken payload
each receive a warning. Emit warnings in discovery order, preserving the
first observation when passes overlap. A new invocation reports the warning
again, including when it reuses a cached discovery result. Cache entries
retain findings, never a process-global "already displayed" bit.

Track skipped envelopes, successfully read dependencies before a failure,
and attempted payload/import lookup locations even when resolution fails.
Missing locations remain lookup observations rather than existing-file
dependencies. Repairing or deleting an envelope, repairing a payload or
import, or creating a missing file or its parent folder must refresh the
result through existing cache checks and editor file events. Cover an
all-invalid first discovery with no previous working registry. Preserve
existing recursive-search refresh limits and explicit rescan behavior;
this fix adds no filesystem polling or new search rules.

### Editor refresh uses the current valid triggers

A scope is the document context for which DMLS prepares a trigger registry.
DMLS installs a completed discovery result containing all current valid
triggers plus its warnings. If a previously valid trigger becomes invalid,
remove that trigger's old schema from validation, completion, and hover.
Changes to valid neighbors still take effect. Do not retain the previous
whole registry because one trigger was skipped, even when every current
trigger is invalid. Baseline, extension, and authored-schema handling keep
their separate existing contracts.

Publish discovery findings as warning diagnostics on disk trigger files,
including those outside the workspace and in prepared scopes with no open
Markdown document. Preserve available cause locations as related
information. Keep each scope's findings independently, but publish each
source warning once; one healthy scope must not clear a warning still
reported by another. Clear it when the source is repaired, deleted, or no
longer contributes a warning to any prepared scope. Repeated unchanged
refreshes must not repeat warning logs or diagnostic publications.

Direct authoring checks of an open trigger remain strict and may report
syntax or schema errors on that file. Those errors do not turn its
recoverable discovery warning into a document preparation failure. Keep
protocol stdout free of human-readable output.

### Preserve actual schema enforcement

This change applies to automatically discovered invalid triggers. An
explicit document `$schema` that cannot load remains an error. A valid
trigger whose schema rejects document values still produces the existing
validation failure. Baseline behavior and the trigger language remain
unchanged. No flag is required to obtain the new warning-and-skip behavior.

Referencing a trigger envelope explicitly through a document's `$schema`
remains invalid, even if automatic discovery already warned about that
same file. Recovery changes discovery ownership, not what a valid schema
reference or trigger declaration means.

## Design Decisions

### Recover at the automatic discovery boundary

- **Decision:** Convert individual envelope and payload failures into warnings
  in automatic discovery while keeping passive parsers strict. Retain errors
  for root/bundle infrastructure and unreadable files that cannot be classified.
- **Alternatives considered:**
  - Relax the parsers.
  - Ignore every discovery error.
  - Recover only in Claudine.
- **Why:** Existing classification deliberately ignores unrelated YAML. Shared
  discovery gives every host the same valid registry without hiding broken roots
  or weakening explicit schema enforcement.
- **Downsides:** Some I/O failures still abort discovery; a caller must distinguish
  recoverable trigger findings from infrastructure errors.

### Validate and install one complete trigger at a time

- **Decision:** Require a valid envelope, fully resolved imports, merge-compatible
  payload, and compilable validator before installation. A skipped trigger keeps
  its filename reserved, including between embedded bundles.
- **Alternatives considered:**
  - Resolve only matched triggers.
  - Catch failures while compiling the final merged schema.
  - Activate a later same-named fallback.
- **Why:** Current loading checks unmatched payloads, and shadowing expresses the
  nearer file's ownership. Final compilation can hide a bad property behind a
  later override or lose which trigger caused it.
- **Downsides:** Individually checking raw payloads may add cold-load work. Cache
  reuse is required, and a broken local override can suppress a valid embedded
  schema until repaired.

### Keep a typed warning with the trigger's identity

- **Decision:** Use `dm.schema.invalid_trigger` across hosts, preserving the typed
  cause, trigger identity, and separate cause location. Report the first load
  failure for each trigger rather than collecting every internal defect.
- **Alternatives considered:**
  - Log a string.
  - Reuse a fatal schema-load code.
  - Point the warning at the prompt.
  - Add a parser that collects all errors.
- **Why:** The trigger owns the defect. Existing report and editor contracts need
  shared codes and accurate locations, and strict loaders already stop at the
  first error.
- **Downsides:** Repairing one defect may reveal another on the next refresh;
  embedded or unlocated findings require a descriptive message rather than an
  exact clickable location.

### Deliver findings even without a successful effective schema

- **Decision:** Carry discovery findings independently of an optional effective
  schema and retain them on subsequent failure paths in Darkmatter and Claudine.
- **Alternatives considered:**
  - Attach warnings only to successful validation reports.
  - Print directly during loading.
- **Why:** All-invalid discovery can legitimately yield no effective schema, and
  Claudine checks schemas before composition. Success-only storage would lose
  warnings in both cases.
- **Downsides:** Host result and failure carriers may need changes; returning
  `None` alone is insufficient to represent discovery's findings.

### Preserve report streams and explicit warning policy

- **Decision:** Ordinary CLI output warns on stderr; existing JSON reports carry
  warnings without repeating them on stderr. Honor explicit composition `--deny`
  selections and existing discovery opt-outs while making default recovery automatic.
- **Alternatives considered:**
  - Always print stderr warnings, including report mode.
  - Exempt this warning from `--deny`.
  - Introduce a new recovery flag.
- **Why:** Darkmatter's report contract forbids repeated diagnostic rows, and its
  policy catalog governs every coded composition warning. Automatic recovery
  needs no extra flag.
- **Downsides:** An explicit strict policy can stop an otherwise valid document;
  scripts must read structured warnings in report mode. This is an intentional
  qualification of the draft's unconditional success wording.

### Deduplicate display within one invocation

- **Decision:** Keep all findings on discovery/cache results and deduplicate at
  the invocation's delivery boundary by full source identity. A multi-file report
  attaches a shared warning to the first input that discovers it.
- **Alternatives considered:**
  - Deduplicate globally for the process.
  - Deduplicate by filename or payload.
  - Repeat a shared warning for every input.
- **Why:** Repeated passes and retries must stay quiet without hiding a warning
  on a later invocation or collapsing different triggers into one.
- **Downsides:** A multi-file report's later entries are not independent warning
  inventories; consumers must inspect the complete report. If contexts produce
  different causes for the same trigger, the first observed cause is displayed.

### Observe failed loads so repairs can recover

- **Decision:** Preserve dependencies and attempted lookup locations from failed
  trigger loads, including on first discovery. Invalidate skipped results through
  existing freshness checks, file events, and rescans, without polling.
- **Alternatives considered:**
  - Watch only accepted triggers.
  - Keep failures forever.
  - Rescan and resolve all triggers on every editor request.
- **Why:** Current caches distinguish existing dependencies from absent candidates.
  Losing either on failure prevents a repaired import or newly created payload
  from activating the trigger.
- **Downsides:** Failed resolution must expose partial read/search observations;
  existing recursive-search limits still require a rescan for some creations.

### Replace DMLS's previous registry with current valid triggers

- **Decision:** Treat skipped-trigger discovery as successful preparation and
  install its valid subset. Publish source warnings from per-scope findings,
  clearing them only when no prepared scope still reports them.
- **Alternatives considered:**
  - Keep the entire previous registry.
  - Keep the previous version of only the broken trigger.
  - Drop every trigger on one failure.
- **Why:** Keeping a broken trigger's old schema would contradict "ignored" and
  diverge from the CLI. The documented previous-registry behavior must deliberately
  change for this recoverable case, while unrelated baseline/document failures
  retain their own behavior.
- **Downsides:** Temporarily broken saved triggers lose their editor assistance
  until repaired; diagnostic ownership must handle shared sources across scopes.

### Verify through existing host paths without expanding the feature

- **Decision:** Cover the discovery boundary and each existing consuming host with
  focused unit and ordinary CLI tests (L1), a passive shipped-schema check, and
  normal invocation tests. Keep existing report formats and discovery consumers;
  no performance study is required.
- **Alternatives considered:**
  - Test only the original Claudine incident.
  - Add new inspection formats.
  - Add a cross-platform performance campaign.
- **Why:** The risk is lost findings, stale caches, and inconsistent host behavior;
  existing caches and targeted correctness tests address it directly.
- **Downsides:** Cold validator-check overhead has no new timing target; normal
  invocation tests must cover several packages.

## Implementation surfaces

Start with these existing owners; implementation may choose the smallest
supporting type changes that satisfy the contracts above:

| Package | Source and purpose |
| --- | --- |
| darkmatter | [TriggerRegistry and discovery](../../lib/src/markdown/schemas/triggers/discovery.rs) load disk and embedded triggers and must retain skipped-source findings. |
| darkmatter | [Payload assembly](../../lib/src/markdown/schemas/triggers/assemble.rs) resolves and caches payloads; preserve failed-load observations and reject invalid payloads before installation. |
| darkmatter | [DarkmatterSchemas](../../lib/src/markdown/schemas/mod.rs) assembles and validates effective schemas; expose discovery findings even when no schema applies. |
| darkmatter | [Composition schema validation](../../lib/src/markdown/compose/schema_validation.rs) delivers findings into normal composition and failure reporting. |
| darkmatter-cli | [Schema validation](../../cli/src/commands/schema/validate.rs), [assignment preparation](../../cli/src/commands/schema/assignment.rs), and [trigger inspection](../../cli/src/commands/schema/triggers.rs) must retain warnings through their existing output paths. |
| darkmatter | [Clean schema context](../../lib/src/markdown/schemas/clean.rs) runs discovery for `md clean` and DMLS formatting; expose its findings to both. |
| darkmatter-cli | [`md clean`](../../cli/src/commands/clean.rs) and its [JSON envelope](../../cli/src/commands/clean/json.rs) deliver warnings on stderr or in the additive `warnings` array. |
| dmls | [Formatting](../../dmls/src/providers/formatting.rs) formats despite skipped triggers and adds no warning text. |
| dmls | [Trigger registry cache](../../dmls/src/overlay/registry.rs) prepares and installs registries; adopt valid subsets, watch failed-load inputs, and update diagnostic transitions. |
| claudine | [Effective-schema loading](../../../claudine/lib/src/composition/schema/mod.rs) supplies launch checks; carry findings without translating them into schema-load errors. |
| claudine-cli | [Composition runner](../../../claudine/cli/src/commands/wrap/composition/runner.rs) participates in ordinary prompt launch; deliver warnings once alongside existing host warnings. |

Integrate warning delivery into the normal `md`, Claudine, and DMLS paths.
In particular, Claudine's effective-schema loading must not translate these
recoverable findings into fatal schema-load failures. Reuse the shared
discovery behavior rather than adding a Claudine-only catch-and-ignore path.

## Acceptance and verification

1. Reproduce the `memory.yaml` example in an isolated repository fixture.
   Compose a prompt with no `$schema` through the normal CLI path: it
   succeeds, stderr names the ignored trigger and reason, and stdout
   contains the expected composed document.
2. Place an invalid trigger beside a valid matching trigger. The valid
   schema still applies; invalid document values still fail validation.
   Repeat with the invalid trigger before and after it in filename order.
3. Exercise malformed claimed YAML, envelope/condition errors, invalid
   payloads and imports, missing payload files, cycles, and embedded
   failures. Each invalid trigger is omitted without a partial payload;
   valid neighbors remain active. Include raw JSON Schema without `$defs`
   that passes the object-shape gate but fails compilation, even when a
   later declaration would replace its bad property. Direct passive parser
   calls must still return their existing typed errors.
4. Cover all-invalid discovery, matcher-only triggers, silently ignored
   ordinary YAML (including malformed unclaimed YAML), and unchanged
   same-filename shadowing after a skip, across disk roots and embedded
   bundles. All-invalid discovery without a baseline or authored schema
   still returns warnings and no effective schema. Infrastructure failures
   remain errors rather than successful empty discovery.
5. Repeated preparation and cached discovery preserve the warning while
   displaying it once per source per invocation. Repairing the invalid
   file, repairing an imported file, or creating a missing payload and its
   parent folder activates it on refresh, including after an all-invalid
   first load. Deleting the invalid trigger clears its warning and allows
   the normal shadowing rules to run again. Distinct paths with the same
   filename, and two triggers sharing a broken payload, stay distinct.
6. Verify `md schema validate`, `md schema triggers`, Claudine prompt
   preparation/launch with a fixture provider, and DMLS all share the
   behavior. Cover assignment preparation and existing commands that use
   composition. Keep machine-readable output valid and protocol stdout clean;
   report mode does not repeat warning rows on stderr. Cover multi-file
   reports, repeated Claudine preparation, and warnings retained before an
   independent schema or composition failure. A new invocation using cached
   discovery reports its warning again.
7. Prove explicit `$schema` load failures and validation failures from
   valid triggers remain errors. Include a passive shipped-schema corpus
   check alongside the CLI regressions.
8. Save a formerly valid DMLS trigger in an invalid state while changing a
   valid neighbor. Validation, completion, and hover lose the invalid
   trigger's properties and reflect the valid neighbor's changes. Test
   first-load all-invalid discovery, multiple simultaneous warnings,
   source warnings shared by scopes, prepared scopes without open documents,
   repair/deletion clearing, and unchanged refreshes without repeated
   publication. Preserve strict diagnostics while directly editing a trigger.
9. Default composition succeeds with these warnings. Explicit `--deny` of
   the code and `--deny all` stop composition through the existing policy
   path before its next effect, without changing the warning's severity or
   attributing it to the prompt. Disabling triggers emits no discovery warning.
   An authored `$schema` referencing the same ignored file still fails.
10. `md clean` beside an invalid trigger succeeds and warns on stderr;
    `md clean --json` reports the warning in its top-level `warnings` array
    with clean stderr and an otherwise unchanged version-1 envelope.
    `--disable schema-triggers` emits no warning. DMLS formatting returns
    its edits with no added warning message.

Use the affected package-area `just test` and `just lint` recipes with
nextest. Use the repository's shared test toolkit and isolated repositories
with controlled home/schema roots; do not launch a real provider or change
the author's incident file. These checks require no focused terminal or
browser windows and must work on macOS, Linux, native Windows, and WSL2.

When implementing, update the schema definition, targeting, and trigger
topic pages; DMLS indexing/failure and schema-support documentation;
Claudine's schema and composition documentation; and the relevant skills
and discovery comments. Replace the whole-scan failure and previous-registry
contracts only for invalid individual triggers. Update CLI policy and
structured-report references with the warning code, source attribution,
and multi-file delivery rule, and document `md clean`'s `warnings` array
in its CLI reference. Remove the existing planned marker from user
documentation when the behavior lands. READMEs need changes only if their
overview promises the old behavior; command references stay in `docs/`.
This specification records the requested fix; it does not implement it.
