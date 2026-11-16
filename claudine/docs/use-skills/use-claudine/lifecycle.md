# Lifecycle events

A prompt document's frontmatter can react to its own run. The grammar and
engine are Darkmatter's. Claudine hosts them with its own event names:
Darkmatter's `composed`/`failed` are called `start`/`blocked` here, and
writing `composed:` or `failed:` is an error. Claudine also adds `success`,
`failure`, and `loop`.

```text
initialize → pre-flight → start → agent runs → result check → success | failure → finalize → loop gate
                 └─ pre-flight fails → blocked → finalize          Ctrl-C → cancelled → finalize
```

| Event | Fires | Shell? |
|---|---|---|
| `initialize` | once per document, before pre-flight and schema validation (its effects stand even if the schema then refuses the document); not repeated on retry or resume | **never** (rejected even in a branch that can't run) |
| `start` | pre-flight passed, just before launch | yes |
| `success` | the agent finished and the result check passed (inline-compose: the body changed; the schema validates) | yes |
| `failure` | the agent failed or the result check failed | yes |
| `blocked` | preparation failed before launch; no recovery possible | only if reached after `start` |
| `cancelled` | Ctrl-C, or a sibling failed in a `fail_fast` group (cleanup only; `stop` is its only directive) | yes, after `start` |
| `finalize` | once per run, after the outcome event; not after `skip`, `proxy`, or an attempt that retry abandons | yes, after `start` |

## Writing an event

**A list runs in the order written:**

```yaml
start:
  - say: "Starting the build"
  - effect: small-group-cheer
  - shell: "git fetch --quiet"
  - when: "fm.target == 'production'"
    then:
      - error: "refusing to run against production"
    else:
      - set_frontmatter: ["state.md", "status", "running"]
```

**A dictionary runs in a fixed order, whatever order you type:**
1. `set`;
2. communication (`stdout`, `stderr`, `info`, `warn`, `success`,
   `message`, `notify`, `effect`, `say`);
3. `shell`;
4. other verbs, alphabetically;
5. the directive, last.

```yaml
failure:
  notify: "Review failed: {{ err.msg }}"
  effect: cartoon-cry
```

- **Values.** Every value is text. Use `{{{ … }}}` to interpolate. For
  literal braces, use a triple-brace literal (three braces on each side):

  ```yaml
  note: "{{{ name }}}"   # the text {{ name }}, never evaluated
  ```

  Only `when`, `while`, and `until` are evaluated as expressions.
- **Conditionals.** `when`/`then`/`else` nest up to five levels. There's no
  `elif`, and `then: []` is an error.
- **Directives.** The first directive that runs ends the whole event (`prep`
  is the exception: it returns). An unconditional directive must be the
  last item in its list.
- **Errors.** `no_error: true` on an action ignores that action's failure.
  It doesn't cover expression errors. Flags (`no_error`, and `edit`'s
  `create_new` and `gui`) take a boolean or Boolean-like text such as
  `"yes"`, never a number or a template; `null` is the same as leaving the
  flag out. `no_error` is refused beside `then`/`else` and on `set`.

## Actions

**Communication.** Each takes one text value; the long form is
`{text: …}`.

| Verb | Does |
|---|---|
| `say` / `speak` | text-to-speech; queued in list order, so put `effect` first to hear it first |
| `effect` | play a sound from the built-in library |
| `message` | send through your configured chat route (Discord, Slack, …) |
| `notify` | desktop notification |
| `stdout` | the only action that writes to stdout |
| `stderr`, `info`, `warn`, `success` | lines on stderr: `stderr` is plain, the others are styled status lines |

**Side effects:**

