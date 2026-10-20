---
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
reviewed_on: 2026-10-18
review_iterations: 1
completed: true
area: darkmatter
status: finalized-spec
created: 2026-10-18
owner: Ken Snyder <ken@ken.net>
origin: 2026-10-18-dmls-cache-optimization
packages:
    - darkmatter
implemented: true
implemented_by: "claude/opus"
---

# DMLS cache optimization follow-ups

## Purpose

When a newly created local file takes precedence over a previously selected
schema dependency, the next affected registry preparation must select that
new file. In an editor session, a supported file-change event must also cause
that preparation to happen. An unchanged registry must keep its existing
fast lookup behavior.

This follows the newly shadowing file observation in the first implementation
review of `2026-10-18-dmls-cache-optimization`. That review reported no
implementation findings; this is a separate correctness repair for a preexisting
limitation, not a reopening of that optimization's completed review.

## Defect and concrete example

The Darkmatter library's
[`file_payload_memo`](../../lib/src/markdown/schemas/triggers/assemble.rs)
keeps up to 32 successfully resolved trigger payloads. Its
`inputs_fingerprint` checks dependency bytes, schema-root directory listings,
and the folders containing selected dependencies before reusing a payload.
It does not record every earlier location that resolution tried and found
absent.

For example, a trigger under `schemas/` selects `schemas/payload.yaml`, whose
named-type import references `Fields@shared/types.yaml`:

```text
repo/
  schemas/
    prompt.trigger.yaml
    payload.yaml
    shared/                 # initially absent
  shared/
    types.yaml              # initially selected
```

An implicit path first tries the authoring folder and then the repository
root. Initially the import selects `repo/shared/types.yaml`. Creating
`repo/schemas/shared/types.yaml` should make that nearer file win. Its parent
is neither a schema root nor the folder of the previously selected dependency,
so the cached payload can remain unchanged even after another scan.

The existing library test for a bare name added to an earlier schema root
already passes: those root listings are tracked. Repeating that fixture alone
would not prove this repair.

There is a second layer in the `dmls` package. Its
[`TriggerRegistries`](../../dmls/src/overlay/registry.rs) records schema
roots and selected dependency folders to decide which registries need a
refresh and which locations to watch. Fixing only the library fingerprint
leaves a ready editor registry unaware of the new location.

## Scope and existing contracts

Cover successful file-backed trigger payloads and all their nonrecursive
local references: top-level schema references, delegated schemas, transitive
named-type imports, and example files. Include every supported local reference
form with ordered candidates, not just bare names. Explicit `./` and `../`
references retain their single authoring location; they gain no fallback.
Recursive `%` references require the separate decision under Open Questions.

Preserve these contracts:

- The Darkmatter library owns schema resolution. The `dmls` package consumes
  its dependency information rather than rebuilding resolution rules.
- All references use the existing captured file-resolution context and
  `biscuit-file` resolution APIs. Do not read process environment, home, or
  working directory again, and do not walk arbitrary ancestor folders.
- Schema-root precedence, typed resolution errors, passive editor analysis,
  embedded bundle isolation, and deny-all remote behavior remain unchanged.
- Keep the existing cache key's context distinctions, capacity of 32,
  least-recently-used eviction, successful-results-only caching, and shared
  schema identity on unchanged hits. Embedded payloads gain no disk watches.
- Ready DMLS lookups continue to do no I/O. Preparation stays on the existing
  registry worker, with existing change replay and stale-result installation
  rules. No worker or scheduling redesign is included.
- No persistent index-store format change, new configuration, dependency,
  logging payload, or cross-platform resolution change is needed.

**Reader's note:** the original draft proposed tracking searched folders only
inside the payload cache. The design below also carries that information to
DMLS invalidation and watching. These are two independent decisions to reuse
old data; both must observe the new file for the editor behavior to change.

## Design decisions

### Record search dependencies during resolution

