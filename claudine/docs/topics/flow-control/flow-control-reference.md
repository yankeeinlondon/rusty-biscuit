# Flow Control Reference

The detailed rules behind the lifecycle flow-control directives. Read [Flow Control](flow-control.md) first. This page assumes you know what each directive is for and only records the edge cases, error names, and guarantees.

## What each event can do at runtime

Placement is checked once, at parse time: every directive is accepted in every event except `skip`, which is accepted only in `initialize` (`LifecycleControlAction::is_valid_for`). At runtime, every event hands its directive to the same event-agnostic dispatch path (`decide_control` followed by `dispatch_terminal_control`). What varies is whether the run is in a state where the directive can do anything:

| Event | `retry` | `resume` | `proxy` |
|-------|---------|----------|---------|
| `initialize` | `LifecycleSetupPhaseRecoveryUnsupported` | `ResumeWithoutSession` | Full handoff |
| `start`, `success`, `failure`, `finalize` | Full | Full, given a live session | Full handoff |
| `blocked` (from compose pre-flight) | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` |
| `loop` (the iteration gate) | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` | `LifecycleSetupPhaseRecoveryUnsupported` |

`stop` and `error` work everywhere. `defer` fails with `LifecycleDeferNotImplemented` in every event until its scheduler exists.

The unsupported cells have no re-entry point to return to, so they raise a typed error instead of quietly doing nothing. Put those recoveries on `start`, `success`, `failure`, or `finalize`.

A `proxy` from `initialize` is a real handoff, not a reduced pre-launch path. The coordinator that tracks the active document sits above both the document loop and the provider harness, so the target is prepared exactly as a directly invoked document would be.

The `loop:` iteration condition (`while`/`until`) is a separate mechanism. It does not go through directive dispatch.

## `retry` and `resume` in detail

### Budgets

`max_attempts` counts attempts *beyond* the one that just ran. The budget is fixed the first time a given directive fires, so re-running the prompt cannot reset it. Once the budget is spent the directive behaves like `stop`: the stack ends and the run keeps the outcome it had.

With `backoff: exponential`, the first retry waits `delay`, and each later retry waits twice as long as the one before. `delay` uses the same duration syntax as `timeout` (`"30s"`, `"5m"`).

### Where a retry re-enters

`retry` picks its re-entry point from whether the agent had launched, not from which event fired it:

- **Agent not launched yet:** pre-flight runs again.
- **Agent launched:** the agent is invoked again, and the run re-enters at `start`.

### What survives a retry or resume

Both directives replace only the **provider attempt**. The active document stays the same. On re-entry:

- The document is re-read from disk and fully validated again.
- The `proxy` `with:` overlay, the proxy history, and the remaining attempt budgets are kept.
- `initialize` does **not** fire again. It fires once per active document, not once per attempt.
- Launch settings are recomputed from the freshly read document. A `model:` edited between attempts takes effect on the next attempt.

### When `resume` refuses

Before continuing a session, `resume` compares a session-compatibility key across the refresh. If a relevant setting changed, it refuses with `CompositionError::LifecycleResumeIncompatible { facets }`, names what changed, and recommends `retry`. The refusal happens after `start` has fired and before the agent is spawned again. It then follows the normal failure path: `failure` fires, then exactly one `finalize`, and both see the refusal as `err`. `success` does not fire. The full list of compared settings is in [Composition — Retry and resume re-entry](../composition.md#retry-and-resume-re-entry).

## `proxy` in detail

### The target becomes the active document

A run has exactly one **active document**. `proxy` replaces it. The target enters at its own `initialize` and owns everything after that: the remaining events, the completion step, and the output.

A clean handoff fires no further events for the source document. A `proxy` from `success` or `failure` skips that attempt's `finalize`, and a `proxy` from `finalize` does not re-enter `finalize`.

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

`with:` is a transient overlay of top-level frontmatter properties for the immediate target.

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

#### Shape rules

`with:` is accepted only in key/value form. `with: {}` means the same as leaving it out. Each of these is a typed frontmatter error that points at the source:

| Mistake | Error |
|---------|-------|
| `with:` is not a mapping | `LifecycleProxyWithNotMapping` |
| A key contains `{{ … }}` | `LifecycleProxyWithDynamicKey` |
| The whole mapping is one span, such as `with: "{{ payload }}"` | `LifecycleProxyWithWholeMapping` (out of scope for v1) |
| `with:` appears on a directive other than `proxy` | `LifecycleProxyOnlyParameter` |
| Positional `proxy:` with a sibling `with:` | `LifecycleStackAmbiguous`, which suggests the key/value form |

#### Value types

Keys are static strings that name target properties. They are never interpolated. Values follow the same typing rule as every other lifecycle parameter, applied recursively through arrays and objects:

- A string containing text and spans (`"phase-{{ iteration }}"`) becomes a string.
- A string that is **exactly one** span keeps the expression's type. `"{{ true }}"` becomes boolean `true`, not the string `"true"`.
- YAML numbers, booleans, arrays, objects, and nulls keep their authored types.

Nested mappings are data here. This is the only place in the lifecycle surface where a nested mapping is a legal action value.

#### Evaluation

The whole mapping is evaluated **once, in the source document**, when the directive fires. Evaluation uses Darkmatter subtree composition against the source's live frontmatter plus the globals the event declares (`err`, `timing`, `group`; see [Lifecycle](lifecycle.md#binding-time-early-vs-late)) and the `current`/`current_env` roots. An absent property is `null`; a genuine evaluation error leaves no overlay at all. A `set_frontmatter` earlier in the same stack is visible. The target receives resolved data, never a template: the overlay reaches the target's composition as a data layer, so a `{{ … }}` or `$( … )` inside a resolved value (for example, text an agent wrote) is kept as text and is never evaluated in the target, on its first read or on a `retry`/`resume`.

A `with:` value for a lifecycle key (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, `loop`) is the exception. The target re-parses it as its own stack or loop and evaluates those strings at event time, so a resolved value that still holds a `{{ … }}` span there fails with `LifecycleProxyWithEvaluationFailed` instead of becoming an instruction.

Evaluation is atomic. The target and the whole mapping resolve before anything changes. If any value fails (a malformed expression, an unknown function, or an out-of-scope variable such as `group` outside a group), no overlay is installed, the target is never touched, the source stays active, and the run follows the normal failure path for the event that fired. `no_error` does not suppress this, because it is an expression error rather than a side-effect failure. The error, `LifecycleProxyWithEvaluationFailed`, names the event, the directive, the target, and the deepest path inside `with:` that it can identify. It never prints a resolved value, which could be a secret.

#### Precedence and lifetime

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
- action: proxy
  target: "@prompts/final.md"
  with:
    spec: "{{ spec }}"      # forwarded only because it is named here
```

An overlay does not give a document a distinct identity for cycle detection.

#### `with:` compared with `set_frontmatter`

| | `proxy` `with:` | `set_frontmatter` / `merge_frontmatter` |
|---|---|---|
| Persistence | In memory, for one target activation | Written to the file on disk |
| Scope | The immediate proxy target | Whichever file the action names |
| Lifetime | Discarded at the next hop | Until something rewrites it |
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

## `--dry-run`

`--dry-run` fires no lifecycle events. It returns before the lifecycle runtime is constructed, so no stack is evaluated, no side effect touches the workspace, and no dynamic `proxy` is followed. Turning a dry run into a lifecycle simulation is an explicit non-goal. See [Composition — Dry Run](../composition.md#dry-run).
