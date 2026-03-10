# Repo Isolation

When we use the `--repo` CLI switch with Claudine's Agent _wrapping_ functionality we aim to create a more focused environment for your Agent to work in. This focus is achieved by masking the "user scope" to a large degree (but not fully):

## Masked Elements

When in the repo isolation mode we will **mask** the following resources:

- User skills
- User slash commands
- User agent definitions
- User MCP tools

Now it's true that the "repo scope" for most (if not all) Agentic CLI's already will prioritize the repo scope when two resources have the same name, there are are times where masking out the user resources is helpful:

- users in an interactive session can be assured that all slash commands are defined for that repo and are not "general purpose"
- **less is more** for the context window when it comes to skills:
    - yes the idea of "progressive disclosure" is used in skills but every skill's name and description (from the Frontmatter) must be brought into the context window so that each skill can be considered during the course of the session.
        - if you have hundreds skills stored away from a rainy day in your user scope this is NOT insignificant and undesirable 
    - furthermore when an Agent is considering which skills to use for a particular problem having _more_ to choose from can lead to a suboptimal choice
        - you might choose a User skill which is less specific or pertinent than what the Agent would have chosen if 
    - as a general "rule of thumb" an ideal number of skills for Agent is "less than 40 but ideally less than 20"
- **sharing is caring**
    - another reason to isolate is that it forces developers to consider "what does THIS repo need"
    - by building this into the repo scope that means _every_ developer working on this repo will benefit from the same focused skills, commands, and agents
    - this provides value to each person working on this repo while also providing consistency
    - the consistency aspect is often the most important aspect as skills can have stated preferences on tech stack, design aesthetic and many other things which really should only be expressed on a repo by repo basis.
- **MCP**
    - similar to the bloat that too many skills can add to the context window, MCP services which don't directly address things the REPO has deemed valuable should be consider wasteful
    - in fact, MCP skills tend to be far more wasteful then skills, in terms of the context window
    - **Note:** this isolation is _independent_ to the [MCP mode](./mcp-mode.md) feature which Claudine also provides; use one, use both, ... each configuration is valid.

### What we do **not** Mask

If a user's profile states any of the following, every attempt is made to preserve this:

- Color mode (e.g., light/dark mode)
- Subscription authentication info
    - really _any_ authentication but it's primarily subscription info which lives outside of the simple API Key approach to authentication
- TODO: are there more things we don't want to mask?

## Technical Approach to Masking

TODO: What is our current approach to providing this masking?

### Preserved Authentication is King

Our aim is to produce an identical environment for the Agent to work in but masked from user skills, agents, and slash commands. 


### Agent Exceptions


