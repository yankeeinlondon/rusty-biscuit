# Claudine Logging

The central actor in the **Claudine** logging solution is the **Rendezvous** service. The Claudine CLI is an active client, calling the Rendezvous's API when:

- the _start_ and _stop_ of any wrapped execution
- all _event hooks_ that the provider CLI exposes
- the CLI will also leverage the Query API whenever a CLI user uses one of the `claudine log ...` commands

We thought it would be worth starting by clarifying the two Claudine _executables_ and their roles in logging process but while the Claudine CLI _does_ provide logging data to the Rendezvous daemon it is by no means the only source.

## Logging Sources

### Claudine CLI

- Claudine CLI will provide START/STOP log messages for Agent CLI sessions which wrap the agent's own conception of start/stop
    - We might consider having this live all in one document / record with the Claudine information just enhancing what the agent is giving us
    - In either case, we will always get the Claudine CLI's messages -- via the gRPC call -- first
- Claudine CLI will provide Hook Event logs
    - this has a similar relationship to the Agent's own log record
- Claudine CLI should capture the PID for not only the Claudine process but also the child process for the Agent CLI; this should be streamed to the daemon whenever a process starts/stops
    - should capture the repo name, the git cloud provider (Github, Gitlab, etc.), and remote URL for the repo as part of this event

### Process Monitoring

- we plan on having the daemon check the host's processes on an interval for:
    - Claudine wrapped sessions
    - CLI or Application direct sessions
- we want to extract as much information as we can from the processes
    - in particular this is important to monitor if a user is running Agent Apps (not CLI) or Agent CLI's that are not wrapped by Claudine
- this information is primarily used for TODAY/NOW information
- it will help to answer "what sessions are currently active? what app or CLI is running these sessions? is the session active, complete, or idle (in an interactive session?)

### Repo Commits

- a user 
