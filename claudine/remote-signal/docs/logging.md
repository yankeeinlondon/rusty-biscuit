# Claudine Logging

The central actor in the **Claudine** logging solution is the **Remote Signal** service. The Claudine CLI is an active client, calling the Remote Signal's API when:

- the _start_ and _stop_ of any wrapped execution
- all _event hooks_ that the provider CLI exposes
- the CLI will also leverage the Query API whenever a CLI user uses one of the `claudine log ...` commands

We thought it would be worth starting by clarifying the two Claudine _executables_ and their roles in logging process but while the Claudine CLI _does_ provide logging data to the Remote Signal daemon it is by no means the only source.

## Logging Sources

