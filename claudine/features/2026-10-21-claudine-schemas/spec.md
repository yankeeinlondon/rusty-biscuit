---
area: claudine
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
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
---
# Claudine Schemas

Claudine gets real schemas. An author editing a Claudine prompt in an editor
running DMLS (the Darkmatter language server) sees typed lifecycle events,
completion and hover for Claudine's frontmatter properties, and an error where
a lifecycle event uses an action it cannot use — for example a `shell` action
in `initialize`. The same schemas join Claudine's launch validation, so a
run catches the same mistakes before it starts. The schemas are
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

- Darkmatter discovers schema roots by an **ancestor walk**: from the
  document's folder it looks for a `schemas/` directory in each parent up to
  the repository root (the workspace root in DMLS), nearest first; a file name
  in a nearer root shadows the same name in a farther one. `md schema triggers`
  lists the roots, the triggers, and which match arm fired.
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
| `claudine.yaml` | `trigger-schema` | matches `any:` of the six event keys or `agent`/`model`, each arm a concrete type with `required` (below); the payload is Claudine's frontmatter catalog |
| `inline-compose.yaml` | `trigger-schema` | matches `prompt: string(required)` with `none: { sequence: ["unknown[](required)", string] }`; the payload is the inline-compose document shape |

Trigger arms use concrete types, because `required` has no effect on
`any`/`unknown` (decided 2026-10-11): `agent: string(required)`,
`model: string(required)`, and each event key as the union of a list and a
mapping, for example `start: ["unknown[](required)", object]`. A `sequence`
value is a list of steps or a file-reference string, hence
`["unknown[](required)", string]`.

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
except `set`, which has no long form. The `shell` verb's command is typed with
the built-in `shell` type:

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
  `shell`**: that is how "no shell in `initialize`" is expressed. The editor's
  error for a `shell` in `initialize` is an ordinary schema-validation error;
  there is no special rule.
- `start`, `blocked`, `success`, `failure`, `finalize`, and the `loop:` block
  (its loose keys and its `gate:`) allow all four.

