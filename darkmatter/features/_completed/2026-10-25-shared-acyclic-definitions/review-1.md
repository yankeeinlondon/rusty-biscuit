---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Blanket exclusions leave compatible structured definitions unshared"
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-26T23:29:57-07:00"
spec: 2026-10-25-shared-acyclic-definitions/spec.md
implemented: true
description: "A **feature** review of `2026-10-25-shared-acyclic-definitions/spec.md`"
feature: 2026-10-25-shared-acyclic-definitions/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/features/2026-10-25-shared-acyclic-definitions/log.md
implemented_by: claude/opus
next: 2026-10-25-shared-acyclic-definitions/review-2.md
---

# Review 1

The feature is **not production ready** because automatic sharing excludes
compatible uses that the spec requests. The ordinary sharing path, metadata
extraction, editor integration, and lower byte budget otherwise passed the
focused checks below. This is the first review; there are no previous findings.

## Implementation decisions

Two independent agents read the spec and implementation log before the code
review. Both found `decisions_for_author: []` and no decision requiring human
review. The previously open hover question was settled during implementation.

The implementation chose to expand shared bodies at their first use, preserving
the import stack; decide compatibility during the second pass; inline unsupported
alias chains; compare metadata through the definitions references reach; keep
stored objects named in hover while listing union alternatives; and compare
structural types through references. These choices have recorded reasoning and
need no reaffirmation.

The measured maximum of 768,262 bytes gives a fixed 1,000,000-byte budget under
the spec's formula. Accepting the bounded 7,514-byte increase in verb-hover
allocation is reasonable: the log attributes it to opening the displayed union
alternatives, while diagnostics and overall allocations fell substantially.
Remaining copied bundles follow authored-reference counting and the union-arm
fallback. Temporary measurement scaffolding was removed as requested.

Two broader exclusions do narrow requested scope: all global catalog declarations
resolve with sharing disabled, and every type containing `file(match(...))`
stays inline even outside a root union. These are repair work, not questions for
the author. The review's decision and its alternatives are recorded under
`Review 1 / Design Decisions` in the implementation log.

## Findings

### R1-F1 (high): Blanket exclusions leave compatible structured definitions unshared

**Authority — spec:** Section A says a named type is selected when its resolved
shape is structured, it has two distinct authored uses, and it is acyclic.
Its resolution rules require: “If it cannot, inline that one use,” and “Selection
is fixed for the resolution operation.” The consumer contract says “All consumers
must treat a new acyclic reference like the value it replaces.” Acceptance groups
1 and 4 require automatic selection and consumer parity. The review instructions
classify narrowed requested scope as high.

**Defect class:** context-free exclusions disable sharing for an entire loader or
type instead of limiting fallback to uses whose existing semantics require it.

In Darkmatter, the catalog resolution routes select `Sharing::Off` in
[resolve.rs](../../lib/src/markdown/schemas/resolve.rs:263) and
[the embedded catalog route](../../lib/src/markdown/schemas/resolve.rs:1322).
The switch prevents every selected acyclic definition from being stored,
regardless of its actual uses. The log explains that existing catalog consumers
expect expanded shapes; that is the consumer adaptation left unimplemented.
Catalog declarations already expose a definition table, but it remains empty for
these repeated acyclic helpers. The baseline projection returns only the copied
shape, so the exclusion also reaches the baseline used by effective schemas.

Separately, [is_shareable](../../lib/src/markdown/schemas/resolve/recursive.rs:658)
rejects every expansion containing a file-match constraint. This preserves a
real root-union distinction, but the predicate has no use context: an ordinary
single-object schema also loses sharing. Both ordinary root lowering and stored
body lowering use suggesting mode in
[convert.rs](../../lib/src/markdown/schemas/simplified/convert.rs:115), so the
root-union reason does not justify disabling these ordinary uses.

#### Reproduction

The catalog probe uses this library, based on the existing two-sites-in-one-body
selection fixture:

```yaml
kind: schema
types:
    entry: "{ n: string }"
    group: "{ first: entry, second: entry }"
```

Resolving `fm: group@./types.yaml` through `resolve_yaml_schema` stores one
`entry` definition. Loading a catalog containing `$schema: { fm:
"group@./types.yaml" }` through either `GlobalCatalog::from_file` or
`GlobalCatalog::from_documents` stores zero. The same two authored sites are
reachable within one resolution in all three cases; no cross-global pooling is
requested.

For the second probe, resolve this ordinary schema through `resolve_yaml_schema`:

```yaml
entry: "{ path: file(match('*.md')) }"
a: entry
b: entry
```

It emits no definition. Removing only `match('*.md')` emits one definition with
two references. Repeating the edit inside the five container shapes already
used by the implementation's root-union regression fixture gives the same result.
Additional probes checked arrays of objects and tuple spreads with the same
single edit. The probes ran as temporary tests inside the declared L1 test binary and were
removed after recording the results; production code was not modified.

#### Instance sweep and repair ledger

Counts below are public definition-table sizes. Each positive control uses the
same authored references. File-match controls change only `file(match('*.md'))`
to `file`.

