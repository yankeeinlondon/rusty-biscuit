---
description: |-
    A partial, never run directly. Transclude it into a prompt whose agent has to write, review, or debug a Claudine prompt:

        ::file ^prompts/_prompt.md

    Sections can be dropped when the task does not need them, for example `::file ^prompts/_prompt.md set.flow=false set.testing=false`.

    This file is itself composed when it is transcluded. Every example is therefore inside a fenced code block, where nothing is evaluated, and every brace span in running text is written as an interpolation literal.
$prompt:
    lifecycle: boolean(required) -> include the lifecycle events, stacks, and flow-control section (_default is `true`_)
    flow: boolean(required) -> include the multi-document section on proxy handoffs, sequences, and loops (_default is `true`_)
    testing: boolean(required) -> include the section on rehearsing a prompt without launching a model (_default is `true`_)
lifecycle: true
flow: true
testing: true
# Rendered in the body on purpose: a file with no real span skips the interpolation
# pass, and its `{{{ … }}}` literals would then reach the agent as triple braces.
as_of: 2026-09-20
# The defect warnings at the end are gated on this spec: each renders only while the spec is
# still at this active path and its finding id is absent from the spec's `fixed:` list. Fixing
# a defect means adding its id there; nothing in this file needs editing. `&` pins the
# repository root, and is used because `ctx` is not available to a transcluded file.
defects_spec: "&claudine/fixes/2026-09-20-lifecycle-handoff-gaps/spec.md"
---

## How Claudine Prompts Work

A Claudine prompt is a Markdown file. Its **frontmatter** declares inputs, defaults, and what Claudine does around the agent. Its **body** is composed by Darkmatter into the text the agent receives. Deterministic work belongs in the frontmatter and in composition; judgment belongs to the agent. A good prompt gives the agent what it would otherwise spend tool calls discovering, and lets Claudine do everything that does not need a model.

Three commands run a prompt:

| Command | What it does |
|---|---|
| `claudine compose <file> [key=value ...]` | composes the body and sends it as the prompt; the source file is not rewritten |
| `claudine inline-compose <file>` | the agent writes the document's own body, driven by a frontmatter `prompt:` |
| `claudine sequence <file>` | runs an ordered list of steps, each its own composition |

This describes Claudine as of **{{ as_of }}**. When it disagrees with the references or with what you observe, they win, so when a fact here matters to your decision, confirm it. `claudine context` lists every `ctx` property, `claudine context --expressions` lists every function and operator, and `claudine context --side-effects` lists every mutation verb. The references are `claudine/docs/topics/composition.md`, `claudine/docs/topics/lifecycle.md`, `claudine/docs/topics/flow-control/`, and `darkmatter/docs/inline/`.

### Composition: what runs before the agent sees anything

- **Interpolation.** {{{ expr }}} is evaluated in the body and in frontmatter values. A frontmatter value that is *exactly one* span keeps the expression's type, so a span holding `true` yields a boolean and not the string. To show brace syntax without evaluating it, wrap it in a third pair of braces.
- **Inline code is scanned; fenced code is not.** Backticks do not protect a brace span. A fenced code block protects everything inside it, including directives.
- **`::file <ref>`** transcludes another file, recursively, as part of the same composition. `set.<key>=<value>` overrides the child's frontmatter and `when="<expr>"` makes the inclusion conditional. Prefixes choose where a reference is searched: `./` and `../` are relative to the authoring file, `^` walks package, package area, then repository root, `&` is the repository root, `@` is Claudine's registered roots, and `~/` is home.
- **`::block when="<expr>"` … `::end-block`** keeps or drops a region of the body. Blocks nest.
- **`::shell <command>`** and **`::shell-block`** splice a command's output into the body. The default timeout is 10 seconds and a non-zero exit fails the composition, so give a command that may fail a fallback (`when_error=` on a block, `--when-error` on a single directive). Output is spliced raw; it cannot be wrapped in a fence.
- **Frontmatter `$(command)`** stores trimmed stdout in a property, and the whole value must be the expression. `::timeout:<seconds>` extends the limit. An exit status can be captured with a shell idiom.