Because there is no object-merge operator, a **dictionary bundle** (several
verbs in one mapping, with optional `when` and `no_error`) cannot be written as
a union of the action types. `action.yaml` therefore carries two bundle types
whose keys are exactly the union of the allowed action types' keys plus `when:
expression` and `no_error: boolean`: `bundle` (all four) and
`initialize-bundle` (no `shell`). An L1 test (AC4) keeps each bundle's key set
equal to the union of its action types', so the copies cannot drift.

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
  The parser's nesting limit of five is unaffected: the schema never rejects a
  legal document and never rejects depth 6; only the parser does.
- **Generated verbs as untyped literal keys.** The 21 hand-authored verbs are
  typed with literal keys. The side-effect verbs and expression functions are
  also listed in `expression` (and in the bundle types) as literal keys, one
  per name, with untyped (`unknown`) values; Claudine's parser validates their
  values at run time. Decided 2026-10-11: there is no catch-all key, so every
  action object is closed and a misspelled verb key fails every arm (typo
  detection), and `- shell:` in `initialize` fails every arm because no
  `initialize` arm lists `shell`. The resolved size and resolve time of this
  shape are measured once (AC9).
- **The generator is retained** at
  `claudine/features/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py`
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
Untyped literal keys add one key per generated name but no value types, so
they stay close to that band.

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

### YAML is authoritative; the binary embeds it

Decided 2026-10-10 (completing the schema-ownership rule the strict-mode fix
deferred):

- The five files are the only definition of Claudine's schemas. No
  hand-written runtime copy may exist.
- The Claudine library embeds them at build time (`include_str!` of the
  repository files) and exposes them as name/text pairs, parsed and checked by
  an L1 test so a malformed file fails the build's tests, not a user's run.
- DMLS reads the same files from disk through the ancestor walk; DMLS does
  not compile them in and gains no Claudine dependency.

### The schemas join Claudine's launch validation

Decided 2026-10-11. Claudine's schemas join Claudine's existing launch
validation (the `SchemaPhase::Launch` verdict described in
`claudine/docs/topics/composition.md`, and the unphased verdict direct
`compose` uses) with that validation's existing behavior:

- a missing required value is asked for (collected interactively, or failing
  as a missing value when collection is not possible), exactly as for a
  document's authored `$schema`;
- a type error blocks the launch, with the same diagnostic and exit path a
  type error in the authored `$schema` produces today.

Launch validation does not load trigger schemas today: it reads only the
document's authored `$schema`, with no baseline or trigger layers. This spec
makes it load them. The launch verdict validates against the effective schema
Darkmatter computes for the document (matching trigger payloads, from the
embedded files for Claudine's own triggers and from the ancestor walk for
any other `schemas/` root, merged with the authored `$schema`), so the
editor and the launch judge a document by the same schema. Because the
`claudine.yaml` payload declares no `required` property, the visible change
for existing prompts is that a value of the wrong type for a Claudine
property (for example a lifecycle stack the schema rejects) now blocks the
launch, instead of reaching the parser or being dropped by the launch
cleaner.

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

Decided 2026-10-11, built in `2026-10-27-darkmatter-lifecycle-ownership`:
Darkmatter's event-action schema names action keys with `suggest`, not
`enum`. Darkmatter's own actions are typed and suggested; an action
Darkmatter does not know (for example Claudine's `say`) passes Darkmatter's
schema without error and is not suggested. A host's schemas add the host's
actions. Claudine's schemas are such a host schema: they type and offer
every action Claudine runs (`say`, `speak`, `effect`, `notify`, the recovery
directives, `break`, `prep`, …) and keep rejecting a verb key Claudine does
not know, as described above.

### Reaching documents outside `claudine/`

The ancestor walk finds `claudine/schemas/` for any document under `claudine/`.
A document elsewhere reaches the schemas through a `schemas/` directory at the
repository root that contains or links the files, a bare-name pointer
`$schema: claudine.yaml`, or a `.dmls.toml` `[schema.extensions]` entry naming
`claudine/schemas/`. The Darkmatter docs and DMLS config example that showed the
deleted `darkmatter/docs/schemas/claudine.yaml` point at `claudine/schemas/`.

### What the schemas cannot say

The parser remains the sole enforcer of: placement (a directive last in its
list); cardinality (one flow-control key per bundle); no `when` on a root-level
dictionary; `gate:` never beside loose verb keys; `skip` only in `initialize`;
and nesting beyond the schema's three typed levels. A misspelled verb key is
rejected by the schema (no arm lists it) and by the parser; a misspelled long-form
parameter of a hand-authored verb is a schema error, because long forms are
closed objects.

## Out of scope

- `sequence.yaml` and typing task-level keys: `2026-10-26-task-inputs-and-providers`.
- Recursive lifecycle types replacing the unroll, and full value typing of the
  generated verbs: `2026-10-30-claudine-recursive-schemas`.
- Host-registered events, allowed action types per host event, Darkmatter's
  open (`suggest`-based) event-action schema, the `schema-trigger` rename,
  AND/OR trigger conditions: `2026-10-27-darkmatter-lifecycle-ownership`.
- The grammar itself and the deletion of Darkmatter's old Claudine schema
  copies: `2026-10-20-lifecycle-grammar`.

## Acceptance criteria

All L1 unless marked; run with `just test` / `just lint`; portable to macOS,
Linux, native Windows, and WSL2; no window focus; no new CI cell.

1. The five files exist with the kinds above;
   `claudine/schemas/partials/lifecycle.yaml` is gone; every file loads through
   `md schema validate` / Darkmatter's loader without error.
2. `action.yaml` defines `communicate`, `flow`, `shell`, `expression`,
   `bundle`, and `initialize-bundle`; the hand-verb keys across the four action
   types equal the parser's hand-verb inventory, and the generated-verb keys
   equal Darkmatter's side-effect and expression-function catalogs (tests
   compare them).
3. `lifecycle.yaml` defines `stack`, `initialize-stack`, and `loop`, three typed
   levels each, loose bodies below.
4. A test proves each bundle type's key set equals the union of its action
   types' keys plus `when` and `no_error`, and that no action type or bundle
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
   | `shell` as a list item, in a `then` body, and in a root dictionary of `initialize` | invalid, at the offending node |
   | `shell` in `start` | valid |
   | `message` + `shell` bundle with `when` in `success` | valid |
   | `when` with both `then` and a sibling verb | invalid |
   | misspelled long-form parameter (`say: { txet: … }`) | invalid, naming the key |
   | misspelled verb key (`inf0:`) | invalid, naming the key |
   | a generated verb with an arbitrary value (`set_frontmatter: [a, b, c]`) | schema valid (values untyped); the parser judges the value |
   | a linked prompt using all eleven linking keys | no unknown-key warning |

   The fixtures reuse `spike/fixtures/` content from the lifecycle-grammar spec
   directory where it applies; they live under `claudine/lib/tests/fixtures/`
   or `claudine/cli/tests/fixtures/`, read per the `rust-testing` skill.
6. The embedded files are byte-identical to the repository files (the embed is
   `include_str!` of those paths) and each parses as its declared kind.
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
   once with the spike harness and recorded in the implementation log; no
   timing assertion is added to the test suite.
10. **Launch validation loads trigger schemas.** CLI L1 tests through
    `claudine compose` and `claudine inline-compose`, with documents carrying
    no `$schema:` pointer:
    - a lifecycle stack with a misspelled long-form key blocks the launch
      with a diagnostic naming the key, and no provider is spawned (a fake
      provider records that it never ran);
    - a document whose authored `$schema` adds a required property that the
      caller omits is asked for it (or fails as a missing value when
      non-interactive) exactly as before, with Claudine's trigger payload
      also applied;
    - a valid document launches exactly as before (the existing launch
      validation tests pass without their assertions being edited);
    - the launch schema verdict and `md schema validate` agree on the schema
      verdict of every AC5 fixture.
11. `claudine/docs/topics/composition.md`'s launch validation section states
    that Claudine's trigger schemas are part of the launch verdict.
12. `just test` and `just lint` are green in `claudine/` and `darkmatter/`.

## Open questions

None.
