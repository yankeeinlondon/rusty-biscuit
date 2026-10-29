---
area: claudine
canonical_docs:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/lifecycle/index.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/inline/interpolation.md
status: finalized-spec
created: 2026-09-21
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
review_iterations: 2
human_review: false
pipeline_step: 10
formerly: 2026-09-21-lifecycle-ergonomics
depends-on:
    - 2026-10-18-fm-namespace
related:
    - 2026-10-19-speak-prepare
    - 2026-10-21-claudine-schemas
    - 2026-10-22-tts-prewarm
    - 2026-10-24-sequence-containers
    - 2026-10-27-darkmatter-lifecycle-ownership
implemented: true
completed: true
---
# Lifecycle Grammar

Lifecycle events today have two shapes at once: a dictionary of notification
fields, and a nested `stack:` of `action:` items. This feature replaces both
with one action grammar — an ordered list with a compact dictionary form —
and adds `else`,
nesting, and a list home for loop concerns. Authors write less, read order
directly off the page, and get a typed error with a replacement hint for every
old spelling.

```yaml
# before
start:
    message: "starting something"
    stack:
        - action:
            - shell: "git status"

# after
start:
    - message: "starting something"
    - shell: "git status"
```

This is a breaking change. There are no production users, so the old grammar
is removed outright, with no period in which both are accepted.

