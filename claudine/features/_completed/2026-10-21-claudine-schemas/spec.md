---
area: claudine
canonical_docs:
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/editors/README.md
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
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
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 3
human_review: false
pipeline_step: 11
depends-on:
    - 2026-10-16-core-schema-types
    - 2026-10-20-lifecycle-grammar
related:
    - 2026-10-13-named-type-presence-and-local-names
    - 2026-10-14-union-type-constraints
    - 2026-10-26-task-inputs-and-providers
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-30-claudine-recursive-schemas
implemented: true
completed: true
---
# Claudine Schemas

Claudine gets real schemas. An author editing a Claudine prompt in an editor
running DMLS (the Darkmatter language server) sees typed lifecycle events,
completion and hover for Claudine's frontmatter properties, and an error where
a lifecycle event uses an action it cannot use — for example a `shell` action
in `initialize`. The same schemas join Claudine's launch validation, so a
run catches the same type mistakes before provider launch within the typed levels described below. The schemas are
developer-editable YAML under `claudine/schemas/`; that YAML is authoritative,
the Claudine binary embeds the same files at build time, and DMLS reads the
same files from disk.

```yaml
---
initialize:
    - info: "preparing"
    - shell: "git fetch"     # editor error: initialize's type has no shell action
start:
    - shell: "git status"    # fine
---
```

This material was split out of the lifecycle-ergonomics feature (now
`2026-10-20-lifecycle-grammar`) during the 2026-10-10 restructure, which also
added the action-type design below.

## Background

These Darkmatter facts are relied on. All of them hold once the pipeline steps
before this one have landed.

- Darkmatter searches five schema locations, most local first: the document's
  package `schemas/`, package-area `schemas/`, file-tree-root `schemas/`, the
  absolute directory named by `SCHEMAS_DIR`, and the captured home's `schemas/`.
  These locations come from the document's file-resolution context; intervening
  ancestor folders are not searched. The first file of a given name shadows
  later files of that name, before matching. Discovery ignores file symlinks
  and requires real repository schema directories. `md schema triggers` lists
  the locations, shadowed files, and matching conditions. See Darkmatter's
  [schema location contract](../../../darkmatter/lib/src/markdown/schemas/roots.rs)
  and [trigger discovery](../../../darkmatter/lib/src/markdown/schemas/triggers/discovery.rs).
- Only `kind: trigger-schema` files **auto-apply**: a `match:` block over
  frontmatter, and a `$schema` payload applied when it matches. A match arm
  needs a presence-requiring condition written with a concrete type
  (`agent: string(required)`; `agent: string` is rejected as vacuous, and so
  is `agent: any(required)`, because `required` has no effect on `any`/`unknown`
  since `2026-10-16-core-schema-types`). Combinators: `all`/`any`/`none`/`min-match`, `$path`
  globs. (The `trigger-schema` → `schema-trigger` rename belongs to
  `2026-10-27-darkmatter-lifecycle-ownership`; this spec uses the spelling the
  code accepts when it lands.)
- `kind: schema` files are **type libraries**, holding only `kind` and
  `types`; they reach a document only when imported.
- A type defined in the same file is referenced by its bare name; `@` is for
  another file (`bundle@./action.yaml`). A built-in type keyword wins a name
  collision with a local type; `@this` selects the local one
  (`2026-10-13-named-type-presence-and-local-names`).
- `required` survives every named-type reference (same spec).
- `[]` applies to a reference to a union-typed named type, and a list of a
  union lowers to `items: { anyOf }` (`2026-10-14-union-type-constraints`).
- `shell` is a built-in SimplifiedSchema type like `expression`: a string that
  holds a shell command (`2026-10-16-core-schema-types`).
- SimplifiedSchema has **no recursion** and no object-merge operator; named
  types are substituted structurally at each reference. Unions are written as
  YAML lists (the `|` operator arrives with `2026-10-28-schema-enhancements`).
- A `<string>` catch-all key lowers to `additionalProperties: <type>`; literal
  keys win; `<pattern::RE>` lowers to `patternProperties`, lookarounds allowed.

## Decided

### Six files become five; the placeholder goes

Five files under `claudine/schemas/` (decided 2026-10-10; `sequence.yaml` moved
to `2026-10-26-task-inputs-and-providers`):

