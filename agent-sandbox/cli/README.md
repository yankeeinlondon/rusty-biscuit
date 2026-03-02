# Sandbox CLI

The `sandbox` CLI provides convenient ways for:

1. The host to boot-up and interact with containers
2. Ways for the containers to communicate back to the host

## Subcommands for Host

- `start <name> <...params[]>`
    - starts a container using the Docker runtime
    - the "name" is optional and if not specified will allow Docker to name for us
    - 
