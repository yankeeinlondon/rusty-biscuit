# Flow Control

Claudine runs three _executable primitives_: a markdown **document**, a **group** of tasks, and a **sequence** of steps. For any of them, Claudine lets the markdown (or YAML) author change the natural flow of execution through [lifecycle events](./lifecycle.md).

This page explains the directives and when to use each one. For edge cases and the exact runtime rules, see the [Flow Control Reference](flow-control-reference.md).

## An Example

A prompt asks an agent to write a file. The agent sometimes reports success without writing it, so the document checks for the file and sends the agent back to finish:

```yaml
---
prompt: "Write the release notes to @output/RELEASE.md"
success:
  stack:
    - when: "!file_exists('@output/RELEASE.md')"
      action: { resume: "You finished without writing @output/RELEASE.md. Please create it now." }
---
```

The pieces:

- `success` is the **event**. It fires when the agent finishes without an error.
- `stack` is an ordered list of **items**. Claudine runs them top to bottom when the event fires.
- `when` is an optional condition, written as a Darkmatter expression. The item runs only when the condition is true. With no `when`, the item always runs.
- `action` is what to do. Here `resume` is the flow-control directive. It continues the same agent session with a follow-up message.

## The directives

| Directive | What it does |
|-----------|--------------|
| [`stop`](#stop) | Ends this event's stack. The run carries on with the outcome it already has. |
| [`skip`](#skip) | Skips the whole document before the agent runs. Only valid in `initialize`. |
| [`error`](#error) | Fails the run with a reason. |
| [`retry`](#retry) | Runs the prompt again from the beginning with a fresh agent session. |
| [`resume`](#resume) | Sends a follow-up message to the same agent session. |
| [`proxy`](#proxy) | Hands the run to a different prompt document as _proxy_ for itself. |
| [`defer`](#defer) | **Planned:** suspends the whole run and schedules it to resume later, typically after a rate-limit window passes. |
| [`break`](#break) | **Planned:** leaves the innermost enclosing loop. |
| [`prep`](#prep) | **Planned:** runs another executable primitive to prepare the environment, then returns control to the caller. |

Directives marked **planned** are part of the documented design and are not yet implemented; `defer` additionally waits on scheduling in the `rendezvous` daemon.

## Rules that apply to every directive

**A directive ends the stack.** When a directive runs, the rest of that event's stack is skipped. That is why a directive must be the **last** action in its item, and why an item can hold at most one directive. Put any messages you want to show *before* the directive:

```yaml
failure:
  stack:
    - when: "err.category == 'cap'"
      action:
        - warn: "Usage cap reached, handing off to Codex"   # runs first
        - proxy: "@prompts/feature-codex.md"                # then ends the stack
```

**Only the first matching directive fires.** Items are checked in order, and once one directive runs, later items are not reached. Put more specific conditions first.

**Any event can use any directive, except `skip`.** A directive reacts to *state*: an error, a missing file, an environment variable, a frontmatter value. An error is just one kind of state, so `retry` or `resume` works in `success` as well as in `failure`. The one exception is `skip`, which only makes sense before anything has run and is therefore limited to `initialize`. Claudine checks this placement rule when it parses the document.

**Some directives need something that may not exist yet.** An event can accept a directive and still be unable to carry it out. `resume` needs a live agent session, so it fails in `initialize`, where no agent has started. `break` needs an enclosing loop, which a document cannot know about until it runs, since the loop may belong to the group or sequence that placed it. `defer` needs the scheduler. The [Flow Control Reference](flow-control-reference.md#what-each-event-can-do-at-runtime) lists these cases. When they happen you get a clear, typed error at the moment the directive fires instead of a silent no-op.

**`--dry-run` fires no lifecycle events.** A dry run never evaluates a stack, so it never exercises a directive.

## Writing a directive

Each directive can be written in a short **positional** form or a longer **key/value** form. Use key/value when you need an optional parameter:

```yaml
# positional
- action: { retry: 2 }

# key/value, same meaning plus a delay between attempts
- action:
    action: retry
    max_attempts: 2
    delay: "30s"
```

Values are literal text. To insert a variable or expression, wrap it in `{{ … }}`, for example `resume: "Fix the {{ err.code }} error"`. [Lifecycle — Action Forms](lifecycle.md#action-forms) covers both forms in detail.

## `stop`

Ends the current event's stack without changing anything else. If the run was succeeding it still succeeds, and later events still fire.

```yaml
success:
  stack:
    - when: "env.CI == 'true'"
      action: stop                      # don't announce anything on CI
    - action: { say: "Review finished" }
```

`stop` takes no arguments. `stop`, `stop:`, and `stop: []` all mean the same thing.

## `skip`

Opts the whole document out of the run. The agent is never launched, and no later events fire for this document (`finalize` and `loop` do not fire either). If the document is one step of a [sequence](sequences.md), the sequence moves on to the next step.

`skip` is only valid in `initialize`, the first event, which fires before any work starts.

```yaml
initialize:
  stack:
    - when: "file_exists('@output/RELEASE.md')"
      action: skip                      # already done, nothing to do
```

`skip` is also how you make a [loop](looping.md) run zero times, because a loop always runs its body at least once.

> **Planned:** a `when:` condition on the primitive itself (a document, group, or sequence step) will decide whether that primitive runs at all, including whether its loop runs even once. `skip` then stays for decisions a document makes about itself after it has been selected to run.

## `error`

Fails the run and records a reason. The reason reaches later events through the `err` variable, so a `failure` or `finalize` stack can react to it.

```yaml
success:
  stack:
    - when: "!file_exists('@output/RELEASE.md')"
      action: { error: "The agent reported success but never wrote RELEASE.md" }
```

Raising `error` in `success` or `finalize` turns a successful run into a failed one. In `success` the run then goes through `failure` and `finalize` as if the agent itself had failed. Use `error` when an agent says it is done but the result does not meet your requirements.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `reason` | no | Human-readable explanation. This is the positional argument. |

## `retry`

Runs the prompt again with a **fresh** agent session. The agent starts over and does not remember the previous attempt.

```yaml
finalize:
  stack:
    - when: "err"
      action: { retry: 1 }              # one more attempt, then give up
```

The number is how many *additional* attempts are allowed. `retry` with no number allows one. When the attempts are used up, `retry` does nothing more and the run ends with the outcome it already had.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `max_attempts` | no | Extra attempts allowed. Defaults to `1`. This is the positional argument. |
| `delay` | no | Wait before each retry, such as `"30s"` or `"5m"`. Defaults to no wait. |
| `backoff` | no | `fixed` (the default) waits the same `delay` every time. `exponential` doubles the wait after each retry. |

```yaml
failure:
  stack:
    - when: "err.is_transient"
      action:
        action: retry
        max_attempts: 3
        delay: "10s"
        backoff: exponential            # waits 10s, then 20s, then 40s
```

## `resume`

Continues the **same** agent session by sending it a follow-up message. The agent keeps everything it has already read and done. That makes `resume` usually better than `retry` when the agent was on the right track but stopped early.

```yaml
---
prompt: "Refactor @src/engine.rs and make the test suite pass"
timeout: 20m
failure:
  stack:
    - when: "err.category == 'timeout'"
      action: { resume: "You were stopped by a timeout. Continue where you left off and finish the task." }
---
```

`resume` needs a live agent session. It fails in `initialize` and anywhere else no agent has been launched yet.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `message` | **yes** | The follow-up prompt. This is the positional argument. |
| `max_attempts` | no | How many times this `resume` may fire. Defaults to `1`. |

## `proxy`

Hands the rest of the run to another prompt document. That document starts from its own `initialize` and takes over completely: its events, its agent, and its output. The original document's remaining events do not fire, so a `proxy` from `success` skips the original's `finalize`.

A common use is switching to another agent when one hits a usage limit:

```yaml
---
agent: claude
prompt: "Implement the feature described in @spec.md"
failure:
  stack:
    - when: "err.category == 'cap'"
      action:
        - warn: "Claude usage cap reached, handing off to Codex"
        - proxy: "@prompts/feature-codex.md"
---
```

Here `@prompts/feature-codex.md` is the same task with `agent: codex` in its frontmatter.

The target is prepared exactly as if you had run it directly with `claudine compose`, including the same validation and shell approval. If the target file does not exist, the handoff fails with an error instead of silently doing nothing.

**Target paths.** A plain path such as `next.md` is looked up next to the current document first, then at the repository root. A path starting with `./` or `../` is relative to the current document only. `@` searches the project's usual prompt locations, then your home prompt directories. The [reference](flow-control-reference.md#how-a-proxy-target-is-found) lists every prefix.

**Passing values to the target.** The key/value form accepts a `with:` mapping. Its entries become frontmatter properties of the target for this handoff only, and nothing is written to disk:

```yaml
success:
  stack:
    - action:
        action: proxy
        target: "@prompts/next.md"
        with:
          attempt: "{{ iteration }}"
          label: "phase-{{ iteration }}"
```

`with:` values are evaluated once, in the current document, when the directive fires, and the target receives the results as data: text an agent wrote stays text there, even if it contains `{{ … }}` or `$( … )`. Values the caller passed on the command line (`--set key=value`) still win over `with:`. The overlay applies only to this target and is not passed on to further handoffs. The [reference](flow-control-reference.md#passing-values-with-with) covers types, precedence, errors, and the security model.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `target` | **yes** | The document to hand off to. This is the positional argument. |
| `with` | no | Frontmatter values for the target. Key/value form only. |

## `defer`

> **Planned.** Claudine already parses `defer` and checks its placement, but it depends on scheduling in the external `rendezvous` daemon, which does not exist yet. Until it does, every `defer` fails with a `LifecycleDeferNotImplemented` error.

Suspends the **whole run** and schedules it to resume later. The primary use is a rate limit: the agent reports that its usage window is exhausted, and the right response is to wait for the window to reset and then carry on, not to fail and not to hand off.

```yaml
failure:
  stack:
    - when: "err.category == 'cap'"
      action:
        - warn: "Usage cap reached; resuming when the window resets"
        - defer: "{{ err.retry_after || '1h' }}"
```

What "the whole run" means: `defer` acts on the root invocation, whatever primitive fired it. A `defer` raised by a document that is one step of a sequence, or one task of a group, suspends the sequence or group along with it. When the run resumes it picks up at the point where `defer` fired, so earlier steps are not repeated. Deferring only the current step would leave the rest of the sequence running against an agent that cannot answer, which is never what a rate limit calls for.

Nothing else in the run happens while it is suspended: no later events fire, no later steps start, and no agent is launched. The `rendezvous` daemon holds the run and restarts it when the delay expires.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `delay` | **yes** | How long to wait before resuming, such as `"5m"` or `"1h"`. This is the positional argument. |
| `reason` | no | Human-readable explanation, shown when the run is suspended and again when it resumes. |

## `break`

> **Planned.**

Leaves the innermost enclosing [loop](looping.md). The loop may belong to a different primitive than the one raising `break`: a document that is one task of a looped group ends the group's loop, and a group that is one step of a looped sequence ends the sequence's loop. With nested loops, only the innermost one ends; an outer loop continues and checks its own condition as usual.

```yaml
success:
  stack:
    - when: "frontmatter(review, 'ready') == true"
      action:
        - message: "Review marked the work ready"
        - break: "ready"
```

Like every directive, `break` ends the current stack. The rest of the current iteration still completes, so `finalize` fires as usual, and then the loop stops instead of evaluating its condition or applying its per-iteration `action`.

A loop that ended through `break` is a distinct outcome from "condition met" and from "`max` reached". Sequence summaries report it with the `reason`, and it can drive the process exit code, which is how a run says "stopped early for a human to look" rather than "finished".

`break` fired with no enclosing loop is a runtime error, in the same class as `resume` with no session: a document cannot know at parse time whether it will be placed inside a looped group or sequence.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `reason` | no | Human-readable explanation, recorded as the loop's outcome. This is the positional argument. |

## `prep`

> **Planned.**

Runs another executable primitive to _prepare_ the environment, then returns control to the caller. Where `proxy` is a hand-off, `prep` is a call: the target runs to completion, and the caller's event stack continues with the item after the directive as if nothing had intervened.

```yaml
# commit.md — stage the tree first when the caller asks for it
initialize:
  stack:
    - when: "stage == 'all'"
      action: { prep: "@prompts/_git/stage-all.md" }
    - when: "length(current.staged_files) == 0"
      action: skip
```

What comes back: only what the target wrote to disk. `prep` does not merge the target's frontmatter or runtime state into the caller; the caller reads any result the same way it reads any file. Inside a sequence the target's final output is appended to `outputs`, so `{{ last(outputs) }}` also works.

The target is prepared exactly as a `proxy` target is: its own `initialize`, its own validation, its own shell approval. Inside a sequence that approval happens at the sequence's preflight along with every other referenced document. A `prep` from `initialize` is allowed even though `initialize` itself is shell-free, because any shell runs inside the target under the target's approval.

A target that fails fails the caller's current event, and the run proceeds through `failure` and `finalize` as for any other failure. `prep` targets obey the same cycle rule and hop limit as `proxy`, so a target cannot `prep` its caller.

| Parameter | Required | Meaning |
|-----------|----------|---------|
| `target` | **yes** | The document, group, or sequence to run first. This is the positional argument. |
| `with` | no | Frontmatter values for the target, as for `proxy`. Key/value form only. |

## Choosing a directive

| Situation | Reach for |
|-----------|-----------|
| The agent finished, but the result is incomplete, and it was on the right track | `resume` |
| The agent got confused, or the failure looks random | `retry` |
| A different prompt or agent should do the work | `proxy` |
| The result is unacceptable and should count as a failure | `error` |
| The document has nothing to do on this run | `skip` (from `initialize`) |
| The remaining stack items should not run | `stop` |
| The agent hit a rate limit and the run should wait it out | `defer` (planned) |
| The loop has done its job early, or should stop for a human to look | `break` (planned) |
| Another prompt must run first, and control should come back | `prep` (planned) |

These combine well across events. For example, `success` can raise `error` when an artifact is missing, and `finalize` can then `retry` once. The [verify-then-retry example](lifecycle.md#verify-an-artifact-on-success-then-retry-once-from-finalize) shows the full document.

## Related

- [Flow Control Reference](flow-control-reference.md): runtime limits per event, retry and resume details, and `proxy` handoff semantics.
- [Lifecycle](lifecycle.md): every event, every action type, and the `err` fields you can match on.
- [Looping](looping.md) and [Sequences](sequences.md): repeating a document, and running several documents in order.