Extend the dependency information produced by the Darkmatter library's
[`ResolvedSchema`](../../lib/src/markdown/schemas/resolve.rs), which carries
resolved schemas and their source dependencies, and propagate it through
[`ResolvedPayload`](../../lib/src/markdown/schemas/triggers/assemble.rs),
which carries a trigger's prepared schema, into
[`TriggerRegistry`](../../lib/src/markdown/schemas/triggers/discovery.rs).
Expose only the search information DMLS needs; keep observation details internal
where possible. Keep searched locations distinct from the existing dependency
file list, whose consumers read and hash file contents.

Record the candidate locations actually attempted through the selected file,
including unsuccessful earlier candidates. Merge the records across nested
schema delegation, imports, and examples, including when an intermediate
resolver reuses work within a resolution. Do not derive this information only
from the final selected file's parent. Sort and deduplicate collected dependency
records after resolution without changing candidate evaluation order.

Use `biscuit-file`'s
[`FileReference`](../../../biscuit-file/lib/src/file_reference/mod.rs)
detailed resolution results or its candidate plan to obtain the existing
candidate order and context. Darkmatter's bare-name schema-root loop must
contribute its attempted roots as well. Do not create a second parser or
candidate-search algorithm. A failed probe that currently stops resolution
must still stop it, rather than silently trying a later candidate.

Store absent paths lexically using `biscuit-file` path identity; they cannot
be canonicalized. Keep their authored candidate locations even when a selected
file also has a canonical identity. Continue using simplified canonical paths
for existing dependency files where the current contract requires them.

### Check searched locations before a payload-cache hit

Retain dependency-content hashes and current schema-root checks. Add the
recorded candidate locations and their parent-folder observations to the
fingerprint, including absent candidate parents. A candidate becoming a regular
file, disappearing, or changing between directory and file must force a miss,
even if a parent listing's entry names stay the same. Directory observations
must be sorted and deduplicated; no recursive workspace scan is needed for
nonrecursive references.

Missing locations are expected observations, distinct from readable empty
folders. A failed or partial directory read cannot validate a cache hit:
resolve again through the normal resolver, preserving its typed errors.
Continue to use `biscuit-hash` for fingerprints. Compute filesystem observations
outside the process-wide cache mutex, as today. This does not introduce an
atomic filesystem snapshot guarantee.

An unchanged hit returns the same shared schema object. A newly shadowing
file that resolves to an invalid schema must produce the normal preparation
failure, not be hidden by a library memo hit. DMLS may then apply its existing
last-good-registry policy; this repair does not change that policy.

### Carry search dependencies into editor invalidation and watching

Include searched locations in the `dmls` package's
[`Dependencies`](../../dmls/src/overlay/registry.rs), which records each
prepared registry's inputs. Creation, deletion, replacement, or rename affecting
an earlier candidate must mark the affected scope stale. Handle directory-path
events as well as events for a file inside a tracked folder. Preserve the
current byte comparison for known dependency-file changes.

Extend the existing
[schema-location watch registration](../../dmls/src/workspace/watch.rs)
using these dependencies. For an absent parent such as `schemas/shared/`,
watch the nearest existing ancestor needed to notice its creation; after that
event, preparation must advance the watch coverage to the candidate folder.
A tool that creates the folder and its file together must also be detected.
Do not require a restart or an unrelated schema edit. Keep the watch set bounded
by recorded search paths, including external roots, rather than watching the
whole workspace recursively. Broader invalidation on a relevant directory
event is acceptable; unrelated file events must not invalidate every registry.

Carry these dependencies through the existing preparation/install change
replay. A relevant event arriving while preparation is in flight must leave
the installed result stale or trigger a follow-up preparation, as it does for
existing dependencies. Do not lose the new watch locations when a failed
preparation retains the last-good registry and later needs to recover.

Keep fallback watch mode's existing guarantees: a server rescan must rebuild
from the new candidate state, and a supported save notification for a candidate
must invalidate it through the same dependency rules. Do not promise immediate
updates for external changes when the client provides no file events.

## Acceptance criteria and verification

Use deterministic L1 library and protocol tests with temporary repositories.
No terminal or browser interaction is required. Extend existing fixtures and
work counters rather than relying on sleeps, elapsed-time thresholds, or
process-global cache clearing that could interfere with parallel tests.

