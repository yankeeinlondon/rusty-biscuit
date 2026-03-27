# Compose Refactor

## Problem

We originally started out by having two CLI commands which we wanted to perform largely the same task:

- `claudine compose <file-ref> ...`
- and `claudine <agent> --compose <file-ref> ...

the only difference was meant to be that the `claudine compose <file-ref> ...` signature meant that we were deferring the decision on which agent provider we'd use until runtime. Unfortunately it appears that these two signature have wildly diverged.

- Read the [compose drift](./drift.md) document for the current DRIFT that exists between these two commands currently!

This problem exacerbated because:

- `claudine <agent> --compose <file-ref>`
- is _very similar_ to `claudine <agent> --prompt-file <file-ref>`

The current situation is untenable and has a lot of unnecessary complexity so we need to refactor.

## Solution

### Simplified Signatures

We have two types of composition:

1. **Direct Composition**

    ```mermaid
    flowchart LR

    CLI["compose &lt;file&gt;"]
    Resolve([Resolve File])
    Compose(_compose_ Markdown)
    InvalidFile(Error)
    Prompt(Prompt Agent)
    FailedCompose(Error)


    CLI --> Resolve -->|resolved| Compose
    Resolve -->|invalid| InvalidFile

    Compose --> Prompt
    Compose -->|failure| FailedCompose
    ```

    The **direct** form of composition is about using Darkmatter's DSL to mutate the referenced file and then using this _composed_ content as the prompt for an Agent. The _composed prompt_ can direct the Agent to save or modify files but the direct flow does not involve or concern claudine's **compose** functionality.

2. Inline Composition

     ```mermaid
    flowchart LR

    CLI["inline-compose &lt;file&gt;"]
    Resolve([Resolve File])
    Compose(["_compose_ **prompt** prop"])
    InvalidFile(Error)
    HasPrompt(["Has **prompt** prop"])
    NoPrompt(Error)

    MutateFile@{label: "File", shape: "doc" }

    Agent(Agent)

    CLI --> Resolve -->|resolved| HasPrompt
    Resolve -->|invalid| InvalidFile

    Compose -->|prompt| Agent --> MutateFile

    HasPrompt -->|true| Compose
    HasPrompt -->|false| NoPrompt
    ```

    The **inline** form of composition's primary goal is to **mutate** the BODY of the file being referenced. This is done by:
        
    - finding an agent prompt in the `prompt` frontmatter property, 
    - _composing_ the `prompt` with the Darkmatter library
    - appending instructions to reinforce that the Agent is to modify the referenced file's _body_
    - passing the finalized prompt to the Agent with the expectation that they will update file's body
    - claudine checks:
        - that the body's been changed
        - if any frontmatter properties were mutated from the start state they are reverted (though new properties should be allowed)
    - claudine sets the `last_updated` property to today's date


#### Direct Composition

- we will retire the `claudine <agent> --compose <file-ref>` signature
- we will retire the `claudine <agent> --prompt-file <file-ref>` signature
- the ONLY signature for direct composition will be `claudine compose <file-ref>` 
- We will add the following CLI switches to `claudine compose` which will choose the Agent eagerly:
    - `--claude`
    - `--codex`
    - `--gemini`
    - `--qwen`
    - `--opencode`
    - etc.
- If none of the eager switches (above) is used then the choice of the agent will be interactive at "runtime"
    - The user will be interactively asked to choose an agent from those they have installed
        - the default choice from the "select" input will be the agent they've marked as their favorite in the config
    - If the user only has one agent installed then we can skip the interactive dialog and just use the one they have

#### Inline Composition Signature

- we will retire the `claudine <agent> --frontmatter-prop <file-ref>` signature
- the ONLY signature for inline composition will be `claudine inline-compose <file-ref>` signature
- the same CLI switches used in direct composition for specifying a particular agent (e.g., `--claude`, `--codex`, etc.) are available for inline composition too

#### Both Inline and Direct

- both forms of composition -- `compose` and `inline-compose` -- default to being non-interactive prompts but can be toggled to be interactive with `--interactive` / `-i`
- all prompts which are run as a non-interactive prompt (_this even includes non-compose prompts_) will stream JSON responses. This will enable:
    - capture of session ID
    - capture of other useful metadata like model, tokens, etc.
    - we can prep the output by running it through the Darkmatter renderer for the Terminal; this gives us:
        - Bold, dim, and italics text instead of Markdown aliases for these stylistic effects
        - Table rendering
        - Word wrapping
        - etc.
- all non-interactive prompts will support [resume]() functionality

### Execution Pipelines, Validations, and Handlers

```mermaid
flowchart LR

Prep(Prep) --> Pipeline(Prompt Processing) --> Closure(Closure)
```

- we want the _prompt processing_ of both **inline** and **direct** compositions to use **THE SAME** logic and processing
- the **inline** form of compositions will:
    - Prep
        - Will add validations to the **Prep** period
            - file reference exists
            - `prompt` property exists in frontmatter
            - user has write access to reference file (by Agent's permissions as well as filesystem permissions)
        - Those validations must pass _before_ the prompt processing begins
        - The content passed into the _prompt processing_ will be the **composed** variant of the `prompt` property
    - Closure
        - will add validations to take during the **Closure** process as well (body was updated, body not empty)
- the default form of a **direct** composition is very basic:
    - Prep
        - Validate the file reference can be resolved to a valid Markdown file
        - Then _compose_ the Markdown file's content and pass into the _prompt processing_
    - Closure
        - _nothing_

The base prep, prompt processing, and closure steps as defined above provide the baseline before we introduce user/document defined **validations** and **handlers**.


#### Validations and Handlers

We recently introduced **Validation** and **Handlers**, with the compose pipeline in Claudine now being broken out in the `Prep` -> `Prompt Processing` -> `Closure` stages we can comfortably fit this functionality into the `Prep` and `Closure` stages of the process. Furthermore, some of the "built-in" validations which we were doing -- like "does document have `prompt` property" -- should be able to be articulated as validations and use the same execution flow as user/document defined validations do.

The intention is to allow all validations and handlers to be used in both **inline** or **direct** compositions but there may be a few which only make sense for one type or the other.

### Resume Functionality

We introduced **resume** as a type of _handler_ which is able to "resume" the context window of the failed prompt and ask a follow on question. This is a very handy feature by itself but it also highlights that whenever Claudine is running non-interactively, we will have captured the session ID and that there is likely value in creating a new subcommand:

- `claudine resume "follow up question"`
- running this command will allow for easy follow-up questions to any non-interactive command
- to make the UX/DX good we will need to create a scratch file for each terminal window where we can keep the _last_ session id
    - The "last" session id is much more powerful if bounded by the terminal window the user is in
    - Beyond the session id we should also capture a timestamp and the agent used
        - the scratch file should keep only the latest (per agent)
    - Having a timestamp will allow for us to start confirming with the user if they really want to do this when the "last session" was more than 3 hours ago. We can present the question as: `The last session <i>in this terminal window</i> used the <b>{agent}</b> agent {#} hours ago. Are you sure you want to resume the session "{session-id}" (Y/n)?`
- in addition we should allow these switches from `claudine resume`:
    - `--list` 
        - lists session id's which are "resumable"
        - 10 sessions are presented
        - the latest session id(s) for that terminal will be highlighted in a different color to help them stand out
        - the other session id's are the 10 most recent across all agents and terminal windows
        - list is sorted from latest to oldest
    - `--id <session>`
        - allows the user to specify the session they want to resume from rather than just defaulting to the most recent

## Final Notes

- There are no active users yet so no need to deprecate any features
- Take the time to do a high quality job; quality is more important than speed
