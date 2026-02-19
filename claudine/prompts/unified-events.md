# Unified Events

## Functional Outcome

A detailed design document written to @claudine/docs/unified-events.md which:

- defines types for each Agentic CLI's Hooks/Event System
- defines a "unified" set of types that can comfortably span across all supported Agentic CLI's Hooks implementations

## Task

To this in a time sensitive as well as context-window sensitive manner you should setup as an orchestrator, you will first kickoff a subagent for Claude Code (which will as our baseline). The details for Claude code are found at @claudine/docs/hooks/claude-code.md .

- you tell the Claude Code subagent to follow the instruction in `### Subagent Design` section
- when the subagent for Claude Code concludes you'll have the _design suggestion_ for Claude Code
- now you will kick off subagents for every file
> Note: each file represents a detailed knowledge about Hooks for a specific Agent CLI

Each subagent will be told do the following:

- read in the file from the filename passed in (this will provide detailed info on a particular )
- follow the instructions in `### Subagent Design`

Once each subagent has provided back their design, you will pass all designs onto the "Finalization Agent":

- the Finalization Agent will be passed all individual designs generated so far
- and instructed to do what's in `### Subagent Finalization`

The final step is then to create a subagent to review the design document at @claudine/docs/unified-events.md :

- the Review Agent will be passed the file path to the design document
- and asked to follow the instructions defined in `### Subagent Review`

Once the review is complete you are done. You should tell the user to look at the result in @claudine/docs/unified-events.md

### Subagent Design

- If you are evaluating Claude Code then you can skip the sub-bullets here but all other Agent CLI's need to follow:
    - You will be passed a design that a developer looking at Claude Code came up with
    - You should review this and understand how it's structured before proceeding to the next step
    - The way this developer designed for Claude Code will almost surely NOT match the
- You are responsible for providing a Rust enum for all events/hooks supported by the Agent CLI, as well as any other supporting types that you think would be an appropriate way to capture all the information needed to capture all metadata for the Hooks/Events for the CLI Agent you are focusing on.
    - Each variant of the enum represents a particular hook/event that is supported by the CLI Agent
    - We will likely need a lookup table of some sort so that each variant can lookup information like:
        - `description` - a `&'static str' description of this event; this NOT an attempt create a description that might work multiple Agent CLI's this is a description for the specific CLI you are focused on
        - `event_payload` - what type of information does this hook provide to subscribers of the hook?
        - `response` - what kind of response (message body and terminal return code) can this event produce?
    - You should define a set of functions which you think would be useful and ergonomic to callers of this enum
    - What other ergonomic features should be added? What types can/should be implemented for `From` trait? `TryFrom`?
    - This design suggestion should be provided as Rust code and encapsulated inside a markdown Rust code block.
- Now add a few bullet points on major design considerations
- If you are NOT evaluating Claude Code then:
    - Iterate over each event you've defined and describe:
        - is this a 1:1 mapping to a Claude Code event?
        - is the triggering event the SAME as as Claude Code event but either or both the event_payload or return types vary?
        - is this a distinct event from anything defined in Claude Code?

You must return to the orchestrator who called you:

- the Rust code block representing your design
- bullet points describing major design considerations
- a markdown unordered list describing how each event supported in this Agentic CLI is similar or varies from those provided by Claude Code (this section will be skipped if you were reviewing Claude Code itself)

### Subagent Finalization

- you will have been provided a design specification from each of the Agentic CLI's we support
- your task is to review these designs, noticing variances and similarities across the various designs
- use this insight to drive a conclusive and finalized design for each Agentic CLI's design requirements
- With the individual designs completed, write these individual design components to the @claudine/docs/unified-events.md
- Now you must now turn toward designing a "unified solution"
    - At the center of the unified solution will be an enum `UnifiedHook` who's variants represent the hooks we will support in Claudine
        - The variants in `UnifiedHook` must have a clear mapping to an individual Agentic CLI's supported events
        - We will need to shape incoming "event payloads" to move from a proprietary format to the unified format
        - Actions:
            - We will need to define how to structure the actions a user can take which leverages a enum called **HookAction**
            - The actions we will support must include:
                - `HookAction::SoundEffect(Effect)` - this will use the sound effects provided by the Playa Library (in this monorepo)
                    - use the `playa` skill for details
                - `HookAction::Say(String)`
                    - this will use the `biscuit-speaks` library (in this monorepo) to convert the passed in string to an audible voice on the host using TTS
                      - use the `biscuit-speaks` skill for details
                    - The passed in string should be treated as a "template" which is allowed to have Handlebar based variable in it. The variables provided is covered in the `### Context Variables` section.
                      - References like `{{ env.MESSAGE }}` will be replaced the content in environment variable `MESSAGE`
                      - References like `{{ env.MESSAGE | "unknown" }}` will be replaced by the environment variable `MESSAGE` if it's defined and otherwise with the text `unknown`.
                - `HookAction::Log(Option<String>)` - this will append events of given type to a JSONL log (you may optionally override the default path for the logging); log files should always be for a specific day (using the hosts timezone to determine day boundaries)
                - `HookAction::FireAndForget(String, Option<Vec<String>>)` - this will run a command (optionally with parameters passed). The command will be executed but the result of that execution will have no effect on the return type provided. If the command results in an error, however, this will be logged to the log stream.
                - `HookAction::Call(String, Option<Vec<String>>, Option<Mapper>)`
                    - this will run a command in a blocking fashion
                    - and use the return of the function to respond to the Agent
                    - you may also provide a `Mapper` which is a callback function that takes the program's return response and maps it to the valid universal return type (it may then need to be converted again to the proprietary return type that the calling Agent expects)
            - All actions will be handled in the "unified scope" first but all return values must be returned back to the Agent in the format it expects

### Subagent Review

- you must use the `rust`, `biscuit-speaks`, and `playa` skills
- the design you have received includes design details for each of the Agentic CLI's we support as well as a "unified design" that allows Claudine to operated in an abstracted manner that can be largely vendor neutral in approach.
- your task is to review the design in two passes:
    - Pass 1: Idiomatic Rust and Ergonomics
        - you will look over all design elements looking for ways in which the code blocks could be more ergonomic
        - you will make sure