| File | Kind | Holds |
|---|---|---|
| `action.yaml` | `schema` | the four action types and the bundle types (below) |
| `lifecycle.yaml` | `schema` | the stack types, unrolled three levels, and the `loop:` block type |
| `linking.yaml` | `schema` | the eleven optional linking keys: `name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns` |
| `claudine.yaml` | `trigger-schema` | matches a non-null value of any lifecycle event key, `loop`, `prompt`, `agent`, or `model`; the payload is Claudine's frontmatter catalog |
| `inline-compose.yaml` | `trigger-schema` | matches non-null `prompt` with no non-null `sequence`; adds inline document metadata types without requiring execution inputs |

Trigger selection identifies a document; it must not require the value to
already have its correct type. Otherwise `start: 42` or `agent: []` alone
would evade the schema intended to reject it. For each identifying property,
use the union of all non-null YAML value types, with required presence:

```yaml
match:
    any:
        - start: [string(required), number, boolean, object, "unknown[]"]
        # Repeat for initialize, blocked, success, failure, finalize,
        # loop, prompt, agent, and model.
```

This follows the non-null union contract in `2026-10-16-core-schema-types`;
`unknown(required)` cannot test presence. Missing and explicit-null identifying
values do not activate these triggers. Null optional values keep Darkmatter's
existing optional-value semantics; the command's parser can impose stricter
execution requirements.

The inline trigger combines the same non-null `prompt` condition with
`none:` containing the same non-null union for `sequence`. Thus a malformed
but non-null `sequence` does not accidentally classify a sequence as an
inline document. Its payload does not redeclare shared properties with
weaker types: duplicate declarations agree with `claudine.yaml`. In particular,
`prompt` is an optional string in the payload, not required or eager;
`inline-compose` retains its own missing-prompt check after caller overlays.
`sequence` is an optional union of an untyped list and a file-reference string;
task-level typing remains deferred. Both matching payloads are mergeable
root objects, not root unions.

The placeholder `claudine/schemas/partials/lifecycle.yaml` is deleted (and the
empty `partials/` directory with it). Its claim that Claudine's `success`
renames a Darkmatter `completed` event was never true. The current near-empty
`claudine/schemas/claudine.yaml` is replaced. `claudine/schemas/review.yaml` is
untouched.

### Lifecycle events are typed as unions of action types

Decided 2026-10-10. `action.yaml` defines four reusable **action types**, one
per category of the v1 Action Inventory in `2026-10-20-lifecycle-grammar`:

