# Update Skill

To be time efficient and context-window efficient you will run as an orchestrator.

- you will initially kick off two agents concurrently:
    1. Library Drift
       - Look at the @claudine/lib/README.md file and make sure it is is in sync with the actual Claudine library source code in @claudine/lib/src
       - Update the README.md file where to ensure it is consistent with what has actually been implemented
    2. CLI Drift
        - Look at the @claudine/cli/README.md file and make sure it is is in sync with the actual Claudine CLI source code in @claudine/cli/src
        - Update the README.md file where to ensure it is consistent with what has actually been implemented
- once completed you will kick off a single subagent to:
    - make sure that the @claudine/README.md has no conflicting information with that found in:
        - @claudine/lib/README.md
        - @claudine/cli/README.md
    - if there are any conflicts then assume the more detailed documents to be correct and update @claudine/README.md
- you will now kick off a subagent to
    - copy the directory @claudine/docs/hooks to @.claude/skills/claudine/research/hooks
    - copy the directory @claudine/docs/cli to @.claude/skills/claudine/research/cli
    - copy the directory @claudine/docs/cross-referencing to @.claude/skills/claudine/research/cross-referencing
- once the agent is complete you will kick off the following subagents concurrently:
    1. Skill File Subagent
       - create a SKILL.md file in @.claude/skills/claudine/SKILL.md
           - NOTE: there may well be an existing skill file there but you will write over it to ensure it's completely fresh
       - this Markdown file must define both a `name`, `description`, and `last_updated` frontmatter properties
           - the `name` is always just 'claudine'
           - the `description` is always 'Details on the Claudine library and CLI, including deep research into Agentic CLI platforms such as Claude Code, Codex CLI, Goose, Opencode CLI, and all other Agentic CLI's supported by the Claudine library.'
           - the `last_updated` property will be the current date in YYYY-MM-DD format
       - the sections of this Markdown file will be:
           - `## Claudine Library`
               - this section will leverage the @claudine/lib/README.md and @claudine/README.md files to describe structurally what the Claudine library is about.
               - this summary description will be followed by a markdown list of topic areas:
                   - the Supported Platforms (`supported-platforms.md`)
                   - the Unified Hook/Event Model (`unified-hooks.md`)
                   - the supported Actions that can be attached to an hook (`hook-actions.md`)
                   - the Linking Strategy (`linking-strategy.md`)
                   - the Logging Strategy (`logging-strategy.md`)
               - each topic area should be a markdown link to a more detailed document
           - `## Claudine CLI`
               - the @claudine/README.md and @claudine/cli/README.md should be read to provide a 1-2 paragraph overview of the Claudine CLI
               - following the prose overview, we should include a table of "subcommands" which the CLI exposes along with descriptions of what each
           - `## Research on Agentic CLI Platforms`
               - In this section we will have three sub-sections:
                   - `### Hooks Research`
                       - description: 'Research into each Agentic CLI's provided hooks, payloads and return types'
                       - link: 'research/hooks/overview.md'
                   - `### Cross-linking Research`
                       - description: 'Research into each Agentic CLI's support for features like agentic skills, slash commands, agents/subagents, and shared scripts folders'
                       - link: 'research/cross-referencing/overview.md'
                   - `### CLI Research`
                       - description: 'Research into the subcommands and switches each Agentic CLI platform provides as well as providing insight into the various means of executing this platform in a non-interactive session, choosing which model to use, and more.'
                       - link: 'research/cli/overview.md'
               - Each of these sections will contain a brief summary of the type of research conducted in markdown list with markdown links to an `overview.md`
     2. Supported Platforms Subagent
           - this subagent is responsible for populating the file @.claude/skills/claudine/supported-platforms.md
           - this document should be created by piping the output of `claudine providers` to this this file
     3. Hook/Event Model Subagent
           -
     4. Supported Actions Subagent
           - a
     5. Linking Strategy Subagent
           - a
     6. Logging Strategy Subagent
           - a
     7. Research Overview Subagent
           - a
- once all subagents have completed their work the skill is now fully updated;
    - communicate that we're done to the user