```yaml
commits_ahead: "$(git rev-list --count origin/main..HEAD)::timeout:30"
tree_is_clean: "$(git diff --quiet && echo yes || echo no)"
```

**Shell approval happens once, up front.** Before anything runs, Claudine collects every command the document could execute, including those in branches that will never be taken, and asks the caller to approve any that are not whitelisted. Approved bytes are executed bytes. A built-in blacklist cannot be overridden: it covers `git push`, `git branch`, `git checkout`, `git reset`, `git rebase`, `rm`, `mv`, `cp`, `curl`, `ssh`, package installs, and `>` redirection, among others. Work the blacklist forbids has to be done by the agent.

### `ctx`, `current`, and when each is read

- **`ctx`** is evaluated **once, eagerly, for each composition run**, and only for the properties that appear on the page. It is available in the body, in frontmatter, and in lifecycle hooks. It describes the world as the run began.
- **`current`** is the lazy counterpart: `current.<key>` has exactly the shape of `ctx`, and each key is read when the expression that names it evaluates. There is no nesting under it, and a path that is not a `ctx` key fails the run. Its use is in lifecycle hooks, which evaluate when their event fires; a body is composed up front, so `current` there is read at composition time.
- Ask which moment your question is about. "Were there staged files when this began?" is `ctx`. "Is anything staged right now?", asked from a hook after the run may have changed things, is `current`. A body that must show the agent the present state uses `::shell`.
- `ctx` is rich, and lists work directly as gates: `ctx.dirty_files` is falsy when empty. Useful groups are git state (`branch`, `dirty_files`, `staged_files`, `untracked_files`, `dirty_source_code_files`, `merge_conflicts`), blast radius (`dirty_packages`, `dirty_package_areas`, and the booleans `current_package_has_dirty_files` and `current_package_area_has_dirty_files`), location (`repo`, `repo_root`, `area`, `area_description`, `current_package`), target (`agent`, `model`), and host and clock (`os`, `now`, `today`, `timestamp`).
- Functions that reach the network, such as `branch_exists_on_remote()` and `pr_list()`, fail the run when the host is not on the network allowlist, which denies everything by default. Do not put one in a gate that must not raise.

### Inputs: `$schema`

```yaml
$schema:
    base: string(required) -> the branch the pull request targets
    on_failure: enum(ask, fix, stop) -> what to do when the push is blocked
    report: file(required) -> the report file shared by every stage
on_failure: ask
```

- A default is an ordinary frontmatter key with the property's name. The caller overrides it with `key=value`, and the caller always wins, over a default and over a `proxy` overlay.
- **A required property with no value makes Claudine ask the caller**, using a widget that fits the type: a picker for an `enum`, a switch for a `boolean`. This is the way to put a question to the caller without an agent: leave the property unset on purpose. Without a terminal the run stops with a typed error naming the property, and an unattended caller supplies it as `key=value`.
- An optional input that was not supplied can raise when it is read in a gate or a lifecycle value. Guard it with a fallback: {{{ title || '' }}} in a value, `review || false` in a `when:`.

::block when="lifecycle"
### Lifecycle: what Claudine does around the agent

```text
initialize → (schema verdict, shell approval) → start → agent → success | failure → finalize → loop
                                                 blocked fires instead when the run stops before the agent launches
```

Each event is a frontmatter key. It holds notification fields and an ordered `stack:` of conditional actions.

```yaml
success:
    stack:
        - when: "frontmatter(report, 'status') == 'passed'"
          action:
              - success: "pushed {{ frontmatter(report, 'branch') }}"
              - action: proxy
                target: ./open.md
                with:
                    report: "{{ report }}"
        - action:
              - error: "the agent did not record an outcome in {{ report }}"
```

- **Where a notification goes.** `info`, `warn`, `success`, and `stderr` print to the terminal. `message` goes to the configured messaging route and prints **nothing** locally. `say` and `effect` are audio. An outcome the caller must see in the terminal needs one of the first group.
- **Values are literal text.** Only `when`, `while`, and `until` are expressions. Everywhere else a brace span opts in, and it is resolved when the event fires, so a hook reports the state at that moment. A brace span inside a quoted string *inside* an expression is rejected; build the string with `+`.
- **Two action forms.** Positional is `verb: value`, or `verb: [a, b, c]` for several arguments. Key/value is `action: verb` plus named parameters, and is required for `with:`, `on_error:`, and `no_error:`. A stack item holds at most one flow-control action, and it must be last.
- **`err`** exists only in `blocked`, `failure`, and `finalize`. Match on `err.code`, `err.category`, `err.is_transient`, or `err.is_throttled`, never on message text. `err.msg` is safe to speak or post.

