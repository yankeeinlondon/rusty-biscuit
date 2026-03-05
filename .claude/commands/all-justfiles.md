---
name: all-justfiles
description: "Modify all justfiles in this monorepo consistently. USAGE: /all-justfiles <action> — e.g., /all-justfiles add a 'play' recipe that wraps playa CLI"
---

The user's requested action is: $ARGUMENTS

If the above is empty or says "$ARGUMENTS", stop immediately and reply with:

> You need to tell me **what** to do to the justfiles. Add your action after the command, like:
>
> - `/all-justfiles add a _sound recipe that plays a sound effect`
> - `/all-justfiles update the commit recipe to use claude instead of opencode`
> - `/all-justfiles remove the _ask_codex recipe from all area justfiles`
>
> Then I'll apply that change across all justfiles concurrently.

Do NOT proceed further — just show that message and wait.

---

## Instructions

You are an orchestrator. Use the `just` skill and ensure all subagents do too.

### Step 1: Identify justfiles

Find all `justfile` files in this monorepo (root + all package areas). There are ~16 of them.

### Step 2: Read the `just` skill

Read the `just` skill's SKILL.md to understand the monorepo justfile conventions, shared boilerplate patterns, ANSI color variables, and the three complexity tiers (minimal/standard/advanced). Ensure subagents follow these conventions.

### Step 3: Dispatch subagents

Launch subagents concurrently — one per justfile (or group small batches if needed). Each subagent should:

1. Read the `just` skill (SKILL.md at minimum)
2. Read its assigned justfile
3. Apply the user's action: **$ARGUMENTS**
4. Follow existing conventions in that file (ANSI variables, error handling patterns, recipe style)
5. Only modify the file if the action is relevant to it — skip files where the change doesn't apply

### Step 4: Report results

When all subagents complete, provide a summary:

- Which files were modified and what changed
- Which files were skipped and why
- Any issues or edge cases encountered

### Step 5: Notify

Run: `so-you-say "Justfiles were updated" --background >/dev/null 2>&1 || exit 0`
