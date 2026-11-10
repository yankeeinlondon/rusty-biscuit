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
kind: feature
created: 2026-11-04
status: finalized-spec
needs_rulings: false
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
clarified: true
implemented: true
implemented_by: "claude/opus"
review_iterations: 4
completed: true
related:
    - 2026-10-28-extract-simplified-schema
    - 2026-11-04-composition-errors
depends-on:
    - 2026-11-04-loading-data
    - 2026-11-05-render-surface
---

# Dynamic Schema Types

Let the expression engine take part in defining a schema. At minimum,
`enum(...)` members and `suggest(...)` candidates must be computable from an
expression. The goal is **broad** support: anywhere a schema author writes a
type expression should eventually accept a `{{ … }}` expression.

The investigations are recorded under "Investigation outcomes", and every
question they raised is settled by Rulings Q8–Q20 (2026-11-08). Every author
decision under Rulings is in force; where a later ruling corrects an earlier
one, the later ruling governs. Its main job is to determine
how dynamic types can run safely with good performance. Narrowing where
expressions may appear is the **last resort**. It is considered only if the spikes show that
no caching or resolution strategy keeps validation and DMLS responsive.

## Motivation

A schema often repeats a list that already lives in a data file. The
motivating case is `claudine/docs/research/sandboxing/providers/sandboxing.yaml`:

```yaml
types:
    os: enum({{ as_csv(claudine/docs/os.yaml) }}; required)
```

Today this fails when the schema loads:

```text
invalid SimplifiedSchema for property os:
expected `,`, `;`, or `)` in enum members, found `a`
```

That file is committed, so the shipped-schema corpus test
`every_kinded_schema_artifact_passes_its_definition_through_the_registry`
currently excludes it. The carve-out fails once the file validates; this
feature removes it when passive template admission works.

In the simplified-schema package,
[`parse_enum_members`](../../../simplified-schema/lib/src/simplified/grammar.rs)
parses enum members as strings,
and schema handling is deliberately passive: parsing, validation, trigger
matching, completion, and hover perform no I/O and evaluate no expressions.
With 2026-11-04-loading-data, the intended spelling becomes:

```yaml
types:
    os: enum({{ load_data("../../../os.yaml", "string[]") }}; required)
```

The replacement uses `../../../os.yaml` because the schema is in
`claudine/docs/research/sandboxing/providers/`. A bare
`claudine/docs/os.yaml` would be relative to that folder, not the repository
root. The implementation updates the shipped schema to this spelling.

## The tension to resolve

Four existing guarantees push against dynamic types. The spikes must keep
each one or replace it with something equally strong:

1. **Passive schemas.** Schema parsing and validation do not execute anything.
   A schema found by trigger discovery — even in DMLS, just by opening a
   folder — must not be able to run code or touch the network unasked.
2. **Validator caching.** In the darkmatter package,
   [`DarkmatterSchemas`](../../lib/src/markdown/schemas/mod.rs) prepares
   validators through a
   process-wide `SHARED_VALIDATOR_CACHE` keyed on the schema and its resolution
   context. A long-running host (Claudine) composes many documents against a
   few schemas and relies on this.
3. **DMLS latency.** Completion, hover, and diagnostics run on every edit and
   must not wait on file reads or expression evaluation on the hot path.
4. **The extraction boundary.** 2026-10-28-extract-simplified-schema moved the
   schema language into the host-neutral `simplified-schema` library. That
   library owns the *passive* expression parser but has **no evaluator**:
   evaluation, function registration, and lookup stay in Darkmatter.

## Proposed model (to be validated by the spikes)

Split schema loading into a passive phase and an explicit, cached
**resolution** phase:

```mermaid
flowchart LR
    A[Schema source] -->|passive parse| B[Schema template<br/>static parts + expression slots]
    B -->|resolve: evaluate slots<br/>via host resolver| C[Concrete SimplifiedSchema]
    C -->|compile| D[Validator]
    B -. fingerprint .-> K[(Resolution cache<br/>key = schema hash + input hashes)]
    K -. hit .-> C
```

- **Parse stays passive.** The grammar accepts a `{{ … }}` slot where a member
  list (and later, other type positions) is allowed, checks its syntax, and
  records its span. Nothing is evaluated. Editor features keep working on the
  template.
- **Resolution is the only effectful step.** `simplified-schema` defines a
  host hook (a resolver trait). Darkmatter implements it with its evaluator.
  The result must have the shape the slot requires (defined below), or
  resolution fails with the slot's span. A factory that passively parses a
  template must not resolve it implicitly.