Flow control:

| Action | What it does | What to know |
|---|---|---|
| `skip` | opts the whole document out: no agent, no `finalize` | valid in `initialize` only; the clean way to not run |
| `stop` | ends this event's stack | **does not prevent the agent launching**; a gate that must prevent a launch uses `skip`, or `error` |
| `error: "reason"` | fails the event | from `initialize` or `start` it stops the run before the agent launches |
| `proxy` | hands the run to another document, which enters at its own `initialize` | one-way; see the next section |
| `retry: N` | re-runs with a fresh agent session | `start` fires again, `initialize` does not |
| `resume: "message"` | continues the same agent session with a follow-up | needs a provider that can resume a session |
| `defer` | not implemented | |

Rules that shape a design:

- **`initialize` is shell-free**, and a document that declares `initialize` may not use a frontmatter `$(…)` at all. Approval cannot lift either rule. Put shell work in `start` or later, or drop `initialize` and gate from `start`.
- **A lifecycle `shell` action cannot hand back its output or exit status.** A non-zero exit fails the item unless `no_error: true`. To branch on a command's result, use the frontmatter form shown above.
- **Verify in `success`; do not trust the agent's exit.** Have the agent record its outcome in a file, read it with `frontmatter(file, 'key')`, and end the stack with an unconditional `error` so that a missing outcome is a failure.
- **Nudge before giving up.** When the outcome is missing, `resume` the session once and ask for it. Runtime state does not survive re-entry, so record that the nudge happened with `set_frontmatter` on a file.
- **A bounded retry counts on disk.** `retry: N` has no exhaustion event. Use `increment_frontmatter` in `start`, which fires on every attempt, and gate a "gave up" branch on the count.
::end-block
::block when="flow"
### More than one document

- **Share state through a file, not through prose.** Give the flow one report file with a documented frontmatter contract. Each stage writes its outcome there and each lifecycle stack reads it. Create it fresh for each run; a reused file carries the previous run's answers.
- **A router needs no agent.** A document can do all its routing in `initialize` or `start` and never launch. Give it a short body that tells an agent that reaches it to do nothing and report the fault.
- **`proxy … with:`** parameterizes the target. Values keep their types, are resolved at the source when the event fires, and are never written to disk. The overlay reaches only the immediate target, so forward what the next hop needs explicitly. It can set any top-level key, including a lifecycle block, which is how a reusable prompt such as `commit.md` can be given a continuation it does not know about.
- **A proxy chain visits a document once.** Returning to an earlier document is rejected as a cycle, and a chain is limited to 16 hops. A flow that must repeat a stage cannot do it with `proxy`; that is what a sequence is for.
- **A sequence** is a static list of steps, each composed at its turn: `prompt:`, `shell:`, `side_effect:`, or a group.
    - Every command across all steps is approved before step 1 starts.
    - Missing required inputs are collected in one pass up front, so an input that only some paths need must be optional.
    - A step becomes conditional when its prompt calls `skip` from `initialize` after reading the shared report.
    - The same document may appear in more than one step.
    - A step's document may declare `interactive: true` and runs interactively. Only the sequence document itself may not.
    - `sequence_id` names the run and is the right seed for a per-run file name.
- **A loop** (`loop: { while | until, action, max }`) repeats one document. Each iteration is a full composition cycle, and `initialize` fires only on the first. **The condition is checked at the end of an iteration**, against the state that iteration ran with, and the `action` is applied only when the loop continues. A loop therefore always runs at least once, and `while: "n < 2"` counting from `0` runs three times (`n` is `0`, `1`, then `2`). For zero iterations, `skip` from `initialize`. The ambient values are `_loop_count`, `_loop_is_first`, and `_loop_is_last`.
- **In this repository every git commit goes through `prompts/commit.md`.** A stage that produces changes stages them and hands off; it never runs `git commit`.
::end-block

