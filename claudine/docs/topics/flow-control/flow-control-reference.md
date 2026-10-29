---
kind: canonical-documentation
---

# Flow Control Reference

The detailed rules behind the lifecycle flow-control directives. Read [Flow Control](flow-control.md) first. This page assumes you know what each directive is for and only records the edge cases, error names, and guarantees.

## What each event can do at runtime

Placement is checked once, at parse time: `skip` is accepted only in `initialize`, and `cancelled` accepts only `stop` from the directive set (`LifecycleControlAction::is_valid_for`). At runtime, every event hands its directive to the same event-agnostic dispatch path (`decide_control` followed by `dispatch_terminal_control`). What varies is whether the run is in a state where the directive can do anything:

| Event | `retry` | `resume` | `proxy` |
|-------|---------|----------|---------|
| `initialize` | `LifecycleSetupPhaseRecoveryUnsupported` | `ResumeWithoutSession` | Full handoff |
| `start`, `success`, `failure`, `finalize` | Full | Full, given a live session | Full handoff |
| `blocked` | `LifecycleSetupPhaseRecoveryUnsupported` | `ResumeWithoutSession` | `LifecycleSetupPhaseRecoveryUnsupported` |
| `loop` (the iteration gate) | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` |

`stop` works in every event. `error` is rejected in `cancelled`. Recovery in the `finalize` after cancellation raises a typed cancellation-recovery error and launches no work. `defer` fails with `LifecycleDeferNotImplemented` in every event until its scheduler exists. `break` and `prep` run in every event except `cancelled`, and in the loop gate; see [`break` and `prep` in detail](#break-and-prep-in-detail).

The unsupported cells have no re-entry point to return to, so they raise a typed error instead of quietly doing nothing. Put those recoveries on `start`, `success`, `failure`, or `finalize`. A refused recovery in a document's `blocked` does not fire `failure`: `finalize` runs with the original error, and the refusal becomes the run's reported error. `blocked` refuses recovery whether it came from pre-flight or from a proxy handoff refused after `start`; see [Lifecycle — How Failures Route](lifecycle.md#how-failures-route).

A `proxy` from `initialize` is a real handoff, not a reduced pre-launch path. The coordinator that tracks the active document sits above both the document loop and the provider harness, so the target is prepared exactly as a directly invoked document would be.

The `loop:` iteration condition (`while`/`until`) is a separate mechanism. It does not go through directive dispatch.

## `retry` and `resume` in detail

### Containers

Sequences and groups have no live provider session, so `resume` raises the no-session error. `retry` reruns the container from the beginning after every old child settles, without repeating `initialize` or finalizing the abandoned attempt. For a container without a loop, child results and outcome records reset while runtime mutations survive. For a looping container, only the current pass restarts: its runtime mutations and output slice return to their state at pass entry, preserving completed passes. The retry overlay is applied after that restoration, and the loop count does not advance; see [Retry inside a looping container](looping.md#retry-inside-a-looping-container). A container's `retry.with` values enter its runtime state, rather than the document-only one-attempt overlay described below. Nothing rolls back disk edits or other external effects. `proxy` hands the container's position to a target and skips the source's remaining events, including `finalize`.

An unhandled child failure propagates to the nearest container's `failure` event after child cleanup. A sequence stops unless recovery succeeds. A group with `fail_fast: false` handles leg failures; its own preparation/event errors and Ctrl-C still fail or cancel it. With default `fail_fast: true`, it stops admission, cancels opted-in running legs, waits for the rest, and fails atomically for in-memory results/mutations. See [Sequences](sequences.md#parallel-groups).

### Budgets

`max_attempts` counts attempts *beyond* the one that just ran. The budget is fixed the first time a given directive fires, so re-running the prompt cannot reset it. Once the budget is spent the directive behaves like `stop`: the event ends and the run keeps the outcome it had.

With `backoff: exponential`, the first retry waits `delay`, and each later retry waits twice as long as the one before. `delay` uses the same duration syntax as `timeout` (`"30s"`, `"5m"`).

### Where a retry re-enters

`retry` picks its re-entry point from whether the agent had launched, not from which event fired it:

- **Agent not launched yet:** pre-flight runs again.
- **Agent launched:** the agent is invoked again, and the run re-enters at `start`.

### What survives a retry or resume

Both directives replace only the **provider attempt**. The active document stays the same. On re-entry:

- The document is re-read from disk and fully validated again.
- The `proxy` `with:` overlay, the proxy history, and the remaining attempt budgets are kept.
- A `retry` that carries its own `with:` layers it over that fresh read, for that one attempt only (see [Retry overlays](#retry-overlays)). A `resume` never carries one.
- The document's environment is kept: `env.*`, env-expanded file references, and shell commands read the map the document captured when it started, not a new one.
- `initialize` does **not** fire again. It fires once per active document, not once per attempt.
- Launch settings are recomputed from the freshly read document. A `model:` edited between attempts takes effect on the next attempt.

### When `resume` refuses

Before continuing a session, `resume` compares a session-compatibility key across the refresh. If a relevant setting changed, it refuses with `CompositionError::LifecycleResumeIncompatible { facets }`, names what changed, and recommends `retry`. The refusal happens after `start` has fired and before the agent is spawned again. It then follows the normal failure path: `failure` fires, then exactly one `finalize`, and both see the refusal as `err`. `success` does not fire. The full list of compared settings is in [Composition — Retry and resume re-entry](../composition.md#retry-and-resume-re-entry).

## `proxy` in detail

### The target becomes the active document

A run has exactly one **active document**. `proxy` replaces it. The target enters at its own `initialize` and owns everything after that: the remaining events, the completion step, and the output.

A clean handoff fires no further events for the source document. A `proxy` from `success` or `failure` skips that attempt's `finalize`, and a `proxy` from `finalize` does not re-enter `finalize`.

The target is a new document, so it captures its own environment after it is resolved and before its `initialize` runs; its `env.*`, file references, model selection, and shell commands read that map, while the source keeps reading its own. The target is resolved in the source's file context, so a `{{VAR}}` in the `proxy` path reads the source's map. See [Composition — One Environment per Document](../composition.md#one-environment-per-document).

The target is bootstrapped by the same preparation service that prepares a directly invoked document. `claudine compose target.md` and a `proxy` to `target.md` therefore go through the same `initialize`, schema validation, shell discovery and approval, and diagnostics. See [Composition — Document Handoffs and the Equivalence Contract](../composition.md#document-handoffs-and-the-equivalence-contract) for what the target recalculates for itself.

Proxy chains are limited to 16 hops per run (`MAX_PROXY_HOPS`). Cycle detection and the hop limit key on resolved document paths.

### How a proxy target is found

Targets resolve through the shared `biscuit-file::FileReference` contract, relative to the document that wrote the `proxy`:

| Form | Where it looks |
|------|----------------|
| `other.md` | Next to the authoring document, then the repository root |
| `./other.md`, `../other.md` | Relative to the authoring document only |
| `@other.md` | The local tree first (Claudine's convention roots, then the package, package-area, and repository or launch-directory roots), then home locations (`~/.claudine/prompts`, home, `~/.claudine`) |
| `&other.md` | The repository root only |
| `^other.md` | Package, then package-area, then repository roots |
| `~/other.md` | Your home directory only |

The target must exist. A missing target fails with a typed `Unresolvable file reference` error. The target is one file, never a glob: a missing `docs/*.md` names a file called `*.md`, and its error says so. When a target writes its own `proxy`, relative lookups start from the target.

### Passing values with `with:`

`with:` is a transient overlay of top-level frontmatter properties for the document a transition reads next. Three directives accept it:

- on `proxy` it parameterizes the **target** of the handoff;
- on `retry` it parameterizes the **fresh re-read of the same document** for the retried attempt (see [Retry overlays](#retry-overlays));
- on `prep` it parameterizes the **called target**, with `proxy`'s shape, typing, and evaluation rules. It is the target's only caller values: the caller's own setters do not reach a `prep` target (see [`prep`](#prep)).

`resume` takes no `with:`. It continues the existing provider session, while
still rereading and validating the active document before sending its follow-up
message. Use `set` for invocation-local values that must survive that refresh.
A sequence's prompt task takes a `with:` with the same shape, typing, and file
rules ([Task inputs](sequences.md#task-inputs-with)).

```yaml
success:
  - proxy:
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

The rules below hold for all three directives unless a section says otherwise.

#### Shape rules

`with:` is accepted only in the long form. `with: {}` means the same as leaving it out. Each of these is a typed frontmatter error that points at the source:

| Mistake | Error |
|---------|-------|
| `with:` is not a mapping | `LifecycleWithNotMapping` |
| A key contains `{{ … }}` | `LifecycleWithDynamicKey` |
| The whole mapping is one span, such as `with: "{{ payload }}"` | `LifecycleWithWholeMapping` (out of scope for v1) |
| `with:` appears on a directive other than `proxy`, `retry`, or `prep`, `resume` included | `LifecycleWithUnsupported` |
| `with:` written beside a short-form directive in the same item (`- proxy: "x"` plus a sibling `with:`) | `LifecycleUnknownVerb`, because a sibling key is read as a verb; use the long form |

#### Value types

Keys are static strings that name target properties. They are never interpolated. Values follow the same typing rule as every other lifecycle parameter, applied recursively through arrays and objects:

- A string containing text and spans (`"phase-{{ iteration }}"`) becomes a string.
- A string that is **exactly one** span keeps the expression's type. `"{{ true }}"` becomes boolean `true`, not the string `"true"`.
- YAML numbers, booleans, arrays, objects, and nulls keep their authored types.

Nested mappings are data here. This is the only place in the lifecycle surface where a nested mapping is a legal parameter value.

#### Relative files resolve from the file that wrote them

A relative file value in `with:` resolves from the document whose lifecycle holds the directive, wherever the target lives:

```yaml
# prompts/router.md
success:
  - proxy:
      target: "@prompts/review/next.md"
      with:
        plan: ./plans/today.md   # prompts/plans/today.md, not prompts/review/plans/today.md
```

- **Only file values move.** A value is resolved only when the target's schema types it as `file`; any other string stays the string you wrote. When the schema allows both a file and a string (`[file, string]`), the value stays as written.
- **The schema decides how.** An eager `file` must exist, from the authoring document's directory, before the target runs. A lazy `file` that does not exist binds to the path it would have there. A remote URL stays a URL.
- **Forwarding keeps identity.** `{{ plan }}` that copies a value the router's own schema already resolved passes that resolved path on. A relative string a document passes on without having resolved it (its schema typed it as a string) resolves from that document, the one whose `with:` names it.
- **`retry` resolves from the same document**, since the document retrying is the one that wrote it. **`prep`** resolves from the calling document.
- **Caller values keep their own origin.** A `key=value` or `--set` value still resolves from the directory the command was launched in, and still outranks `with:`.

This is a change: a `proxy.with` path used to resolve as if the target had written it (and a `prep.with` path from the launch directory). A `with:` that relied on that must be rewritten relative to the document that holds it.

#### Evaluation

The whole mapping is evaluated **once, in the source document**, when the directive fires. Evaluation uses Darkmatter subtree composition against the source's live frontmatter plus the globals the event declares (`seq`, `err`, `timing`, `group`, `loop`; see [Lifecycle](lifecycle.md#binding-time-early-vs-late)) and the `current` root. An absent property is `null`; a genuine evaluation error leaves no overlay at all. A `set_frontmatter` earlier in the same event is visible. The target receives resolved data, never a template: the overlay reaches the target's composition as a data layer, so a `{{ … }}` or `$( … )` inside a resolved value (for example, text an agent wrote) is kept as text and is never evaluated in the target, on its first read or on a `retry`/`resume`.

A `with:` value for a lifecycle key (`initialize`, `start`, `success`, `blocked`, `failure`, `cancelled`, `finalize`, `loop`) is the exception. The target re-parses it as its own event or loop and evaluates those strings at event time, so a resolved value that still holds a `{{ … }}` span there fails with `LifecycleWithEvaluationFailed` instead of becoming an instruction.

Evaluation is atomic. The target and the whole mapping resolve before anything changes. If any value fails (a malformed expression, an unknown function, or an out-of-scope variable such as `group` outside a group), no overlay is installed, no transition happens, and the run follows the normal failure path for the event that fired: a `proxy` never touches its target and the source stays active; a `retry` starts no attempt and spends none of its budget. `no_error` does not suppress this, because it is an expression error rather than a side-effect failure. The error, `LifecycleWithEvaluationFailed`, names the event, the directive, the document the overlay was for (the `proxy` target, or the current document for `retry`), and the deepest path inside `with:` that it can identify, for example `` `failure[0].then[0].retry.with.reason` could not be resolved for the `retry` to `<current document>` ``. It never prints a resolved value, which could be a secret.

#### Precedence and lifetime

This section describes a `proxy` overlay; [Retry overlays](#retry-overlays) covers how a `retry` overlay differs.

The overlay is merged into every read of the target's frontmatter, both the bootstrap read before `initialize` and the fresh read after it, before composition and schema validation, and composition receives the same values as data. From lowest to highest precedence:

1. The target's own frontmatter.
2. The immediate proxy's `with:`.
3. Caller overrides (`key=value` / `--set`).

The original caller always wins. A routing document cannot overwrite a value the caller set explicitly.

The merge is shallow. A scalar or array replaces the target's value, an object replaces the target's object at that key instead of deep-merging, and `null` removes the target's property. A caller override can restore a key that the overlay removed.

The overlay applies to the immediate target only:

- It survives that target's retries, resumes, and loop iterations.
- Every event and the body composition of that target can see it.
- It is never written to disk. Neither document's bytes nor its `hash:` change.
- It is discarded when the target proxies onward.

Forwarding a value down a chain must be explicit. A second-hop `proxy` without `with:` installs an **empty** overlay; it does not inherit the first hop's:

```yaml
success:
  - proxy:
      target: "@prompts/final.md"
      with:
        spec: "{{ spec }}"      # forwarded only because it is named here
```

An overlay does not give a document a distinct identity for cycle detection.

#### Retry overlays

A `retry` overlay is evaluated exactly like a `proxy` overlay, but it applies to **one transition** of the same document:

```yaml
failure:
  - when: "err.is_transient"
    then:
      - retry:
          max_attempts: 3
          with:
            reason: "{{ err.msg }}"
```

- **One transition.** The overlay applies to the retried attempt's fresh read of the document, and to nothing after it. Both reads that attempt makes see it: the shell pre-flight read and the read the provider runs against. The prompt can read it as `fm.reason`.
- **Layered over a retained `proxy` overlay.** On a document that a `proxy` handed to, the retained `proxy` overlay still applies, and the `retry` overlay is merged key by key on top of it, so the `retry` value wins a shared key. Caller overrides (`--set`) still win over both.
- **Never carried forward.** A later `retry` without `with:`, a `resume`, and a `proxy` adoption never see it. A later `retry` with its own `with:` replaces it.
- **Data, not templates, and never on disk.** Values reach the fresh read as resolved data, are never evaluated again as templates, and are never written to the file.
- **A failure spends nothing.** If any value fails to evaluate, the `retry` takes no transition and spends none of its attempt budget; `no_error` cannot hide the failure. An overlay that the document's `$schema` rejects fails that attempt's refresh, as any invalid frontmatter would.

Attempt budgets, `delay`, `backoff`, and re-entry are unchanged: an overlay cannot reset a budget or skip the refresh validation.

#### `with:` compared with `set_frontmatter`

| | `proxy` / `retry` `with:` | `set_frontmatter` / `merge_frontmatter` |
|---|---|---|
| Persistence | In memory, for one target activation (`proxy`) or one attempt (`retry`) | Written to the file on disk |
| Scope | The immediate proxy target, or the retried document | Whichever file the action names |
| Lifetime | Discarded at the next hop (`proxy`) or after one attempt (`retry`) | Until something rewrites it |
| Visible to | That target's composition, events, schema, and body | Any later reader of the file |
| A `{{ … }}` or `$( … )` in a resolved value | Data in the target; never evaluated | Stored as a literal token; read back as the same text |

Use `with:` to parameterize the document you are handing to. Use `set_frontmatter` when the value must outlive the run. Neither can hand a template to another document: both deliver resolved values as data (see [Frontmatter an action writes is data](lifecycle.md#frontmatter-an-action-writes-is-data)).

#### Trust model

For ordinary parameters, declare them in the target's `$schema` so the contract is visible and validated:

```yaml
# prompts/next.md
$schema:
  attempt: 'number(required)'
  label: string
```

`with:` can still set **any** top-level key, including control keys such as `agent`, `model`, `loop`, `$schema`, `timeout`, MCP settings, or a lifecycle event block. This capability is intended for trusted prompts. It does not escalate authority, because a document that can `proxy` could already choose those behaviors itself. It is still executable configuration, not inert data, so treat a `with:` that sets control keys as you would treat handing someone your config file.

These guarantees hold regardless:

- The target re-parses and re-validates every structural value the overlay installs. A malformed control key fails as the target's own parse error, before launch.
- An overlay that breaks the target's schema fails before any agent launches.
- An overlay cannot enable shell commands during initialization. Shell commands in later events are discovered and approved by the target's own pre-flight, so the approved bytes are the executed bytes.
- Status output may say that a handoff carries an overlay, and tracing may record property names and counts. Neither prints overlay values.

## `break` and `prep` in detail

### `break`

| Rule | Detail |
|------|--------|
| Which loop | The nearest enclosing one: the document's own `loop:`, or a looping group or sequence around it. An outer loop is never ended by an inner loop's `break`. |
| No loop | A typed run-time error of the firing event, `LifecycleBreakWithoutLoop` ("… fired outside any loop"). `no_error` does not suppress it. |
| `code` | An integer `0`–`255`, default `0`. When the broken loop is the root invocation's (`claudine compose` of a looping document, or a looping root sequence), the process exits with it. An inner loop's `code` appears only in the summary line, and the run continues. |
| `reason` | Optional text; may interpolate `{{ … }}`. The summary prints `loop broken: <reason> (code N)`, or `loop broken (code N)` without one. Loops that end by their condition or by `max` print no such line. |
| Precedence | Any failure beats a pending `break`: an unhandled sibling failure, a failing `finalize`, or a `break` fired from `failure`. A leg failure handled by `fail_fast: false` does not. Ctrl-C overrides it (exit `130`). A `retry` discards it. |
| Concurrent group | The group carries the first breaking leg in declaration order, after every leg settles. |

### `prep`

| Rule | Detail |
|------|--------|
| Return | Exit `0` returns, including a target whose `initialize` skipped or whose own root loop ended by `break` with code `0`. Any other status fails the call. |
| Failure | An action failure of the caller's current event, `LifecyclePrepFailed` ("`prep` of `<target>` failed: <cause>"). It takes the event's ordinary error route; `no_error: true` on the `prep` tolerates it. In `failure` or `finalize` the event is not re-entered and the original error stays primary. |
| Cycles | Calls and `proxy` handoffs share one cycle domain and the 16-hop limit (`MAX_PROXY_HOPS`) over the active chain: `LifecyclePrepCycle` ("would re-enter an active call or exceed the hop limit"). A returned call is released, so calling the same helper again is legal. Concurrent legs keep separate chains. |
| Output | Reported under the target. Nothing is appended to the caller's `seq.outputs`, and the target's frontmatter, loop frames, and `set` writes never merge back. |
| Resolution | Relative to the document that authored the directive, by the [`proxy` rules](#how-a-proxy-target-is-found). |

## `--dry-run`

`--dry-run` fires no lifecycle events. It returns before the lifecycle runtime is constructed, so no event runs, no dynamic `proxy` is followed, and no `prep` target runs. Composition shell expansions still execute and may change the filesystem or perform other real work. Sequence `shell:` tasks are rendered, not run, unless `--dry-run-shell` is also given; `side_effect:` tasks never run. Turning a dry run into a lifecycle simulation is an explicit non-goal. See [Composition — Dry Run](../composition.md#dry-run) and [Sequences — Dry run](sequences.md#dry-run).