- **Resolution is cached by its inputs.** The cache key is the schema content
  hash plus a fingerprint of everything evaluation read (file content hashes
  via `biscuit-hash`, environment values, `ctx.*` groups). A warm hit avoids
  evaluation and recompilation; establishing that the
  recorded inputs are unchanged may still require I/O. DMLS does that work
  outside completion, hover, and diagnostic requests.
- **Evaluation context is the schema's, not the document's.** Relative paths
  resolve against the schema file. One schema then resolves once for every
  document that uses it within an equivalent schema context, preserving
  validator sharing. Imported definitions retain their defining file's context,
  not the importing schema's folder. Derive each context from the caller's
  captured request through biscuit-file's `FileReference` and context APIs;
  never recapture process state.

## Required semantics

These rules make the existing author decisions implementable. What may be
evaluated, and when, is governed by Ruling Q5; nothing here widens it.

### Slot boundaries and result values

A slot is one complete list element, for example
`enum(none, {{ load_data("../../../os.yaml", "string[]") }}; required)`.
Multiple slots and literal elements may alternate. Flatten each returned
array one level, preserving declaration order and array order. A scalar,
null result, mapping, or nested array is a positioned resolution error; an
empty returned array contributes no candidates. The final enum or suggestion
list must contain at least one candidate.

- Enum slots return **strings only**, matching the existing enum value
  domain. There is no implicit conversion of numbers, booleans, or null to
  enum members. Authors who want conversion request it in the expression or
  in `load_data`'s explicit contract.
- Suggestion slots return scalar candidates. Strings use the existing
  target-directed suggestion interpreter; native numbers use the shared
  numeric model and canonical numeric text, and booleans/null use their
  literal text. Interpret that text exactly as a literal candidate for the
  target type. Thus a string suggestion `true` remains a string, and an
  invalid numeric suggestion is retained for linting rather than becoming a
  schema-load error. Existing eligibility, one-list-per-property, union,
  array-item, and stricter `alphanumeric` rules still apply.
- Apply the author's duplicate decision after candidate interpretation:
  when a list contains slots, keep the first occurrence of each interpreted
  candidate, including collisions between slots. Leave duplicate handling
  for entirely static lists unchanged. This intentionally relaxes dynamic
  suggestion lists relative to the static duplicate-error contract.

**Reader's note:** requiring strings for dynamic enums corrects the earlier
“array of scalars” wording; it does not introduce typed enums. Broader slot
positions remain a goal. Returned values are inserted into the typed schema
representation, never concatenated into schema source and reparsed. A data
string containing commas, `; required`, or `{{ … }}` stays one literal value
and cannot introduce a constraint or another evaluation.

Reuse the simplified-schema package's
[expression scanner](../../../simplified-schema/lib/src/expression/scan.rs)
for quote- and brace-balanced slot boundaries. Quoted literal text containing
`{{` remains literal. Parsing records the expression and authored source
mapping, including quoted and block YAML carriers; it must not introduce a
second closing-delimiter rule.

### Passive templates and concrete snapshots

In simplified-schema, the
[`SimplifiedSchema` object](../../../simplified-schema/lib/src/schema/mod.rs)
is the caller-facing schema representation. Its factories must accept valid
unresolved templates without lowering them eagerly. Pure inspection and
SimplifiedSchema YAML export preserve expressions without evaluating them.
JSON Schema export, validator preparation, conversion, and layering that
requires lowering return a typed unresolved-schema error until explicit
resolution succeeds. A type query must expose pending information rather
than report a partial member list as complete.

The explicit resolver produces a separate immutable concrete snapshot; a
failed or canceled resolution never mutates the template or publishes a
partially resolved schema. Re-resolving starts from the template, not from
last time's substituted values. Calls that resolve another dynamic schema
(for example through a data contract) share the active resolution stack and
report a typed cycle error rather than recurse indefinitely or wait on their
own cache entry. Preserve the existing whole-property layer
precedence and resolve before Darkmatter merges lowered layers. Static
schemas retain their current fast path and validator sharing.

When resolving a named definition, retain its origin through imports and
aliases. Reject `fm.*` and other document/runtime-only
bindings (including lifecycle state) passively in every expression branch,
not merely when evaluation reaches them. `ctx.*` and environment access refer
to the derived schema request context. Baseline and kind-definition slot
prohibitions apply through imports as well as inline declarations; an ordinary
`kind: schema` envelope is still allowed to contain a template.

An unresolved template is neither an invalid template nor a validated document.
Passive schema-definition checks judge template syntax without resolving it;
a command asked to validate document values against an unresolved schema
reports an unavailable check and cannot report success. The
[`shipped-schema corpus`](../../lib/tests/l1/shipped_schema_corpus.rs) in
Darkmatter remains passive: remove the sandboxing carve-out when template
admission works, and prove resolved document validation separately through
explicit composition. Otherwise that corpus would either execute expressions
or reject every intentionally unresolved template.

