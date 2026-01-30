# Updating Claude Code Skills

We will start by describing what a Claude Code "skill" is and the best practices surrounding it. Your TASK will be described in detail at the bottom but will involve creating or updating one of the skills in this monorepo.

## What a Claude Code skill is (mechanically)

A skill is a folder containing a required `SKILL.md` file:

- **YAML frontmatter** at the top tells Claude *what the skill is* and *when/how it can be invoked*.
- **Markdown body** contains the instructions Claude should follow *when the skill is loaded*.
- Optional supporting files live alongside it and can be referenced from `SKILL.md`. ([code.claude.com](https://code.claude.com/docs/en/skills))

Crucially, only **skill metadata** (name/description) is preloaded broadly; the **full `SKILL.md`** is loaded when relevant/invoked; **linked files** are loaded on demand. This is the progressive disclosure model. ([platform.claude.com](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices))

---

## Step 1: Pick the right scope + create the folder

Claude Code supports multiple locations; choose based on who should see the skill:

- **Personal (all your projects):** `~/.claude/skills/<skill-name>/SKILL.md`
- **Project (repo-local):** `.claude/skills/<skill-name>/SKILL.md`
- Plus enterprise-managed and plugin-provided skills; precedence is enterprise > personal > project (plugin skills are namespaced to avoid conflicts). ([code.claude.com](https://code.claude.com/docs/en/skills))

Create the directory:

```bash
mkdir -p ~/.claude/skills/<skill-name>
# or
mkdir -p .claude/skills/<skill-name>
```

---

## Step 2: Author `SKILL.md` with “triggerable” frontmatter

A minimal working `SKILL.md` looks like:

```md
---
name: my-skill
description: What it does. Use when the user asks for X, Y, or Z.
---

# Instructions
...
```

Claude Code’s own docs emphasize that `name` becomes the `/slash-command`, and `description` is what Claude uses to decide when to load the skill automatically. ([code.claude.com](https://code.claude.com/docs/en/skills))

### Naming + description constraints (important)

Across Anthropic’s skills interfaces, frontmatter has explicit constraints:

- `name`: lowercase letters/numbers/hyphens only; max length 64; reserved words include “anthropic” and “claude”; no XML tags. ([platform.claude.com](https://platform.claude.com/docs/en/build-with-claude/skills-guide))
- `description`: non-empty; max length 1024; no XML tags. ([platform.claude.com](https://platform.claude.com/docs/en/build-with-claude/skills-guide))

### Write descriptions that actually trigger

Best practice is to embed:
- **What it does**
- **When to use it** (trigger conditions and user phrasing)

This aligns with the idea that metadata is “just enough information” for Claude to decide relevance without loading everything. ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))

Example:

```yaml
description: Summarizes and critiques PRs in this repo. Use when the user says "review my PR", "code review", or shares a GitHub PR link.
```

If a skill triggers too often, tighten the description or disable model invocation (next section). ([code.claude.com](https://code.claude.com/docs/en/skills))

---

## Supported frontmatter properties in Claude Code

Claude Code documents the following frontmatter fields (all optional; `description` is “recommended” for good auto-discovery). ([code.claude.com](https://code.claude.com/docs/en/skills))

| Field | What it does |
|---|---|
| `name` | Display name / slash command. If omitted, directory name is used. Lowercase letters/numbers/hyphens only; max 64 chars. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `description` | Used for automatic relevance matching; if omitted, first paragraph of markdown is used. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `argument-hint` | Autocomplete hint for expected args (e.g., `[issue-number]`, `[path] [format]`). ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `disable-model-invocation` | If `true`, Claude won’t auto-load/run it; you must invoke via `/name`. Default `false`. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `user-invocable` | If `false`, hides from the `/` menu; Claude can still invoke it unless you also disable model invocation. Default `true`. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `allowed-tools` | Tools Claude may use without asking permission while the skill is active (e.g., read-only sets). ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `model` | Model to use when the skill is active. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `context` | Set to `fork` to run in a forked subagent context. ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `agent` | Subagent type/config to use when `context: fork` is set (built-in or from `.claude/agents/`). ([code.claude.com](https://code.claude.com/docs/en/skills)) |
| `hooks` | Hooks scoped to this skill’s lifecycle (format documented under Hooks). ([code.claude.com](https://code.claude.com/docs/en/skills)) |

### String substitutions you can rely on

Claude Code skills support argument/session substitutions inside the markdown body:

- `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N`
- `${CLAUDE_SESSION_ID}`

Also, if you pass arguments but don’t include `$ARGUMENTS`, Claude Code appends them as `ARGUMENTS: <value>`. ([code.claude.com](https://code.claude.com/docs/en/skills))

---

## Step 3: Structure the markdown using progressive disclosure

Progressive disclosure is not just a platform feature; it should drive how you *write* skills:

**Level 1 — Frontmatter:** always loaded, so keep it tight and purely for routing (what + when). ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))
**Level 2 — `SKILL.md` body:** loaded only when relevant; should be the minimal “operating manual” for the workflow. ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))
**Level 3 — Linked files:** deep reference, long examples, specs—only load when needed. ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))

Claude Code explicitly recommends keeping `SKILL.md` focused (e.g., “under 500 lines”) and moving detail into separate files linked from the skill. ([code.claude.com](https://code.claude.com/docs/en/skills))

### A best-practice `SKILL.md` template

```md
---
name: <kebab-case-skill-name>
description: <What it does>. Use when user asks <trigger phrase 1>, <trigger phrase 2>, or provides <artifact type>.
argument-hint: "[optional args]"
disable-model-invocation: false   # set true for side-effect workflows
user-invocable: true              # hide if it’s background knowledge only
allowed-tools: Read, Grep         # keep minimal; tighten for safety
context: fork                     # only if you want an isolated subagent
agent: general-purpose            # or Explore/Plan/custom agent
---

## Purpose
State outcome in 1–3 bullets. Avoid repetition of description.

## Inputs
- What inputs are expected (files, paths, URLs, arguments)
- How arguments map to `$ARGUMENTS` (include examples)

## Procedure
A numbered checklist Claude should follow.
- Include guardrails (what NOT to do)
- Include validation steps
- Include stop conditions / when to ask questions

## Output contract
Specify the exact shape: headings, JSON, patch format, etc.

## Safety & permissions
- Which tools are allowed
- When to request permission
- Any “never do” items (deploy, delete, email, etc.)

## Examples (short)
Keep to 1–3 concise examples.

## Additional resources (loaded only if needed)
- Deep reference: [reference.md](reference.md)
- Examples: [examples.md](examples.md)
- Runbook: [runbook.md](runbook.md)
```

Why this works:
- The body is optimized for “loaded when relevant,” but still short enough not to dominate the context window. ([platform.claude.com](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices))
- “Additional resources” explicitly tells Claude where to go next, instead of dumping everything in the body. ([code.claude.com](https://code.claude.com/docs/en/skills))

### Organizing supporting files

Claude Code supports bundling additional files in the skill directory to keep `SKILL.md` lean; you link to them so Claude can load them on demand. ([code.claude.com](https://code.claude.com/docs/en/skills))

Example layout:

```text
my-skill/
├── SKILL.md              # overview + workflow
├── reference.md          # deep API/spec details (only load if needed)
├── examples.md           # lots of examples (only load if needed)
└── scripts/
    └── validate.sh       # executable helper
```

---

## Step 4: Control invocation + reduce accidental side effects

Claude Code provides two complementary controls: ([code.claude.com](https://code.claude.com/docs/en/skills))

- `disable-model-invocation: true`
  Use for workflows with side effects (deploy, commit, send messages). This prevents Claude from deciding to run it automatically. ([code.claude.com](https://code.claude.com/docs/en/skills))
- `user-invocable: false`
  Use for background knowledge that should inform answers but doesn’t make sense as a user command. ([code.claude.com](https://code.claude.com/docs/en/skills))

If you want Claude to *never even see* the skill in normal sessions, disabling model invocation removes the skill from Claude’s context entirely (per the permissions section). ([code.claude.com](https://code.claude.com/docs/en/skills))

---

## Step 5: Tool permissions as part of skill design

`allowed-tools` is a best-practice security lever: allow only what’s needed for the workflow (e.g., a “safe reader” skill that can only read/search). ([code.claude.com](https://code.claude.com/docs/en/skills))

Example:

```yaml
allowed-tools: Read, Grep, Glob
```

Design implication: if the skill’s procedure requires editing or executing scripts, don’t accidentally ship it with read-only tool permissions (it will fail or constantly ask). Conversely, don’t grant broad tools “just in case”—it increases risk and makes behavior harder to predict.

---

## Step 6: Test like you mean it

Anthropic’s guidance is explicit: good skills are concise, well-structured, and **tested with real usage**; the context window is shared across everything, so token discipline matters. ([platform.claude.com](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices))

A pragmatic test loop:

1. **Trigger tests:** write 10–20 prompts that *should* trigger the skill and 10–20 that *shouldn’t*, then refine `description` until behavior is stable. (The “Complete Guide” suggests measuring trigger rates explicitly.) ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))
2. **Invocation tests:** run `/skill-name …args…` with representative args and confirm `$ARGUMENTS` substitution behaves as expected. ([code.claude.com](https://code.claude.com/docs/en/skills))
3. **Regression tests:** when you add capabilities, keep “core path” examples short in `SKILL.md` and move larger test cases to `examples.md` so you don’t bloat the always-loaded body when the skill is active. ([code.claude.com](https://code.claude.com/docs/en/skills))

---

## Step 7: Share and learn from examples

- For canonical examples, Anthropic maintains a public repository of skills demonstrating patterns across technical and creative use cases. ([github.com](https://github.com/anthropics/skills?utm_source=chatgpt.com))
- In Claude Code, project skills live in `.claude/skills/` and can be committed to version control for team reuse. ([code.claude.com](https://code.claude.com/docs/en/skills))

---

## Common mistakes (and how to avoid them)

1. **Vague descriptions** → skill won’t trigger or triggers randomly
   Fix: include explicit trigger phrases and artifact types in `description`. ([resources.anthropic.com](https://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf?hsLang=en))

2. **Stuffing everything into `SKILL.md`** → once loaded, it competes with conversation context
   Fix: keep `SKILL.md` as the workflow “router,” link deep docs into `reference.md`, `examples.md`, etc. ([platform.claude.com](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices))

3. **Letting Claude auto-run side-effect skills**
   Fix: `disable-model-invocation: true` for deploy/commit/send workflows. ([code.claude.com](https://code.claude.com/docs/en/skills))

4. **Over-broad tool permissions**
   Fix: use `allowed-tools` to minimize capability to the least privilege that still completes the workflow. ([code.claude.com](https://code.claude.com/docs/en/skills))

## YOUR TASK

**USE THE "claude" SKILL to do this task.**

- Review the @playa/README.md @playa/cli/README.md @playa/lib/README.md for documentation context on the Darkmatter package
- then review the source code in each sub-package under schematic and update the documents where there has been any drift in the documentation.
- Review the monorepo's @CLAUDE.md and see if there is anything which should be updated here
- Now create (or update if it already exists) a Claude Code skill for "playa" locally in @.claude/skills .


**IMPORTANT:** Remember that skills must have a `SKILL.md` file but that file should exhibit clear signs of being as short as possible by using using the best practice of progressive disclosure.

- If you only have ONE file in a skill you have almost certainly NOT done this correctly!
- If the SKILL.md is more than 500 lines then you almost certainly have NOT done this correctly!
