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
    - shared protobuf/gRPC stubs, identity, signed envelopes, invitations, and the typed `LocalEndpoint` contract
2. `rendezvous-daemon` 
    - the long-running service
    - owns local-endpoint and data-root authorization, listener setup, and cleanup
3. `rendezvous-client` (TESTING)
    - a thin gRPC test client, plus the portable `connect(&LocalEndpoint)` used by the Claudine CLI
    - the local client drives the daemon over the platform's local endpoint;
    - the daemon persists session logs through a `redb → Loro → DuckDB` pipeline and syncs with remote peers over an authenticated QUIC mesh.

## Two Planes

The daemon speaks over two deliberately separate planes:

| Plane | Transport | Reach | Authorized by |
|---|---|---|---|
| **Local control** | tonic gRPC over a Unix-domain stream socket (macOS, Linux, WSL) or a Windows named pipe (native Windows) | Same host, same OS user | The OS: socket/directory ownership and modes on Unix, a current-user pipe DACL on Windows |
| **Remote mesh** | QUIC (`quinn`) | Explicitly paired nodes, any host | Per-node Ed25519 signatures on every envelope |

Local gRPC is never exposed across hosts, and QUIC is never a local fallback.
Both are implemented and runtime-tested on macOS, Linux, and Windows.

The local plane is **per stable OS user** — the effective UID on Unix, the
process token's account SID on Windows, never a username. Exactly one daemon,
node identity, data root, and endpoint per account.

**The authoritative local-IPC contract is
[`claudine/docs/rendezvous/local-ipc.md`](../docs/rendezvous/local-ipc.md)**:
transport selection, endpoint resolution and overrides, Unix permissions and
cleanup, the Windows DACL and accept behavior, WSL separation, the client
error/retry vocabulary, and the threat boundary.

## High Level Architecture

### Local Interaction Architecture

On a single device/host, one portable `spawn_local_server` binds the endpoint to
a transport-neutral daemon that is built exactly once:

```mermaid
flowchart TB
    subgraph clients["Same host · same OS user"]
        CLI["Claudine CLI<br/>(dashboard · requeue · hooks · session reports)"]
        TC["rendezvous-test-client"]
    end

    CONNECT["rendezvous_client::connect(&LocalEndpoint)"]

    subgraph transport["local_transport (the only platform-specific code)"]
        UNIX["unix.rs<br/>0700 dir · bind · 0600 socket<br/>instance-safe unlink"]
        WIN["windows.rs<br/>current-user DACL · byte mode<br/>first_pipe_instance · reject remote"]
    end

    PREP["prepare_daemon(config)<br/><i>transport-neutral · runs once</i><br/>redb · projection · batcher · identity<br/>registers · QUIC · discovery · workers"]
    SVC["RendezvousService<br/>(tonic gRPC)"]

    CLI --> CONNECT
    TC --> CONNECT
    CONNECT -->|"UDS<br/>macOS · Linux · WSL"| UNIX
    CONNECT -->|"named pipe<br/>Windows"| WIN
    UNIX --> SVC
    WIN --> SVC
    PREP -->|"serve_local_incoming"| SVC
```

### Distributed Architecture





```mermaid
flowchart TB
    subgraph client["rendezvous-client"]
        TC["Test client<br/>(tonic gRPC over the local endpoint)"]
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

    TC -->|"gRPC over UDS / named pipe<br/>Ping/Status/AppendEntry/Query/Pairing/Sync"| SVC

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
  - **gRPC** via `tonic` over Unix domain sockets (macOS, Linux, WSL) or Windows named pipes
  - per stable OS user; modeled by the typed `LocalEndpoint`, never a bare path
  - contract: [`local-ipc.md`](../docs/rendezvous/local-ipc.md)
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
