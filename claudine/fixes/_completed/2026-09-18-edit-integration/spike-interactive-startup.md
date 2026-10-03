---
created: 2026-09-28
spec: 2026-09-18-edit-integration
phase: 1
spikes: [S1, S2, S3]
---

# Spike: Interactive Startup-Prompt Delivery Across the Fleet

This records Phase 1 spikes S1–S3 of the `2026-09-18-edit-integration` plan.
Each row answers one question: when Claudine launches the provider
**interactively** with a startup prompt, which native form submits that prompt
as the first turn and leaves the terminal session usable?

## How this evidence was gathered

The plan asked for a live run of every installed CLI inside a detached tmux
session. **That tier could not be produced in this session.** The agent
session that ran Phase 1 was non-interactive, and its permission policy
refused both `tmux` and every provider binary (`pi --help` and `tmux -V` were
refused alike). It also refused reads outside the worktree, including the
installed npm packages. The evidence below therefore comes from these tiers,
most authoritative first:

| Tier | Meaning |
|---|---|
| **source** | Upstream CLI source on its default branch, read on 2026-09-28 |
| **changelog** | Upstream changelog or issue tracker |
| **docs** | Upstream reference docs |
| **research** | This repository's `claudine/docs/research/agent-cli/*.md` (captured from `--help` and local probes on the date the research doc records) |
| **plan** | Help output quoted in the plan (observed by the planner on 2026-09-28) |

A live terminal confirmation is still owed for every row. Phase 4's
`real_pi_interactive_startup.rs` covers Pi. The other rows need one manual
pass in a session where `tmux` and the provider binaries are permitted
(see "Outstanding live checks" below).

## S1 — Fleet results

