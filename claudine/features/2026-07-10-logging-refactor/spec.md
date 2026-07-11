# Logging Refactor

We have had logging for a long time but it was never implemented well and hasn't been used in a long time in it's current form. In more recent times we've had a few very important developments:

1. Provider Research -> Metadata

    - we have a much more structured and thorough process for managing intelligence about the Agentic CLI's we support
    - this intelligence regarding logging formatting from each of these vendors

2. Rendezvous

    - We have a basic implementation of Rendezvous implemented
    - Rendezvous is a daemon process that Claudine interacts with
    - Logging will be largely owned by Rendezvous (and a monitor client which streams updates to the Rendezvous daemon via gRPC)


> **Important:** this is a major refactor of logging and the current solution is being fully replaced. If there is anything that we feel is worth bringing over to the new implementation that is fine but do not be constrained by the current way of doing things.

## High Level Architecture

- The three actors are:
    - **Rendezvous** daemon
    - **Claudine** CLI client
    - **Logging Monitor** client
- When **Rendezvous** starts up it will spawn a **Logging Monitor** to monitor the various agent logs
    - note: we could also have a monitor _per_ agent installed on the host
    - note: also possible that we have multiple variants of the logging monitor:
        - one that monitors files
        - another one which monitors a Database
