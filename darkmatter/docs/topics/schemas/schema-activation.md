# How Schema Activation Is Evaluated

Schema activation decides which conditional schemas apply to a document. The
authoring rules are described in
[Passive Activation Expressions](./authoring-schemas.md#passive-activation-expressions).
This document explains the evaluation boundary shared by Darkmatter consumers,
including the CLI and DMLS.

## Implementation Status

This document describes the intended behavior. The current implementation does
not yet support the complete activation model or predicate family described in
[Authoring Schemas](./authoring-schemas.md#schema-triggers).

The current trigger parser uses OR for a top-level list. The documented authoring
model uses AND between top-level conditions and OR within an explicit `group`.
Existing rules need a deliberate migration that preserves their meaning.

The current tagged schema parser also treats `types` as its schema payload.
The documented schema envelope instead separates an exported `$schema` from
reusable `types`. Authoring definitions remain at their documented locations
while parser support catches up; their presence does not establish support.

Some predicate spellings are not finalized. The initial refresh policy is
described below; it does not establish a fixed polling interval or editor
latency requirement. Update this status as support lands.

## Refreshing Time-Based Conditions

DMLS updates time-dependent schema activation while a document remains open,
even when its text has not changed. It schedules a new check at the next relevant
time boundary, such as the beginning or end of a time window. It also rechecks
after the computer wakes or its clock changes, since a scheduled boundary may
have passed in the meantime.

CLI commands evaluate activation once per invocation. They do not start a
background service to monitor later changes.

This is the initial scheduling policy. Performance tuning may adjust scheduling
later; no particular polling interval is part of the authoring contract.

## Gathering Facts Before Matching

Darkmatter first parses the rules and identifies the facts their conditions
request. The host integration gathers only those facts, using focused Sniff
observation APIs and biscuit-file's `FileReference` for file resolution.
File operands retain the declaring schema's source context; an explicit
workspace operand uses the consuming document's workspace context. See
[File Predicate References](./authoring-schemas.md#file-predicate-references).

The resulting snapshot is immutable for one activation pass. All clock-dependent
conditions use one captured instant, so the clock cannot advance between two
conditions in the same pass. This activation snapshot does not change when a
calling application captures its own runtime context or lifecycle values.

Each fact records its result, including the distinction between absence and
failure. A missing executable is a negative availability result. A failed
availability check retains its cause. Executable discovery never launches the
program being checked.

Fact gathering can inspect operands from branches that matching will not reach.
For example, both members of an OR group may request file facts, even when the
first member will match. Once the snapshot exists, matching performs no I/O,
invokes no lazy runtime providers, and dispatches no actions or shell expansion.

## Expressions and Function Eligibility

Activation expressions use the existing Darkmatter parser, operators and
evaluator. Document lookups retain normal missing-value semantics: a missing
document property evaluates to null.

Eligible functions compute from supplied data. Eligible clock functions use the
captured activation instant rather than reading the live clock. Filesystem,
remote-read and host-discovery calls do not become eligible merely because
their expression arguments are literals. Those observations belong in dedicated
conditions whose literal operands can be identified before matching.

Darkmatter's shared function registry declares each function's activation
capability and supplies its descriptor to consumers. DMLS must not maintain a
separate allowlist. An evaluator mode named "pure" is insufficient evidence of
eligibility if its handler still reads the clock or performs another observation.

Rule preparation checks the entire authored expression for unsupported calls,
including calls in inactive branches. It does not run those branches to search
for arithmetic errors or other failures that require evaluation. Eligible
computations retain normal short-circuit behavior: once an AND or OR result is
decided, unnecessary operands are not evaluated.

## Failures and Incomplete Validation

A malformed rule or prohibited function call is a preparation error. During
matching, an observation failure becomes an error when the condition using that
fact is reached. A failure collected for an unvisited branch does not, by itself,
make the rule fail. Failed observations must never be converted into false or
"not found" results.

When a failure prevents Darkmatter from deciding whether a schema applies, the
consumer reports incomplete validation and identifies the source and cause.
Checks that depend on that schema cannot be reported as current. Independent
checks continue; DMLS likewise withholds dependent hover and completion data
without falling back to old definitions. A successful refresh restores those
checks and clears the associated failure.

Failure scope follows discovery scope. A broken automatically discovered package
rule cannot affect sibling packages. A rule supplied through `SCHEMA_DIR` can
have workspace-wide applicability. Intentionally removing an optional rule is
different from losing an import required by a rule that is still active.

Runtime validation required by the calling application remains independent of
whether an optional editor rule is active. Passive activation is a way to select
schemas; it does not bypass mandatory validation or authorize runtime effects.
