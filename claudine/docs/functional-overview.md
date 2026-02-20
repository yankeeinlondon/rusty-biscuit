# Claudine: Functional Overview

Claudine interacts with your Agentic CLI's in two distinct ways:

1. **Hooks**

    - Hooks provide a way to _hook into_ the execution flow of your Agentic CLI
        - They setup a set of **events** and you may optionally choose to have the Agent call programs when this event is fired
        - These events broadly come in two distinct forms:
            - **Informational**
                - many of the events are fired and provide _informational_ context but do not allow the Agent's overall flow to be changed in any way
            - **Flow Interaction**
                - some events allow the _return type_ of your "hooks" to determine how to proceed
                - as a result, these events will pause to wait to hear from you before they proceed
                - most, if not all, Agent's provide one or more hooks per event which means that "order matters"; how the Agent will respond when multiple hooks are added to one of these events is not always very well documented. For this reason, Claudine will always try to make sure it is the FIRST hook registered for each exposed event.
    - **Actions**
        - We use the term "actions" in Claudine to specify a set (`0:M`) of actions which Claudine should perform when a specific event is fired.
        - The _kinds_ of actions you can take are:
            - **Built-in Actions:**
                - We provide a set of easily configured actions which you can choose from to attach to any hook event

                    | Action      | Returns | Description |
                    | ------      | ------- | ----------- |
                    | SoundEffect(Effect) | void | Plays a sound effect from small library of effects (from the Playa library) |
                    | Speak(String)  | void    | Uses TTS to communicate a message to host's audio device |
                    | Log            | void    | Logs the event to a JSONL event stream |
                    | Output(String) | void    | Provides a message to the STDOUT of the terminal's


            - **Bespoke Actions:**
    - **Initialization and Event Abstraction**
        - Initialization
            - When you first initialize Claudine with `claudine init` it will immediately subscribe to ALL _events_ on ALL installed Agentic CLIs's
            - It will not only subscribe to all events but ensure that Claudine's events are subscribed to FIRST in the cases where other non-claudine events are also subscribed
        - Event Abstraction
            - The hook event names which each Agentic CLI provides are not always the same. Sometimes two Agents will have in essence the "same event" but use different names, in other cases they will have an event that is quite distinct and not like the others.
            - Claudine will use it's knowledge of these various Agents and Agentic Hooks to provide a "unified" set of event names.
            - When you are configuring **actions** to an event hook you will be using the "unified" set of event names to attach your actions to.
            - For details on the mapping you should read the: [Unified Events](./unified-events.md) document
            - You can also call the `claudine hooks --mapping` from the CLI to get the overview
    - **Status**
        - You can run `claudine hooks` to get a basic overview of the current status for your current configuration
        - The basic information tells you which of the supported Agentic CLI's that Claudine supports is _installed_ on your computer and which "events" you have defined **actions** assigned to
        - **Note:** if a Agentic CLI is not installed then we will NOT attempt to add any hooks configuration until such as time as you do install it
        -


    Hooks are not always provided on all Agentic CLI's nor do they provide a completely uniform set of _events_ which provide a hook.

2. **Linking**