### Cache correctness and refresh

The read set supplied by 2026-11-04-loading-data identifies resources; it is
not a fingerprint and cannot by itself validate a cache entry. Resolution
must additionally record the exact bytes or observations used, including
transitive schema imports, missing-file probes, and captured context values.
Hash bytes with biscuit-hash while reading them, rather than reopening a file
after evaluation and potentially labeling an old result with new bytes.
Retain dependencies on failures so creation or repair can trigger a retry.

Cache selection includes schema source revisions, defining-file identity,
resolution roots and magic-root bindings, relevant captured context, and
resolver/function-catalog version. Check current permissions before using a
cached result; a result produced with remote or shell permission grants no
permission to another request. Never persist secret environment values as
cache-key text. Replay expression-read dependencies into the current command's
read set on a cache hit, preserving the loading-data contract.

A command pins one concrete snapshot for each equivalent schema context.
Approved effectful resolution must share the command's data runtime and
read set; passes with fewer permissions cannot borrow an approved result to
circumvent their own policy. Integrate with loading-data's existing memo
invalidation after commands mutate files; Ruling Q7 requires re-resolution
before the next validation, so a known-stale schema never validates new
values.

For DMLS, watch actual dependencies regardless of extension, including missing
candidates and imported schemas. Refresh unsaved data/schema buffers through
the existing source-buffer mechanism, and reject an older worker result when
a newer edit has arrived. Publish a concrete snapshot atomically. During a
refresh, last-good values may support explicitly stale completion/hover, but
must not produce current “valid” diagnostics. On first load or refresh failure,
retain syntax diagnostics and report pending/failed resolution; never silently
substitute an unconstrained schema. Environment/context changes need an
explicit refresh or a new captured request; file events alone cannot detect
them.

## Spikes

Use one host and a quick sample for the performance work. Share the baseline
and cache/refresh prototype across the investigations; do not repeat the same
measurements for every design question. Reuse existing measurements when they
already answer the question. Safety, API ownership, and grammar breadth are
design reviews, not separate performance exercises. Each investigation ends
with a recommendation, not a prototype merged to `main`. No cross-OS benchmark
or statistical gate is required.

### S1. Baseline and budget

Measure today's cold and warm cost for schema load, validator compile, and
validation (`md schema validate`, `md compose`, DMLS diagnostics). From those
numbers, set the budgets the other spikes are judged against. A proposed
comparison: show warm overhead against a static schema and cold resolution
against validator compilation. Treat those as comparisons to explain costs,
not pass/fail thresholds for a quick sample.

### S2. Input-fingerprinted resolution cache

Prototype the resolution cache. Record every input an evaluation touches and
key on those inputs. File and URL reads come from the read set that
2026-11-04-loading-data delivers under “Read set”; it holds no content hashes, and it
does not cover environment values or `ctx.*` groups, which still come through
the existing demand-driven context capture. Measure hit rate and cost across a Claudine-style many-document run. Decide
whether the cache is process-wide (like the validator cache), persisted on
disk, or both.

### S3. Invalidation and DMLS

Determine how DMLS learns that an input changed (the watch-directories
mechanism, file events, or a fingerprint check on save). This spec owns using
the read set for DMLS invalidation; 2026-11-04-loading-data provides the library
read set only, and DMLS today watches only `.md` and `.yaml` files. Resolution must run
off the request path: DMLS serves the last resolved schema and marks it stale
until a fresh one is ready. Measure added latency on completion and hover.

### S4. Function determinism classes

Classify catalog functions as **pure** (same inputs → same output),
**input-reading** (fingerprintable: `load_data`, `file_exists`), or
**volatile** (time, git state, untracked network state, shell execution).
Pure and input-reading slots are cacheable only when every dependency and permission is accounted for.
`load_data` may read either a local file or a URL, so classify the actual
invocation and all transitive calls, not just the function name. Decide
whether volatile slots get a TTL, resolve on every use, or are rejected. This classification describes caching, not permission
to evaluate. Only rejecting volatile functions would count as narrowing.

### S5. Safety and approval

The model is decided (Ruling Q5). This design review settles how to build it:
how the pre-flight walk collects slot targets passively from template
expressions, including through imports and trigger payloads; how slot grants
join the shell approval batch and policy-file store without merging the two
rule sets; how the clean-file check reads the request's captured repository
observation (through Sniff) rather than recapturing it; and the work limit
for pure evaluation, set from S1's numbers.

### S6. Lazy and partial resolution

