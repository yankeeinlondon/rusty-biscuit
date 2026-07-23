# Composition Lifecycle Notifications

## Contents

- Lifecycle Properties
- Binding Time: Early vs Late
- When Lifecycle Properties Interpolate
- Notification Fields
- Stacks
- Lifecycle Context
- Loop Gate Concerns
- Examples
- Sound Effects
- State Machine
- Validation
- Non-Object Frontmatter
- Integration Notes
- Related Topics

Use heading search to jump to the listed subsystem.


Claudine compositions support **lifecycle notifications** declared in Markdown frontmatter. These notifications emit side effects—TTS speech, sound effects, terminal messages, and outbound messaging—at key moments during a composition run.

## Lifecycle Properties

Seven frontmatter properties control lifecycle behavior. Each accepts an object with notification fields and an optional ordered `stack:` of actions:

| Property | Emitted when |
|----------|-------------|
| `initialize` | Prompt file has been identified and frontmatter has parsed, before schema validation and shell pre-flight |
| `start` | Pre-flight checks have passed; immediately before provider invocation |
| `success` | The provider session completed without error |
| `blocked` | Composition exited before the provider child was spawned (e.g., pre-flight denial, schema validation failure) |
| `failure` | The provider session exited with an error |
| `finalize` | Once per iteration, immediately after `success`/`blocked`/`failure` |
| `loop` | Post-`finalize` gate that evaluates the loop's `while`/`until` condition and can run additional lifecycle concerns |

Legacy prompts that only configure `start`, `success`, `blocked`, and `failure` continue to work unchanged.

## Binding Time: Early vs Late

Every frontmatter property of a lifecycle event interpolates **when that event fires**, not during the initial compose. This is what lets a lifecycle message report the state at the moment it runs — including the runtime globals (`err`, `timing`, `current`) that do not exist at compose time. So `failure.message: "❌️  {{err.code}}"` renders the real error's code, and a `failure` stack `message: "❌️  {{err.code}}"` does too.

The variables a lifecycle `{{{ … }}}` span can read fall into two groups:

- **Early-binding** (resolvable before the run): `doc.*` (frontmatter), `ctx.*`, `env.*`, and read-side functions (`parent_dir`, `dirname`, `frontmatter`, `file_exists`, …).
- **Late-binding** (only exists at event-time): `err` (in `blocked`/`failure`/optional-error `finalize`), `timing`, `current`.

A lifecycle property's interpolation resolves against the union of both, at event-time. Bare frontmatter references (`{{phase}}`, `{{artifact.path}}`) read the **current** effective document state at the moment the event fires — not a copy captured at the initial compose — so a `set_frontmatter` side effect that mutates `phase` between loop iterations is visible to the next iteration's lifecycle message.

This is the consistent rule used everywhere else: a value is literal text and `{{{ … }}}` is how you opt into the expression engine. The document body and ordinary (non-lifecycle) frontmatter keys still interpolate at compose-time and are unchanged.

## When Lifecycle Properties Interpolate

Lifecycle strings keep their authored `{{{ … }}}` spans through the prepare stage — Darkmatter defers the seven lifecycle keys from compose-time resolution (DM1, `ComposeOptions::with_exclude_keys`) — and Claudine re-interpolates each property/action string through Darkmatter (DM2, `SubtreeCompose`; the same composition engine, no second interpolator — the bespoke `lifecycle_executor::interpolate` + `LifecycleLookup` runtime path was removed) just-in-time, immediately before it is used:

- **Communication and action bodies** (`say`, `message`, `notify`, `stderr`, `info`, side-effect args, …) resolve at the instant the event fires, against the live document state plus the in-scope late-binding globals. Resolution is **just-in-time**, not a single snapshot: a `set_frontmatter` run by stack action #1 is visible to action #2 in the same event's stack.
- **Resolution fails closed.** A malformed expression, an unknown function, an unknown root (a typo), or a late-binding global used outside its legal event fails the event with a typed error *before any side effect is dispatched* — a lifecycle string never silently renders empty for these cases. A *known* surface (a declared frontmatter key, `ctx`/`env`/`doc`, or an in-scope late-binding global) that resolves to `null`/empty still renders empty, as today. To tolerate an *unknown* optional name, opt in with explicit fallback syntax: `{{ maybe || '' }}`.
- **An evaluation error halts the run on every phase.** This fail-closed raise is an *expression-layer* error (a crashed `when:` guard or interpolation), distinct from a side-effect dispatch failure (below). It is carried as the typed `CompositionError::LifecycleEvaluationError` and surfaced to stderr as a styled error **at the point of error — before the catch events (`failure`/`finalize`) fire — and exactly once**, so the original crash is visible ahead of any catch-event output rather than buried beneath it. The run exits non-zero. On terminal-phase events (`success`/`failure`/`finalize`/`loop`) it does **not** retroactively fire `failure` (the provider already ran) but does fire `finalize` once with the error exposed as the `err` global, so an author can catch it. If a catch event *itself* raises a new evaluation error, that later crash is the surfaced (and exit-determining) one. A raise inside `finalize` itself surfaces and halts without re-entering `finalize`. A `when:` that evaluates cleanly to `false` is *not* a raise — it just skips its item, unchanged.

### The `shell` exception

`shell` commands (positional `shell: "…"` and key/value `command:`) are the single early-binding exception. They are approved during pre-flight, so they are resolved **then**, against early-binding surfaces only (`doc.*`, `ctx.*`, `env.*`, read-side functions). The approved command is byte-identical to the executed command. A late-binding reference (`err`/`timing`/`current`) inside a shell command is rejected at prepare time with a typed error naming the property path — those values do not exist yet at pre-flight.

## Notification Fields

Each lifecycle property is an object containing any of these fields:

| Field | Type | Description |
|-------|------|-------------|
| `say` | string | Text to speak via TTS. Mutually exclusive with `say_first`. |
| `say_first` | string | Text to speak via TTS, but **before** sound effects. Mutually exclusive with `say`. |
| `effect` | string | Sound effect to play (kebab-case name; see [Sound Effects](#sound-effects)). |
| `message` | string | Message dispatched via the configured messaging route (Discord, Slack, Signal, WhatsApp, or webhooks). |
| `notify` | string | Local desktop notification title. Zero-config; does not require a messaging route. |
| `stdout` | string | Plain prose line written to **stdout**. **No status glyph.** Note: stdout is otherwise reserved for pipeable command output, so this interleaves with the composed/provider output on the same stream — use deliberately. |
| `stderr` | string | Plain prose line written to stderr. **No status glyph** — inline styling and links are honored, but no status decoration. |
| `info` | string | Status line rendered with an info style. |
| `warn` | string | Status line rendered with a warning style. |
| `success` | string | Status line rendered with a success style. |
| `stack` | list | Ordered list of conditional actions (see [Stacks](#stacks)). |

Most lifecycle output is written to stderr, messaging routes, or desktop notifications. The `stdout` channel is the lone exception: it writes to stdout, which is otherwise reserved for pipeable command output, so reach for it only when you specifically want lifecycle text on stdout.

### Audio Ordering

When both speech and an effect are configured, the order depends on which speech field is used:

- `say` + `effect` → **effect first, then speech**
- `say_first` + `effect` → **speech first, then effect**

This lets you choose whether a sound effect acts as an introduction (`say_first`) or a conclusion (`say`) to the spoken message.

## Stacks

A `stack` is an ordered list of conditional actions executed after the top-level notification fields. Each stack item can include:

- `when` — an optional Darkmatter condition expression. Omitted means the item always runs.
- `action` — a single action or a list of actions.
- `no_error` — when `true`, a side-effect **dispatch** failure (a channel/TTS/shell action that evaluated fine but whose effect failed) is logged but does not stop the stack or change the composition outcome. It does **not** suppress an expression-layer evaluation error (a crashed `when:` or interpolation) — those always halt.

```yaml
success:
  stack:
    - when: "env.SEND_MESSAGE == 'true'"
      action: { message: "Build passed on main" }
    - action:
        - say: "Done"
        - effect: "confirmation"
```

Actions run in order. The first lifecycle control action (`skip`, `stop`, `error`, etc.) terminates the stack for that event.

### Action Forms

An action is written in one of exactly two forms — **positional** or **key/value**. Both follow a single evaluation rule:

> **Every value in a lifecycle action is literal text. Use `{{{ … }}}` to inject a variable or expression. The only expression-evaluated keys in the entire lifecycle surface are the boolean predicates `when`, `until`, and `while`.**

**Positional** — an object whose single key is a known verb; the value carries the argument(s):

```yaml
success:
  stack:
    - action:
        - success: "review {{iteration}} is production ready"
        - message: "✅ review #{{iteration}} completed"
        - effect: "small-group-cheer"
```

- **Scalar value → one argument.** Scalars may be strings, numbers, or booleans (`message: "review {{iteration}} passed"`, `retry: 3`, `proxy: "other-prompt.md"`).
- **Array value → positional arguments**, zipped against the verb's canonical signature:
  ```yaml
  - set_frontmatter: ["state.md", "status", "production ready"]
  - append_line: ["log.md", "review {{iteration}} done"]
  ```
- **Null value, empty array, or bare verb-name string → zero arguments.** All three spellings are equivalent for no-arg (and all-optional-arg) verbs:
  ```yaml
  - stop:        # null value
  - stop: []     # empty array
  - stop         # bare verb-name string
  ```

Positional form covers a verb's canonical call signature only. Optional named parameters that have no positional slot (`route`, `on_error`, `no_error`, `backoff`, `delay`, `max_attempts`, …) require key/value form.

**Key/value** — an object with an explicit `action` verb-discriminator key plus named parameters:

```yaml
success:
  stack:
    - action:
        action: shell
        command: "git push origin HEAD"
        on_error: "push failed"
        no_error: true
    - action:
        action: message
        message: "Deployed {{version}}"
        route: "deployments"
```

Reach for key/value form when you want self-documenting parameter names or an optional named parameter. Key/value parameter values follow the same literal-default rule as positional values.

A stack item's `action:` value may be a single positional map (`action: { success: "…" }`), a single key/value map, a bare verb-name string (`action: stop`), or an **array** mixing positional and key/value elements. A single action need not be wrapped in an array-of-one. The two forms are distinguished structurally: an object with an `action:` key is key/value; an object whose single key names a known verb is positional.

#### Typed argument values

A value whose trimmed content is exactly one `{{ expr }}` span resolves to the expression's **typed** value, matching Darkmatter's whole-value frontmatter rule — so `set_frontmatter: ["s.md", "ready", "{{ true }}"]` writes boolean `true`, `"{{ 3 }}"` writes number `3`, and `"3"` writes the string `"3"`. A bare token is always literal text, never a variable: `set_frontmatter: ["s.md", "status", "done"]` writes the literal string `done`.

#### Object-valued arguments

Some side-effect verbs take an object argument (`merge_frontmatter`, `append_jsonl`, key/value `http_post`). Direct nested YAML maps are **not** accepted inside action values. Place the object in frontmatter or context and pass it through a whole-value `{{{ … }}}` span:

```yaml
payload:
  owner: ken
  status: ready
success:
  stack:
    - action: { merge_frontmatter: ["state.md", "{{ payload }}"] }
    # key/value equivalent:
    - action:
        action: merge_frontmatter
        file: "state.md"
        obj: "{{ payload }}"
```

A literal nested map used directly as an action value (`merge_frontmatter: { owner: ken }`) is a typed object-data-through-interpolation error.

#### Migration from short form

The `verb(args)` **short form** (`say(All done)`, `shell(git push)`, `set_frontmatter('a','b','c')`) has been **removed**. A document still using it fails with a typed did-you-mean error that prints the positional rewrite (`success("x")` → `success: "x"`; `set_frontmatter('a','b','c')` → `set_frontmatter: ["a","b","c"]`). The error highlights the offending frontmatter line in TTY output and stays escape-free in non-color output. Two breaking changes to migrate:

- **`verb(args)` is gone.** Rewrite each call to positional (`message: "…"`, `set_frontmatter: ["…", "…", "…"]`) or key/value form.
- **Key/value string parameters are now literal by default.** `target: next_prompt` means the literal string `next_prompt`; write `target: "{{ next_prompt }}"` to evaluate it as an expression. Likewise `message: "ctx.area"` sends the text `ctx.area` while `message: "{{ ctx.area }}"` sends the context value.

A bare verb-name string with no parentheses (`- stop`) is **not** short form — it survives as the zero-argument positional spelling.

### Flow Control Actions

Lifecycle flow control actions terminate the current event's stack and influence runtime flow:

| Action | Valid in | Effect |
|--------|----------|--------|
| `stop` | every event | End this event's stack cleanly; composition continues with the current outcome |
| `skip` | `initialize` only | Whole-document opt-out: no provider invocation, no `finalize`, no `loop` |
| `error: "reason"` | every event | Mark this event as failed; at `success`/`finalize` it converts success to failure |
| `proxy: "@other.md"` | every event | Hand off to another prompt document, entering the target at its own `initialize`. Key/value form accepts an optional `with:` overlay — see [Proxy Handoffs](#proxy-handoffs) |
| `retry: 3` | every event | Retry the current prompt N additional times (re-runs pre-flight pre-launch, re-invokes the agent post-launch) |
| `resume: "message"` | every event | Resume the agent session with a follow-up message. Needs a live session — pre-launch it surfaces a `ResumeWithoutSession` error |
| `defer: "5m"` | every event | Defer this prompt to **run again later** — a fresh scheduled run after the delay (not an in-place pause), via the rendezvous deferred-execution scheduler. **Not implemented yet:** `defer` parses and dispatches but currently surfaces a typed `LifecycleDeferNotImplemented` error until the rendezvous backend is ready. |

At most one flow-control action may appear in a stack item, and it must be the last action.

**Proxy target resolution.** A `proxy` target resolves through the shared
`biscuit-file::FileReference` contract, anchored on the document that authored
it: a bare implicit path (`other.md`) is repository-root first, then next to the
authoring document; an explicit `./`/`../` path is source-relative only; `@` is a
magic-root search (repository root, configured roots, home — **not** a repo-root
join); and `~/` is home-pinned. The target must name an existing document, so a
missing target fails loudly with a typed `Unresolvable file reference` rather than
handing off to a nonexistent path. When a proxied target authors its own proxy,
the target becomes the new source for that reference.

**Flow control is universal.** Flow control reacts to **state** — an error, a missing file, an `env` value, frontmatter — and an error is just one kind of state. So `error`/`stop`/`retry`/`resume`/`defer`/`proxy` are valid in **every** event. The headline example: a `success` stack can `resume: "you finished but never wrote abc.md — create it as instructed"` when the agent completed cleanly but an expected artifact is missing. The only placement rule is `skip` (`initialize`-only). Apparent event-specific behavior is **runtime capability**, not placement: `resume` needs a live session (pre-launch → `ResumeWithoutSession`) and `retry`'s re-entry point is derived from whether the provider had launched. This is enforced once, at parse time (`LifecycleControlAction::is_valid_for` → `LifecycleActionPlacement`); at runtime every event's stack dispatches its control through the same event-agnostic path (`decide_control` + `dispatch_terminal_control`). The iteration `loop:` (while/until) is a separate mechanism and is never coupled to handler dispatch.

The provider run-loop events — `start`, `success`, `failure`, `finalize` — dispatch `retry`/`resume`/`proxy` fully (this is where `success` + `resume` lives). `proxy` from `initialize` is equally complete: the active-document coordinator sits above both the document loop and the provider harness, so an `initialize` handoff is a real transition rather than a reduced pre-launch path, and the target it hands to is prepared exactly as a directly-invoked document would be (see [Proxy Handoffs](#proxy-handoffs)). What remains unsupported is narrower than it once was: `retry` from `initialize`, and `retry`/`resume`/`proxy` from a compose pre-flight `blocked` or from the `loop` gate, have no re-entry loop to act on and surface a typed `LifecycleSetupPhaseRecoveryUnsupported` rather than a silent no-op. `resume` from `initialize` surfaces `ResumeWithoutSession` — there is no session yet to continue. Put those recoveries on a post-launch event. `defer` (deferred re-execution) is **not implemented in any event yet** — it always surfaces `LifecycleDeferNotImplemented` until its rendezvous backend lands.

#### Retry and resume re-entry

`retry` and `resume` replace only the **provider-attempt slice** of the active document; they do not change document identity. Both refresh the document canonically — a fresh read from disk with full validation — and both keep the document's `with:` overlay, its proxy provenance, and their own decrementing budgets (a `retry` cannot reset its budget by replacing the attempt). `initialize` does **not** re-fire: it is once per active document, not once per attempt.

Launch identity is recomputed at that fresh-read boundary, against the document about to run, so a `model:` changed between attempts actually reaches the child environment rather than being pinned to an adoption-time snapshot.

`resume` additionally checks a **session-compatibility key**. It retains the live provider session only when the key still matches across the refresh; when a facet moved, the resume refuses with `CompositionError::LifecycleResumeIncompatible { facets }`, names the changed facets, and recommends `retry` for a fresh session. Note the ordering: `start` fires before the comparison, so a refusal is a post-`start`, pre-spawn failure and owes the ordinary lifecycle tail — `failure` then exactly one `finalize`, both seeing the refusal as `err.*`. `success` does not fire, and the provider is never spawned a second time. Full facet list, reachability, and coverage: [composition.md — Retry and resume re-entry](composition.md#retry-and-resume-re-entry).

#### Lifecycle events and `--dry-run`

`--dry-run` fires **no lifecycle events at all**. The dry-run seam returns before the lifecycle runtime is constructed, so a stack carrying `append_line`, `set_frontmatter`, or `shell` cannot touch the workspace during a rehearsal, and no dynamic `proxy` route can be traversed. Do not author lifecycle stacks expecting a dry run to exercise them; turning dry run into lifecycle simulation is an explicit non-goal. See [composition.md — Dry Run](composition.md#dry-run).

### Proxy Handoffs

`proxy` hands the run to another prompt document. The target enters at its own `initialize` and becomes the **active document**: it owns the remaining lifecycle, the closure, and the output. A clean handoff synthesizes no source-side terminal, `finalize`, or `loop` event — a `proxy` from `success`/`failure` skips that attempt's ordinary `finalize`, and a `proxy` from `finalize` does not re-enter it.

The target is bootstrapped and prepared by the **same canonical preparation service** that prepares a directly-invoked document, so `claudine compose target.md` and a `proxy` to `target.md` see the same stages: the same `initialize`, the same schema validation, the same shell discovery and approval, and the same typed diagnostics. See [composition.md — Document Handoffs and the Equivalence Contract](composition.md#document-handoffs-and-the-equivalence-contract) for what the target recalculates for itself.

Positional form stays compact and is unchanged:

```yaml
- proxy: "@prompts/next.md"
```

#### The `with:` overlay

Key/value form accepts an optional `with:` mapping — a **transient overlay** of top-level frontmatter properties for the immediate target:

```yaml
success:
  stack:
    - action:
        action: proxy
        target: "@prompts/next.md"
        with:
            attempt: "{{ iteration }}"
            ready: "{{ true }}"
            label: "phase-{{ iteration }}"
            files: "{{ changed_files }}"
            metadata:
                source: router
                area: "{{ ctx.area }}"
```

`with:` is accepted **only** on key/value proxy. `with: {}` parses and means exactly what omitting it means. A non-mapping `with:` (`LifecycleProxyWithNotMapping`), a dynamic key (`LifecycleProxyWithDynamicKey`), a whole-mapping interpolation such as `with: "{{ payload }}"` (`LifecycleProxyWithWholeMapping`, a named v1 out-of-scope), and `with:` on any other action (`LifecycleProxyOnlyParameter`) are all typed, source-aware frontmatter errors. Positional `proxy:` with a sibling `with:` remains the existing `LifecycleStackAmbiguous` diagnostic, whose actionable rewrite points at key/value form.

#### Typed values

Keys are static YAML strings and are never interpolated — they name target frontmatter properties. Values follow the same rule as every other lifecycle action parameter, recursed through nested arrays and objects:

- a mixed string (`"phase-{{ iteration }}"`) resolves to a string;
- a string whose trimmed content is **exactly one** `{{ … }}` span keeps the expression's resolved type — `"{{ true }}"` installs boolean `true`, not the string `"true"`;
- authored YAML numbers, booleans, arrays, objects, and nulls keep their authored types; and
- strings nested inside arrays and objects follow the same rule.

Arrays and objects here are **data**, not positional action arguments — this is the one place in the lifecycle surface where a nested mapping is a legal action value.

#### Source-time evaluation

The whole mapping resolves **once, at the source**, when the event fires — through the same Darkmatter DM2 subtree composition the rest of the lifecycle surface uses, in strict mode, against the source document's live frontmatter plus the globals in scope for that event (`err`, `timing`, `current`). A `set_frontmatter` run earlier in the same stack is visible to `with:`.

What lands on the target is therefore resolved data, not a template. A raw `{{ … }}` span can never survive into the overlay and be re-evaluated at target time — that would make the binding ambiguous, so it is rejected instead.

Evaluation is **atomic**: the target and the complete mapping resolve before any state changes. On failure — an unknown root, a malformed expression, an unknown function, an out-of-scope global — no partial overlay is installed, the target is never touched, the source stays active for diagnostic attribution, and the run follows the normal failure routing for the event that requested the handoff. `no_error` is accepted (it is a universal key/value field) but does **not** suppress this: proxy has no side-effect dispatch phase, and an overlay failure is an expression-layer error. The diagnostic (`LifecycleProxyWithEvaluationFailed`) names the lifecycle event, the proxy action, the target, and the deepest representable path inside `with` — never a resolved value, which may hold a secret.

#### Precedence and lifetime

The overlay merges into **every** read of the target's authored frontmatter — the bootstrap read before target `initialize` and the fresh read after initialize-time mutations — before composition and schema validation. Precedence, lowest to highest:

1. target-authored frontmatter;
2. the immediate proxy's `with:` mapping;
3. caller-supplied compose overrides (`key=value` / `--set`).

**The original caller stays authoritative.** A router cannot silently overwrite an explicit caller value with `with:`.

The merge is shallow at the top level: a scalar or array replaces the target's value, an object **replaces** the target's object at that key rather than deep-merging into it, and `null` removes the target-authored property before composition. A caller override can restore a key the overlay removed.

The overlay is scoped to the **immediate** target:

- it survives that target's retry, resume, and loop-iteration refreshes;
- it is visible to every lifecycle event and to body composition for that target;
- it is **never written to disk** — neither document's bytes or `hash:` change; and
- it is discarded when that target proxies onward.

Forwarding down a chain is explicit. A downstream `proxy` receives only its own `with:` plus the immutable caller overrides — omitting `with:` on the second hop installs an *empty* overlay rather than inheriting the first hop's:

```yaml
- action: proxy
  target: "@prompts/final.md"
  with:
      spec: "{{ spec }}"      # forwarded because it is named here
```

Cycle detection and the hop limit are keyed on resolved document paths; an overlay does not create a distinct document identity.

#### `with:` versus `set_frontmatter`

Both put a value in front of a document. They are not interchangeable:

| | `proxy.with` | `set_frontmatter` / `merge_frontmatter` |
|---|---|---|
| Persistence | In-memory for one target activation | Writes the file on disk |
| Scope | The immediate proxy target only | Whatever file the action names |
| Lifetime | Discarded at the next hop | Permanent until something rewrites it |
| Visible to | That target's composition, lifecycle, schema, and body | Any later reader of that file |

Reach for `with:` to parameterize the document you are handing to. Reach for `set_frontmatter` when the value must outlive the run.

#### Trust model: prefer schema-declared data properties

For ordinary parameter passing, declare the properties in the target's `$schema` and pass them as data. The schema is what makes the contract visible, validated, and completable:

```yaml
# prompts/next.md
$schema:
  attempt: 'number(required)'
  label: string
```

`with:` may nevertheless set **any** top-level key, including control-plane keys — `agent`, `model`, `loop`, `$schema`, `timeout`, MCP, or a lifecycle event block. This is an **advanced, trusted-prompt capability**, and it is deliberate rather than an authority escalation: a source prompt that can `proxy` at all could already select those behaviors itself or run the equivalent lifecycle actions. It is still executable configuration, not inert data — so treat a `with:` that writes control-plane keys the way you would treat handing someone your config file.

The safety properties that hold regardless:

- the target **reparses and revalidates** every structural value the overlay installs — a malformed control-plane overlay fails as the target's own parse error, pre-launch;
- an invalid overlay fails the target's schema **before any provider launches**;
- a shell command installed by an overlay is discovered and approved by the *target's* pre-flight, subject to normal target-side policy — approved bytes equal executed bytes; and
- status output may report that a handoff carries an overlay, and tracing may record property names and counts, but neither prints overlay values.

### Shell Actions

The `shell` action runs an approved shell command. Commands are collected during pre-flight shell approval alongside `::shell` directives and `$(...)` frontmatter expressions.

```yaml
start:
  stack:
    - action:
        action: shell
        command: "npm run typecheck"
        on_error: "typecheck failed"
```

A non-zero exit code is an action error unless `no_error: true` is set.

### Side-Effect Actions

Any Darkmatter side-effect verb can be invoked by name:

```yaml
start:
  stack:
    - action: { set_frontmatter: ["state.md", "status", "in-progress"] }
success:
  stack:
    - action: { set_frontmatter: ["state.md", "status", "done"] }
```

Long-form side-effect actions accept named parameters that are reordered into the verb's positional signature:

```yaml
success:
  stack:
    - action:
        action: http_post
        url: "https://example.com/hook"
        body: "{{payload}}"
```

### Expression-Function Actions

Any Darkmatter read-only expression function can be invoked for its result. The result is logged in the lifecycle/status style.

```yaml
start:
  stack:
    - action: { file_exists: "@docs/plan.md" }
```

### `no_error`

The `no_error` flag can be set on any action category. When `true`, an unintentional side-effect **dispatch** failure is logged but does not stop the stack or change the composition outcome. Its scope is the side-effect layer only: an expression-layer evaluation error (a crashed `when:` guard or a `{{{ … }}}` interpolation that raised) always halts and is never suppressed by `no_error`.

```yaml
start:
  stack:
    - action:
        action: shell
        command: "git status --short"
        no_error: true
    - action: { info: "continuing" }
```

## Lifecycle Context

Stack expressions have access to three lifecycle-only globals in addition to frontmatter, `ctx.*`, `env.*`, and `doc.*`:

| Global | Available in | Fields |
|--------|--------------|--------|
| `err` | `blocked`, `failure`, `finalize` | faceted fields below (`code`, `category`, `disposition`, `origin`, `detail.*`, plus promoted conveniences) |
| `timing` | every event | `document_ms`, `total_ms`, `step_ms` (all optional) |
| `current` | every event | `current.ctx.*`, `current.env.*` (lazy snapshots at event time) |

`err` is only meaningful in events that can carry an error. Using bare `err` (or `err.*`) in `initialize`, `start`, `success`, or `loop` is rejected at parse time.

### `err` Fields

Match handlers on these **faceted** fields — a stable, versioned contract (see the error catalog, and [error-architecture.md](error-architecture.md) for how a failure's facets are selected). Matching on these instead of human prose is what makes a lifecycle handler portable across providers and codes.

Every field below describes the **effective diagnostic** — the one error in the failure's cause chain selected to speak for it. The terminal block you see, the `err.*` you match on, and the serialized machine output are all projected from that same selection, so what you match can never be a different error than what was rendered.

| Field | Type | Description |
|-------|------|-------------|
| `err.code` | string | Stable dotted code, e.g. `composition.invalid_file_reference`, `cap.plan_limit`, `timeout.step_silence`. The most specific handle. |
| `err.category` | string | Coarse domain — the dotted prefix of `code` (`composition`, `cap`, `timeout`, `provider`, `document`, `vcs`, `io`, `config`, `usage`, `runaway`, `auth`, `internal`). |
| `err.disposition` | string | Generic remediation strategy: `transient`, `throttled`, `correctable`, `needs_input`, or `unrecoverable`. |
| `err.origin` | string | Who remediates: `provider`, `author`, `caller`, `environment`, or `internal`. |
| `err.severity` | string | Operator-facing severity: `info`, `warning`, or `error`. Defaulted from `disposition` (`transient`/`throttled`/`needs_input` → `warning`, `correctable`/`unrecoverable` → `error`) and overridable per code. |
| `err.detail.*` | typed | Per-instance payload — the fields that vary per occurrence (`err.detail.reference`, `err.detail.property`, `err.detail.reset_at`, …). Shape depends on `code`; an absent field reads as `null`. |
| `err.msg` | string | Concise, notification-safe rendering of the selected error — see [below](#errmsg). |
| `err.cause.*` | typed | The next registered diagnostic below the selected one — see [below](#errcause). |

#### A registered code always carries a detail object

If `err.code` names a code in the catalog, `err.detail` is **always an object**, with **every field that code declares present as a key**. A value the run could not determine reads as `null`.

This matters for two things that look alike but are not:

- `err.detail.reset_at == null` — the catalog says `cap.rate_limit` has a `reset_at`, and this occurrence did not carry one. Honest, and matchable.
- `err.detail.reset_at` against a *scalar* `err.detail` — a bug. It cannot happen: a code with no per-instance data available still projects the full all-`null` object rather than a bare `null`.

So `err.detail.<field>` is always a safe read for a registered code, and the value — not the shape — is what tells you whether the field was known. Only bare `when: "err.detail"` sees a difference, and it is a truthiness test on a container that no handler should be making.

Promoted conveniences (sugar over the canonical fields, present only when the error is classifiable):

| Field | Type | Description |
|-------|------|-------------|
| `err.is_transient` / `err.is_throttled` / `err.is_correctable` | bool | Predicate sugar derived from `err.disposition`. |
| `err.reset_at` | string | RFC 3339 timestamp lifted from `err.detail.reset_at` (cap codes); `null` otherwise. |
| `err.retry_after_ms` | number | Suggested wait lifted from `err.detail.retry_after_ms` (cap codes); `null` otherwise. |

```yaml
failure:
  stack:
    - when: "err.code == 'composition.shell_expansion'"
      action: { notify: "Shell command was denied" }
```

#### `err.msg`

The selected diagnostic's **concise** message: single-line, escape-free, and length-clamped (~240 characters), so a TTS route, a webhook, or a desktop notification can use it verbatim.

It is deliberately **not** the multi-line rendered block — that block is for a terminal, and pushing it through a speech synthesizer or a Slack message is what the clamp exists to prevent. It is also not a classifier input: match on `err.code`, never on this text.

```yaml
failure:
  stack:
    - action: { say: "{{ err.msg }}" }          # safe: one clean line
    - action: { notify: "{{ err.code }}: {{ err.msg }}" }
```

For a provider attempt failure, `err.msg` keeps the established `harness::failure_message` precedence (its headline, timeout, and stderr fallbacks, plus the `(attempt N)` suffix) rather than being replaced by a typed error's `Display`. Both producers pass through the same hygiene stage, so a typed error and a provider failure cannot reach a notification under different rules.

#### `err.cause`

The **next registered diagnostic** below the selected one, when the chain has one. Unregistered prose causes are walked through, so `err.cause` is the next thing with facets — not merely the next `source()`.

| Field | Type |
|-------|------|
| `err.cause.code` / `err.cause.category` / `err.cause.disposition` / `err.cause.origin` / `err.cause.severity` | string |
| `err.cause.detail.*` | typed |
| `err.cause.msg` | string |

It is a strict **one-level** projection. **`err.cause.cause` is not exposed in v1** — it is unrepresentable in the underlying type rather than merely undocumented, so it cannot quietly start working. `err.cause` is `null` when the selected error has no registered cause, and always `null` for a facet-less failure.

```yaml
failure:
  stack:
    # the write failed; the cause says why
    - when: "err.code == 'io.write_failed' && err.cause.code == 'io.permission_denied'"
      action: { notify: "Cannot write {{ err.detail.path }} — check permissions" }
```

#### Deprecated aliases

The original `err` fields remain available for backward compatibility but are **deprecated** — new documents should match the faceted fields above.

`err.kind` and `err.variant` are the deprecated *spellings* of `err.category` and `err.code`: for a classifiable error they carry exactly those facet values, so prefer the faceted names directly. They fall back to Claudine's internal Rust error labels **only** for a facet-less action failure (a generic `shell`/`set_frontmatter` verb that maps to no diagnostic code), where there is no faceted equivalent — those residual values describe the internal shape, drift, and are not portable.

| Deprecated field | Type | Description |
|------------------|------|-------------|
| `err.kind` | string | Deprecated alias of `err.category`. Mirrors the category for a classifiable error; falls back to the internal Rust error *type* name (`ClaudineError`, `HarnessError`, `CompositionError`) only for a facet-less action failure. |
| `err.variant` | string | Deprecated alias of `err.code`. Mirrors the code for a classifiable error; falls back to the internal Rust enum *arm* name (`Io`, `ShellCommandDenied`, `SchemaLoad`) only for a facet-less action failure. |

### `doc.err` Escape Hatch

A frontmatter property literally named `err` can still be reached through the `doc` namespace. This is the only way to reference an `err` value in no-error events.

```yaml
err: "user-configured reason"
start:
  stack:
    - action: { stderr: "{{doc.err}}" }
```

## Loop Gate Concerns

The `loop` property carries both iteration controls (`while`/`until`, `action`/`actions`, `max`, `fail_fast`) and lifecycle concerns (`say`, `stack`, etc.). Lifecycle concerns inside `loop` fire on every gate pass, including the terminal pass that exits the loop.

```yaml
loop:
  while: "iteration < max_iterations"
  actions:
    - increment(iteration)
  stderr: "checking loop condition"
  stack:
    - action: { info: "loop gate reached" }
```

Loop execution runs `initialize` once at the start, then re-enters each iteration at `start` without re-running `initialize`, schema validation, or shell pre-flight. `success`, `failure`, and `finalize` fire once per iteration. The loop condition is evaluated **after** lifecycle concerns and **before** per-iteration mutations are applied.

## Examples

### Minimal: terminal status only

```yaml
---
start:
  stderr: "Starting code review..."
success:
  stderr: "Code review complete"
---
```

### TTS with sound effect

```yaml
---
start:
  say: "Starting the deployment pipeline"
  effect: "confirmation"
success:
  say: "Deployment finished successfully"
  effect: "crowd-applause"
failure:
  say: "Deployment failed"
  effect: "sad-trombone"
---
```

### Initialize and finalize

```yaml
---
initialize:
  stderr: "Setting up workspace"
start:
  stderr: "Running agent"
success:
  stderr: "Agent finished"
finalize:
  stderr: "Cleaning up"
---
```

### Conditional stack with `err`

```yaml
---
failure:
  stack:
    - when: "err.category == 'composition'"
      action: { notify: "Composition failed" }
    - action: { say: "Something went wrong" }
---
```

### Positional actions with interpolation

Action values are literal text; `{{{ … }}}` interpolates a value:

```yaml
---
start:
  stack:
    - action: { info: "running {{agent}}" }
    - action: { shell: "git fetch origin {{branch}}" }
---
```

### `no_error` shell action

```yaml
---
start:
  stack:
    - action:
        action: shell
        command: "which optional-tool"
        no_error: true
    - action: { info: "continuing" }
---
```

### Loop lifecycle concerns

```yaml
---
iteration: 1
max_iterations: 3
loop:
  while: "iteration <= max_iterations"
  actions:
    - increment(iteration)
  stderr: "loop gate"
  stack:
    - action: { info: "iteration {{_loop_count}}" }
---
```

### Recover from a usage cap by switching providers

When a provider hits a usage cap or rate limit, the failure surfaces in the `failure` event classified into the locked `cap.*` codes — `cap.rate_limit`, `cap.plan_limit`, or `cap.billing`. The classifier folds each provider's raw label into one of these, so the guard matches the **contract**, not provider-specific strings. There is no in-place "switch provider" action; instead `proxy` hands the same task off to a sibling prompt that pins a different agent.

```yaml
---
agent: claude
prompt: "Implement the feature described in @spec.md"
failure:
  stack:
    - when: "err.category == 'cap'"
      action:
        - warn: "Claude usage cap reached — handing off to Codex"
        - proxy: "@prompts/feature-codex.md"
---
```

`@prompts/feature-codex.md` is the same task with `agent: codex` in its frontmatter; `proxy` starts it fresh at its own `initialize`. Matching `err.category == 'cap'` catches every cap — rate limit, plan/usage quota, and billing stop. To react only to the *waitable* caps (rate limit and plan/usage quota, which auto-lift) and not a hard billing stop, match `err.is_throttled` instead; to target one code, use `err.code == 'cap.rate_limit'`.

### Verify an artifact on success, then retry once from `finalize`

`success` can double-check that the agent actually produced what it claimed. Raising `error` there converts the outcome to failure and routes through the `failure` event **before** `finalize`. Because `finalize` is the optional-error terminal event, it carries that `err` and can recover from it — so the verification lives in `success` and the single retry lives in `finalize`.

```yaml
---
prompt: "Generate the release notes and write them to @output/RELEASE.md"
success:
  stack:
    # The agentic loop returned cleanly — confirm the file really exists.
    - when: "!file_exists('@output/RELEASE.md')"
      action: { error: "agent reported success but @output/RELEASE.md was never written" }
finalize:
  stack:
    # `finalize` carries `err` after the success-side downgrade. Retry the
    # whole run exactly once; the retried attempt re-enters at `start`.
    - when: "err"
      action: { retry: 1 }
---
```

On the retried attempt the agent runs again and `success` re-verifies the file. With `retry: 1` the budget allows exactly one extra attempt; if the file is still missing after it, `finalize` carries `err` once more, the retry budget is spent, and the run ends in failure. To announce that terminal case, add a guarded `warn` ahead of the `retry` item (the first matching control action ends the stack, so order the `warn` before the `retry`).

### Resume after a timeout

Both the wall-clock `timeout` and the step-silence `step_timeout` surface in the `failure` event under the locked `timeout.*` codes — `timeout.wall_clock` and `timeout.step_silence`. `resume` continues the **same** agent session — context intact — with a follow-up message, which is usually better than `retry` for a timeout (retry re-runs the invocation from scratch).

```yaml
---
prompt: "Refactor @src/engine.rs and make the test suite pass"
timeout: 20m
step_timeout: 5m
failure:
  stack:
    - when: "err.category == 'timeout'"
      action: { resume: "You were stopped by a timeout. Continue from where you left off and finish the task." }
---
```

`err.category == 'timeout'` matches both kinds; to distinguish them, match `err.code == 'timeout.wall_clock'` or `err.code == 'timeout.step_silence'`.

`failure` is the natural home for this recovery, but it is not the only legal one: `resume` is valid in **every** event, and needs only a live provider session (pre-launch it surfaces `ResumeWithoutSession`). A `success` stack can resume the agent just as well when the run finished cleanly but left something undone. `resume` defaults to a single attempt (`max_attempts: 1`), and its string argument binds to the required `message:` parameter.

## Sound Effects

The `effect` field accepts a kebab-case name from the built-in catalog. Names are matched after stripping hyphens and lowercasing.

- see sound effects for an enumeration of sound effects

## State Machine

Claudine uses a `LifecycleRunGuard` to enforce correct state transitions and guarantee terminal signals are emitted.

### Signal States

| Signal | Terminal Status | stderr Style |
|--------|----------------|--------------|
| `Initialize` | Non-terminal | Info |
| `Start` | Non-terminal | Info |
| `Success` | Terminal | Success |
| `Blocked` | Terminal | Error |
| `Failure` | Terminal | Error |
| `Finalize` | Non-terminal | Info |
| `Loop` | Non-terminal | Info |

### Drop Safety-Net

If `start` is emitted but no terminal signal (`success`, `blocked`, `failure`) is emitted before the guard drops, a terminal signal is automatically fired:

| `start` emitted? | Provider launched? | Drop emits |
|------------------|-------------------|------------|
| No | — | Nothing |
| Yes | No | `Blocked` |
| Yes | Yes | `Failure` |

This ensures compositions that panic or exit unexpectedly still report a terminal lifecycle signal.

### Explicit Control

Code using the guard can:

- Call `emit_initialize_once()` to emit `initialize` (idempotent; fires exactly once across a loop run)
- Call `emit_start_once()` to emit `start` (idempotent)
- Call `mark_provider_launched()` after the provider child spawns
- Call `emit_terminal(signal)` to emit a terminal signal and suppress the drop safety-net
- Call `emit_blocked_or_failure()` to emit `Blocked` (pre-launch) or `Failure` (post-launch)
- Call `emit_finalize_once()` to emit `finalize` once after a terminal signal
- Call `reset_for_next_iteration()` to reset per-iteration state for the next loop iteration
- Call `defuse()` to suppress the drop safety-net without emitting anything

## Validation

Claudine validates lifecycle frontmatter at parse time. Errors prevent composition from starting.

### `LifecycleSayConflict`

Both `say` and `say_first` are present in the same notification. Only one speech field is allowed.

```yaml
start:
  say: "Starting"
  say_first: "Also starting"  # ERROR: conflict
```

### `LifecycleUnknownEffect`

The `effect` field references a sound effect name not in the compiled catalog.

```yaml
start:
  effect: "unknown-sound"  # ERROR: unknown effect
```

### `LifecycleInvalid`

The notification object contains unknown fields or a type mismatch (e.g., `say: 123`).

```yaml
start:
  say: "Valid"
  unknown_field: "value"  # ERROR: unknown field
```

### `LifecycleInterpolationLeak`

Because lifecycle strings are interpolated at event-time (see [When Lifecycle Properties Interpolate](#when-lifecycle-properties-interpolate)), their authored `{{{ … }}}` spans are **not** prepare-time leaks — they are deferred by design. This guard (`reject_surviving_spans`) runs **after** the event-time resolution, immediately before dispatch: a side-effect string that still contains a `{{{ … }}}` span at that point (e.g. a frontmatter value that is itself raw template text) is a typed error and the side effect is not sent. For non-lifecycle surfaces the unchanged prepare-time guard `validate_no_interpolation_leaks` still runs.

### `LifecycleUndefinedVariable`

A reference to a genuinely-unknown root — a typo such as `{{spec_fil}}` for `{{spec_file}}` — fails the event closed at event-time via Darkmatter's strict mode. A *known* root that resolves to empty (`{{spec_file}}` when the key is legitimately absent) renders empty and does not error. To tolerate an unknown optional name, use explicit fallback syntax: `{{ maybe || '' }}`.

```yaml
success:
  stderr: "Done: {{undefined_kee}}"  # ERROR at event-time: unknown root (typo)
```

### `LifecycleErrNotAvailable`

`err` is referenced in an event that never carries an error (`initialize`, `start`, `success`, `loop`). The scan walks the `{{{ … }}}` spans inside communication/action strings **and** the whole `when:` expression, and rejects at parse time (`validate_no_err_in_no_error_events`, using `literal_spans_reference_err` for the interpolation spans). `timing`/`current` are allowed everywhere — via the shared `LATE_BINDING_ROOTS` known-root authority, also consulted by `resolves_outside_frontmatter`; `doc.err` remains the escape hatch.

```yaml
start:
  stack:
    - action: { stderr: "{{err.code}}" }  # ERROR: err not available in start
```

### Empty String Normalization

Empty strings and whitespace-only strings are normalized to `null`, so these are equivalent:

```yaml
start:
  say: ""
  message: "   "
```

Both `say` and `message` are treated as absent.

## Non-Object Frontmatter

If the frontmatter is not an object (e.g., a bare string or list), lifecycle parsing returns a default empty configuration with no notifications.

## Integration Notes

- **TTS**: Uses the global TTS configuration from `claudine.toml` (voice, rate, provider). If no TTS settings are configured, uses system defaults.
- **Messaging**: Requires a configured messaging route. See Configuring Actions.
- **Desktop notifications**: Zero-config. Emitted via `notify` independently of messaging routes. Failures are non-fatal.
- **stderr/info/warn**: Rendered as styled status badges using the terminal's capability detection (circular theme with color-coded state).
- **Audio playback**: Blocking. Sound effects and TTS play sequentially, not in parallel, to avoid overlapping audio.
- **stdout**: The lone lifecycle channel that writes to stdout (all others target stderr, messaging, or desktop notifications). Because stdout is otherwise reserved for pipeable command output, lifecycle `stdout` text interleaves with the composed/provider output on that stream — opt in deliberately when a pipeline (`claudine compose <file> | other-tool`) should see the text.

## Related Topics

- [Composition](composition.md) — the composition pipeline and loop behavior
- Configuring Actions — messaging routes and action configuration
- Non-Interactive Sessions — stderr rendering and terminal output
