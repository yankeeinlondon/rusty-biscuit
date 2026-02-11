# Agent Skills

The concept of an [Agent Skill](https://agentskills.io/home) for LLM Agents was initially defined by Anthropic for Claude Code but is now supported by many other Agentic platforms as well, including:

| Agentic CLI | User Scope (skills path) | Repo Scope (skills path) |
|---|---|---|
| [Claude Code](https://code.claude.com/docs/en/skills) | `~/.claude/skills/<skill-name>/SKILL.md` | `.claude/skills/<skill-name>/SKILL.md` |
| [Codex CLI](https://developers.openai.com/codex/skills/) | `~/.codex/skills/<skill-name>/SKILL.md` | `.codex/skills/<skill-name>/SKILL.md` *(community suggested project convention)* |
| [OpenCode CLI](https://opencode.ai/docs/skills/) | `~/.config/opencode/skills/<name>/SKILL.md` | `.opencode/skills/<name>/SKILL.md` |

## Creating an Agent Skill

An **Agent Skill** is a packaged, discoverable workflow that an agent can **select and load on demand**. A typical skill is:

- A **directory** (the skill package)
- A required **`SKILL.md`** file containing:
    - **YAML frontmatter** (machine-readable metadata)
    - A **Markdown body** (human-authored instructions)
- Optional supporting files:
    - `scripts/` (deterministic executables)
    - `references/` or `docs/` (deep documentation)
    - `assets/` (templates, examples, icons)

Many modern agent systems use **progressive disclosure**: they preload only a lightweight **index** of skill metadata (e.g., `name` + `description`) and only load the full `SKILL.md` when the skill is actually selected.

## Progressive disclosure and context windows

### What problem it solves
Context windows are scarce. If you inject every skill’s full instructions into context, you:

- Spend tokens on irrelevant information
- Increase instruction conflicts
- Raise the likelihood the agent misses the most relevant procedure

### The layering model (recommended)
A practical, scalable pattern is 3–5 layers:

1. **Metadata only (always loaded)**
   - Purpose: routing/selection (“should I use this skill?”)
   - Typically includes: `name`, `description`, and sometimes file path and additional optional metadata
   - Very low token cost per skill

2. **Primary instructions (`SKILL.md` body; loaded on use)**
   - Purpose: the playbook (what to do, in what order, and what to produce)
   - Must be concise and directive

3. **Secondary references (linked files; loaded only if needed)**
   - Purpose: deep tables, edge cases, API specifics
   - Keep these “one hop” away from `SKILL.md` for predictable retrieval

4. **Executable scripts (run, not read)**
   - Purpose: deterministic parsing, validation, transforms, formatting
   - Used to avoid “winging it” and to compress noisy inputs into concise outputs

5. *(Optional)* **Forked/sub-agent contexts**
   - Purpose: keep execution chatter from polluting the primary conversation
   - Some runtimes support this as a frontmatter option; behavior varies by tool/version

### Why it matters
Progressive disclosure improves:

- **Routing quality**: selection happens via metadata rather than scanning huge docs
- **Context hygiene**: only relevant instructions are loaded
- **Maintainability**: you can accumulate many skills without bloating every session

---

## Skill package structure

### Recommended directory layout

```text
my-skill/
  SKILL.md                 # required
  references/              # optional deep docs
    advanced.md
    api-tables.md
  scripts/                 # optional deterministic utilities
    validate.sh
    parse.py
  assets/                  # optional templates/examples/icons
    report-template.md
    icon.png
  agents/
    openai.yaml            # optional in some ecosystems (UI + deps)
```


## Task

Your task is to update the {{LIBRARY}} skill located in @/.claude/skills/{{LIBRARY}}/ . To provide insight we've just finished updating the following documents in the {{LIB_NAME}} package area to be completely aligned with the source code: $(just readme_files) {{DOCS}} {{ARGS}}. The summary description of what changed to these documents is as follows:

{{SUMMARY}}

The latest information should inform a good deal of how you'll update the existing agent skill but you can also use git to detect how these documents have changed. Read the @/.claude/skills/{{LIBRARY}}/SKILL.md first to understand the current skill structure. Try to make sure you retain existing knowledge found in the skill; you are not just writing a new skill you're updating an an existing one.


