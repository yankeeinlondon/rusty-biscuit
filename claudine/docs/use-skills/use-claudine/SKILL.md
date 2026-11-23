---
name: use-claudine
description: Use the Claudine CLI and Rust library as a caller. Covers wrapped execution of agentic CLIs (`claudine claude|codex|gemini|opencode|kimi|qwen|goose|kilo|pi|antigravity`), the universal switches (`--yolo`, `-i`, `--edit`, `-m`, `--asp`/`--rsp`, `--timeout`, `--dry-run`, `--mcp`), executable Markdown with `claudine compose`, `inline-compose`, and `sequence` (Darkmatter composition by proxy — `{{{ }}}` interpolation, `ctx.*`, `::file` transclusion, `::block when=`, `::shell`, `$(...)`, `$schema`), lifecycle events (`initialize`, `start`, `blocked`, `success`, `failure`, `cancelled`, `finalize`) and their actions (`say`, `effect`, `message`, `notify`, `shell`, `set`, `retry`, `resume`, `proxy`, `prep`, `edit`, `break`), `loop:`, sequences and groups, provider/model selection, shell pre-flight approval, shared skills/commands/agents sync, hooks and actions in `~/.claudine/config.yaml`, MCP, and the library/`claudine-contract` APIs. For changing Claudine's own source, load the repo's `claudine` skill instead.
---

# Using Claudine

Claudine sits in front of agentic CLIs such as Claude Code, Codex, and
Gemini. It does five things:

1. **Wraps** each agent behind one set of switches. `claudine codex -y 'fix
   the test'` works the same way for every provider.
2. **Runs Markdown as a program.** `claudine compose prompt.md` composes the
   document with **Darkmatter** (interpolation, transclusion, conditionals,
   shell output), sends the result as the prompt, and runs the document's
   **lifecycle events** around the agent's run.
3. **Orchestrates** multi-step work with `claudine sequence`: prompts, shell
   commands, nested sequences, and concurrent groups.
4. **Syncs shared resources** (skills, slash commands, agent definitions,
   MCP servers) across every installed agent.
5. **Handles agent hook events** with configured actions: speech, sound,
   messages, shell, and a blocking `call`, plus the **Protect** service that
   blocks dangerous tool calls.

This skill is for **callers**. To change Claudine itself, load the repo's
`claudine` skill.

## Pick your task

| You want to… | Read |
|---|---|
| Run an agent through Claudine, or learn the switches and admin commands | [cli.md](cli.md) |
| Write a prompt document for `compose`/`inline-compose` (Darkmatter features, provider choice, system prompt) | [composition.md](composition.md) |
| React to a run: speech, notifications, retry, hand off, loop | [lifecycle.md](lifecycle.md) |
| Run several steps or parallel work | [sequences.md](sequences.md) |
| Configure hooks, actions, TTS, Protect, MCP, or shared skills | [config.md](config.md) |
| Call Claudine from Rust | [library.md](library.md) |

For Darkmatter features in depth, load the `use-darkmatter` skill. For
editor support of Claudine documents, load `use-dmls`.

## Seven facts that answer most questions

1. **A prompt on the command line means non-interactive.** `claudine claude
   'why?'` runs once and exits. With no prompt, or with `-i`, the session is
   interactive.
2. **Wrapped runs start at the repository root.** Your original folder is
   remembered and exposed as `PACKAGE`/`PACKAGE_AREA` in a monorepo.
3. **Environment variables that look like secrets are removed** before the
   agent starts: names containing `TOKEN`, `SECRET`, or `API_KEY`, or
   ending in `_KEY`, and so on. The provider's own auth variables are kept.
   Keep any other variable with `--include NAME`.
4. **Composition is Darkmatter.** Everything in a prompt document's body
   (`{{{ }}}`, `::file`, `::block`, `::map`, `::shell`) and its `$schema` is
   Darkmatter's grammar. Claudine adds the provider choice, lifecycle
   events, Claudine's own frontmatter keys, and the agent launch.
