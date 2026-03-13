# Frontmatter Prompts

We are going to add a new switch for all wrapped execution commands in claudine.

- `--frontmatter-prompt <file>` or `--fp <file>` is the CLI switch
- the goal for this switch is to read an LLM prompt from the `prompt` property of the frontmatter of the referenced file and then send that prompt to a agent as a non-interactive prompt
    - the response of this interactive prompt will replace the BODY of the of the reference file.
    - the `last_updated` frontmatter property will be created/updated to be the current date (`YYYY-MM-DD`)
    - all other frontmatter properties will be left unchanged
- Darkmatter parsing:
    - the prompt defined in the `prompt` property will be treated as Markdown content and passed to the Darkmatter library's **compose** functionality which will mutate the content based on the Markdown pipelining process that Darkmatter uses.
    - once the Darkmatter's compose operation has completed we will then kick off a non-interactive prompt on an Agent
- Agent Selection:
    - Default Behavior
        - by default the agent we will use for the prompt will be determined by the repo's (falling back to user's) configuration of "favorite" agents:
            - the most favorite agent will be tried but if it fails it will automatically retry with the second favorite agent
            - if the second favorite agent fails too then we will return an error
    - Override Behavior with the `agent` Property
        - ENV Override:
            - in any case where the AGENT environment variable matches one or more valid supported agents we will replace the frontmatter's `agent` property with what is defined in `AGENT`
        - Singular Match:
            - if the reference file that this command was passed has a frontmatter property called "agent" then we will use it to help us determine the agent to use
            - if the lowercased version of the `agent` property is a string subset of one and only one of our "supported CLI's" then we will use that as the preferred agent to use:
                - if the current host system does not have this agent installed we will report an error:
                - `<red><b>ERROR:</b></red> The agentic platform "{agent}" -- <i>which the referenced file specifies as the preferred agent</i> -- is not installed on this computer!\n\nInstall the agent software or if you want to <b>override</b> the agent to be used you can set the environment variable AGENT to the agent you prefer (e.g., <blue>AGENT=claude claudine --frontmatter-prompt {file}</blue>)`
        - Multi Match:
            - if the lowercased version of the `agent` property is a string subset of _more than one_ of our "supported CLI's" then we will present an interactive select dialog to force them to choose explicitly
        - Interactive Match:
            - if the `agent` property is **true** or 'interactive' then we will simply present all the _installed_ agents on the current host (if no agents are installed then error)

## LSP Hints

- FUTURE: we do not yet have an LSP for Darkmatter or Claudine but when we do we will have it enumerate the `agent` frontmatter property to have valid agentic platforms or the values `true` and `interactive`:
    - both `true` and `interactive` will result in an interactive prompt being shown to allow the user to choose the agent they would like to use.