| Provider | Version evidence | Current interactive delivery | Verified native interactive form | Leading `-` handling | Verdict | Evidence tier |
|---|---|---|---|---|---|---|
| Claude | research 2.1.200 | positional; `--` before a `-`-prefixed prompt | `claude "<p>"` starts an interactive session with an initial prompt | `--` end-of-options (existing test) | **keep** | research (`claude.md:74`, `:1011`) |
| Codex | research 0.142.5 | positional; `--` before a `-`-prefixed prompt | `codex "<p>"` launches the TUI with an initial prompt | `--` (existing test) | **keep** | research (`codex.md:68`, `:1133`) |
| Gemini | research 0.49.0 | `--prompt-interactive <p>` (`=` when `-`-prefixed) | `--prompt-interactive` | attached `=` (existing test) | **keep** | research (`gemini.md:163`) |
| Qwen | research 0.19.6 | `--prompt-interactive <p>` (`=` when `-`-prefixed) | `--prompt-interactive` "runs an initial prompt and then stays interactive" | attached `=` (existing test) | **keep** | research (`qwen.md:62`, `:208`) |
| OpenCode | research 1.17.13 | `--prompt <p>` (`=` when `-`-prefixed) | `--prompt` "Initial prompt to use in TUI mode" (auto-submits, OpenCode PR #4510) | attached `=` (existing test) | **keep** | research (`opencode.md:307`), profile comment |
| Goose | not installed on the dev Mac; upstream `main` | `run -t <p>` in both modes | `goose run -t <p> --interactive` (`-s` short). `interactive` is "Continue in interactive mode after processing initial input", and the handler calls `session.interactive(input_config.contents)` | clap `-t/--text` has **no** `allow_hyphen_values`, so `-t "- item"` is rejected as an unknown flag. `--text=<p>` binds any value | **repair** | source (`block/goose` `crates/goose-cli/src/cli.rs`), research (`goose.md:444`) |
| Kilo | research 7.3.45 / 7.4.1 | `-- <p>` with no `run` entrypoint in interactive mode | TUI `kilo --prompt <p>` ("Top-level prompt to use"). Usage is `kilo [project]`, so today's positional is read as a **project directory** | Kilo is an OpenCode (yargs) fork, so use the OpenCode pattern: `--prompt=<p>` when `-`-prefixed | **repair** | research (`kilo.md:98`, `:317`, `:932`, `:964`) |
| Antigravity | research 1.1.0 | `--print <p>` in both modes (headless one-shot) | `agy --prompt-interactive <p>` (alias `-i`): "Runs an initial prompt interactively and continues the session"; needs a TTY | Go flag parsing reads the next argv item as the value whatever its first character. `--prompt-interactive=<p>` is accepted by both Go `flag` and `pflag`, so the attached form removes the question | **repair** | research (`antigravity.md:125–136`, `:518`); generated `antigravity/data.rs` already lists `--prompt-interactive` as a non-interactive conflicting flag |
| Pi | plan: 0.87.1 on the dev Mac (the binary resolves into `@earendil-works/pi-coding-agent`) | `Stdin(prompt)` in both modes | `pi [options] [--] [@files...] [messages...]`. Positional messages reach `InteractiveMode` as `initialMessages` | `--` is end-of-options from **0.84.3** (S2). After `--`, a token starting with `@` is **still** read as a file argument | **repair** | source (`packages/coding-agent/src/cli/args.ts`, `src/main.ts`), changelog |
| Kimi | plan: 2.0.2 on the dev Mac; research 0.22.2 | `--prompt <p>` | **None found.** `kimi [options]`; `-p/--prompt` "Run a single prompt non-interactively and stream the Assistant output to stdout"; no positional message; no initial-prompt option, env var, or config key | n/a | **N2 fires** | docs (`MoonshotAI/kimi-code` `docs/en/reference/kimi-command.md`), plan help quote, upstream feature request |

### Pi: why stdin cannot work interactively

The source settles it. `src/main.ts`:

```ts
function resolveAppMode(parsed, stdinIsTTY, stdoutIsTTY) {
  if (parsed.mode === "rpc") return "rpc";
  if (parsed.mode === "json") return "json";
  if (parsed.print || !stdinIsTTY || !stdoutIsTTY) return "print";
  return "interactive";
}
// …
stdinContent = await readPipedStdin();
if (stdinContent !== undefined && appMode === "interactive") appMode = "print";
```

A piped stdin always selects **print** mode. So `claudine pi "x" -i` today
runs a one-shot print session, not a TUI. Interactive delivery must be argv:
`AppendArgs(["--", prompt])`, which `args.ts` routes into `messages` and
`main.ts` hands to `InteractiveMode` as `initialMessages`. The source read did
not show whether `InteractiveMode` auto-submits `initialMessages`. The help
text's "Multiple messages (interactive)" example suggests it does. The Phase 4
real Pi test must confirm it.

**`@` edge case.** `args.ts` handles `--` like this:

```ts
if (arg === "--") {
  for (const positionalArg of args.slice(i + 1)) {
    if (positionalArg.startsWith("@")) result.fileArgs.push(positionalArg.slice(1));
    else result.messages.push(positionalArg);
  }
  break;
}
```

A prompt that begins with `@` (for example `@alice please review`) becomes a
file reference even after `--`. The Phase 3 Pi repair should decide on this
explicitly (see the message to the next agent in the spec).

### Kimi: no interactive startup surface (N2)

- Current reference docs list `--prompt` only as the non-interactive route,
  and the usage has no positional message.
- Upstream feature request `MoonshotAI/kimi-cli#2240` (May 2026) asks for a
  `--prompt-interactive` or `--init-prompt` option. It states that `--prompt`
  exits after one turn and that piping stdin skips interactive mode. The
  successor `kimi-code` docs show no such option.
- The only workaround anyone has found is two commands:
  `kimi -p "…"`, then `kimi --continue`. That is a second process and a new
  delivery mechanism, so rulings N1 and R2 forbid it.
- So today `claudine kimi "x" -i` runs a **non-interactive** one-shot and
  exits, while claiming an interactive session.

Per ruling N2, no refusal, fallback, warning, or allowlist was added. The spec
is set to `status: human-in-the-loop` for the author to amend the fleet-wide
promise.

## S2 — Pi version floor for `--`

- **Floor: Pi 0.84.3.** From the changelog under `[0.84.3]`: "Fixed
  dash-prefixed prompts being parsed as options by supporting `--` as an
  end-of-options delimiter (#7269)."
- The profile comment's claim that `--` is rejected was true for 0.80.3, which
  the research doc records, and is stale from 0.84.3 on. The dev Mac runs
  0.87.1 (plan), which is above the floor.
- Profile shape decided: interactive → `AppendArgs(["--", prompt])`;
  non-interactive stays `Stdin(prompt)`. Pi 0.80.3–0.84.2 would reject `--`
  with `Unknown option: --`, so interactive Pi needs ≥ 0.84.3. Phase 5 records
  that floor in `docs/research/agent-cli/pi.md`. There is no version sniffing.
- A live check of `pi -- "<multiline - prompt>"` was not possible (see
  "How this evidence was gathered").

## S3 — Argument-size headroom

- **Only Pi newly moves to argv.** The plan assumed Antigravity and Goose might
  too. They already deliver the prompt on argv in both modes (`--print <p>`,
  `-t <p>`), so their repairs change the flag but not the channel. Kilo keeps
  argv as well.
- Existing guards: OpenCode has two inline `ARG_MAX_HEADROOM` (768 KiB) guards
  and Kilo has one. Pi is the one new call site, so it gets a **per-profile
  guard**, not a shared helper (Rule 2).
- **Risk: Pi upstream issue #9200.** Pi 0.83.0 and 0.85.1 are reported to die
  with SIGKILL (exit 137, no output) as soon as one **positional** message
  argument reaches about 993 bytes. `@file`, stdin, and `--system-prompt`
  carry the same content fine. Upstream closed it as not planned
  (`no-action`), with no maintainer diagnosis in the thread. If the bug is real
  on current Pi, a 768 KiB guard would not help, and any edited prompt of
  about 1 KB would crash interactive Pi. That size is common. This could not
  be reproduced here. It is escalated to the author in `human_review_items`.
  The Phase 4 real Pi test should include a prompt of about 2 KB to settle it.

## Outstanding live checks

Run these in a session where `tmux` and the provider binaries are permitted.
Keep the session detached and never attach it.

```sh
tmux new-session -d -s spike-<p> -x 200 -y 50 '<argv from the table>'
sleep 20; tmux capture-pane -p -t spike-<p>   # first turn answered?
tmux send-keys -t spike-<p> 'Reply with the single word AGAIN.' Enter
sleep 20; tmux capture-pane -p -t spike-<p>   # second turn accepted?
tmux kill-session -t spike-<p>
```

Use the prompt "Reply with the single word READY." and a multiline prompt
whose first line is `- item`. For Pi, also try a prompt of about 2 KB (#9200).
