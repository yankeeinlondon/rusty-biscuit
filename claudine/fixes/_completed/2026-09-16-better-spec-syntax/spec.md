---
created: 2026-09-16
status: proposed
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-15
implemented: true
implemented_by: "codex/gpt-5.6-sol"
review_iterations: 5
completed: true
area: claudine
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - dmls
---

# Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Problem

The intuitive way to update several runtime properties is a mapping:

```yaml
initialize:
    stack:
        - action:
            - set:
                epilog: "{{message_to_agent}}"
                message_to_agent: null
```

Claudine currently rejects this with `CompositionError: object value not
allowed here`. Its generic action parser expects positional arguments or an
explicit `action: set` object, even though the author's mapping already
unambiguously identifies both the destination keys and their values.

This also conflicts with the authoring convention used by `proxy.with`, where
a mapping supplies named values. Supporting several equivalent `set` forms
adds complexity without helping authors. Replace the existing Claudine action
forms with one canonical mapping payload everywhere the shared lifecycle
action grammar is used.

## Reported Case and Evidence

The error occurred after the implementation router proxied to
`prompts/_implement/implement-plan.md`. The reported target contained both:

```yaml
- set:
    epilog: null
```

and the two-property mapping shown above. Both should be valid. The current
working-tree copy has temporarily expressed the same intent as two positional
actions (`epilog` and `agent_message`); those are migration inputs, not the
desired contract.

Current source and documentation establish the existing contract:

- `claudine/docs/topics/flow-control/sequences.md`, **Mutating state with
  set**, documents `set: [key, value]` and `{action: set, key: …, value: …}`.
- `claudine/lib/src/composition/lifecycle/executor.rs` dispatches `set` as a
  single `(key, value)` mutation through `apply_runtime_set`, updating both
  the invocation's runtime layer and the working state.
- `claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs`
  covers the existing forms, typed values, reserved keys, and runtime state.
- `darkmatter/lib/src/effects/verbs.rs`, `EffectEngine::set`, provides the
  underlying single-property mutation operation. This Rust API is distinct
  from the lifecycle authoring grammar.

This is a deliberate syntax change. It is separate from the initialization
ordering bug specified in `../2026-09-15-initialize-after-proxy/spec.md`.
Accepting the mapping does not itself fix premature transclusion discovery.

> **Reader's note — two APIs named `set`:** this change narrows Claudine's YAML
> action grammar only. Darkmatter's `EffectEngine::set`, its
> `set(key, value)` descriptor, and the separate Claudine loop-control
> `set(prop,value)` DSL remain positional APIs. Keeping those lower-level APIs
> stable avoids making a Claudine authoring preference a breaking Darkmatter
> change. Claudine must therefore parse `set` as a deliberate grammar exception
> rather than deriving this one action's authored shape from Darkmatter's
> positional descriptor.

## Decisions

1. A literal key-to-value mapping is the only authored Claudine lifecycle
   `set` payload.
2. Migrate active uses; do not retain compatibility aliases or a deprecation
   period for the previous forms.
3. Evaluate all values against the state immediately before this action, then
   apply the complete update. YAML key ordering must not change the result.
4. The action updates runtime state only. It does not write frontmatter to disk.
5. The universal `no_error` modifier remains available as metadata beside the
   `set` payload; it is not a destination key and does not create another
   payload form.
6. A multi-key update is one typed action and one runtime-state transaction. It
   is not desugared into several existing single-key actions.

The snapshot behavior in decision 3 is the semantic choice for this syntax;
tests and documentation must make it explicit.

## Required Behavior

### R1. One authoring shape

Accept `set` followed by a mapping of literal property names to values wherever
the shared lifecycle action grammar is accepted: event stacks, nested action
lists, task setup/teardown, and sequence side-effect tasks. Existing enclosing
stack syntax remains valid. These examples show the canonical payload in an
event stack and as sequence work:

```yaml
initialize:
    stack:
        - action:
            - set:
                ready: true
                retries: 0
                details:
                    phase: implementation
                items: [one, two]
                last_error: null

tasks:
    - side_effect:
          set:
              ready: true
```

A mapping with one entry and a mapping with several entries are equally valid.
An empty mapping is a valid no-op. Keys are literal, nonempty strings subject
to the existing top-level-key and reserved-key restrictions. Do not introduce
dynamic key interpolation, YAML merge-key expansion, non-string destination
keys, or dotted-path writes. Duplicate destination keys must fail during YAML
parsing rather than silently selecting one value.

**Ruling 2, approved by the author on 2026-09-17:** enforce duplicate-key
rejection in Darkmatter's shared frontmatter parser for every YAML mapping,
including nested mappings, before conversion can discard duplicate entries.
This applies consistently to direct parsing, indentation normalization, and
expression-protection fallbacks. Preserve typed errors and source-location
information; a fallback must not hide a duplicate-key failure. Claudine
consumes this shared behavior without reparsing frontmatter.