| Child ID | Site and shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Disk catalog declaration, `group` containing two `entry` uses | 0 definitions; ordinary resolver control has 1 | Store `entry` once and preserve catalog consumers |
| R1-F1.2 | Embedded catalog declaration, identical virtual files | 0 definitions; ordinary resolver control has 1 | Same selection as disk and ordinary resolution |
| R1-F1.3 | Ordinary schema, `{ path: FILE }` | plain file: 1; matched file: 0 | 1 for both, with unchanged file suggestions and validation |
| R1-F1.4 | Ordinary schema, `{ inner: { path: FILE }[] }` | plain: 1; matched: 0 | 1 for both |
| R1-F1.5 | Ordinary schema, `{ <pattern::^n>: FILE }` | plain: 1; matched: 0 | 1 for both |
| R1-F1.6 | Ordinary schema, `tuple([FILE, number])` | plain: 1; matched: 0 | 1 for both |
| R1-F1.7 | Ordinary schema, `FILE[] | { n: number }` | plain: 1; matched: 0 | 1 for both |
| R1-F1.8 | Ordinary schema, `{ path: FILE }[]` | plain: 1; matched: 0 | 1 for both |
| R1-F1.9 | Ordinary schema, `tuple([string, ...FILE[]])` | plain: 1; matched: 0 | 1 for both |
| R1-F1.10 | Catalog result consumed by `GlobalDeclaration::definitions` and `GlobalCatalog::baseline` | catalog table empty; baseline contains copied child objects | Retain sharing through the effective-baseline path without dangling references |

The same sweep checked these unaffected decisions and compatibility cases:

| Site | Shape or invariant checked | Result |
| --- | --- | --- |
| Graph selection | Repeated object/dictionary/tuple/structured array/union; single use; primitive-only shapes; root/body revisit; namespaces and callable-only references | Existing selection tests pass; counts and identity match the spec |
| Ordinary disk, embedded trigger, reserved import | Repeated structured imports | Existing consumer tests pass and produce definitions |
| Per-use constraints and alias fallback | Added key bound; changed excluded keys; all uses falling back; alias chain | Existing tests pass, including unchanged ordinary use and no unused definition |
| Root-union file-match contexts | The implementation's five original container shapes | Existing public validation tests pass: fallback keeps the previous suggesting behavior; preserve this during repair |
| Other body and union compatibility checks | Nullable union; optional file union; multiple suggestions; default conflict; callable value | Existing tests preserve optionality, the empty-file sentinel, and typed rejection; these are required compatibility cases rather than a request to force references everywhere |
| Definition publication and merges | Reachability pruning, baseline/trigger/document and root-union collisions, inter-definition references | Existing consumer tests pass; fix the earlier exclusions rather than adding another merge-time sharing pass |

**Repair:** complete sharing for the compatible uses in the ledger and add public
result tests that distinguish those uses from the incompatible contexts. Preserve
root-union file-match behavior and adapt catalog/baseline consumers to carry the
definitions they read. Do not simply delete the guards: the root-union regression
and definition-free convenience APIs need deliberate handling. Catalog propagation
can be larger than the immediate byte saving, but dropping that requested scope
is not an available repair. Keep any compatibility projection at the API boundary
that needs it rather than disabling the entire resolver operation. Update docs and
skills that currently describe the blanket exclusions as permanent rules.

## Observations

None.

## Verification and limits

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Automatic selection, local constraints, depth, cycle safety, deterministic output | L1 public resolver results; 43 focused sharing tests passed. Temporary negative probes reproduced the finding and its seven file-match shapes. |
| Validation positions, launch/completion rules, coercion, eager files, deferred values, merge identity and dependency invalidation | L1 public consumer tests in `shared_acyclic_definitions` and `shared_acyclic_consumers`; passed. |
| CLI validation and editor diagnostics, completion, hover, navigation and refresh at both uses | L1 shipped CLI and DMLS protocol requests, included in `just test shared`; 150 tests passed. No terminal input or visual styling contract is introduced. |
| Claudine metadata through either trigger and normal launch, both hash forms, bundle inventory, lifecycle/profile acceptance, byte budget | L1 `just test claudine_schemas`; 799 tests passed, including the new 1 MB budget. |
| Frozen representation corpus | L1 `just test recursive_schema_golden`; 17 tests passed without blessing. Frozen input files and manifest have no diff from the recorded baseline. |
| Shipped schema artifacts | L1 `just test shipped_schema_corpus`; 9 tests passed. |
| Existing terminal behavior and lint | The implementation log records both areas' full L1, L2 and lint gates passing. This review did not repeat terminal suites; the changed feature promises no keyboard, emulator, or browser behavior requiring a new higher-level test. |

New test modules are declared in the consolidated L1 binary, and the targeted
runs demonstrate they are selected. `just check-tier-coverage darkmatter` found
no stranded tests. The Input Robustness Matrix is inapplicable:
this change resolves a text grammar and changes schema representation; it adds no
configuration field validator or document loader. The new catalog wrapper selects
an expansion mode and reuses the existing loader.

The broad substring run also selected an unchanged content-policy baseline test
that took 5.104 seconds while the Claudine run was active. It is outside this
feature's changed tests; this concurrent measurement does not establish a new
feature-related timeout. No cross-platform proof is claimed or required for this
review. No commits, formatting, or implementation repairs were performed.
