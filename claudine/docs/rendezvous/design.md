---
parent_feature: 2026-05-24-remote-signal
---

## Actors and Interaction

The two modes of _interaction_ that take place with the **rendezvous** daemon are:

1. **gRPC** API

    - communicates locally on host system 
    - uses gRPC messaging over a Unix domain **sockets** (macOs, Linux) and **Named Pipes** (windows)
    - provides a strongly typed RPC protocol so that
        - Claudine can communicate with **rendezvous** 
        - any given host is likely to have many Claudine sessions running concurrently
        - in the future, other clients will be able to communicate locally using this channel

2. **CRDT** Syncing over Mesh

    - **rendezvous** daemons can communicate to other **rendezvous** daemons (on other hosts)
    - this is done over a mesh network:
        - TODO
    - The mechanism of ensuring "eventual consistency" across all daemons is achieved through CRDT Delta syncing

## gRPC API Surface
