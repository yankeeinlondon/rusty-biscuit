# Rendezvous

## Service Overview

The **Rendezvous** daemon process is meant to compliment the Claudine CLI by providing a long running service which can provide:

1. [Logging](./docs/logging.md)
2. [Scheduling & Queuing](./docs/scheduling-and-queuing.md)
3. [Dreaming](./docs/dreaming.md) (_for Memory Files_)
4. [Remote Execution & Interaction](./docs/remote-execution.md)

## Crate Structure

The implementation is split across three crates:

1. `rendezvous-core` 
    - shared protobuf/gRPC stubs, identity, signed envelopes, invitations, IPC helpers
2. `rendezvous-daemon` 
    - the long-running service
3. `rendezvous-client` (TESTING)
    - a thin gRPC test client. 
    - the local client drives the daemon over a Unix Domain Socket; 
    - the daemon persists session logs through a `redb → Loro → DuckDB` pipeline and syncs with remote peers over an authenticated QUIC mesh.


## High Level Architecture

### Local Interaction Architecture

On a single device/host the interactions are:

### Distributed Architecture





```mermaid
flowchart TB
    subgraph client["rendezvous-client"]
        TC["Test client<br/>(tonic gRPC over UDS)"]
    end

    subgraph daemon["rendezvous-daemon"]
        direction TB

        subgraph control["Control plane"]
            SVC["RendezvousService<br/>(tonic gRPC impl)"]
        end

        subgraph persist["Session-log persistence"]
            SLM["SessionLogManager<br/>(loro: one LoroDoc per chunk)"]
            STORE[("Storage / redb<br/>OLTP source of truth")]
            BATCH["Batcher<br/>(flume + blocking thread)"]
            PROJ[("Projection / DuckDB<br/>OLAP query projection")]
        end

        subgraph identity["Identity & crypto (core)"]
            NID["NodeIdentity<br/>(ed25519-dalek)"]
            ENV["SignedEnvelope<br/>(ed25519 sig + blake3 hash)"]
            INV["Invitation<br/>(bech32)"]
        end

        subgraph mesh["Mesh networking"]
            REG["PeerRegistry"]
            QUIC["QuicEndpoint<br/>(quinn + rustls/rcgen)"]
            DISC["Discovery<br/>(mdns-sd)"]
            SYNC["SyncService<br/>(Loro state-vector delta exchange)"]
        end
    end

    PEER["Remote daemon peer"]

    TC -->|"UDS · gRPC<br/>Ping/Status/AppendEntry/Query/Pairing/Sync"| SVC

    SVC --> SLM
    SVC --> PROJ
    SVC --> REG
    SVC --> SYNC

    SLM -->|"1. export snapshot · persist"| STORE
    SLM -->|"2. queue row"| BATCH
    BATCH -->|"micro-batch flush"| PROJ
    STORE -.->|"rehydrate on startup"| SLM

    NID --> ENV
    NID --> INV
    SLM --> NID

    REG --> QUIC
    DISC -->|"resolved peers"| REG
    INV -->|"manual pairing"| REG
    REG --> SYNC
    SYNC --> SLM
    SYNC --> STORE

    QUIC <-->|"QUIC bidi · signed deltas"| PEER
    DISC <-.->|"_agentgrid._udp.local"| PEER
```

### Tech Stack

- **CRDT Engine:** [`loro`](../docs/research/rendezvous/loro.md) 
  - chosen for its high-performance Movable Tree capabilities and efficient version tracking
  - research found at @claudine/docs/research/rendezvous/loro.md
- **Mesh Networking:** [`web-transport`](../docs/research/rendezvous/web-transport.md)
  - backed natively by `quinn` for QUIC/UDP, 
  - providing a future-proof path to WebAssembly/Browser clients
  - research found at @claudine/docs/research/rendezvous/web-transport.md
- **Inter-Process Communication (IPC):** 
  - **gRPC** via `tonic` over Unix Domain Sockets (UDS) or Named Pipes.
  - research found at @claudine/docs/research/rendezvous/tonic.md
- **Transactional Storage (OLTP):** `redb` 
  - Embedded, high-performance Key-Value store for CRDT binary snapshots and deltas).
  - research found at @claudine/docs/research/rendezvous/redb.md
- **Analytical Storage (OLAP):** `duckdb` 
  - embedded columnar database for fast SQL queries on resolved document state
  - research found at @claudine/docs/research/rendezvous/duckdb.md
- **Async Runtime:** `tokio`
  - we will use the `tokio` runtime
  - research found at @claudine/docs/researchrendezvous//tokio.md
- **Signaling / Bootstrap:** `axum`
  - HTTPS/WebSocket for initial peer discovery
  - research found at @claudine/docs/research/rendezvous/axum.md
