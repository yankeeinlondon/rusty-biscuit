# CLI Interaction

## What is CLI Interaction

- Agentic CLI's provide a lot of useful command line switches and parameters that we should be aware of.
- We will source this information from website documentation as well as via the terminal directly if the host system has the CLI installed.


## Your Task
Review each document in @claudine/docs/agent-cli/ for correctness and completeness.

You will need to do research on the given Agent CLI's website to make sure that you are completely current on the Agentic CLI's parameters and switches before performing this task.

In order to be efficient with context windows as well as be time efficient you should act as an orchestrator and have each document perform this operation on a per file basis (which maps to also being on a per Agentic CLI basis). Each subagent which you give responsibility to you will:

- tell the agent/subagent which document they are responsible for (full filepath)
- tell them to do an overview read of the document first so they have an understanding of which Agentic CLI they are responsible for as well as some hints on the most useful URLs to check for documentation
    - Note: this document might be blank if this is the first time running this or the particular Agent is new
- then tell them to do research online on the given Agentic CLI's documentation to make sure we have a complete picture of the CLI's parameters and switches:
    - if the host computer has the CLI installed then we should run `{CLI BINARY} --help` (or comparable) to validate that the documentation we've read is aligned with the installed
- they should then update the document they are responsible for and summarize the changes they made when reporting back to you (the orchestrator)

Things you should always make sure the document contains are:

1. **Frontmatter:**
      - `homepage` should be a URL to the Agentic CLI's home page
      - `docs` should be a URL to the Agentic CLI's documentation page
      - `cli_docs` should be a URL to the Agentic CLI's command line parameters and switches
2. **Body Content:**
      - Ensure there is a section `## Model Specification`
          - Most Agent CLI's will allow you specify the "default model" that should be used explicitly by using using CLI parameters/switches; specify how this is done with the specific Agent CLI you are focused on and any limitations or unexpected behaviors this Agent might have in it's ability to specify this
          - When the Agent starts up it will need to have a "default model" that it uses and if this wasn't specified explicitly in the CLI's invocation then we need to express what this Agent CLI uses as business logic to determine this. Can it be specified in the in a configuration file? Multiple config files? Is it just whatever was last used when running the Agent CLI?
      - Ensure there is a section `## Non-interactive Engagement`
          - Most (probably **all**) Agent CLI's will not only allow you to enter into an interactive session with them but also to send them a task to complete without allowing for interactive "human in the loop" engagement. Having this functionality allows for callers to organize agents into more complex orchestrations with the confidence that the Agent will stall out waiting for user input.
          - In this section you need to indicate if this non-interactive mode is supported
          - Then, assuming it supported at least in some fashion, you must enumerate all the _ways_ you can call/execute the agent in this non-interactive manner. For each method indicate specifically how you do it, what benefits and limitations this approach comes with
      - Ensure there is a section `## Subscription versus Per Call API`
          - Most Agent CLI's will allow usage to use a user's subscription versus using a per call API pricing method.
          - This section is meant to specify exactly how you start a **non-interactive** session in subscription or per-call API pricing modes for this particular Agent CLI
              - if you don't know and can't find the answer online then specify this; DO NOT GUESS
      - Ensure there ia a section `## System Prompt`
          - Some CLI Agent's will allow you to overwrite or modify in some way the `system prompt`
          - If the CLI you are focusing on allows for this specify that it does and how you do it
          - Also be clear whether this a "supplement" to the normal system prompt or a full replacement
      - Ensure there is a section `## Permissions`
          - How are permissions for this Agent CLI setup by default? Specify the config files and properties used.
          - How does the CLI let you modify the default permissions?
          - Does the Agent support a "yolo" mode? What is the CLI switch used to put it into `yolo` mode?
      - Ensure there ia a section `## Thinking Level`
          - Does the Agent CLI allow specifying a particular _level_ of thinking for the agentic task? What CLI switches are used?
          - What configuration files influence the default thinking level if the CLI doesn't explicitly state the thinking level?
          - What levels does it support?
      - Ensure there is a section `## Logging`
          - An Agent CLI is likely to produce traces or logs as it's running that it saves to the file system
          - This is often placed somewhere underneath the User's (or possibly the repo's) configuration base directory
          - You should specify what types traces and logs this repo produces
              - specify the structure of the file where you can
              - it is much better to say you don't know (after having researched online) than to make things up!
      - Ensure there is a section `## CLI Options`
          - The goal here is to have two Markdown tables:
              - Table 1: Subcommands
                  - all subcommands which the CLI provides
                  - each subcommand should have the following columns:
                      - parameter(s)
                      - description
              - Table 2: Switches
                  - all switches listed by the CLI's help system
                  - columns are:
                      - switch (including a shortcut alias if present)
                      - description
      - Ensure there is a section `## Sources`
          - This section should be a Markdown list
          - Each entry in the list should be a Markdown list to useful resource covering this Agent CLI or topics related to it.
