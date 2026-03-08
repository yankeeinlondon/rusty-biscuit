## What is Cross Referencing

When we discuss the idea of "cross referencing" Agentic CLI's  we primarily mean cross referencing **Agentic Skills** so that a "skill" available on one Agentic CLI is available on all other Agentic CLI's.

- the most important thing is WHERE a skill should reside and typically an agentic platform will have two answers to this question based on "scope":
    - The platform will typically have a "User Scope" which provides useful artifacts (including agent skills) for the user and is always available for that user
    - The platform will also typically have a "Repo Scope" which provides useful artifacts (including agent skills) which is stored at the root of a git repo
        - These skills are part of the repo and therefore benefit all users of this repo not just the user logged into the host system
        - These skills also _almost always_ (if not always) will override the skills at the `User scope` if there is a conflict in names.

### Beyond just Skills

- the other artifacts we typically find an Agentic CLI provide are:
    - **Agents**/**Subagents** (a definition of a )
    - **Slash Commands** (a set of pre-defined prompts that a user can call with a "slash command" when interactively working with a Agentic CLI)
    - **Scripts** (a common location for executable scripts to be stored)
- it is worth noting that:
    - it is common for Agents and Subagents to be effectively the same thing and some platforms may make no reference to subagents.
    -

## Your Task

Review each document in @claudine/docs/cross-referencing/ for correctness and completeness. You will need to do research on the given Agent CLI's website to make sure that you are completely current on the capabilities of that Agentic CLI before performing this task.

In order to be efficient with context windows as well as be time efficient you should act as an orchestrator and have each document perform this operation on a per file basis (which maps to also being on a per Agentic CLI basis). Each subagent which you give responsibility to you will:

- tell the agent/subagent which document they are responsible for (full filepath)
- tell them to do an overview read of the document first so they have an understanding of which Agentic CLI they are responsible for as well as some hints on the most useful URLs to check for documentation
- then tell them to do research online on the given Agentic CLI's documentation to make sure we have a complete picture of the current capabilities of this platform
- they should then update the document they are responsible for and summarize the changes they made when reporting back to you (the orchestrator)

Things you should always make sure the document contains are:

1. **Frontmatter:**
      - `homepage` should be a URL to the Agentic CLI's home page
      - `docs` should be a URL to the Agentic CLI's documentation page
      - `skills` should be a URL to the Agentic CLI's documentation about skills
      - `agent` should be a URL to the Agentic CLI's documentation on agent/subagents
      - `slash` should be a URL to the Agentic CLI's slash commands documentation
      - `scripts` should be a URL to describing where and how to use "scripts" with this Agentic CLI platform
2. **Body Content:**
      - Ensure there is a section `## Skills`
          - Indicate which whether the given Agentic CLI supports the idea of Agentic Skills (e.g., a tree of documents starting with a `SKILL.md` linking to various other docs which are organized to provide a context-efficient way of learning about a topic through "progressive disclosure")
          - Indicate where these skills are located in the filesystem, being sure to distinguish between user scope and repo scope
          - Mention if there are any pre-requisites to use skills (e.g., a plugin installed, etc.)
          - Describe any best practices which are explicitly mentioned in this provider's docs
          - Describe what frontmatter properties are used in skills as well as which are required versus optional
          - Mention, if known, when "skills" were first introduced (date and/or version number)
          - Also indicate if the Agentic CLI will read Claude Code's directories as well as their own directories (as this is often done because Claude Code is often seen as a leader in the space). Obviously ignore this information if the Agentic CLI you're focusing on **is** Claude Code.
      - Ensure there is a section `## Slash Commands`
          - Indicate whether the given Agentic CLI supports "slash commands" (aka, being able to execute a certain reusable prompt when working interactively in the Agentic CLI by typing `/` and then the name of the slash command)
          - Indicate what directories are looked at for slash commands and how subdirectories of the base slash commands folder are treated.
              - Also indicate if the Agentic CLI will read Claude Code's directories as well as their own directories (as this is often done because Claude Code is often seen as a leader in the space). Obviously ignore this information if the Agentic CLI you're focusing on IS Claude Code.
          - Detail which frontmatter properties are required and which ones are optional
      - Ensure there is a `## Agent / Subagents` section
          - Indicate whether the given Agentic CLI support the idea of "Agents" or "Subagent" which typically come with a their own "system prompt" like instruction as well as possible a set of skills which the agent will use.
          - Describe how an "orchestrator" is meant to call agents/subagents. As a one-off or as a concurrent call to multiple agents/subagents.
          - Describe the explicit vernacular used to describe agents / subagents within this Agentic CLI
          - Describe what frontmatter properties are required and which ones are optional
          - If the Agentic CLI is **not** Claude Code, then describe how it varies in implementation, approach or orchestration approach to Claude Code
      - Ensure there is a `## Scripts` section
          - Describe if the Agentic
      - Ensure there is a `## Sources` section of the document
          - This section should be a markdown list of markdown links to ALL of the sites which were used during research or are deemed to be have useful reference material for the topics covered here

Each agent/subagent should update the document they are assigned to and then report back to the orchestrator a summary of what they changed.
