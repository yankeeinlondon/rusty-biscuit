# Remote Signal

## Service Overview

The **Remote Signal** daemon process is meant to compliment the Claudine CLI by providing a set of long running services:

1. Logging
2. Session Tracking
3. Scheduling & Queuing
4. Dreaming (for Memory Files)
5. Remote Execution & Interaction

## High Level Architecture

```mermaid

```

### Tech Stack

- [loro](../docs/research/remote-signal/loro.md) - provides the foundation for the CRDT functionality this module uses to communicate across peers
- [tonic](../docs/research/tonic.md) - provides the **gRPC** foundation used to communicate between 
- [Plumtree](../docs/research/plumtree.md) - provides the foundation of the gossip network communication
- [foca](../docs/research/foca.md)