Measure whether resolving only the slots a given document reaches — and
compiling static parts once, with dynamic parts substituted later — is a
material win over resolving the whole schema up front.

### S7. Extraction boundary

Specify the resolver trait in `simplified-schema` and the template type it
operates on. The extraction (2026-10-28-extract-simplified-schema) has landed
in code, so the hook is added to that library's public surface and must keep
it host-neutral: no Darkmatter types and no dependency from
`simplified-schema` back to Darkmatter. Slots must be resolved before layers reach
`EffectiveSchema`'s merge of already-lowered schemas; report whether that merge can stay
as it is, or whether it should move up to `SimplifiedSchema`.

### S8. Breadth

Starting from the `enum`/`suggest` slots, evaluate which other positions can
accept a slot at no extra cost: `literal(...)`, `default(...)`, numeric bounds
(`min`/`max`), pattern constraints, and whole type expressions (a slot that
yields a `type-definition`). For each position, state its expected result
shape, admission rules, and diagnostics; a whole type must use the existing
passive type-definition parser and import rules. Only positions that the
baseline, caching, and resolution investigations show cannot meet the budget
are candidates for narrowing.

## Investigation outcomes (2026-11-07)

All eight investigations ran as read-only reviews plus one out-of-tree
performance prototype; nothing was merged. Measurements: Apple M4 Max,
16 cores, 128 GB, macOS 27.2, one-minute load below 4 except two flagged
batches (4.1–5.5), A/B interleaved, medians of 7–9 runs. A `{{ }}` slot
cannot load today, so resolution was simulated (read, hash, parse,
substitute, lower, compile); evaluator cost is a compose-derived upper bound.

**No slot position fails the budget, so performance gives no grounds for
narrowing.**

### S1. Baseline and budget

Command setup dominates. Cold `md schema validate` is 42 ms (static sandbox
copy) to 166 ms (provider `_schema.yaml`); cold `md compose` is 58–334 ms, of
which schema validation is 0.33–3.8 ms; first DMLS diagnostics are 84–142 ms.
A slot loading `os.yaml` adds 13–30 µs raw and at most 91 µs through the
evaluator; a 500-member data file adds 0.13–0.57 ms. A single-file Git
index/`HEAD` check is about 3–5 ms; a full `git status` walk is 109 ms.

Budgets: warm overhead at most 5% per document; a cold slot at most 2× the
lowering and compile of the schema it produces and at most 1% of the
surface's cold time; DMLS requests do no I/O and gain at most 0.1 ms. All
pass; the 500-member evaluator upper bound sits at the edge. A per-slot
pure-evaluation limit of about 25 ms leaves roughly 40× headroom.

### S2. Resolution cache

500 documents against 3 schemas: 99.4% hit rate, and a warm hit returns the
same `Arc`. Per-hit re-check cost: trusting the command's pinned snapshot
0.04 µs; `stat` (size + mtime) 2.25 µs (1.8% of the 125 µs per-document
validation); rehashing every input 20.6 µs (16%, fails the budget). An
on-disk tier saves about 37 µs per process. **Recommendation:** a
process-wide cache only; trust the pinned snapshot within a command and
`stat`-check across commands (see Open Question 12).

### S3. Invalidation and DMLS

DMLS watches only `.md` and `.yaml` today. Every recorded dependency
(imports, missing-file probes, `.git/index` and `HEAD` for the clean-file
grant) needs its own watch, with a `stat` re-check as the fallback. Serving
a snapshot from memory costs 4 ns against 1.8–9.1 ms for today's completion
and hover; a background re-check costs 2 µs and a full re-resolve 91 µs.
Slot resolution must not run on the registry job lane, because requests wait
on it. Today's request path was not audited for I/O. Side finding: DMLS
completion inside the sandbox schema's `os[]` enum list returns no items
(Open Question 13).

### S4. Function determinism classes

Of 120 built-in functions: 56 pure, 28 input-reading (7 of them become
remote reads on a URL argument: `load_data`, `file_exists`, the four
`frontmatter`/`markdown_*` readers, `validate_schema`), 5 captured-context,
31 live-volatile. **Recommendation, not narrowing:** move the clock,
timezone, and `PATH` onto the captured request context (16 volatile
functions become fingerprintable), resolve the remaining volatile slots once
per command snapshot and never reuse them across requests, and key the
validator cache on resolved content. A wall-clock TTL is rejected: it changes
diagnostics without an edit and a stale-but-in-TTL snapshot would report
"valid". The class becomes a required fact on `FunctionBinding`
(`compose/expression/functions/mod.rs`) with a passive `slot_class(expr)`
walk; `REMOTE_READ_FUNCTIONS` is derived from those facts.

