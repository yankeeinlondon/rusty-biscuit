# Logging Refactor

We have had logging for a long time in Claudine but it was never implemented well and hasn't been used in a long time in it's current form. In more recent times we've had a few important feature developments:

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

![architecture](architecture.excalidraw.svg)

- in terms of **state**, the Rendezvous daemon will maintain state in two separate databases:

    1. **redb** - transactional system of record (kv store)
    2. **duckdb** - analytical/reporting database

- all -- _or at least most_ -- of the data stored in **redb** will be CRDT documents
- the CRDT documents will then be be pushed into **duckdb** for reporting purposes

## Reporting Requirements

We keep track of logs so we can report on them to the user in a way that provides utility. The Rendezvous daemon and Log Monitor client are both long running processes who's job it is to capture log information and store it in a way which can be easily queried as well as synchronized across other Rendezvous daemons on other hosts.

Major entities we want to track:

- Sessions (this is the primary unit of execution for Agentic CLI and comes in both interactive and non-interactive forms)
- Sequences (this is a Claudine concept but plays a very important _grouping_ function across otherwise separate non-interactive sessions)
- Agent (understanding activity by Agent and reflecting it back to the user is helpful in understanding how much a user is using one agent versus another, whether they are favoring interactive or non-interactive sessions on that platform, and much more)
- Model (understanding which models are being used )
- Repo (understanding which repos a user is working on is useful in the short term for situational awareness but spanned over time it also tells an interesting story about the user's focus over time)
- Commit
- PR 
- CI/CD
- Project
    - i see this one as a more v2 entity but the idea is that we _can_ interact with all of the project management / kanban boards that are out there; then for any given repo we can associate a kanban board
    - we already have API support for some of the most significant project management platforms (trello, clickup, asana, etc.)
    - we would want to add support for github, gitlab, gitea(?)
    - the movement of items on a kanban board can have strong correlations to commits, PRs, etc.
    - being able to add in this information to Repos, PRs,
