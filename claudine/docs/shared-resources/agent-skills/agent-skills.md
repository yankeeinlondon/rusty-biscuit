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

## Agent Skill Synchronization

Claudine's ability to share Agent Skills across agentic platforms is done via a process we will refer to as _synchronization_. You kick off this process with:

::file ./cli-sync.md

This process must ensure that all canonical Agent Skill definitions are identified and "upgraded" to be correct and portable. What this _means_ will be explored next.

### Property Mutations

The following Frontmatter properties for each canonical agent skill will be evaluated and updated where appropriate:

- `name` - if missing or different from the skill directory it is in will result in the name being updated
- `description` 
    - if missing then the default behavior is to:
        - report an error as part of CLI output (but continue onto next skill not "fail fast")
        - remove any pre-existing symbolic links to this skill
    - if the user included the `--fix` flag when requesting the synchronization we will instead use a agent to produce a "description" for us
        - by default this will result in an interactive dialog asking which agent should be used but the caller may specify `--agent <agent>` to avoid the interactive dialog
        - by default the agent's default model will be used but this can be specified with `--model <model>`

        > Once an agent has been chosen -- interactively or with a CLI switch -- it will be used consistently throughout the run instead of re-asking each time a description if missing
- `model`
    - Claudine provides a set of [abstracted model names](claudine/docs/topics/abstracted-model-names.md) that can be used to characterize the kind of model you want to use. This preserves portability while allowing for these abstracted names to be replaced with a real model name at runtime.
    - When the model property is set to a value we will keep it "as is" if it is one of these portable model names
    - If the model is a valid model name but not abstract than we will convert it to an abstracted model name; this will be clearly communicated to the caller as part of the process
    - If the model is an invalid model name then we will report the invalid model reference while removing any symbolic links which may already point to this canonical agent skill
    - If the caller included `--fix` and the model is invalid then the invalid model reference will be removed entirely leaving `model` undefined

All other Frontmatter properties that might exist on an agent skill definition 


When the agent skills are _synchronized_ we must run through a process who's goal is to ensure that every skill is available to every agent, ensure the skills are valid, and optimize for portability where possible. The end result of this process is that every _canonical_ version of the skill is:

1. Identify all canonical `SKILL.md` files in User and Repo (_when in a repo_) scope
2. Identify which agentic providers are installed on the system
3. Iterate over each canonical skill definition:
    a. ensure that baseline/core properties are valid (`name`, `description`, `model`), and portable (`model`)
    b. look at the extended properties and optimize portability and semantic value
    c. update the canonical skill
    d. create symbolic links for each _installed_ agentic provider on the host pointing to the canonical skill's `SKILL.md`

This process is executed with one of the following commands:


The output of this process is a set of _symbolic links_ which point to the canonical definition found in the file system. In order for the agent skill to be both valid and portable for all agentic CLI's we will go through an "upgrade" process which:

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