Defects found: 14 date functions registered `pure` read the live clock or
host timezone; `has_command`/`has_binary` read the process `PATH` directly;
23 input-reading functions record nothing in the read set and `load_data`
drops its not-found probe; pre-fetch URL discovery matches
`REMOTE_READ_FUNCTIONS` exactly but evaluation folds case and underscores,
so `loaddata("https://…")` is missed. `load_data` (through contract imports)
and `validate_schema` can re-enter schema resolution and need the shared
resolution stack.

### S5. Safety and approval

- **Targets** are collected from the resolved template (document `$schema`,
  every import at its defining file, selected trigger payloads only).
  Literal arguments and expressions over literals and pure functions are
  known up front; both branches of `?:` and `||` count, as in shell
  pre-flight. Computed targets go through one check in the resolution
  context, placed before the shared `load_data` memo so a document's ungated
  read cannot hand a slot the file.
- **Approval** runs in the same pre-flight step as shell, before the shell
  decisions, with its own handler and rule types. Data outputs fail with a
  new `dm.schema.slot_approval_required` code.
- **Clean files** are judged on the bytes actually read: Git blob hash of the
  worktree bytes equals the index entry equals the `HEAD` entry. The captured
  repository observation holds no Git status today, so a once-per-request
  cleanliness component is added; Sniff lacks a per-path check. Windows
  autocrlf and LFS can make clean files look dirty.
- **Surfaces:** `md schema validate <document>` and `md clean` do not use the
  pre-flight walk and need a small collect-and-approve step; reference
  checks use only automatic and persisted grants; DMLS needs an
  `executeCommand` handler for the grant code action.
- **Work limit:** reuse the existing iteration budget (visits and bytes) per
  slot. Missing: result-size charging, evaluator cancellation, a `load_data`
  size cap.
- **Correction to Q5:** the shell policy files are not user-level (`md
  compose` keeps them beside the document; library callers fall back to the
  repository root), and a site grant fed into the request's shared
  `FetchPolicy` would widen document reads (Open Questions 1–2).

### S6. Lazy and partial resolution

Resolving only reached slots saves 1.7%, because lowering (1.25 ms) and
compiling (0.55 ms) the whole schema dominate. Compiling static parts once
and substituting dynamic parts saves about 1.8 ms only on re-resolution and
requires restructuring the validator. **Recommendation:** resolve the whole
effective schema up front.

### S7. Extraction boundary

- **Template:** one `SimplifiedSchema` type with a template state (as Q2
  implies). New `Constraint::TemplateMembers` / `TemplateSuggest` variants
  hold literal elements and `SchemaSlot`s (expression, exact text, span,
  defining file, expected shape); concrete `Members`/`Suggest` stay unchanged,
  so their 28 reader files need no edits. Slot boundaries reuse
  `scan_expression` (made crate-visible); spans map to YAML as suggestion
  spans do today. The file loader records each slot's defining file through
  import inlining and aliases.
- **Resolver:** a host-neutral `SlotResolver` trait with an associated host
  error and opaque observation type, `resolve_slot(&SlotRequest)`, and
  `is_canceled()`. `SlotRequest` carries the slot, expected shape, defining
  directory and derived context, and a shared `ResolutionStack` for cycle
  detection. Failures are `SlotResolutionError<E>`, carried erased inside
  `SchemaError` so Darkmatter can downcast to `DataError` (M3).
  `with_reads_recorded` returns the exact schema-file text read, for the host
  to hash. `resolve_with` returns a new concrete snapshot and never mutates
  the template.
- **Merge:** keep `merge_baseline` on lowered layers. Whole-property
  precedence makes per-layer resolution equivalent, and existing memos hold if
  a warm hit returns the same `Arc`.
- **Passive checks:** a generic free-reference walker in the library;
  Darkmatter applies Q3's rules wherever it admits a template. Q4 is checked
  after import expansion; trigger match conditions also reject slots.
- **Blast radius:** `effective_for` gains an `UnresolvedTemplate` error,
  reaching 78 Darkmatter files, Claudine (15), and Messenger (3). Claudine's
  CLI completion uses `.ok()?` and would silently show nothing for a
  template. Also affected: `resolve_schema_with_roots` / `ResolvedSchema`
  callers (a template-returning call is added beside it), non-exhaustive
  `SchemaError` matches, trigger payload resolution and its memo, the
  ownership guard, and both shipped-schema corpus tests.

### S8. Breadth