1. Resolve the concrete implicit-path import example, then repeat unchanged.
   Confirm a cache hit and the same shared schema object. Create the nearer
   file in an initially absent folder; the next scan must miss, select its
   distinct fields, and report the new dependency. Delete it and verify
   fallback to the original file.
2. Cover the same precedence change for a payload path, a transitive import,
   and an example file. Retain the existing bare-name root-shadowing test as
   a control. Check that explicit relative references do not acquire fallback.
3. Cover an already-existing candidate parent and a candidate changing from
   directory to regular file without changing its entry name. Confirm failed
   observations do not count as valid hits using existing filesystem test
   seams where available; avoid permission-bit tests that behave differently
   under elevated users or native Windows.
4. In a DMLS session, begin with a ready registry, create the nearer folder and
   file, and deliver the relevant watched event. Assert watch coverage and
   changed schema behavior without an explicit test-only invalidation. Also
   cover removal, a supported save notification, and rescan in fallback mode.
5. Hold preparation with the existing test gate, deliver a relevant creation
   event, and verify change replay prevents a stale response. A malformed
   nearer file must follow last-good/error policy and recover after repair.
6. Verify unchanged and unrelated inputs retain cache hits and ready lookups
   perform no reads, hashes, or payload resolution. Existing context-isolation,
   capacity, eviction, and embedded-schema tests must remain green.

At implementation time, run `just test` and `just lint` from `darkmatter/`.
Tests must use portable temporary paths and repository fixtures and compile
on macOS, Linux, native Windows, and WSL2. No new CI matrix or gate is requested.

No performance spike is required. This repair addresses a reproducible stale
result; deterministic work counts and identity checks protect the existing
optimization. It adds observations only for the reference candidates that
resolution already considered, without a workspace-wide scan.

## Documentation and readiness

When implementation lands, update the payload-sharing and registry-change
sections of [DMLS indexing](../../docs/lsp/indexing.md) with the covered
search locations, absent-folder handling, and fallback-mode limits. Update
the relevant Darkmatter skill guidance if dependency propagation changes.
Review comments on the changed resolver, cache, and registry structures: the
current claim that root and selected-folder listings cover every searched
directory is broader than the implementation, so replace it with the actual
coverage. Keep this spec as the decision snapshot; current topic documentation
must not link back to it.

Success means the library regression and editor event path both select the
new dependency while unchanged inputs retain the optimized reuse behavior.
The agent's completion state is implementation complete, ready for review;
only the author moves this fix into the completed lifecycle directory.

## Open Questions

### Frontmatter status needs author reconciliation

The existing `status: unscheduled` is preserved as requested, but it is not
in the newly requested status enum. Before validating this document against
that schema, the author should change the status to `draft-spec` (recommended
for this draft; loses the explicit scheduling state), retain `unscheduled`
and expand the enum (preserves that state; changes the requested schema), or
track scheduling separately and use `draft-spec` (separates the two meanings;
adds metadata). This review leaves the existing value and requested enum intact.

### Should recursive local references be repaired in the same change?

`biscuit-file` reports recursive `%` candidates as search roots rather than
individual file probes. A listing of those roots alone cannot detect a new
matching file several folders below them. The nonrecursive design above must
not be described as solving recursive shadowing.

- **Defer recursive search tracking to a separate repair (recommended).**
  Pros: keeps this change focused on the demonstrated defect and avoids adding
  recursive walks to the cache-hit path. Cons: recursive payload references
  retain their existing search-invalidation limitation, which the indexing
  documentation must state explicitly until repaired. This is recommended
  because recursive traversal and watch coverage need their own bounded design.
- **Include complete recursive search tracking now.** Pros: offers one
  invalidation contract for every local reference. Cons: requires recording
  descendant searches and defining recursive watch coverage, and could make
  cache validation substantially more expensive. The author would need to
  approve that expanded scope before implementation.
- **Bypass payload memoization for recursive dependencies.** Pros: avoids this
  library-cache reuse problem without a recursive fingerprint. Cons: loses
  payload reuse for those schemas and still needs a separate editor watch and
  invalidation solution; bypassing the library memo alone is insufficient.
