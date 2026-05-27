---
phases: 6
created: 2026-05-24
start_phase: 1
source_files_during_phase_1:
  - Cargo.toml
  - claudine/remote-signal/justfile
  - claudine/remote-signal/core/Cargo.toml
  - claudine/remote-signal/core/build.rs
  - claudine/remote-signal/core/proto/remote_signal.proto
  - claudine/remote-signal/core/src/lib.rs
  - claudine/remote-signal/core/src/socket.rs
  - claudine/remote-signal/daemon/Cargo.toml
  - claudine/remote-signal/daemon/src/lib.rs
  - claudine/remote-signal/daemon/src/main.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/service.rs
  - claudine/remote-signal/client/Cargo.toml
  - claudine/remote-signal/client/src/lib.rs
  - claudine/remote-signal/client/src/main.rs
  - claudine/remote-signal/client/tests/uds_round_trip.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/remote-signal/core/Cargo.toml
  - claudine/remote-signal/core/proto/remote_signal.proto
  - claudine/remote-signal/core/src/lib.rs
  - claudine/remote-signal/core/src/session_log.rs
  - claudine/remote-signal/daemon/Cargo.toml
  - claudine/remote-signal/daemon/src/batcher.rs
  - claudine/remote-signal/daemon/src/lib.rs
  - claudine/remote-signal/daemon/src/main.rs
  - claudine/remote-signal/daemon/src/projection.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/service.rs
  - claudine/remote-signal/daemon/src/session_log.rs
  - claudine/remote-signal/daemon/src/storage.rs
  - claudine/remote-signal/client/src/main.rs
  - claudine/remote-signal/client/tests/session_log_round_trip.rs
  - claudine/remote-signal/client/tests/uds_round_trip.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/remote-signal/core/Cargo.toml
  - claudine/remote-signal/core/src/lib.rs
  - claudine/remote-signal/core/src/identity.rs
  - claudine/remote-signal/core/src/envelope.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/session_log.rs
  - claudine/remote-signal/daemon/src/service.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/remote-signal/core/Cargo.toml
  - claudine/remote-signal/core/proto/remote_signal.proto
  - claudine/remote-signal/core/src/lib.rs
  - claudine/remote-signal/core/src/invitation.rs
  - claudine/remote-signal/daemon/Cargo.toml
  - claudine/remote-signal/daemon/src/lib.rs
  - claudine/remote-signal/daemon/src/main.rs
  - claudine/remote-signal/daemon/src/quic.rs
  - claudine/remote-signal/daemon/src/discovery.rs
  - claudine/remote-signal/daemon/src/peers.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/service.rs
  - claudine/remote-signal/daemon/tests/peer_discovery.rs
  - claudine/remote-signal/client/src/main.rs
  - claudine/remote-signal/client/tests/uds_round_trip.rs
  - claudine/remote-signal/client/tests/session_log_round_trip.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/remote-signal/core/proto/remote_signal.proto
  - claudine/remote-signal/core/src/lib.rs
  - claudine/remote-signal/core/src/sync.rs
  - claudine/remote-signal/daemon/src/lib.rs
  - claudine/remote-signal/daemon/src/peers.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/service.rs
  - claudine/remote-signal/daemon/src/session_log.rs
  - claudine/remote-signal/daemon/src/storage.rs
  - claudine/remote-signal/daemon/src/sync.rs
  - claudine/remote-signal/daemon/tests/pairing_and_sync.rs
  - claudine/remote-signal/client/src/main.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/remote-signal/justfile
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/tests/phase6_integration.rs
  - claudine/remote-signal/scripts/poc-demo.sh
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - remote-signal-core
  - remote-signal-daemon
  - remote-signal-client
---

# Remote Signal POC Execution Plan

This plan outlines the implementation of the `remote-signal` daemon proof-of-concept, focusing on a distributed, local-first mesh architecture for session-log synchronization.

## Phase 1: Foundations & IPC
Establish the project structure and the communication bridge between the client and the daemon.

- [ ] Create `@claudine/remote-signal` package area with `daemon`, `client` (test client), and `core` (shared logic) crates.
- [ ] Define the gRPC Protobuf schema for the IPC interface (e.g., `remote_signal.proto`).
- [ ] Implement the `remote-signal-daemon` boilerplate using `tokio` and `tonic`.
- [ ] Implement the gRPC server in the daemon listening on a Unix Domain Socket (UDS).
- [ ] Implement the `remote-signal-test-client` to connect to the daemon over UDS.
- [ ] **Validation:** Verify the test client can send a "Ping" or "Status" command to the daemon and receive a response.

## Phase 2: Persistence & Document Model
Implement the dual-storage strategy (CQRS) and the Loro-based session-log document model.

- [ ] Integrate `loro` for CRDT document management in the daemon.
- [ ] Implement the `redb` storage layer for persisting Loro snapshots and signed deltas.
- [ ] Define the `SessionLog` document type and the chunking logic (`session/{node_id}/{session_id}/part/{index}`).
- [ ] Implement `duckdb` projection logic for analytical queries.
- [ ] Implement the `flume` micro-batching pipeline to sync Loro state changes to DuckDB asynchronously.
- [ ] **Validation:** Verify that appending to a session log via the test client persists data to `redb` and eventually appears in `duckdb` after the batching interval.

## Phase 3: Identity & Security
Establish node identity and secure the synchronization protocol.

- [x] Implement node keypair generation and secure storage using `ed25519-dalek`.
- [x] Implement the `SignedEnvelope` structure for all network-bound payloads.
- [x] Implement signing logic for outgoing Loro deltas.
- [x] Implement verification logic for incoming envelopes (signature, hash, and message ID deduplication).
- [x] **Validation:** Unit tests verifying that signed payloads are accepted and altered payloads (invalid signature or hash mismatch) are rejected.

## Phase 4: Networking & Peer Discovery
Enable nodes to find and connect to each other over the mesh.

- [x] Implement the `web-transport` (via `quinn`) server and client in the daemon.
- [x] Implement mDNS discovery using `mdns-sd` to advertise and browse for `_agentgrid._udp.local` services.
- [x] Implement manual peer invitation logic: encode connection info (IP, Port, PubKey) into Bech32/Base58 strings.
- [x] Implement the `ConnectToPeer` gRPC command to initiate a WebTransport connection from an invitation string.
- [x] **Validation:** Start two daemons locally and verify they discover each other via mDNS and can establish a QUIC connection.

## Phase 5: Pairing & Sync Logic
Implement explicit pairing and the direct synchronization protocol.

- [x] Implement pairing state management in the daemon (persisted in `redb`).
- [x] Implement the pairing approval flow (manual approval of discovered peers).
- [x] Implement the synchronization protocol: State Vector exchange followed by Delta pushing.
- [x] Implement the "Direct Sync" backend for paired peers.
- [x] **Validation:** Verify that two nodes, once paired, exchange Loro state vectors and synchronize missing deltas upon connection.

## Phase 6: Integration & Validation
Perform end-to-end tests to ensure system stability and convergence.

- [x] **Convergence Test:** Two nodes append to the same session-log chunk and converge to the identical state.
- [x] **Chunking Test:** Verify that exceeding the chunk threshold creates a new deterministic document ID and syncs correctly across peers.
- [x] **Restart/Replay Test:** Stop a daemon, restart it, and verify it rebuilds state from `redb` and resumes sync without data loss.
- [x] **Security Test:** Verify that an unpaired node, even if connected via WebTransport, cannot sync session-log data.
- [x] **Final POC Demo:** A script/command-sequence using the test client to demonstrate the full multi-node sync flow.