| Position | Recommendation |
| --- | --- |
| `literal(...)` | include; about enum cost |
| `default(...)` | include; about enum cost |
| `min` / `max` | include; about enum cost |
| `pattern(...)` | include, with a size or backtracking cap (data-supplied regexes run on every validation, outside Q5's work limit) |
| whole type expressions | follow-up feature: a new `TypeExpr` variant touching about 44 files, and editor features have no type until the slot resolves (Open Question 9) |

## Minimum requirements

Whatever the spikes conclude, the finalized spec must deliver:

- **M1.** `enum(...)` members from a `{{ … }}` slot that yields an array of
  strings, mixable with the `; required` / `default(...)` tail.
- **M2.** `suggest(...)` candidates from a slot, on every type where
  `suggest` is allowed today.
- **M3.** Slot syntax errors and wrong result shapes are positioned schema
  errors, reported in `md schema validate` and DMLS like any other schema
  error. They follow the diagnostic position rule: a line and column only when
  proven, otherwise null, never a placeholder. A slot whose `load_data` fails
  keeps that failure's `dm.data.*` code and `data_location`. The primary source
  is the authored slot in its defining file, including an imported file; data
  positions remain secondary. Preserve typed causes rather than wrapping them
  into an unstructured schema-load message.
- **M4.** `sandboxing.yaml` uses the schema-relative spelling above, is admitted
  passively as a template, and validates its documents after explicit
  resolution. The shipped-schema corpus carve-out is removed.
- **M5.** No guarantee from "The tension to resolve" is lost without an
  explicit, documented replacement.

## Verification and delivery

Before implementation planning, record the investigation outcomes in this
spec and resolve Open Questions. Keep this draft status until then. The
implementation must cover:

- Passive parse, query, and export without evaluation; slots in imported and
  recursive types; round-trip template export and reparse; static-schema parity.
- Mixed literals/slots, duplicate normalization, empty lists, wrong shapes,
  delimiter-like data, suggestions' existing lint rules, and authored ranges.
- A normal `md compose` invocation using the real sandboxing schema and OS
  data, plus `md schema validate` and DMLS behavior for unresolved schemas.
- Cache reuse and invalidation on source/data/import changes, missing-file
  creation, context changes, permission changes, and failures. Prove that a
  cache hit preserves the command read set and does not rerun the evaluator.
- Trigger discovery, kind selection, reference checks, and pre-flight walks
  performing no unapproved evaluation. DMLS requests do no I/O or evaluation;
  stale worker results and unsaved edits cannot overwrite newer snapshots.
- The Q5 approval model: the automatic grant for a clean same-repo file, and
  its refusal for dirty, untracked, ignored, symlink-escaping, other-repo,
  and no-repo targets; every grant scope, once and persisted, and deny;
  batched pre-flight prompting beside shell commands; computed targets
  checked at evaluation; typed `approval-required` failures on data outputs
  and for library callers without a handler; DMLS code actions writing the
  user-level policy; and proof that repository content cannot grant access.
- A pending trigger payload that is never selected reads nothing, and a
  lifecycle write to a snapshot dependency forces re-resolution (Q6, Q7).

Use package-area `just test` and `just lint` for simplified-schema and
Darkmatter, including the library's minimal-feature checks. Editor workers
and CLI invocation tests use ordinary isolated tests; no focused terminal or
browser windows are needed. Update current language, schema, composition,
and DMLS topic docs and the affected skills when implementation lands. The
spec remains an active document for author review; agents do not move it to
`_completed`.

## Open Questions

None. The questions raised by the investigations were ruled on 2026-11-08
(Rulings Q8–Q20), each as recommended.

## Rulings

- **Q1. Decided (2026-11-06): literal and dynamic members mix.** Each slot's array is
  flattened into the list, so a literal `none` beside a slot that loads
  `[macOS, Linux, Windows]` means `enum(none, macOS, Linux, Windows; required)`. `suggest(...)` works the same way. A
  member duplicated between a literal and a slot is kept once rather than rejected, so
  a changed data file cannot break a schema. Literals stay visible to the editor before
  the slot resolves.
- **Q2. Decided (2026-11-06): callers work with the struct, and JSON Schema is derived.**
  `SimplifiedSchema` holds literal members and unresolved slots, and resolving is the
  only effectful step. Conversion to JSON Schema is lossy and remains an explicit export,
  derived and cached as today. On an unresolved schema, `to_json_schema` and validation
  return a typed error, as a function-typed property already does; there is no
  placeholder output, because a passive caller has the struct.
- **Q3. Decided (2026-11-06): a slot evaluates in the schema's context only.** `fm.*` in
  a slot is a schema error. One schema then resolves once for every document that uses
  it, and no slot relies on values it is about to validate. A document's inline `$schema`
  resolves relative paths against that document. This can be relaxed later without
  breaking a document.
- **Q4. Decided (2026-11-06): no slots in the baseline or in kind definitions.** A slot
  in either is a schema error, and a kind claim holding one is reported as broken. These
  load without a document asking for them, so they stay free of evaluation, and kind
  selection stays passive. This does not affect an ordinary `kind: schema` document named
  by `$schema`, such as `sandboxing.yaml`. It can be relaxed later without breaking a
  document.
- **Q5. Decided (2026-11-07): schema-slot reads use a per-resource approval model,
  patterned on shell approval.** It applies to schema slots only; a document's own
  `load_data` and `::file` reads keep their current rules (local reads unrestricted,
  remote reads under `--allow host=`).
  - *Automatic grant.* Pure expressions need no grant. A local file is read without
    asking when it lies in the same Git repository as the slot's **defining** schema
    file (an imported definition uses its own file) and is clean: tracked and
    identical to `HEAD` in both the index and the working tree, judged after
    `canonicalize_simplified`, so a symlink that leaves the repository gets no free
    pass. An unsaved editor buffer that differs from disk counts as dirty. Untracked,
    ignored, dirty, other-repository, and no-repository files get no automatic grant.
    The repository state comes from the request's captured observation, never a
    fresh capture.
  - *Everything else asks.* The pre-flight walk collects slot targets passively and
    raises them in the same batched approval as shell commands. Choices for a file:
    allow **this file**, or allow **this file's directory** (recursive). For a URL:
    allow **this exact URL** with its query; allow **this URL whatever its query**
    (offered only when it has one); or allow **this site** (scheme, host, and port;
    no subdomains). Each grant is **once** (this run) or **persisted**; **deny** is
    persisted too.
  - *Where grants live.* Persisted grants and denials go in a user-level policy file
    beside the shell whitelist and blacklist (corrected by Q8: a fixed home path).
    Repository content, schemas, and workspace configuration can never grant
    access. A site grant feeds (corrected by Q9: a slot-only fetch policy)
    `biscuit_file::FetchPolicy`; `--allow host=HOST` remains a one-run site grant.
  - *Computed targets.* A target the walk cannot determine passively (for example
    `load_data(ctx.env.DATA_FILE)`) is checked when it is evaluated: an interactive
    run prompts then, and a non-interactive one fails with a typed
    `approval-required` error.
  - *Surfaces that cannot prompt.* A data output fails with a typed
    `approval-required` diagnostic naming the grants needed, as shell does today.
    DMLS never prompts or blocks a request: it reports a positioned diagnostic on
    the slot whose code actions offer the same grant choices, writes the chosen
    grant to the user-level policy, and refreshes. A library caller passes an
    approval handler; without one, an unapproved read is `ApprovalRequired`.
  - *Excluded and unchanged.* Shell syntax is not part of slot grammar; adding it
    needs its own syntax and policy decision. Time- or state-dependent functions need
    no grant; their caching is S4's question. Pure evaluation runs under a work
    limit so a schema cannot stall DMLS.
  - *Replacement for the passive-schema guarantee (M5).* Opening a folder may now
    evaluate pure slots and read clean files from the schema's own repository, and
    nothing else. What it can expose is limited to that repository's committed
    content.

  | Surface | Behavior |
  | --- | --- |
  | `md compose` | Resolves with automatic and persisted grants; asks for the rest in pre-flight (formatted output) or fails `approval-required` (data output) |
  | `md schema validate <document>` | Same gate as compose; an unapproved slot is an unavailable check with exit 1, never success |
  | `md schema validate <schema>` | Passive slot-syntax check only; no resolution |
  | `md clean` | Same gate; an unapproved slot reports its constraint unavailable and never reports the document clean |
  | Pre-flight walk | Collects slot targets passively and raises the batched approval; the command pins the resulting snapshot |
  | Reference checks (`md graph`, `md validate refs`) | Use the command's pinned snapshot or resolve with automatic and persisted grants only; never prompt; defer an unapproved slot as `for_reference_check` defers today |
  | DMLS | Resolves off the request path with automatic and persisted grants; an unapproved slot is pending with a grant code action |
  | Library callers | Resolve only through an explicit call that takes a policy and an optional approval handler |

- **Q6. Decided (2026-11-07): trigger payloads keep valid templates pending.** The
  registry holds a valid template as pending (unresolved, or awaiting approval).
  Matching stays passive, and only a selected payload resolves, at the authorized
  boundary of the command using it. A trigger that no document selects reads
  nothing. Compilation errors that depend on slot results are reported when the
  payload resolves, positioned at the slot in the trigger file. Genuinely invalid
  triggers keep their existing handling; a pending template is never reported as
  broken, skipped, or empty.
- **Q7. Decided (2026-11-07): a mutated dependency forces re-resolution.** A write
  that loading-data's memo invalidation records against a path in a pinned
  snapshot's dependencies marks that snapshot stale, and the next validation
  re-resolves it from the template under the current grants. A failed re-resolution
  is a typed error (keeping its `dm.data.*` code), never a pass, and no stale
  snapshot validates new values.
- **Q8. Decided (2026-11-08): persisted grants live in one fixed home file (corrects
  Q5).** The shell policy files are not user-level (`md compose` keeps them beside the
  document, and library callers fall back to the repository root), so "beside the shell
  whitelist" would make grants repository content. Grants and denials go in
  `<home>/.config/darkmatter/schema-read-policy`: line-based, one rule per line, deny
  wins. No workspace setting (`.dmls.toml`, editor settings) can move it, and the shell
  rule sets are unchanged and never merged with it.
- **Q9. Decided (2026-11-08): slot reads use their own fetch policy (corrects Q5).** A
  site grant never enters the request's shared `FetchPolicy`, so slot grants cannot
  widen document reads. The slot policy matches scheme, host, and port.
  `--allow host=HOST` keeps its meaning for document reads (any scheme or port) and
  also acts as a one-run site grant for slots.
- **Q10. Decided (2026-11-08): effectful functions are schema errors in a slot.** The
  network and compose functions (`pr`, `cicd`, `ping`, `ping_under`, `as_markdown`) and
  the login-shell probes (`has_alias`, `has_builtin_function`, `has_user_function`,
  `can_execute`) are rejected passively.
- **Q11. Decided (2026-11-08): listings and probes.** `find_files` and other directory
  listings never get an automatic grant and need a directory grant. `file_exists` is
  automatic inside the defining repository and needs a grant outside it.
  `has_command` and `has_binary` read the `PATH` captured with the request.
- **Q12. Decided (2026-11-08): runtime-only bindings are rejected.** In addition to
  Q3's `fm.*`: bare identifiers (they fall through to document properties), host
  globals, `ctx.cwd`, document-identity values (`ctx.self`, `ctx.last_updated`,
  `ctx.hash`), and per-run nonces (`ctx.id`, `ctx.sid`). `current.*` and `local.*` are
  allowed only inside a callback.
- **Q13. Decided (2026-11-08): a schema outside the request's repository gets no
  automatic grant** (for example one under `~/schemas`); every read it makes needs a
  grant.
- **Q14. Decided (2026-11-08): only the effective schema's slots resolve.** A slot under
  a property that a higher layer redeclares is never read or prompted for.
- **Q15. Decided (2026-11-08): Q4's baseline prohibition covers the DMLS extension
  layer.**
- **Q16. Decided (2026-11-08): breadth.** This feature delivers slots in `enum(...)`,
  `suggest(...)`, `literal(...)`, `default(...)`, `min`/`max`, and `pattern(...)`, the
  last with a size or backtracking cap. Whole type expressions follow in a separate
  spec; that is sequencing, not narrowing. That spec resolves imports in a returned type
  against the slot's defining file.
- **Q17. Decided (2026-11-08): a grant is decided once per run.** Writes carry no
  per-path attribution, so each target's grant is decided once per run and kept for that
  run; Q7's re-resolution after a write still happens.
- **Q18. Decided (2026-11-08): approval details and scope additions.** The prompt adds a
  "deny for this run" choice, also used on cancel or end of input. Unapproved slot reads
  report `dm.schema.slot_approval_required`. S4's read-set gaps (23 input-reading
  functions that record nothing, `load_data`'s dropped not-found probe) and the
  alias-folding gap in pre-fetch URL discovery are fixed in this feature.
- **Q19. Decided (2026-11-08): cross-command re-check by `stat`.** Within a command the
  pinned snapshot is trusted (Q7 governs writes). Across commands, size and mtime are
  checked first and a change triggers a rehash; DMLS also re-checks on file-watch events.
  A same-size rewrite within the filesystem's mtime resolution can be missed; this is a
  documented limitation, accepted over a 16% per-document rehash cost.
- **Q20. Decided (2026-11-08): DMLS enum completion is fixed here.** Completion inside
  the sandbox schema's `os[]` enum list returns nothing today; investigate and fix it in
  this feature, since it is the motivating editor case.
- **Q21. Decided (2026-11-08): a missing target is an existence probe.** A content read
  (for example `load_data`) whose target does not exist inside the defining repository
  is judged like `file_exists` under Q11: no grant is needed, and it fails with the
  ordinary typed not-found error, which `try(...)` can recover. Reading a missing file
  reveals no more than an existence probe. A missing target outside the defining
  repository still needs a grant.
