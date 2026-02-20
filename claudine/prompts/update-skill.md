# Update Skill

To be time efficient and context-window efficient you will run as an orchestrator.

- create an empty file at @.ai/logs/claudine-skill-update.md
    - This file will be used to log:
        - Fixed identified drift (`- Fixed:`)
        - Possible issues that were not fixed (`- Issue:`)
- you will initially kick off two agents concurrently:
    1. Library Drift
       - Look at the @claudine/lib/README.md file and make sure it is is in sync with the actual Claudine library source code in @claudine/lib/src
       - If drift between doc and source code is detected, update the README.md file to ensure it is consistent with the source code
           - if updates were needed append the line `- Fixed: Library README.md drift` to @.ai/logs/claudine-skill-update.md
    2. CLI Drift
        - Look at the @claudine/cli/README.md file and make sure it is is in sync with the actual Claudine CLI source code in @claudine/cli/src
        - Update the README.md file where to ensure it is consistent with what has actually been implemented
            - if updates were needed append the line `- Fixed: Claudine CLI README.md drift` to @.ai/logs/claudine-skill-update.md
- once completed you will kick off a single subagent to:
    - make sure that the @claudine/README.md has no conflicting information with that found in:
        - @claudine/lib/README.md
        - @claudine/cli/README.md
    - if there are any conflicts then assume the more detailed documents to be correct and update @claudine/README.md
      - if updates were needed append the line `- Fixed: Claudine base README.md drift` to @.ai/logs/claudine-skill-update.md
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
                       - links: add links to each document in the @.claude/skills/claudine/research/hooks directory
                   - `### Cross-referencing Research`
                       - description: 'Research into each Agentic CLI's support for features like agentic skills, slash commands, agents/subagents, and shared scripts folders'
                       - links: add links to each document in the @.claude/skills/claudine/research/cross-referencing directory
                   - `### CLI Research`
                       - description: 'Research into the subcommands and switches each Agentic CLI platform provides as well as providing insight into the various means of executing this platform in a non-interactive session, choosing which model to use, and more.'
                       - links: add links to each document in the @.claude/skills/claudine/research/cli directory
     2. Supported Platforms Subagent
           - this subagent is responsible for populating the file @.claude/skills/claudine/supported-platforms.md
           - this document should be created by piping the output of `claudine providers` to this this file
           - Check if any of the documentation in @claudine/lib/README.md conflicts with the documentation you've just written
               - If it does, then:
                   - Update the @claudine/lib/README.md to reflect the more precise understanding of "supported platforms" in your documentation
                   - append the line `- Fixed: Library README.md drift based on supported platform inconsistencies` to @.ai/logs/claudine-skill-update.md
     3. Hook/Event Subagent
           - you are responsible for creating the documentation file @.claude/skill/claudine/unified-hooks.md
           - add a section `## Event Mapping to Providers` and then pipe the output of `claudine hooks --mapping` into this section
           - add a section `## Event Support by Provider` and then pipe the output of `claudine hooks --support` into this section
           - review the code in the "events" module of the Claudine library: @claudine/lib/src/events
           - document any important structs, enums, or traits which are defined
           - describe how these symbols help to achieve the functional goals of event/hook processing
           - save all your changes to @.claude/skill/claudine/unified-hooks.md
     4. Supported Actions Subagent
           - focus your attention on the `actions` module in the Claudine library
           - list out all the supported actions a user can use to respond to an event
               - list the event name, describe what it does, and provide the calling signature as well as the return payload (if any)
           - add a section `## Context Variables`
               - add the static text "Context variables are available for you to use in all string parameters of the actions you configure.\n"
               - run `claudine hooks --variables` and pipe the results into this section
           - Check if any of the documentation in @claudine/lib/README.md conflicts with the documentation you've just written
               - If it does, then:
                   - Update the @claudine/lib/README.md to reflect the more precise understanding of "supported actions" in your documentation
                   - append the line `- Fixed: Library README.md drift based on supported action inconsistencies` to @.ai/logs/claudine-skill-update.md
     5. Linking Strategy Subagent
           - Review the Claudine source code and focus on the linking module
           - Once you have a good overview of the code here, provide some summary documentation
           - The documentation should lead with business logic/rules and functional goals but can also include references to key struct's or enum's
           - Save the documentation to @.claude/skills/claudine/linking-strategy.md
           - Then check if any of the documentation in @claudine/lib/README.md conflicts with the documentation you've just written
               - If it does, then:
                   - Update the @claudine/lib/README.md to reflect the more precise understanding of "linking" functionality in your documentation
                   - append the line `- Fixed: Library README.md drift based on Linking inconsistencies` to @.ai/logs/claudine-skill-update.md
- once all subagents have completed their work the skill is now fully updated;
    - communicate that we're done to the user
    - print the result of the @.ai/logs/claudine-skill-update.md file for the user's review
    - remove the @.ai/logs/claudine-skill-update.md file