**Terms.** A **lifecycle event** is one of the six frontmatter keys Claudine
fires at a fixed point of a composition run: `initialize`, `start`, `blocked`,
`success`, `failure`, `finalize`. (Claudine keeps exposing the names `start`
and `blocked`; the host-naming machinery that maps them onto Darkmatter-owned
moments arrives with `2026-10-27-darkmatter-lifecycle-ownership`.) A
**stack** is any place the lifecycle action grammar is accepted: the six
events, the `loop:` block's lifecycle concerns (see
[Loop Lifecycle](#loop-lifecycle)), and a sequence task's `setup:` and
`teardown:`, which the same lifecycle parser reads today. Everything this spec
says about an event's grammar holds for every stack. An **action** is one thing
a stack does (print a line, speak, run a shell command, change flow). A
**flow-control directive** is the subset of actions that decide what the run
does next (`stop`, `skip`, `error`, `retry`, `resume`, `proxy`, `defer`, and the
planned `break` and `prep`) rather than producing a side effect.

Expressions in the examples use the `fm` root for frontmatter, which
`2026-10-18-fm-namespace` introduces before this feature lands; a bare name
still reads frontmatter through the implicit namespace.

## Package ownership

Everything that defines the grammar lands in **Claudine**: the parser, the
validator, the executor's audio-ordering change, and the migration. Darkmatter
gains no lifecycle parser or executor.
Darkmatter's existing Claudine schema extension, editor path classifiers, and
tests must move with the grammar, as listed in [Migration Sweep](#migration-sweep).
These edits preserve existing integration surfaces; the lifecycle parser and
executor remain in Claudine.

The parser and action-model slice — `parse.rs`, `action_shape.rs`,
`actions.rs`, `validate.rs`, `signatures.rs`, and `source_map.rs` under
`claudine/lib/src/composition/lifecycle/` — is written **lift-ready**: it may
not import Claudine runtime types, specifically global settings
(`crate::events::GlobalSettings`, `crate::config`), the provider layer,
messaging, or TTS/Playa types (`biscuit_speaks`, `playa`). AC9 enforces this
with a guard test. Today `parse.rs` checks `effect` names against
`playa::SoundEffect::from_name`; that check is handed to the parser by its
caller (a name predicate or catalog the caller supplies), so the error
(`LifecycleUnknownEffect`) and its timing are unchanged while the import goes. The point is that
`2026-10-27-darkmatter-lifecycle-ownership` lifts the slice into Darkmatter as
a unit without untangling it first.

## Decided

### Ergonomic shift

The preferred event form is a list; the compact dictionary form expands into
the same action model. There is no `stack:` key and no `action:` aggregator. Unguarded
actions sit at the root of the list:

```yaml
success:
    - message: "build complete"
    - shell: "git status"
```

A guarded block is a `when` item whose actions sit under `then`:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
          - message: "we're starting something in the summer"
          - shell: "run-summer-program"
```

The available actions are unchanged:

- communication: `say`/`speak`, `effect`, `message`, `notify`, `stderr`,
  `stdout`, `info`, `warn`, `success`
- `set`, the lifecycle mutation of frontmatter state
- the side-effect verbs and expression functions Darkmatter provides
  (`set_frontmatter`, `http_post`, `file_exists`, ...)
- `shell` (forbidden in `initialize`, as today)
- flow-control directives

The event-time globals (`err`, `timing`, `current`) are available exactly as
today. The full verb list is the [Action Inventory](#action-inventory).

### Execution model

The parsed stack contains ordered actions and conditional nodes with ordered
branch bodies. Dictionary bundles are expanded during parsing; the executor
never iterates a dictionary to choose action order. Actions dispatch serially.
Playa playback and tracked message delivery keep their existing background
behavior; this feature adds no concurrent action dispatch.

- Items run strictly in the order written. A `set` mutation is visible to
  every action after it (today's behavior: the executor is a plain ordered
  loop, and `set` commits against a pre-write snapshot).
- A **dictionary bundle** — a `when` item with sibling verb keys, or a whole
  stack written as a dictionary (see [Dictionary grammar](#dictionary-grammar))
  — is **expanded at parse time** into an ordered action list. The order is
  fixed:
    1. the `when` guard (if present; only a list item can carry one — a
       root-level dictionary never does)
    2. `set`
    3. communication verbs, in the executor's existing order: `stdout`,
       `stderr`, `info`, `warn`, `success`, `message`, `notify`, then the audio
       verbs `effect`, `say`
    4. `shell`
    5. Darkmatter side-effect verbs and expression functions, sorted by
       canonical verb name in ascending ASCII order
    6. the single flow-control key, last
- A root-level dictionary **never carries `when`**: a guarded action needs
  the list form. This is today's rule — the key set the parser reads at a
  stack root excludes `when` — and it is what lets the `loop:` block read its
  loose verb keys the same way.
- Audio (`say`/`speak`, `effect`) is **published** to Playa in stack order and
  never blocks on playback. See [Background audio](#background-audio).
- An event is **complete** when its synchronous work has finished, message
  deliveries have been registered, and its audio has been published. Existing
  message delivery tracking and CLI shutdown draining still apply; returning
  from an event does not prove that an outbound message has arrived.
- Audio drains across flow-control transitions: when `retry`, `resume`, or
  `proxy` fires, audio already published keeps playing in queue order and the
  next flow state's audio queues behind it. There is no cancel API.
- The first flow-control directive that fires **ends the entire event**,
  wherever it sits — at the root, inside a `then` body, or inside a nested
  `else`. Nothing after it in any enclosing list runs.

### Flow-control placement

Because a directive ends the whole event, an action written after an
unconditional directive can never run. Claudine treats that as an authoring
error:

- The rule is **list-local**. Within any one list — a stack root, a `then`
  body, or an `else` body — an item that unconditionally fires a directive
  must be the **last** item. A violation is the parse error
  `LifecycleActionOrder` (today it covers ordering within one item; its
  meaning widens to ordering within a list).
- A `when` item **never counts as unconditional**, even when both bodies end
  in a directive. Claudine performs no cross-branch reachability analysis.
- More than one flow-control key in one dictionary bundle is the parse error
  `LifecycleMultipleLifecycleActions`.
- The rule applies unchanged to `break` and `prep` when they are built.

```yaml
failure:
    - when: "err.is_throttled"
      then:
          - info: "throttled; retrying"
          - retry: 3
      else:
          - warn: "not a throttle: {{ err.msg }}"
    - message: "failure handled"      # legal: the when item is conditional
```

Enforcement is the parser's (it runs before anything executes). Schemas can
describe shapes but cannot express "must be last", so the editor does not flag
an unreachable action.

### Dictionary grammar

The backbone of a stack is a list, but a leaf may use a key/value shape:

```yaml
start:
    - message: "hi"
    - when: "ctx.season == 'summer'"
      message: "the heat is overwhelming"
      say: "do you have any water?"
```

The bundle expands in the canonical order above: the guard first, then
`message` before `say`. The author does not choose the order; that is the trade
for the compact spelling. A bundle:

- cannot order its operations; the canonical order applies
- holds each canonical verb at most once (`say` and `speak` count as the
  same verb and cannot coexist in one dictionary)
- uses short-form actions when it contains several verbs; long form is
  permitted only in a dictionary containing one verb, as described below
- holds at most one flow-control key, which runs last
- may not carry `then` or `else`; an item that needs `else` uses `then` for the
  true branch instead of sibling verbs

One extra sibling key is permitted: `no_error: true`, applying to every
eligible action in the item. A mixed bundle may contain `set`, but the sibling
flag never applies to that mutation; `{set: {...}, no_error: true}` by itself
is rejected. The flag must be a YAML boolean, and defaults to `false`.
A dispatch failure is a channel, TTS, or shell operation that failed after
its parameters evaluated successfully. With `no_error`, that failure is
logged and does not stop the stack or change the outcome. The flag never
suppresses an expression-evaluation error.

```yaml
success:
    - when: "ctx.notify_team"
      message: "done: {{ fm.title }}"
      shell: "post-summary.sh"
      no_error: true
```

A whole stack may be one dictionary:

```yaml
start:
    message: "hi"
    say: "nice work"
    shell: "doit"
```

It runs `message`, then `say`, then `shell`. A root dictionary follows the
bundle rules above and **never carries `when`**.

### Shape validation and branch execution

An omitted or `null` event, `[]`, and `{}` are empty stacks. An explicitly
provided `then:` or `else:` must contain at least one action; `null`, `[]`,
and `{}` are errors there. An item containing only `when` or `no_error` is
also an error. Unknown item keys and unknown long-form parameters are errors,
never silently discarded. Validate removed keys before filtering lifecycle
concerns out of `loop:`, so an old `loop.stack` cannot disappear unnoticed.
Duplicate YAML keys remain subject to the frontmatter parser's existing
handling; this feature does not introduce a second YAML decoder.

Parse and validate both branches before execution, including branch shape,
verb placement, expression syntax, nested-span checks, and shell approval.
The `initialize` shell prohibition also covers whole-value `$(...)` values
inside `set`, even in an unselected branch. Preserve the existing early
binding rule: approved shell command bytes are fixed at preflight, and all
nested branches contribute commands to that approval pass.

At event time, evaluate a `when` exactly once against the current live state.
Use Darkmatter's existing truthiness rules. Execute only the selected body;
an omitted `else` does nothing when the guard is false. Apart from shell
approval, unselected operands are not evaluated and produce no effects.
An expression failure is never treated as a false guard and cannot select
`else`. Preserve the current per-event failure routing and exactly-once
`finalize` behavior; no new catch-event recursion is introduced.

Preserve each action's authored source path while expanding dictionaries.
For example, an error in `failure[0].else[1].shell.command` names that path,
not a synthetic action index. The shared expression walker and source map
must recurse through `then` and `else`, including `set` and `retry.with`
values, for binding checks, nested-span diagnostics, and shell collection.
DMLS classifies the same authored paths. This keeps validation from missing
an action merely because it is inside a new branch shape.

### Long form

The short form `{verb}: {value}` covers the one required parameter. The long
form `verb: { <params>, no_error?: bool }` exposes optional parameters.
A one-verb dictionary at a stack root, as a list item, or as a branch body
may use long form: `start: {shell: {command: "git status"}}`. Several sibling
verbs use short form only; use a list when several actions need named
parameters. A sibling `no_error` and an inner `no_error` on the same action
are an ambiguous shape and are rejected, even if their values agree.

A mapping under a verb is its named-parameter form, except `set`, whose
mapping is always mutation data. Preserve today's value typing: ordinary
strings are literal; `{{ ... }}` evaluates an expression; whole-value spans
retain their native type. The existing rules for object-valued arguments,
required parameters, positional arity, and optional defaults remain in force.
For example, a short-form `http_post` body that is an object is passed through
a whole-value interpolation, rather than mistaken for action parameters.

`no_error` has exactly two homes: a parameter of any long-form action map
(`- shell: { command: "post-summary.sh", no_error: true }`), and the
dictionary-item sibling above. It is not accepted beside `then`/`else` on a
`when` item. `set` has no long form and accepts no action-specific
`no_error`; a bundle-level flag excludes `set`, as described above. This
rule covers all mutation failures, including a failed shell-valued assignment.
The change does not reclassify dispatch failures as expression failures.

`proxy` and `retry` take an optional `with` overlay (`prep` does when built).
The overlay is never written to disk or merged back into its source. `proxy`
keeps its current immediate-target semantics, including retention when that
active target is re-read for retry or resume. `retry` gains the same typed
mapping and source-side evaluation here. A `retry.with` overlay is applied to
the target of that transition, for that transition only: it is not retained
for a later retry or resume, and a later `retry` without `with` supplies no
retry overlay. A retained `proxy` overlay still applies beneath it on the
proxy-owned active document.

```yaml
failure:
    - retry:
        max_attempts: 3
        with:
            reason: "{{ err.msg }}"
```

For `retry` the target is the fresh re-read of the same document, so the next
attempt can read `fm.reason`. `resume` takes no `with`: it sends a follow-up
message into the live provider session. It still re-reads and validates the
active document, recomputes launch settings, and applies the existing session
compatibility check. Keeping `with` off `resume` is an intentional API limit,
not a claim that it avoids refresh; use `set` for invocation-local state,
subject to its existing persistence rules. `break`,
`stop`, `skip`, `error`, and `defer` take none either.

Evaluate the whole `retry.with` mapping once when the directive fires, against
the source's live frontmatter and event globals. Preserve native values,
static top-level keys, and literal data on target refresh exactly as for
`proxy.with`. Reject whole-mapping interpolation and unresolved executable
spans in lifecycle-key values. Any evaluation failure installs no overlay,
consumes no retry transition, and follows the existing failure path;
`no_error` does not suppress it. Apply the resolved overlay before the fresh
read's schema validation and composition, without re-evaluating its data as
templates. File references continue through the existing request-scoped
`FileReference` resolution; this feature adds no path-rebasing rule.

Attempt budgets, backoff, retry re-entry, and event counts are unchanged:
`max_attempts` counts additional attempts beyond the original, and
`initialize` still fires once per active document. An overlay cannot reset
attempt budgets or bypass refresh validation.

### Action Inventory

This is the **v1 inventory** the parser, the docs, and (in
`2026-10-21-claudine-schemas`) the schemas agree on. It was taken from
`actions.rs`, `action_shape.rs`, `parse.rs`, and `signatures.rs` under
`claudine/lib/src/composition/lifecycle/` and from the flow-control topic pages.
`no_error` applies to every row except `set`.

| Verb | Category | Short-form value | Long-form parameters | Notes |
|---|---|---|---|---|
| `stop` | control | none | `no_error` only | any event |
| `skip` | control | none | `no_error` only | `initialize` only |
| `error` | control | `reason` string (opt) | `reason` | |
| `proxy` | control | `target` file-ref (req) | `target` req; `with` mapping | |
| `retry` | control | `max_attempts` number (opt) | `max_attempts`; `delay` duration; `backoff` `fixed`\|`exponential`; `with` mapping | `with` new |
| `resume` | control | `message` string (req) | `message` req; `max_attempts` | needs a live session; no `with` |
| `defer` | control | `delay` duration (req) | `delay` req; `reason` | **planned**; runtime returns `LifecycleDeferNotImplemented` |
| `break` | control | `reason` string (opt) | `reason`; `code` int 0..255 | **planned** (`2026-10-25-loops-and-flow-control`) |
| `prep` | control | `target` file-ref (req) | `target` req; `with` mapping | **planned** (`2026-10-25-loops-and-flow-control`) |
| `say` / `speak` | communication | text (req) | `text` req | `speak` is an alias |
| `effect` | communication | sound-effect name (req) | `text` req | validated against the effect catalog (`LifecycleUnknownEffect`) |
| `message`, `notify`, `stderr`, `info`, `warn`, `success`, `stdout` | communication | text (req) | `text` req | |
| `shell` | shell | `command` string (req) | `command` req; `on_error` | forbidden in `initialize`; `on_error` is a message emitted when the command fails, not an ignore switch |
| `set` | runtime set | mapping only | (none) | no `no_error` |
| `set_frontmatter` | side-effect | `[file, prop, value]` | `file`, `prop`, `value` | |
| `merge_frontmatter` | side-effect | `[file, obj]` | `file`, `obj` | |
| `delete_frontmatter` | side-effect | `[file, prop]` | `file`, `prop` | |
| `increment_frontmatter` / `decrement_frontmatter` | side-effect | `[file, prop]` | `file`, `prop` | |
| `append_frontmatter` / `prepend_frontmatter` | side-effect | `[file, prop, value]` | `file`, `prop`, `value` | |
| `ensure_file` | side-effect | `file` or `[file, content]` | `file` req; `content` | |
| `ensure_dir` | side-effect | `dir` | `dir` | |
| `append_line` | side-effect | `[file, text]` | `file`, `text` | |
| `append_jsonl` | side-effect | `[file, obj]` | `file`, `obj` | |
| `http_post` | side-effect | `[url, body]` | `url`, `body` | |
| expression functions (`file_exists`, `frontmatter`, `length`, `has_command`, ...) | expression-function | scalar, or array zipped to the signature | the catalog's parameter names | the set is whatever `darkmatter/docs/schemas/expression-functions.yaml` holds (110 at the time of writing); variadic `and`/`or` are positional-only |

Rules:

1. **Every verb and directive has a long form**, except `set` (rule 4) and the
   variadic expression functions. The short form is the verb's single
   positional payload: a scalar for a one-parameter verb; an array zipped to
   the signature for a multi-argument side-effect or expression function; a
   bare name, `null`, or `[]` for a zero-argument verb (`- stop`,
   `- stop: null`).
2. **Side-effect and expression-function long-form parameter names are the
   catalogs' parameter names** (`darkmatter/lib/src/effects/catalog.rs`,
   `darkmatter/docs/schemas/expression-functions.yaml`).
3. **`break` and `prep` are not built here.** Their parameter tables are
   recorded above so the schema can list them as planned; the parser keeps
   treating them as it does today until `2026-10-25-loops-and-flow-control`
   builds them. `defer` stays enumerated and planned.
4. **`set` has no long form and no `no_error`**: `set: { key: value }`.
5. **`with` is accepted on `retry` and `proxy`** (and `prep` when built) and
   rejected everywhere else, `resume` included, with `proxy`'s existing overlay
   semantics and shape rules. Today `LifecycleProxyOnlyParameter` rejects
   `with` anywhere but `proxy`; that error and the `LifecycleProxyWith*` family
   are renamed for `with` in this feature, because `with` is now shared.
6. **The communication long form has one payload parameter, `text`.** The `message`
   and `sound` aliases are dropped. The `route` parameter is dropped from the
   grammar: it is parsed and validated but never read by the executor. It may
   return when messaging routes are wired. Both dropped payload aliases raise
   the same removed-grammar diagnostic with a `text` replacement hint; a
   `resume.message` parameter remains valid.
7. **The count parameter on `retry` and `resume` is `max_attempts`.**
8. **Confirmed as today:** `shell` takes `command` (required) and `on_error`,
   no `cwd`, `env`, or `timeout`; `error` takes `reason` only; `say`/`speak`
   take no `voice` or `provider`, because TTS configuration lives in
   `~/.claudine/config.json`; `skip` remains `initialize`-only.

Two long forms side by side:

```yaml
failure:
    - say: { text: "the run failed", no_error: true }
    - retry:
        max_attempts: 3
        backoff: exponential
        with:
            reason: "{{ err.msg }}"
```

Two doc-drift items the inventory exposed are fixed in the sweep: the `break`
table in `claudine/docs/topics/flow-control/flow-control.md` omits `code`, and
`defer`'s `LifecycleDeferNotImplemented` text still says "requeue". The `with`
rules in `flow-control-reference.md` are rewritten to rule 5.

### Else block

`else` pairs with `when` on the **same item**: `then` holds the actions for a
true condition, `else` for a false one.

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
      else:
          - info: "it is not summer"
    - message: "all is well that ends well"   # runs in both cases
```

Rules:

- `then` and `else` are keys of the `when` item. There is no stand-alone
  `- else:` item.
- `else` requires `then`. A `when` item has either `then` (optionally with
  `else`) or sibling verb keys (a bundle), never both.
- A `then` or `else` body accepts exactly what a stack accepts — a list or a
  dictionary bundle — recursively. A body written as a bundle may carry
  `no_error`; `no_error` stays rejected as a sibling of `then`/`else`.
- Parse errors, reported as `LifecycleStackInvalidShape`: an empty `then` or
  `else`; a `when` with neither `then` nor sibling actions; `then` or `else` on
  an item with no `when`.

```yaml
failure:
    - when: "err.category == 'cap'"
      then:
          warn: "usage cap hit"
          proxy: "@prompts/feature-codex.md"
      else:
          - warn: "{{ err.msg }}"
```

### Conditional nesting

Conditionals nest. A root-list `when` is level one; each `when` inside a
selected or unselected branch adds one level, and sibling guards do not.
The limit is **five** levels; a sixth is the parse error
`LifecycleStackInvalidShape` naming the path. The limit lives **only in the
parser**. The editor-side schema types fewer levels and accepts deeper bodies
loosely, so it never rejects a legal document and never catches a sixth level
(see `2026-10-21-claudine-schemas`).

A nested body is a list that contains further `when` items. There is no `elif`;
an else-if chain is a `when` inside `else`, and each nested `when` counts one
level:

```yaml
failure:
    - when: "err.code == 'cap.rate_limit'"
      then:
          - retry: 3
      else:
          - when: "err.category == 'cap'"
            then:
                - proxy: "@prompts/feature-codex.md"
            else:
                - warn: "unrecoverable: {{ err.msg }}"
```

### Background audio

`say`/`speak` and `effect` already block only on **publication** to Playa's
per-user queue, and Playa preserves publication order. The problem is the
ordering API: `say_first` and a built-in effect-before-`say` ordering
(`audio_phases`) require the author to know how Claudine and Playa order
things. This feature:

- **removes `say_first`** and the fixed `audio_phases` ordering. Order the
  list: `effect` before `say` plays the effect first; `say` before `effect`
  speaks first.
- publishes audio **in stack order**; Playa serializes playback.
- deletes the top-level notification-field execution path (the fixed-order
  dictionary code in the executor), so there is one execution path.

```yaml
success:
    - effect: "chime"
    - say: "the build is green"
    - message: "build complete"
```

Warming the TTS cache before an event fires is not part of this feature; see
`2026-10-19-speak-prepare` and `2026-10-22-tts-prewarm`.

### Loop values move to the `loop` root

The five loop values lose their `_loop_` prefix and move under the top-level
host-declared root `loop`. This feature introduces that declaration;
`2026-10-18-fm-namespace` deliberately leaves it for this step. Decided
2026-10-10: the root is `loop`, not `state.loop`.

| Removed | Replacement |
|---|---|
| `_loop_count` | `loop.count` |
| `_loop_is_first` | `loop.is_first` |
| `_loop_is_last` | `loop.is_last` |
| `_loop_last_output` | `loop.last_output` |
| `_loop_last_exit_code` | `loop.last_exit_code` |

```yaml
start:
    - when: "loop.is_first"
      then:
          - info: "first pass"
      else:
          - info: "pass {{ loop.count }}; last exit {{ loop.last_exit_code }}"
```

The member values and availability inside a loop are unchanged: `loop.*`
resolves wherever `_loop_*` resolved. The new root also needs an explicit
scope rule. Declare `loop` through Claudine's binding views and
supply it through runtime bindings and loop lookups, rather than adding it to
Darkmatter's fixed root catalog. Bare `loop` is the ambient object; dotted
and bracket access resolve its five members. In a declared Claudine scope a
missing member never falls back to frontmatter. Outside a loop the declaration
is unavailable with a typed scope diagnostic, rather than reading the
configuration object. Plain `md compose` does not gain a loop global.
The `loop:` frontmatter block (the loop's configuration) is read as `fm.loop`, per the reserved-root rule. `reserved.rs`
no longer seeds `_loop_*`; every affected composition, lifecycle, approval,
and loop-condition evaluation surface uses the captured host binding. The
loop engine supplies `loop.*` and
`looping/expression.rs` resolves them. Reads of the five exact retired names,
including `fm._loop_count` and the other explicit frontmatter forms, are removed-grammar errors. Detect reads
through the shared expression parsing/validation surfaces, not a raw text
search: quoted literals, ordinary Markdown, code examples, and unrelated
properties such as `_loop_notes` remain data. Static bracket reads are checked
as well; dynamically computed keys follow ordinary lookup semantics. Before
launch, reject discoverable reads in the document being prepared; if a later
render introduces an expression, reject it when that expression is prepared.
There is no alias period.

## Loop Lifecycle

The `loop:` block mixes iteration controls (`while`/`until`, `action`, `max`,
`fail_fast`, ...) with lifecycle concerns. The iteration controls, when the
condition is checked, what `action` mutates, and `break` belong to
`2026-10-24-sequence-containers` and `2026-10-25-loops-and-flow-control`. This
feature says one thing: the `loop:` block's **lifecycle concerns use the same
grammar as every other stack**. They fire on every gate pass — the point at
which the loop condition is checked — including the pass that exits the loop.

A `loop:` block spells them in one of two ways (decided 2026-10-10, unchanged
from the original design):

- **Loose verb keys** at the loop root, read as a root-level dictionary bundle:
  canonical order, each verb at most once, one flow-control key last, no
  `when`. (`action`/`actions` at the loop root is the iteration-control key;
  the looping parser consumes it before the lifecycle parser sees the block.)

```yaml
loop:
    while: "fm.iteration < fm.max_iterations"
    action: increment(iteration)
    stderr: "loop gate"
    info: "finished iteration {{ fm.iteration }} of {{ fm.max_iterations }}"
```

- **`gate:`**, the loop's list home. It holds exactly what any stack accepts,
  and is where a loop puts concerns that need an order or a guard.

```yaml
loop:
    while: "fm.iteration < fm.max_iterations"
    action: increment(iteration)
    gate:
        - effect: "tick"
        - when: "fm.iteration == fm.max_iterations"
          then:
              - warn: "iteration cap reached"
        - info: "finished iteration {{ fm.iteration }} of {{ fm.max_iterations }}"
```

Two shapes are typed parse errors (`LifecycleStackInvalidShape`, or a new name
chosen at implementation): **mixing loose verb keys with `gate:`** in one
`loop:` block, and **`when` at the loop root** — a guard belongs in `gate:`.
Rejecting root `when` also removes the old `when`/`action` collision at the
loop root.

## Schemas

The editor schemas for this grammar (`action.yaml`, `lifecycle.yaml`, and the
Claudine trigger) are `2026-10-21-claudine-schemas`. The schema scale spike
that measured their shape is recorded in [`spike-results.md`](spike-results.md),
with its schemas, fixtures, generator, and harness under `spike/`; those files
stay here because other specs and tests reference them.

## Migration Sweep

The removed grammar — `stack:`, `action:` as the item aggregator (`- action:`)
or as a sibling of `when` on a list item, `say_first`, the communication
`route` parameter, and the `_loop_*` names — was spelled in 65 non-Rust and 62
Rust files when inventoried on 2026-09-28. All are rewritten. Re-take the
inventory at implementation with the migration search in Acceptance Criteria; the buckets below say where to
look, not an exact count. Whether the sweep lands as one commit or per area is
an implementation choice.

**(a) Executed prompt documents.** Extend their existing opt-in parser and
execution coverage with the new grammar; keep live internal prompts out of CI.

- Internal `darkmatter/prompts/` and `claudine/prompts/` wherever the
  implementation inventory finds lifecycle syntax.
- Root `prompts/` (24 at inventory): `_add/add-context-variables`,
  `_add/add-expressions`, `_clarify/findings`, `_implement/implement-plan`,
  `_implement/implement-review`, `_implement/implement-suggestions`,
  `_pr/diagnose`, `_pr/dirty`, `_pr/fix`, `_pr/open`, `_pr/push`, `_pr/triage`,
  `_prompt`, `_reviews/cross-platform`, `_reviews/dry`,
  `_reviews/feature-review`, `clarify`, `commit`, `format`, `implement`,
  `merge-conflicts`, `plan`, `pr`, `review`. `_prompt.md` and
  `_implement/implement-plan.md` also name `_loop_*`; both move to `loop.*` in
  the same pass, so each file is swept once.
- `claudine/docs/research/_TEMPLATE.md` and every
  `claudine/docs/research/*/_fleet.md`.
- `claudine/docs/getting-started/index.md`.

Shipped-prompt edits are part of this feature's implementation (the
restructure that produced this spec did not touch them).

**(b) Docs and skills.**

- Claudine READMEs wherever they show lifecycle grammar or loop-variable
  examples.

- `claudine/docs/topics/flow-control/lifecycle.md`, `flow-control.md`,
  `flow-control-reference.md`, `looping.md`, `sequences.md`,
  `state-management/side-effects.md`, `frontmatter-properties.md` (the
  `_loop_*` table becomes the `loop.*` table).
    - `looping.md` names `state.loop.*` and promises a one-release alias period
      for `_loop_*`. Both are drift: the root is `loop`, and the old names are
      a hard error. Rewrite that note.
    - `sequences.md` task `setup:`/`teardown:` examples are stacks and are
      rewritten with the rest of the page.
- `.claude/skills/claudine/SKILL.md` and any skill page that names `say_first`
  or `stack:`. `timeline.md` is history and stays.

**(c) Markdown fixtures.**

- `claudine/cli/tests/fixtures/nested_span_regression/commit.md`.
- All mirrored prompts under `claudine/lib/tests/fixtures/frozen_prompts/`
  and `claudine/cli/tests/fixtures/frozen_prompts/`. Preserve the behavior each
  frozen fixture proves while updating its lifecycle syntax.
- `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`,
  hash-pinned by `shipped_prompt_route_drift.rs`; after reviewing the fixture
  delta, refresh the opt-in target from the repository root:
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli -F prompt-tests,test-fixtures --test prompts shipped_prompt_route_drift::`.
  Default `just test-cli` does not select that target.
- `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`.

**(d) Claudine Rust.**

- `claudine/lib/src/composition/lifecycle/**` and `claudine/lib/tests`;
  `claudine/cli/tests/l1`; `claudine/cli/tests/level2`.
- `claudine/lib/src/composition/sequence/task/**`, where `setup:`/`teardown:`
  are parsed by the lifecycle parser.
- `_loop_*`: `looping/expression.rs`, `looping/engine.rs` (and its
  `engine/tests/`), `reserved.rs`, `looping/types.rs`,
  `reporting/queries/common.rs`, `cli/src/commands/compose/loop_run.rs`, and
  their tests.
- Repository file reads in tests are spelled per the `rust-testing` skill
  (`workspace_root().join(...)`) so editing the files they read schedules the
  narrowed CI cell.

**(e) Darkmatter Rust that names `_loop_*`.** The `dasherized_identifier_*`
tests and `darkmatter/lib/src/markdown/compose/expression/parser.rs`.

**(f) Existing schema integration and DMLS.** Update the existing extension
at `darkmatter/docs/schemas/claudine.yaml` and its type library
`darkmatter/docs/schemas/claudine-types.yaml` to accept the new grammar in this
change. Keep their paths and references working. The full action typing,
new authoritative files under `claudine/schemas/`, and eventual retirement of
the old extension belong to `2026-10-21-claudine-schemas`.

**Reader's note:** the earlier draft deleted these files before their
replacement existed. That would leave editor users without lifecycle
validation and examples pointing at missing files between rollout steps.
Retaining the existing integration until the schema replacement lands avoids
that gap without pulling the full schema feature into this one. The interim
extension may accept nested bodies loosely where its type system cannot
express them; the parser remains authoritative for the five-level limit.

- `darkmatter/dmls/src/diagnostics/nested_span.rs` — drop `say_first` and
  `stack` from structural field lists and classify list items, `then`/`else`
  bodies, root dictionaries, and `gate:`. Update `nested_span_tests.rs` paths.
- Keep `lifecycle_inventory_matches_the_authored_claudine_schema` working
  against the retained extension, and migrate it to the authoritative schema
  files when the next feature lands; do not remove the coverage between steps.
- `darkmatter/dmls/src/overlay/frontmatter.rs` pointers change from
  `initialize.stack[0].when` to `initialize[0].when`.
- Extension-loading tests and configuration examples continue to use existing
  valid paths. Run `mapping_only_corpus`; re-bless only an intentional changed
  diagnostic using `DMLS_BLESS_MAPPING_CORPUS=1`, with the reason recorded in
  the implementation log. Never re-bless simply to hide missing validation.

**Not swept:** spec and review snapshots; `.claude/skills/claudine/timeline.md`;
`claudine/docs/research/hooks/*.md`, whose tables describe provider hooks.

## Out of scope

- Claudine's schema files, embedding, and the editor typing of each event:
  `2026-10-21-claudine-schemas`.
- `Speak::prepare` and the cache-consult fix: `2026-10-19-speak-prepare`.
- TTS pre-warm and the `tts.prewarm` setting: `2026-10-22-tts-prewarm`.
- `sequence.yaml` (the sequence document schema):
  `2026-10-26-task-inputs-and-providers`.
- Task `setup:`/`teardown:` removal, nested sequences, `seq`/`group` roots:
  `2026-10-24-sequence-containers`. Until then `setup:`/`teardown:` are stacks
  and take the new grammar here.
- `break`, `prep`, loops on any primitive: `2026-10-25-loops-and-flow-control`.
- The one path-resolution rule for every `with:`, `params:` → `with:`:
  `2026-10-26-task-inputs-and-providers`.
- Moving the parser and action model into Darkmatter, host-named events:
  `2026-10-27-darkmatter-lifecycle-ownership`.

## Acceptance Criteria

Tests are tiered: **L1** tests are hermetic (no network, no real terminal, no
audio) and run with `just test` in a package area; **L2** tests carry the
`level2_` prefix and run with `just test-l2`. `just lint` in `claudine/` also
runs `lint-lifecycle-doc-facets`, a grep gate over the lifecycle topic pages.
Tests never focus a terminal or browser window and remain portable to macOS,
Linux, native Windows, and WSL2. Existing CI scheduling selects affected
packages and environments; this feature adds no new environment or gate.
Internal prompt corpus and live-prompt execution tests stay opt-in through
`prompt-tests` and `just test-prompts`, never in default L1 or CI. Default
behavior tests use frozen prompt copies. Other criteria are L1 unless marked.

**Parser**

- **AC1** Every stack — each of the six events, a `loop:` block's `gate:`, and
  a task's `setup:`/`teardown:` — accepts a flat action list; a root-level
  dictionary is accepted, never carries `when`, and expands in the canonical
  order.
- **AC2** `when`/`then`/`else` nest to depth 5; depth 6 is a typed parse error
  naming the path.
- **AC3** In any stack, an action after an unconditional flow-control item in
  the same list is `LifecycleActionOrder`; the same verb in a sibling `else`
  list is legal; a `when` item never counts as unconditional.
- **AC4** `no_error` is accepted in both homes (long-form parameter;
  dictionary-item sibling, a bundle body included) and rejected beside
  `then`/`else` and on a standalone `set`; in a mixed bundle it cannot
  suppress a `set` failure. Conflicting inner and sibling flags are rejected.
- **AC5** `with` is accepted on `retry` and `proxy` and rejected elsewhere,
  `resume` included; a `retry` overlay is visible to the fresh re-read and never
  persisted; it applies to that transition only, so a later retry or resume
  does not see it (cover repeated retry, retry followed by resume, and a
  proxy-owned active document, with no real provider launch). The
  renamed `with` errors replace `LifecycleProxyOnlyParameter`
  and the `LifecycleProxyWith*` family.
- **AC6** In any stack, the `loop:` block included: `stack:`; `action:` as the
  item aggregator or as a sibling of `when` on a list item; `say_first:`; the
  communication `route`, `message`, and `sound` parameters; and reads of the
  five exact retired `_loop_*` names raise a typed
  removed-grammar `CompositionError` (name chosen at implementation) carrying
  the replacement and authored path, before provider launch for statically
  discoverable expressions, in `compose`,
  `inline-compose`, and `sequence`. The `_loop_*` scan covers frontmatter
  expressions and body spans; where a body read is discoverable only at render
  time, the same typed error is raised there. The CLI-boundary test follows
  `compose_removed_validation_keys.rs`. `action`/`actions` at the loop root
  (iteration controls) are exempt.
- **AC7** `loop.count`, `loop.is_first`, `loop.is_last`, `loop.last_output`,
  and `loop.last_exit_code` resolve with the values `_loop_*` produced, in
  every place `_loop_*` resolved (the existing `gate_ambient.rs` coverage is
  migrated, not dropped); `reserved.rs` seeds no `_loop_*`. Cover bare,
  dotted, and bracket lookup, `fm.loop` access to configuration, scope
  unavailability, and the lack of frontmatter fallback. Literal old names
  and unrelated `_loop_notes` data are not removed-grammar errors.

**Audio**

- **AC8** `say_first` and `audio_phases` are gone; audio is published in stack
  order. A hermetic executor test records publication calls (`effect` then
  `say`, and `say` then `effect`) with a fake publisher and proves already
  published audio survives a flow transition. Publication-boundary coverage
  uses `test_toolkit::LockedAudioSpool` and stubbed TTS preparation with dry
  run disabled only for that isolated fixture, so durable queue records exist
  but playback cannot run. Dry-run prompt tests separately assert no audio
  is published. No test plays sound or reaches a real TTS provider.

**Lift-ready slice**

- **AC9** A guard test fails if any of `parse.rs`, `action_shape.rs`,
  `actions.rs`, `validate.rs`, `signatures.rs`, or `source_map.rs` under
  `claudine/lib/src/composition/lifecycle/` names `crate::config`,
  `crate::events`, `crate::provider`, `crate::messaging`, `biscuit_speaks`, or
  `playa`.

**Shipped artifacts**

- **AC10** Extend the opt-in `claudine/cli/tests/prompts/shipped_prompts.rs`
  corpus to run the lifecycle parser over every `.md` under the internal
  prompt directories and every `claudine/docs/research/**/_fleet.md` plus
  `_TEMPLATE.md`; zero removed-grammar errors and zero parse errors. Extend
  the default L1 shipped-artifact corpus for frozen fixtures. Keep a
  representative CLI execution case with a fake provider that proves a
  selected branch executes in order and the other branch performs no work.
- **AC11** `shipped_prompt_route_drift.rs` hashes are refreshed after its
  fixture is re-derived.
- **AC12** The existing Darkmatter Claudine extension and its type library
  still load at their documented paths and accept new-grammar fixtures;
  their inventory guard stays active. The DMLS `nested_span.rs` mirror
  classifies new-grammar paths (list item,
  `then`/`else` body, root dictionary, `gate:`) as its tests assert; the
  `overlay/frontmatter.rs` pointer tests use new paths; `mapping_only_corpus`
  passes (re-blessed if needed, with the reason logged).

**Docs and gates**

- **AC13** The lifecycle, flow-control, flow-control-reference, looping,
  sequences, side-effects, frontmatter-properties, and getting-started pages
  show only the new grammar and `loop.*`; the looping page promises no alias
  period; `just lint-lifecycle-doc-facets` passes.
- **AC14** `.claude/skills/claudine/` names neither `say_first` nor `stack:`
  outside `timeline.md`.
- **AC15** `just test`, `just test-l2`, and `just lint` are green in
  `claudine/`; `just test` and `just lint` are green in `darkmatter/` (DMLS
  included). Run `just test-prompts` from the repository root and inspect its
  results: the recipe is informational and exits zero even when prompt checks
  fail, so its exit code alone is not passing evidence. The migrated lifecycle
  corpus and route-drift tests must pass; record unrelated draft-prompt failures
  separately rather than widening this feature's scope.
- **AC16** Re-take the migration inventory with `rg` for
  `^\s+stack:|say_first|^\s+- action:|_loop_(count|is_first|is_last|last_output|last_exit_code)`
  over `prompts/`, `claudine/docs/`, `claudine/lib/`, `claudine/cli/`,
  `darkmatter/docs/`, `darkmatter/dmls/`, `darkmatter/lib/`, and
  `.claude/skills/claudine/` returns only documented intentional hits:
  historical records, removed-grammar tests and diagnostics, and literal-data
  tests. Also search for old `action` discriminator maps and communication
  `message`/`sound`/`route` parameters; classify those by structure so legitimate
  loop actions, `resume.message`, and message verbs remain valid. Record every
  remaining hit and its reason in the implementation log. Text search
  inventories the migration; parser tests enforce rejection.
- **AC17** Parser and executor coverage proves empty-stack rules, single-verb
  long forms versus multi-verb bundles, alias collisions, deterministic
  dictionary ordering, and rejection of unknown keys and parameters. Both
  branches are validated and contribute shells to approval; only the selected
  branch evaluates event-time operands. A nested directive stops the enclosing
  event, and nested `initialize` shell or `set` shell commands are rejected.
  Diagnostics retain authored paths through dictionary expansion and nesting.

## Open Questions

None.