| Verb | Does |
|---|---|
| `shell: "cmd"` or `{command, on_error, no_error}` | run a command; a non-zero exit is a failure |
| `set: {key: value}` | change runtime state (in memory only). `set: {sha: "$(git rev-parse HEAD)"}` captures a command's output |
| `set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, `prepend_frontmatter` | edit a file's frontmatter: `set_frontmatter: ["s.md", "status", "done"]` |
| `ensure_file`, `ensure_dir`, `append_line`, `append_jsonl`, `http_post` | file and HTTP effects (a short-form array of arguments) |
| `edit: path` or `{file, create_new, gui}` | open the file in your editor and wait for it to close, then continue; a missing file is created (`create_new: true` refuses an existing one), `gui` picks a graphical or terminal editor; a number target such as `edit: 5` names the file `5` |

`claudine context --side-effects` lists every verb and its arguments
(`edit` is a lifecycle action, not a catalog verb, so it is not listed).

File actions (`edit`, the frontmatter writers, `ensure_file`, `ensure_dir`,
`append_line`, `append_jsonl`) write only inside the working tree Claudine
was launched in (the repository or worktree top level, or the launch
directory outside a repository), fixed for the whole run. A repository at
the launch directory that cannot be read (a corrupt `.git/config`) stops
`compose`, `inline-compose`, and `sequence` with an error before any
lifecycle action runs. A relative path
resolves like a read from the document that wrote the action: an existing
file is the one a read finds, a missing one is created beside that document
(a referenced sequence or group file is its own document). A shared prompt
outside your project therefore writes into it through an absolute path such
as `{{{ ctx.cwd }}}/notes.md`, never beside itself; a target outside the
boundary, through a symlink too, is refused before anything is created.

`edit` never picks an editor that cannot wait. A terminal editor needs a real terminal, so it fails
without one and in a parallel group member, which may run only a GUI
editor. No timeout closes an open editor; Ctrl-C or a failing sibling
does.

**Flow control:**

| Directive | Does |
|---|---|
| `stop` | end this event and keep the outcome |
| `skip` | `initialize` only: don't run the document at all (no `finalize`) |
| `error: "reason"` | fail the run; in `success`, this turns the outcome into failure |
| `retry: N` or `{max_attempts, delay, backoff: fixed\|exponential, with: {…}}` | start a fresh session (default 1 extra attempt); `with` overrides values for that attempt |
| `resume: "msg"` or `{message, max_attempts}` | continue the **same** session with a follow-up message. Example: "you didn't set `ready` in the frontmatter" |
| `proxy: target` or `{target, with}` | hand the run off to another document; its `finalize` is skipped; at most 16 hops |
| `prep: target` or `{target, with, no_error}` | run another document or sequence to completion, then continue with the next action; you get its file changes, not its variables |
| `break: "reason"` or `{reason, code}` | end the nearest loop; `code` is the exit status when it ends the top-level loop |
| `defer` | **planned**: parsed, but fails as not implemented |

`blocked` and the loop gate refuse `retry` and `proxy`, and `resume` needs a
live session.

## What an event can read

| Name | Where | Holds |
|---|---|---|
| `err` | `blocked`, `failure`, `cancelled`, `finalize` (`null` on success) | `code` (e.g. `cap.rate_limit`, `timeout.wall_clock`), `msg`, `category`, `detail.*`, `cause.*`, `is_transient`, `is_throttled`, `is_correctable`, `reset_at`, `retry_after_ms` |
| `timing` | every event | `document_ms`, `total_ms`, `step_ms` |
| `current` | every event | live `ctx` values |
| `seq` | every event | sequence position and `seq.outputs` |
| `loop` | inside a loop | `count` (from 1), `is_first`, `is_last`, `last_output`, `last_exit_code` |
| `group` | inside a group | `legs`, `succeeded`, `failed`, `cancelled` |

Plus everything a document can read: frontmatter, `fm.*`, `ctx.*`, and
`env.*`. Reading `err` in `initialize`, `start`, or `success` is rejected
when the document is parsed. Shell commands can't read `err`, `timing`, or
`current`.

`claudine errors` lists every `err.code`.

### A recovery pattern

```yaml
failure:
  - when: "err.is_throttled"
    then: [ {warn: "rate limited; retrying"}, {retry: {max_attempts: 2, delay: "60s", backoff: exponential}} ]
  - when: "err.code == 'composition.completion_schema'"
    then: [ {resume: "You did not set `ready` in the frontmatter; set it now."} ]
  - notify: "{{ fm.title }} failed: {{ err.msg }}"
```

`composition.completion_schema` is the code for a result that fails the
document's completion schema. `claudine errors` lists all 50 codes, with
details.

## `loop:`: repeat a document

```yaml
phase: 1
loop:
  until: "phase >= 6"        # or while:; checked AFTER each pass, so the first pass always runs
  action: "increment(phase)" # or actions: [...]; ops: increment, decrement, set(p,v), append, prepend, merge
  max: 10                    # default 100; "{{ cap }}" allowed
  on_rate_limit: pause       # pause | abort | continue (documents only)
  gate:                      # lifecycle actions after each pass's finalize
    - info: "pass {{ loop.count }} done"
```

- **The cap.** If the condition still asks for another pass at `max`, the
  loop fails (`LoopLimitExceeded`) instead of succeeding. Cap precedence:
  `--max-iterations`, then `CLAUDINE_MAX_ITERATIONS`, then `loop.max`, then
  100.
- **When the action runs.** It changes state only when another pass is
  coming.
- **Running zero times** needs `skip` in `initialize`, or a task `when:`.

## Old spellings you may meet

| Removed | Use instead |
|---|---|
| `composed:` / `failed:` | `start:` / `blocked:` |
| `stack:` wrapper, `- action:` items, `{action: shell, command}` | list the verbs directly; `shell: {command}` |
| `say_first`, `message:`/`sound:` in long forms, `route:` | list order; `text:`; messaging config |
| `_loop_*`, `loop.max_iterations`, `loop.fail_fast` | `loop.*`, `max`, a `failure` handler |
