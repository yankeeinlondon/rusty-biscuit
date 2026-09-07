# Agent Skills in Claudine

## Back Story

The idea of an **Agent Skill** came from Anthropic as a way to provided detailed knowledge on a topic but in a way that did not overwhelm the _context window_ of the LLM model. This balance is achieved by something which is referred to as "progressive disclosure" which for an Agent Skill means that the structure of the skill always is originated in a single `SKILL.md` file and this file's responsibility is primarily to provide an overview of the topic with _links_ into more detailed documents. This structure allows the LLM to get an overview of the given topic and then pursue those sub-topics which are relevant to the problem it's trying to solve. The context window ends up getting what it needs but not _everything_ that the skill knows about the given topic.

While Agent Skills came from Anthropic (and were made available in Claude Code first), the utility it provided was immediately obvious and so every Agentic CLI provider quickly added this functionality too. Agent Skill today are now a "soft standard":

- all agents provide the functionality, 
- it's implemented in very similar (but not identical) ways
- each vendor puts these skills in under their own proprietary directory (e.g., `.claude/skills`, `.codex/skills`, etc.)

## Agent Skill Variances

### Location

Agent Skills tend to _always_ vary in their base location and this is not surprising because while Agent Skills are ubiquitous across vendors there is no official standard and each vendor wants to _namespace_ their agent skills so that they can dictate and match up the fine details of their implementation to a skill in a specific file location.

> **Note:** the more cynical amoungst you will undoubtedly be pointing out that this variance is due to an attempt to help vendor lock in. I think it's fair to say _both_ could be true.

Fortunately while the "base directory" for skills tends to vary by vendor, the _directory structure_ does not:

- `{base-directory}/skills/{skill-name}/SKILL.md`

### File Content

The entry point to an Agent Skill is always a `SKILL.md` file that is placed in a subdirectory of the "skills" directory. The prose content of a `SKILL.md` is always the same across providers and well structured skills will ensure they take advantage of [progressive disclosure](./progressive-disclosure.md) to get the best results. For variance, we instead need to look at the Frontmatter properties that either _must_ or _can_ be in a `SKILL.md` file.

::file ./skills-properties-core.md

::file ./skills-properties-extended.md

## Operationalizing Agent Skill Sharing

In order to make sure all of a user's or a repo's Agent Skills are shared across all of the installed agentic CLI's you will call into the Claudine CLI:

::file ./cli-sync.md



The output of this process is a set of _symbolic links_ which point to the canonical definition found in the file system. In order for the agent skill to be valid for all agentic CLI's, however, we may have to "upgrade" the canonical definition first to make it portable. This upgrade starts with the basics:

1. if the Agent skill doesn't define a `name` Frontmatter property then it will be added (this is 100% inferrable based on the directory structure)
2. if the Agent skill doesn't define a `description` Frontmatter property then it is invalid across all agentic CLI's (as it's the description which is used LLM to determine whether this skill should be used)
    - by default a lack of a description means that 
        - the skill will NOT be synchronized
        - the invalid format will be reported to the user
        - the synchronization process, however, will not "fail fast" and will attempt to complete the remaining skills
    - if you instead want an LLM to provide a good description for you then you can add the `--fix` flag to the end of the CLI commands


from the original source to all other locations for the providers installed on the host and for the same scope. For instance:

- if the original Agent skill was defined in Claude Code as a repo-scoped skill, it would reside at: `{repo-root}/.claude/skills/{skill-name}/SKILL.md`
- then let's imagine that the given host had both **Codex** and the **Kimi CLI** installed (_in addition to **Claude Code**_); we would then create a symbolic links at the following locations which point to the Claude Code skill:
    - `{repo-root}/.codex/skills/{skill-name}/SKILL.md` -> `../../.claude/{skill-name}/SKILL.md`
    - `{repo-root}/.kimi/skills/{skill-name}/SKILL.md` -> `../../.claude/{skill-name}/SKILL.md`

to the originally authored content. However, if a _conflict_ in conventions is detected then a transformed file will be created. Fortunately with Agent Skills the symbolic link is almost always sufficient.

> **symbolic links** not only provide a skill to another agent but are done in a way that is **DRY** and where any change to this skill will be automatically reflected in all hosts.