This explicitly expands scope beyond Claudine lifecycle syntax to all
Darkmatter frontmatter consumers. Documents relying on last-wins parsing
must be corrected. Duplicate-looking text inside expression strings remains
expression content; explicit merging between separate documents is unchanged.

Preserve the universal per-action `no_error` behavior without placing it inside
the assignments mapping:

```yaml
initialize:
    stack:
        - action:
            - set:
                optional_status: "{{ maybe_status }}"
              no_error: true
```

The same sibling modifier is valid when the stack item contains only that one
action. `no_error` suppresses an evaluation or dispatch failure according to
the existing lifecycle contract; because the update is atomic, suppression can
never expose a partial write. A property literally named `no_error` remains
writable only when nested under `set`.

Remove the old positional and explicit action forms from this grammar:

```yaml
- set: ["ready", true]
- action: set
  key: ready
  value: true
```

Reject any lifecycle function-call spelling of `set` that serves as another
authoring form, if currently accepted. Do not introduce `set: "{{mapping}}"`
as an alternative whole-action form. Expressions belong in the mapping's
values. This removal does not apply to the separate loop-control
`action: "set(prop, value)"` DSL. Other verbs retain their existing syntax,
including `proxy.with`.

A false `when` condition still requires a statically well-formed mapping, just
as it requires every other action to parse. It prevents value evaluation and
mutation at execution time; it must not make the parser reject an otherwise
valid mapping or eagerly evaluate its expressions.

### R2. Preserve typed values and execution-time evaluation

Accept native scalar, null, array, and object values. Preserve their types:
`false` differs from `"false"`, and `null` differs from `"null"`.
A whole-value expression preserves the type of its result, including an
array, object, or null. Mixed literal/interpolated strings retain the existing
string interpolation semantics.

Resolve expressions at action execution time, so the action sees successful
updates from earlier actions. Apply the same value-resolution rules recursively
to arrays and object values. Assign object values as complete values; do not
implicitly merge their properties. Treat `null` as an assigned null value,
not deletion.

Represent these values as a recursive typed action-value tree at parse time,
analogous to `proxy.with`, rather than forcing arrays, objects, or null through
the positional `Vec<Expr>` representation. Retain assignment order for stable
diagnostics and result serialization, but never give that order evaluation
semantics.

### R3. Snapshot evaluation and all-or-nothing updates

Validate all destination keys and resolve all values before publishing any
update to the working state or shared runtime layer. Capture prior values from
the same pre-action effective working state. A bad key or expression must leave
both layers unchanged by this action.

Every value reads the same state from immediately before the action. For
example, with `left: A` and `right: B`:

```yaml
- set:
    left: "{{right}}"
    right: "{{left}}"
```

The result is `left: B`, `right: A`, regardless of mapping key order. Authors
who need one assignment to read another's result use two consecutive `set`
actions. Preserve existing serial/parallel task visibility and isolation
boundaries; this change does not introduce cross-task transactions.

Use the existing runtime mutation authority and reserved-key validation.
Mapping syntax must not bypass protection of keys such as `outputs`, including
execution paths without a shared runtime cell. With a shared runtime cell,
validate and resolve outside the lock, then commit the complete mutation map
under one mutex acquisition. Update the executor's working map only after that
commit succeeds. Without a shared runtime cell, perform the same validation and
snapshot evaluation before changing the working map.

Do not infer whether a prior runtime mutation exists from its value. An
explicitly assigned null is present state, not the sentinel for absence. The
batch path must therefore use key presence when selecting the effective prior
value; this also corrects the existing single-key ambiguity where a runtime
null can incorrectly fall back to a non-null document value.

### R4. Result and persistence semantics

Where the side-effect protocol exposes an action's result, return one object
mapping each assigned key to its value in the pre-action effective state, using
null for an absent key. An explicitly null prior value also returns null; the
result intentionally does not distinguish those two cases. An empty update
returns an empty object. Keep the surrounding task output serialization
contract unchanged: a sequence `side_effect` task serializes this object as its
ordinary textual output and appends that text as one `outputs` entry.

Successful updates remain visible to subsequent actions, loop iterations,
serial tasks, and later steps according to the existing runtime-state contract.
Authored `state`, `previous`, and `next` views remain immutable. No source file
is modified by lifecycle `set`.

The underlying Darkmatter `EffectEngine::set` API, `EFFECT_DESCRIPTORS` row,
`claudine context --side-effects` capability signature, and separate
loop-control DSL are outside this syntax change. Do not remove or relabel a
general-purpose Rust, expression, or capability API merely because its name is
also `set`. Documentation that presents both surfaces must explicitly label
`set(key, value)` as the capability/loop API and `set: {property: value}` as
Claudine lifecycle YAML.

