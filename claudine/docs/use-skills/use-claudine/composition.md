# Prompt documents: composition in Claudine

`claudine compose`, `inline-compose`, and `sequence` run a Markdown document
through **Darkmatter's compose pipeline**, the same engine as `md compose`.
They send the result to an agent, then run the document's lifecycle events
around that run. So two grammars are in play:

| Darkmatter supplies (identical to `md compose`) | Claudine adds |
|---|---|
| `{{{ expr }}}` interpolation, `ctx.*`, `env.*`, `fm.*`, and the expression functions | provider and model selection (`agent`, `model`, and the switches) |
| `::file`, `::code`, `::toc-linking`, `::file-links` transclusion | the lifecycle events and actions ([lifecycle.md](lifecycle.md)) |
| `::block when="…"`, `::map`/`::filter` | `loop:` and `sequence:` ([sequences.md](sequences.md)) |
| `::shell`, `::shell-block`, frontmatter `$(cmd)` | the `@` prompt roots, `system-prompt.md`, and `inline-compose` |
| `$schema`, schema triggers, `replace:` | Claudine's trigger schemas, which type its own keys |

For the Darkmatter grammar in depth, load `use-darkmatter` and read its
`authoring.md`. The short version is below.

## The pipeline, in order

1. Resolve the file reference, and read frontmatter without running shell.
2. `initialize` (shell-free; it can `skip` the run). It runs before schema
   validation, so what it does stands even when the schema then refuses the
   document.
3. **Pre-flight.** Walk every transcluded document and every branch, and
   approve every shell command once.
4. **Compose.**
   - Frontmatter: `{{{ }}}`, then `$(…)`, then `{{{ }}}` again, then schema
     validation.
   - Body, in order: `replace:`, `::block`/`::map`/`::filter`, `{{{ }}}`,
     `::shell`, then transclusion (each child goes through the same
     pipeline), cleanup, and heading re-leveling.
5. Choose the provider and model.
6. `start` → launch the agent → check the result → `success` or `failure` →
   `finalize` → loop gate.

**The agent receives the composed body only.** Frontmatter is never sent.
Lifecycle keys stay uncomposed until their event fires, so they can read
`err`, `timing`, and `current`.

## Darkmatter features you'll use most

```markdown
---
target: src
depth: 1
files: "$(git diff --name-only main)"      # frontmatter shell expansion (needs approval)
---
# Review {{ target }}

Repo {{ ctx.repo }}, package {{ ctx.current_package }}, agent {{ env.AGENT }}.
Today is {{ ctx.today }}. Dirty files:

{{ as_unordered_list(ctx.dirty_files) }}

Fallbacks and ternaries: {{ reviewer || "anyone" }}, {{ depth > 1 ? "deep" : "quick" }}

::block when="depth > 1"
Go deep: read every caller.
::end-block

::file ./rules.md exclude="## Bad Ideas"      <!-- transclusion; headings re-leveled to fit -->
::file ./internal.md when="env.AUDIENCE == 'internal'"
::code &src/main.rs

::shell git log --oneline -5
```

- **`ctx.*`** is Darkmatter's runtime context: dates, repository, packages,
  git state, OS, and hardware. `claudine context --values` lists every key
  with its current value. `current.*` reads the same keys live, at the
  moment of evaluation. `env.*` is the environment captured once per
  document.
- **An undeclared name** renders `""` with a warning. Use `||` when the value
  is optional.
- **`::map` / `::filter`** sources need a type declared in `$schema`, for
  example `items: "{ name: string }[]"`.
- **`claudine context --expressions`** lists every function.

## Choosing the provider and model

- **Provider:**
  - An explicit `--provider P`, `--claude`, `--codex`, and so on, always
    wins.
  - **On a terminal**, a picker always opens. Frontmatter `agent` and your
    favorite agent only set its default.
  - **Without a terminal**, the order is: frontmatter `agent` (one value,
    or a list where the first installed wins), then the configured favorite
    agent, then an error.
  - `--exclude P` drops a provider from automatic choice.
- **Model:** `--model`, then `<PROVIDER>_MODEL`, then `MODEL`, then
  frontmatter `model` (a list picks the first known entry), then the
  provider's default. An unknown model is passed through with a warning.
  OpenCode needs a model when non-interactive.

## Claudine's frontmatter keys

| Key | Meaning |
|---|---|
| `agent`, `model` | default provider and model (see above) |
| `prompt` | the prompt for `inline-compose` |
| `sections` | the prompt as a heading → content mapping; schema-checked only, since `inline-compose` can't run it yet |
| `interactive` | run interactively (not allowed in a sequence document) |
| `yolo` | turn on the provider's permission bypass; it does **not** approve shell |
| `argv` | reserved: the extra bare words from the command line |
| `timeout`, `step_timeout` (+ `_warn` variants) | run limits |
| `loop`, `sequence` | repetition and orchestration |
| `initialize`, `start`, `blocked`, `success`, `failure`, `cancelled`, `finalize` | lifecycle events |
| `hash`, `last_updated` | maintained by `inline-compose` |
| `title`, `description` | shown in dry runs and errors |
| `$schema` | your schema (Darkmatter) |

Claudine's trigger schemas type these keys automatically, even with no
`$schema`.