### Writing the body

- **Give facts, not chores.** Anything the agent would certainly look up (the branch, the commits being proposed, which packages changed, whether `main` is behind its remote) belongs in the prompt, gathered by `ctx` or a shell block. Tell the agent the facts are current so it does not re-derive them.
- **One stage, one job.** A stage that pushes should not also diagnose. A small body wastes less context on paths that are not taken, and each stage can pin the `agent:` and `model:` its job deserves.
- **Name the deliverable and its format.** When a lifecycle stack reads a value, the body must tell the agent exactly which file, property, and values to write.
- **Say what is out of bounds.** A stage that must not commit, push, or repair has to be told so, along with which stage does.
- Claudine already appends a system prompt telling a non-interactive agent that nobody can answer it. Do not repeat that; add only what is specific to the task, such as where to record what it cannot ask.

::block when="file_exists(defects_spec)"

### Defects to design around

Each item below is an open defect, specified in the fix `2026-09-20-lifecycle-handoff-gaps`. The list is read from that specification when this prompt is composed, so an item that appears here has not been fixed. Do not copy a workaround out of `prompts/_pr/` for a defect that is not listed.

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F1')"
- **`ctx` git state is reused across composition runs.** A proxy target, a later sequence step, a later loop iteration, and a retried attempt all see the first run's working tree. A hook that needs the present state reads `current`, and a body uses `::shell`.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F2')"
- **A `proxy` fired from inside a looping document is recorded but not performed.** Use `retry` for bounded repetition when the document also has to hand off.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F3')"
- **A shell command in a transcluded partial is not pre-approved when it interpolates a value that arrived through `proxy … with:`.** Keep such commands inline in the target.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F4')"
- **An `error` in a `success` stack fires `failure` and `finalize`, but the process exits `0` and prints no error.** Put a `warn` beside it, and do not depend on the exit code.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F5')"
- **A `ctx` property mentioned only in a transcluded file renders empty.** Claudine evaluates the properties found on the root page and does not look through `::file`. A partial that reads `ctx` works only when the prompt including it mentions the same property, so have a partial take what it needs through `set.<key>=` instead.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F7')"
- **A `proxy` inside a sequence `prompt:` task is refused** with a "no owning coordinator" error. A sequence step cannot hand off; make the target its own step.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'F8')"
- **The `loop:` block's own `info`, `warn`, `message`, and stack cannot read the `_loop_*` values**, and referencing one there fails the run. Report loop progress from `start` or `success`, which can.
::end-block

::block when="!contains(frontmatter(defects_spec, 'fixed'), 'D1')"
- **Interpolation literals are converted only in a file that also contains a real span.** A file whose every brace span is a literal reaches the agent with its triple braces intact. Give such a file one real span.
::end-block

::end-block
::block when="testing"

### Rehearsing a prompt without a model

- **`--dry-run`** composes and prints the body. It fires **no lifecycle events** and follows no `proxy`, so it proves the body and the expressions and nothing about the flow. It still runs `::shell` and `$(…)` for real.
- **Stub the provider** to exercise the real flow. Put an executable named after the provider first on `PATH`; a provider is only a program that reads a prompt, prints text, and exits. Have it recognize each stage by the body's first heading and edit the report the way a real agent would. Run it in a scratch repository whose `origin` is a local bare repository, with `HOME` pointed at an empty directory so that no messaging route or voice is configured, and with `PLAYA_DRY_RUN=1` and `CLAUDINE_RENDEZVOUS_REPORT=false`. Every route can then be driven to completion in seconds, and nothing is pushed.
- **To stop before the agent launches**, which is how a lifecycle-only probe is written, end the `start` stack with an `error`.
- **`cargo nextest run -p claudine-cli --test l1 shipped_prompt_contract::`** checks every file under `prompts/`: schemas parse, expressions parse, and no lifecycle value nests a brace span inside a string literal. Run it after any change to a prompt.
- Claudine's banner for each stage says whether it launched `Interactive`, which a stub run does show. A stub cannot show that an interactive session behaved, or that `resume` works, because it has no session. Those need the L2 tmux harness (see the `rust-testing` and `biscuit-test-harness` skills).
::end-block