### R5. Diagnostics and migration

Malformed payloads and removed forms must produce a typed, actionable error
identifying the source document and a stable semantic action path, for example
`initialize.stack[0].action[1].set.epilog` or
`tasks[2].side_effect.set.ready`. Show the accepted shape
`set: {property: value}`. A bad destination must identify the specific key; a
failed expression must identify its value path, including nested object keys
and array indexes. Attach the existing frontmatter excerpt when the current
diagnostic infrastructure can locate it, but do not introduce a second YAML
parser solely to manufacture line and column data.

Use dedicated typed parse/evaluation/dispatch errors as needed and carry them
through the existing `Diagnostic`/`BlockError` selection seam. Rendering,
`err.*`, and machine output must select the same effective diagnostic. Reuse an
existing catalog code when the author remedy and disposition are unchanged;
if a new code or detail field is justified, update the catalog and its guard
tests in the same change. Do not reuse the current instruction to avoid object
values or recommend a removed form.

Migrate active prompts, active feature/fix specifications, executable fixtures,
examples, and tests using the old lifecycle forms. Update lifecycle and
sequence documentation, the Claudine skill, and any Claudine schema or action
descriptor that advertises accepted YAML forms. Keep historical completed
specs intact unless they are executable inputs. Do not rewrite loop-control
`set(...)`, Darkmatter descriptor examples, or unrelated APIs found by a broad
text search. Review comments on behavior-changing symbols for drift.

## Acceptance Criteria

1. Both reported mappings in `implement-plan.md` parse and execute through the
   normal lifecycle path. A false surrounding condition does not cause a valid
   mapping to be rejected during parsing.
2. The mapping shape works across event stacks, task setup/teardown, and
   sequence side-effect tasks through the shared parser and executor.
3. Tests cover one key, multiple keys, an empty mapping, native versus quoted
   scalars, null, arrays, nested objects, typed whole-value expressions,
   recursive interpolation, and `no_error` both as a modifier and as a literal
   destination key.
4. The swap example succeeds in either key order. Separate consecutive actions
   observe each other's successful updates.
5. Invalid/reserved keys and failed expressions produce no partial update in
   either the shared runtime cell or working map. Cover a failure following an
   otherwise valid entry, with and without a shared runtime cell. Cover an
   explicitly null prior runtime value over a non-null document value.
6. Removed positional, explicit `action: set`, and any formerly accepted
   lifecycle call syntax fail with migration guidance. Nonmapping payloads
   cannot silently fall back to another form.
7. Observable action results report the prior-value mapping, including absent
   keys and an empty update. A sequence side-effect task serializes that object
   through the existing output path. Downstream actions and sequence steps see
   the new state; source files remain unchanged.
8. Passive corpus coverage checks active shipped artifacts. A hermetic CLI
   regression exercises the real implementation prompt's mapping shape and
   normal invocation path, isolating this syntax change from the separately
   specified initialization-ordering bug.
9. Existing runtime-state isolation, reserved-key protection, and other verbs'
   syntax remain covered. Loop-control `set(...)` and Darkmatter's positional
   API remain covered. No compatibility branch for removed Claudine lifecycle
   `set` forms remains in production parsing or execution.
10. Diagnostics name the source and full semantic value path, offer only the
    canonical mapping rewrite, and retain typed identity across terminal,
    `err.*`, and machine projections. Static malformed mappings fail even under
    a false `when`; valid guarded mappings do not evaluate or mutate.

## Validation and Completion

Use nextest-backed package-area `just test` and appropriate lint checks.
For the shared parser change, also run `just test` and `just lint` in
`darkmatter/`. Cover top-level and nested duplicates (including mappings in
arrays), each fallback path, valid expression-bearing values, and unchanged
explicit document merging. Include passive shipped-artifact coverage and a
hermetic end-to-end regression through normal CLI invocation. Run the tests
consuming the migrated DMLS fixture.

CLI tests must use `CliProcessFixture`, fake providers, isolated state, and
suppressed lifecycle audio. Tests must not focus terminal/browser windows or
invoke real providers. Ensure the implementation works on macOS, Linux, native
Windows, and WSL2; reuse qualifying passing evidence and run missing required
coverage according to repository policy.

Before changing code, run the required GitNexus impact analysis and identify
the shared action parsing, runtime mutation, schema/catalog, and sequence
consumers. Prefer a single mapping-based action representation over translating
the new syntax into a sequence of old single-key actions, which would violate
snapshot and all-or-nothing semantics. Add a batch operation at Claudine's
runtime-state boundary; do not broaden or break Darkmatter's public
single-property mutation API to implement a Claudine-only transaction.

Completion requires migrated active artifacts, updated docs and skill guidance,
passing targeted regression and relevant broader checks, and recorded validation
evidence. This specification does not authorize implementation or a commit.