- An invalid lifecycle stack, `loop`, `agent`, `model`, or timeout **blocks
  the launch**: `blocked` and `finalize` run, and no agent starts.
- A schema problem with an optional key that your own `$schema` declares
  is dropped with a warning.
- Redeclaring a Claudine key in your `$schema` takes it over.
- An inline composition (`prompt`, `sections`, or `kind: inline-compose`,
  and no `sequence`) must set exactly one of `prompt` and `sections`. Both,
  or `kind: inline-compose` with neither, is a schema error. In the editor
  such a document gets diagnostics but no completion or hover from
  Claudine's schemas.

In your own `$schema`, a missing key and `null` are the same value:

```yaml
$schema:
  depth: number(default(2))             # missing or null → 2 (optional defaults are filled too)
  retries: number(required; default(3)) # missing or null → 3; never prompted
  count: number(required)               # missing → prompted or MissingProperties; `count: null` → fails the type
```

- **`required` only asks whether the key was written.** A written `null`
  meets it and is then judged by the type.
- **`inline-compose` at launch** leaves a missing required, non-`eager`
  value to the agent, but a value that resolves to `null` is type-checked
  then: `foo: string(required)` holding `null` refuses the launch.

## Overrides from the command line

```bash
claudine compose @review.md target=lib depth=2 'tags=["a","b"]'
claudine compose @review.md --set '{target: "lib"}' depth=3     # setters beat --set
```

Values are parsed as JSON5 and fall back to a string. A relative file value
resolves from the launch folder. Setters reach transcluded children, as in
`md compose`. If `prompt_for_missing` is `true` in config, Claudine asks for
missing required values on a terminal.

## The system prompt

- `--asp FILE` appends to the provider's system prompt, and `--rsp FILE`
  replaces it. They can't be combined, and either one skips discovery.
- **Discovery** starts from the launch folder: the package root, the package
  area root, the repository root, then `~/.claudine/system-prompt.md`. The
  first `system-prompt.md` found wins. It is composed like any document, and
  its frontmatter `mode: append|replace` decides how it's applied. An empty
  composed body turns the system prompt off.
- **Non-interactive runs** also append `.claudine/non-interactive.md` (repo,
  then home, then a built-in default).
- **Delivery** depends on the provider: a file flag for Claude,
  `developer_instructions` for Codex (64 KB cap). Kilo and Antigravity aren't
  supported and get a warning.

## Shell approval

- **What needs approval:** every `::shell`, `::shell-block`, frontmatter
  `$(…)`, and lifecycle `shell:`. Pre-flight asks for them all at once,
  before the run starts, including commands in branches that won't run.
- **Choices:** exact command, typed shape
  (`template git log origin/{{{ string }}}..HEAD`), whole program, once,
  deny, or blacklist.
- **Where approvals are saved:** `.darkmatter-shell-whitelist` and
  `.darkmatter-shell-blacklist`, at the prompt's repository root.
- **Without a terminal** on stdin and stderr, nothing is asked: an
  unapproved command fails the run. `--yolo` or `CLAUDINE_YOLO` approves
  everything for this run without saving anything, but the built-in
  blacklist still refuses `rm`, `mv`, `curl`, `sudo`, `git push`, and
  others.
- **A placeholder that evaluates to `null`** fails pre-flight. A command
  never runs with a missing value.

## `inline-compose`: documents that write their own body

```markdown
---
prompt: |-
  Summarize the open issues in {{ ctx.repo }} as a table.
---
```

```bash
claudine inline-compose --claude @notes/status.md
```

1. **Compose and launch.** The `prompt` value is composed (all of
   Darkmatter applies), wrapped with the document's path and the guidance
   in `.claudine/inline-compose.md`, and sent to the agent. The agent runs
   with the narrowest write permission (for example, Claude's
   `acceptEdits`). An explicit read-only mode refuses the run.
2. **The agent edits the file itself.** Its reply is only a summary.
3. **Claudine checks the result.** The body must be non-empty and actually
   changed. Claudine then re-levels it, restores `prompt`, stamps `hash`
   and `last_updated` (today's date), and writes the file atomically.
4. **Rollback.** A provider failure, Ctrl-C, broken YAML, or an unchanged
   body restores the original file.

A document containing both a non-null document-level `prompt` and `sequence`
defines an **inline sequence**. Run it with `claudine sequence`, not `compose`
or `inline-compose`. Each body step composes the document's `prompt` with its
own `seq` context and updates the document. Later steps re-read the live file,
so earlier body and frontmatter edits are available to them. A step-level
`prompt`, by contrast, is a reference to a separate prompt document.

## Previewing

```bash
claudine compose @review.md target=lib --dry-run > composed.md   # composed prompt on stdout; report on stderr
claudine compose @review.md --dry-run --json                     # one JSON report: file, outcome, diagnostics, summary
```

Nothing runs: no agent, lifecycle event, `$(…)` expansion, `::shell`, or
question, and nothing is written. What needs a command stays as written,
marked `[not evaluated in a dry run]`. The provider is chosen as a real run
chooses it, so name one (`--claude`, `agent:`) for a clean report. Exit `2`
when any finding is an error or warning, else `0`.