5. **Every shell command must be approved before the run starts.** That
   covers `::shell`, `$(…)`, and lifecycle `shell:`, in every branch. On a
   terminal you're asked once. Without a terminal, an unapproved command
   fails the run. `--yolo` approves everything for one run, but the built-in
   blacklist still applies.
6. **`initialize` can never run shell.** Shell actions are allowed from
   `start` onward.
7. **Exit codes:** `0` success; `1` failure, including an agent that exited 0
   but whose result failed Claudine's check; `2` usage error; `130` Ctrl-C.
   A looping sequence that ends with `break` exits with the `break` code.

## Quick start

```bash
cd claudine && just install            # builds and installs `claudine` from the rusty-biscuit repo
claudine skills                        # first run sets up ~/.claudine/config.yaml (interactive on a TTY)
claudine claude 'summarize this repo'  # wrapped, non-interactive
claudine codex -i                      # wrapped, interactive
claudine compose @prompts/review.md target=src --dry-run   # see the composed prompt; launches nothing
claudine compose @prompts/review.md target=src --codex     # run it with Codex
```

A minimal prompt document, `prompts/review.md`:

```markdown
---
agent: claude                # default provider when not run on a terminal
target: src
success:
  - say: "Review of {{ target }} finished"
failure:
  - notify: "Review failed: {{ err.msg }}"
---
# Review {{ target }}

You are in {{ ctx.repo }} on branch {{ ctx.branch }}.

::file ./review-rules.md
```

## Gotchas

- **Claudine options may go anywhere before `--`.** Position never changes
  meaning: `claudine claude "fix it" -m opus` is the same as
  `claudine claude -m opus "fix it"`. Only what follows the first `--` goes
  to the provider (`claudine claude "hi" -- --output-format json`). If an
  option's position seems to matter, that is a Claudine bug.
- **`-h`/`--help` anywhere before `--` shows help** for the command you
  named (`claudine compose -h`, `claudine claude "hi" -h`), and exits 0.
- **`-o`/`--output` is not implemented yet.** Claudine refuses it (exit 2)
  rather than passing it to the provider.
- **There are ten wrappers**, including `kilo`, `pi`, and `antigravity`.
  Binaries built before 2026-11-08 left those three out of `claudine -h` and
  listed a nonexistent `budget` command.
- **There is no `claudine init`.** Setup runs on the first command that needs
  config; `claudine setup` reruns it without prompting and keeps your
  settings (run it when `md` warns about a format 1 `darkmatter-host`
  manifest). Change settings later with `claudine config`.
- **On a terminal, `compose` always opens a provider picker** unless you
  pass `--provider`/`--claude`/…. Frontmatter `agent:` only sets its
  default. Without a terminal, `agent:` decides.
- **`--dry-run` still runs `$(…)` composition expansions**, with real
  effects. It skips the provider, the lifecycle events, and sequence
  `shell:`/`side_effect:` tasks.
- **Frontmatter `yolo: true` turns on the provider's permission bypass**, but
  doesn't approve shell commands. Only `--yolo` or `CLAUDINE_YOLO` does
  that.
- **A stale binary.** If `claudine` disagrees with the docs, rebuild it with
  `cd claudine && just install`.
- **Tests and scripts that compose shipped prompts** should set
  `PLAYA_DRY_RUN=1`, so lifecycle speech and sounds stay silent.

## Canonical documentation

These files summarize the repo docs. The authoritative pages are in the
rusty-biscuit monorepo (main checkout `/Volumes/coding/personal/rusty-biscuit/`):

- `claudine/docs/getting-started/index.md`: the long-form tour
- `claudine/docs/topics/composition.md` and `frontmatter-properties.md`
- `claudine/docs/topics/flow-control/lifecycle.md`, `flow-control.md`,
  `looping.md`, and `sequences.md`
- `claudine/docs/topics/system-prompt.md`, `pre-flight-checks.md`,
  `timeouts.md`, and `schemas.md`
- `claudine/docs/cli/sequence.md` and `steer.md`
- `claudine/contract/README.md`: the `InferenceAdapter` crate
- `darkmatter/docs/`: everything the composition pipeline does