| Type | Verbs |
|---|---|
| `communicate` | `say`, `speak`, `effect`, `message`, `notify`, `stderr`, `stdout`, `info`, `warn`, `success` |
| `flow` | `stop`, `skip`, `error`, `proxy`, `retry`, `resume`, `defer`, `break`, `prep` (the last three marked planned in their descriptions) |
| `shell` | `shell` |
| `expression` | `set`, plus the Darkmatter side-effect verbs and expression functions (listed as untyped literal keys; see [Interim shape](#interim-shape)) |

Each verb's value is the union of its short form and its long form, exactly as
the Action Inventory gives them; every long form carries `no_error: boolean`
except `set`, which has no long form. Zero-argument verbs include the parser's
null, empty-array, and supported long-form spellings; required payload fields
inside a supplied long form remain required even though the verb itself is
optional. Do not mark a reusable action type required: that would make an
event mandatory wherever the type is imported. The `shell` verb's command
is typed with the built-in `shell` type:

```yaml
kind: schema
types:
    shell:
        shell:
            - shell -> run a shell command (not in initialize)
            - "{ command: shell(required), on_error: string, no_error: boolean }"
```

(Inside the local type named `shell`, the bare word `shell` is the built-in
type, because built-in keywords win. Same-file references to the local types
named `shell` and `expression` are written `shell@this` and `expression@this`;
from `lifecycle.yaml` they are `shell@./action.yaml` and
`expression@./action.yaml`.)

An event's type says which action types it allows:

- `initialize` allows `expression | flow | communicate`. Its type **excludes
  `shell`** at the three typed levels: that is how direct shell actions in
  `initialize` are rejected by the schema. The editor's
  error for a `shell` in `initialize` is an ordinary schema-validation error;
  there is no special rule.
- `start`, `blocked`, `success`, `failure`, `finalize`, and the `loop:` block
  (its loose keys and its `gate:`) allow all four.

Because there is no object-merge operator, a **dictionary bundle** (several
verbs in one mapping, with optional `when` and `no_error`) cannot be written as
a union of the action types. `action.yaml` therefore carries two bundle types
whose keys are exactly the union of the allowed action types' keys plus `when:
expression` and `no_error: boolean`: `bundle` (all four) and
`initialize-bundle` (no `shell`). The bundle copies only each verb's **short-form** value type: the grammar
forbids long-form actions in bundles. A single-verb list item can use the
category arm for a long form; a root dictionary or an item with sibling verbs
must match the short-form bundle. `set` remains a mapping of arbitrary
frontmatter destinations to values. An L1 test keeps bundle keys and short-form
value types aligned with the categories, so these copies cannot drift.
Root dictionaries are also checked by the parser for their placement rules.

A stack at each typed level is the union of a list of items and a bundle. A
list item is a union of:

- each allowed action type (a single-verb item matches one of these, and its
  hover names the category),
- the matching bundle type (a multi-verb item, optionally guarded),
- a conditional item `{ when: expression(required), then: <body>(required),
  else: <body> }` whose bodies are the next level's stack type,
- a bare-name arm: an `enum` of exactly the verbs the parser accepts as a bare
  list item (`- stop`). This spelling was parser-only while Darkmatter could
  not type a list over a union; that limit is gone.

Because bundle and conditional are separate arms, the editor now also rejects
`then`/`else` beside sibling verbs; the parser still enforces it too.

```yaml
# lifecycle.yaml (shape, abbreviated)
kind: schema
types:
    stack:
        - item-1[]
        - bundle@./action.yaml
    item-1:
        - communicate@./action.yaml
        - flow@./action.yaml
        - shell@./action.yaml
        - expression@./action.yaml
        - bundle@./action.yaml
        - conditional-1
        - bare-verb
    conditional-1:
        when: expression(required)
        then: stack-2(required)
        else: stack-2
    # ... stack-2/item-2/conditional-2, stack-3/item-3/conditional-3,
    # then the loose body below level 3; and the initialize-* family without shell
```

### Interim shape

The shape the schema spike chose (recorded in the lifecycle-grammar spec
directory's `spike-results.md`) stays, decided 2026-09-28 and unchanged on
2026-10-10:

- **Three fully typed levels.** Conditional items at nesting depths 1–3 are
  typed; a `then`/`else` body inside a depth-3 conditional is a loose
  list-or-map (`[any[], "{ <string>: any }"]`) accepted without deep checking.
  The parser's nesting limit of five is unaffected: legal deep stacks remain
  accepted by the schema, and only the parser rejects depth 6. Below the
  typed levels, misspelled verbs, malformed long forms, mixed conditional and
  sibling actions, and `initialize` shell actions may pass schema validation.
  The parser still rejects them before dispatch. Editor validation alone is
  therefore not proof that a lifecycle stack can execute.
- **Generated verbs as untyped literal keys.** The hand-authored verbs are
  typed with literal keys. The side-effect verbs and expression functions are
  also listed in `expression` (and in the bundle types) as literal keys, one
  per name, with untyped (`unknown`) values; Claudine's parser validates their
  values at run time. Decided 2026-10-11: there is no catch-all key, so every
  action object within the three typed levels is closed and a misspelled verb key fails every arm (typo
  detection), and `- shell:` in `initialize` fails every arm because no
  typed `initialize` arm lists `shell`. This does not detect a shell expansion
  hidden in a `set` value; initialization's existing parser rejection and
  runtime backstop remain authoritative. The resolved size and resolve time of this
  shape are measured once (AC9).
- **The generator is retained** at
  the `spike/gen-action-schema.py` script in `2026-10-20-lifecycle-grammar`
  so full typing of the generated verbs can be switched on later. The
  production files are hand-maintained YAML; nothing regenerates them in the
  build. A test keeps the generated-verb key list equal to Darkmatter's
  side-effect catalog (`darkmatter/lib/src/effects/catalog.rs`) and
  expression-function catalog (AC2).
- Recursion and full value typing of the generated verbs replace the unroll
  and the untyped keys in `2026-10-30-claudine-recursive-schemas`.

The interim shape was chosen because DMLS re-resolves on every validation and
the spike measured the catch-all band (about 1.4 MB resolved, 82 ms to
resolve, 266 ms per CLI validate, on the spike's host) as the only cheap one.
Untyped literal keys add one key per generated name but no value types. The
existing figures justify the three-level limit; they do not measure the final
closed-key shape. The single quick measurement below checks that change on
one host, without additional workloads or timing thresholds.

### The `loop:` block type

`lifecycle.yaml`'s `loop` type holds the iteration controls as they exist when
this lands (`while`/`until` as `expression`, `action`/`actions`, `max`,
`fail_fast`, `on_rate_limit`), `gate:` typed as `stack`, and the loose verb keys
of all four action types. Mixing `gate:` with loose verb keys and `when` at the
loop root stay parser errors; the schema does not attempt them.

### The `claudine.yaml` payload

The payload types every Claudine frontmatter property listed in
`claudine/docs/topics/frontmatter-properties.md` at implementation time, which
on 2026-10-10 means:

- composition core: `prompt`, `agent`, `model`, `interactive`, `$schema`,
  `last_updated`, `hash`, `fail_fast`, `sequence`, `loop`, `argv`, `title`,
  `description`
- the six lifecycle events: `initialize` typed as `initialize-stack`, the other
  five as `stack`, all from `lifecycle.yaml`
- timeouts: `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn`
- run controls: `exit_expressions`, `guard_settings`, `operation`, `yolo`,
  `verbosity`, `mode`
- the sequence overlay properties the page lists
- the linking keys, imported from `linking.yaml`, so a linked prompt raises no
  DMLS unknown-key warning

The payload declares no `required` property: it only adds type information
where a key appears. A later spec that adds, renames, or removes a frontmatter
property updates this payload in the same change (Drift Maintenance).

### Public frontmatter stays open

Only lifecycle action objects and their hand-authored parameter maps are
closed. The document's top-level payload remains open so a prompt can keep
application data such as `ticket: 123` beside Claudine controls. Linking types
must preserve supported input forms, including string and string-list tool
lists, and string or list argument hints; use the linking reader's accepted
forms rather than narrowing them to one provider's preferred output format.
Opaque user metadata and `with` overlays remain open mappings. Planned verbs
are documented separately from executable verbs: schema acceptance of `break`
or `prep` does not make the current parser or runtime support them.

### YAML is authoritative; the binary embeds it

Decided 2026-10-10 (completing the schema-ownership rule the strict-mode fix
deferred):

- The five files are the only definition of Claudine's schemas. No
  hand-written runtime copy may exist.
- The Claudine library embeds them at build time (`include_str!` of the
  repository files) and exposes them as name/text pairs, parsed and checked by
  an L1 test so a malformed file fails the build's tests, not a user's run.
- Embedded imports such as `stack@./lifecycle.yaml` and
  `shell@./action.yaml` resolve relative to the embedded bundle, without
  requiring a checkout or writing temporary schema files. Disk imports retain
  their disk origin and use the ordinary file-reference resolver; they must
  not silently switch to an embedded file with a matching basename. Resolving
  every exported type and both payloads is part of the loader test.
- DMLS reads the same files from disk through the five schema locations;
  DMLS does not compile them in and gains no Claudine dependency.

### Embedded schemas are the last-resort location

Decided 2026-10-11. The embedded bundle is a final fallback location, not a
baseline or a second validation pass:

- Discover disk files through the existing schema locations with their
  existing filename shadowing. An on-disk copy of a Claudine trigger always
  wins over the embedded one.
- Only a Claudine trigger with no on-disk copy is supplied from the embedded
  bundle. Each winning file resolves its imports from its own origin (disk
  imports from disk, embedded imports from the embedded bundle).
- The authored `$schema` is applied last, as today. Darkmatter's merge order
  (baseline, matching triggers, authored `$schema`) and its per-top-level
  property replacement are unchanged.
- The strict launch policy below applies to properties whose winning
  declaration is a Claudine declaration, whichever origin supplied it. An
  override can therefore weaken Claudine's schema; the parser still enforces
  execution rules regardless.
- A malformed discovered trigger fails visibly; it never silently falls back
  to the embedded copy.
- An installed command works without a checkout. The editor still needs a
  disk bundle, since DMLS does not compile the schemas in.

### Retiring Darkmatter's old Claudine schema copies

`darkmatter/docs/schemas/claudine.yaml` and
`darkmatter/docs/schemas/claudine-types.yaml` were kept working by
`2026-10-20-lifecycle-grammar` so editor users had lifecycle validation until
this replacement existed. Both are **deleted** here, once `claudine/schemas/`
is their one home. Every Darkmatter test, config example, or doc that loads
either file (find them with
`grep -rln "claudine-types.yaml\|docs/schemas/claudine.yaml" darkmatter`; at
inventory: `dmls/tests/l1/mapping_only_corpus.rs`, `dmls/src/config/mod.rs`,
`dmls/src/providers/frontmatter/sequence_tests.rs`, `dmls/src/overlay/mod.rs`,
`dmls/docs/editors/README.md`, `docs/topics/schemas/dmls-schema-support.md`)
stops depending on it. A test whose subject is not Claudine's schema uses an
inline schema of its own; a doc or config example that showed the Claudine
extension points at `claudine/schemas/`.
`lifecycle_inventory_matches_the_authored_claudine_schema` moves to
`claudine/schemas/` in the same change (criterion 7), so the coverage never
lapses. A baseline that changes because the old extension is gone
(`mapping_only_corpus`) is re-blessed with `DMLS_BLESS_MAPPING_CORPUS=1`, with
the reason recorded in the implementation log.

### The schemas join Claudine's launch validation

Decided 2026-10-11. Claudine's schemas join Claudine's existing launch
validation (the `SchemaPhase::Launch` verdict described in
`claudine/docs/topics/composition.md`, and the unphased verdict direct
`compose` uses), preserving its phase rules and explicitly changing the
cleanup policy for Claudine controls:

- `compose` keeps its unphased authoring verdict; `inline-compose` keeps
  `SchemaPhase::Launch`, which defers missing required non-eager values until
  completion. Missing required-eager values use existing interactive
  collection, or fail when collection is unavailable. Trigger selection does
  not add any required execution inputs.
- Invalid Claudine control values block provider launch. This is an **intended
  change** from dropping invalid optional non-eager values and retrying.
  Apply this strict policy to properties whose winning declaration comes from
  Claudine's payload; unrelated authored application properties retain their
  existing cleanup policy. Preserve winning-layer provenance so the cleaner
  cannot discard `start` and launch without the author's intended actions.
  Schema sources follow the last-resort rule above.

Launch validation does not load trigger schemas today: it reads only the
document's authored `$schema`, with no baseline or trigger layers. This spec
makes it load them. The launch verdict validates against the effective schema
Darkmatter computes for the document (matching trigger payloads, from the
five document-context schema locations, with the embedded bundle as the
last-resort location, merged with the authored `$schema`), so the
editor and launch use the same resolution and validation rules when supplied
the same schema sources and input snapshot. Installed Claudine has embedded
schemas even when the editor has no disk bundle; parity does not imply that
the editor can discover those embedded files. Because the
`claudine.yaml` payload declares no `required` property, the visible change
for existing prompts is that a value of the wrong type for a Claudine
property governed by a winning Claudine declaration (for example a lifecycle
stack the schema rejects) now blocks the launch, instead of reaching the parser or being dropped by the launch
cleaner.

Validation remains passive: it does not execute lifecycle expressions or shell
commands. Preserve deferred lifecycle interpolation and pending authored
values, and decode literal data tokens using Darkmatter's existing validation
rules. Do not move the stabilized launch verdict ahead of `initialize`:
initialization remains shell-free and precedes that verdict. `start` and the
provider cannot run after a failed verdict; early failure/finalize handlers
retain their shell prohibition. Retries, resumes, proxy targets, and sequence
prompt preparation use this same schema seam at their existing fresh-read
boundaries. Inline completion uses the effective schema frozen at launch;
agent edits to `$schema` take effect on the next run. No new per-action or
per-loop-iteration schema resolution is introduced.

```yaml
---
prompt: "review the diff"
start:
    - shell: { cmd: "git status" }   # misspelled long-form key
---
```

`claudine compose` of this document fails launch validation naming the
`start` stack and the unknown key `cmd`, and no provider is launched.

### Darkmatter's own action schema is open; Claudine's is closed

Planned follow-up in `2026-10-27-darkmatter-lifecycle-ownership`:
Darkmatter's event-action schema names action keys with `suggest`, not
`enum`. Darkmatter's own actions are typed and suggested; an action
Darkmatter does not know (for example Claudine's `say`) passes Darkmatter's
schema without error and is not suggested. A host's schemas add the host's
actions. Claudine's schemas are such a host schema: they type and offer
Claudine actions within the three typed levels (`say`, `speak`, `effect`,
`notify`, the recovery directives, `break`, `prep`, …) and keep rejecting a verb key Claudine does
not know, as described above.

### Reaching documents outside `claudine/`

The package-area location makes `claudine/schemas/` available to documents
whose context identifies Claudine as their package area. An arbitrary nested
`schemas/` directory does not activate automatically. Outside that area,
place a complete regular-file bundle in the document's repository-root
`schemas/`, set `SCHEMAS_DIR` to the absolute path of a complete bundle, or
install it in `~/schemas`. Copy all five files together so relative imports
resolve; linking individual files is not a supported trigger-discovery method.

A bare `$schema: claudine.yaml` is **not** a route to a trigger payload:
Darkmatter rejects trigger-envelope files as payload references to prevent
cycles. This feature provides automatic trigger selection through the schema
locations above; it does not add another explicit-schema entry point. Do not
claim `.dmls.toml` extension settings configure the CLI too. The Darkmatter docs
and editor example that reference the old Darkmatter-owned Claudine schema
must describe these supported locations and the complete bundle.

### What the schemas cannot say

The parser remains the sole enforcer of: placement (a directive last in its
list); cardinality (one flow-control key per bundle); no `when` on a root-level
dictionary; `gate:` never beside loose verb keys; `skip` only in `initialize`;
and nesting beyond the schema's three typed levels. A misspelled verb key is
rejected by the schema within its typed levels (no arm lists it) and by the
parser. Within those levels, a misspelled long-form parameter of a
hand-authored verb is a schema error because long forms are closed objects.

## Out of scope

- `sequence.yaml` and typing task-level keys: `2026-10-26-task-inputs-and-providers`.
- Recursive lifecycle types replacing the unroll, and full value typing of the
  generated verbs: `2026-10-30-claudine-recursive-schemas`.
- Host-registered events, allowed action types per host event, Darkmatter's
  open (`suggest`-based) event-action schema, the `schema-trigger` rename,
  AND/OR trigger conditions: `2026-10-27-darkmatter-lifecycle-ownership`.
- The grammar itself: `2026-10-20-lifecycle-grammar`.

## Acceptance criteria

All L1 unless marked; run with `just test` / `just lint`; portable to macOS,
Linux, native Windows, and WSL2; no window focus; no new CI cell.

1. The five files exist with the kinds above;
   `claudine/schemas/partials/lifecycle.yaml` is gone; every file loads through
   `md schema validate` / Darkmatter's loader without error.
2. `action.yaml` defines `communicate`, `flow`, `shell`, `expression`,
   `bundle`, and `initialize-bundle`; the hand-verb keys across the four action
   types equal the parser's hand-verb inventory plus explicitly documented planned verbs; assert that
   planned-only entries are exactly `break` and `prep` until implemented.
   The generated-verb keys
   equal Darkmatter's side-effect and expression-function catalogs (tests
   compare them).
3. `lifecycle.yaml` defines `stack`, `initialize-stack`, and `loop`, three typed
   levels each, loose bodies below.
4. A test proves each bundle type's key set equals the union of its action
   types' keys plus `when` and `no_error`; bundle value types are the
   corresponding short forms. Resolved action and parameter objects have
   `additionalProperties: false`, while document and user metadata mappings
   stay open. No action type or bundle
   has a catch-all (`<string>` or `<pattern::…>`) key.
5. A fixture suite, trigger-discovered with no `$schema:` pointer, validated
   through `md schema validate` and through Darkmatter's
   `with_trigger_discovery` + `effective_for` + `validate` path DMLS uses, has
   these verdicts:

   | Fixture | Verdict |
   |---|---|
   | stacks at depth 1, 3, 5 | valid |
   | depth 6 | schema valid; the parser rejects it |
   | root dictionary; bundle as a `then` body; `loop:` with `gate:`; long forms | valid |
   | bare `- stop` item | valid |
   | `shell` as a list item, in a `then` body within the typed levels, and in a root dictionary of `initialize` | schema invalid; diagnostic identifies the event and offending key |
   | `initialize` shell action below the typed levels, or shell expansion in `set` | schema may accept; parser rejects before dispatch |
   | `shell` in `start` | valid |
   | `message` + `shell` bundle with `when` in `success` | valid |
   | `when` with both `then` and a sibling verb | invalid |
   | misspelled long-form parameter (`say: { txet: … }`) | invalid, naming the key |
   | misspelled verb key (`inf0:`) within the typed levels | invalid, naming the key |
   | long-form action inside a multi-verb bundle or root dictionary | invalid |
   | only `start: 42`, only `agent: []`, or only `loop: 42` | trigger matches; payload rejects the value |
   | empty frontmatter or unrelated `ticket: 123` | Claudine triggers do not match |
   | valid Claudine controls beside `ticket: 123` | valid; application data is preserved |
   | `prompt` with non-null malformed `sequence` | inline trigger does not match; Claudine payload rejects `sequence` |
   | a generated verb with an arbitrary value (`set_frontmatter: [a, b, c]`) | schema valid (values untyped); the parser judges the value |
   | a linked prompt using all eleven linking keys | no unknown-key warning |

   The fixtures reuse `spike/fixtures/` content from the lifecycle-grammar spec
   directory where it applies; they live under `claudine/lib/tests/fixtures/`
   or `claudine/cli/tests/fixtures/`, read per the `rust-testing` skill.
6. The embedded files are byte-identical to the repository files (the embed is
   `include_str!` of those paths) and each parses as its declared kind.
   Resolve all embedded imports with
   disk schemas absent and the test CWD unrelated to the checkout; the
   installed binary must not depend on repository paths.
7. DMLS: `lifecycle_inventory_matches_the_authored_claudine_schema` is
   re-created against `claudine/schemas/` (event keys from `claudine.yaml`'s
   payload, communication verbs from `communicate` in `action.yaml`) and
   passes; DMLS picks up the trigger for a document under `claudine/`.
8. `darkmatter/docs/topics/schemas/dmls-schema-support.md`, the DMLS editor
   README, and `claudine/docs/` gain a page (or section) describing the five
   files, the action types, which events allow which types, and how a
   document outside `claudine/` reaches them; `claudine/schemas/README.md`
   records the file classification (library vs trigger).
9. Resolved size and resolve time of the `claudine.yaml` payload are measured
   once on one host with the spike harness and recorded in the implementation
   log; no additional workloads or statistical thresholds, and no timing assertion is added to the test suite.
10. **Launch validation loads trigger schemas.** CLI L1 tests through
    `claudine compose` and `claudine inline-compose`, with documents carrying
    no `$schema:` pointer except the authored-schema preservation case:
    - a lifecycle stack with a misspelled long-form key blocks the launch
      with a diagnostic naming the key, and no provider is spawned (a fake
      provider records that it never ran);
    - a document whose authored `$schema` adds a required property that the
      caller omits is asked for it (or fails as a missing value when
      non-interactive) for `compose`; `inline-compose` defers non-eager
      required absence and rejects required-eager absence exactly as before, with Claudine's trigger
      payload also applied;
    - a valid document launches exactly as before (the existing launch
      validation tests pass without their assertions being edited);
    - passive validation agrees across CLI and editor for the lifecycle
      fixture suite with the same sources; compare launch results on the
      same effective values and phase, allowing the documented distinction
      between passive validation and composition coercion;
    - malformed optional Claudine controls are not scrubbed; unrelated
      authored optional application values retain their cleanup behavior;
    - initialization remains before the launch verdict, and a rejected
      document never dispatches `start` or starts the fake provider;
    - shared preparation and inline completion retain their existing phase,
      pending-value, literal-token, and frozen-schema behavior.
11. `claudine/docs/topics/composition.md`'s launch validation section states
    that Claudine's trigger schemas are part of the launch verdict.
12. `just test` and `just lint` are green in the affected Claudine and
    Darkmatter packages, including DMLS schema tests. CLI tests use the
    established isolated process fixtures and fake providers; they never
    launch live agents, audio, or focus terminal windows.
13. Schema-location fixtures prove nearest-name shadowing, ignored file
    symlinks, complete relative-import resolution, and a trigger-envelope
    `$schema` reference failure. Embedded-versus-disk fixtures prove the
    last-resort rule: a same-name disk trigger wins over the embedded copy;
    with no disk bundle the embedded trigger applies; an authored `$schema`
    override still applies last; an invalid discovered trigger fails visibly
    rather than falling back to the embedded copy.
14. `darkmatter/docs/schemas/claudine.yaml` and
    `darkmatter/docs/schemas/claudine-types.yaml` are deleted, and nothing in
    the repository outside spec snapshots names them; `mapping_only_corpus`
    passes (re-blessed if needed, with the reason logged).

## Open questions

None.
