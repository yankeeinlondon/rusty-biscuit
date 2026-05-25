---
phases: 6
created: 2026-05-24
start_phase: 1
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

- [ ] Implement node keypair generation and secure storage using `ed25519-dalek`.
- [ ] Implement the `SignedEnvelope` structure for all network-bound payloads.
- [ ] Implement signing logic for outgoing Loro deltas.
- [ ] Implement verification logic for incoming envelopes (signature, hash, and message ID deduplication).
- [ ] **Validation:** Unit tests verifying that signed payloads are accepted and altered payloads (invalid signature or hash mismatch) are rejected.

## Phase 4: Networking & Peer Discovery
Enable nodes to find and connect to each other over the mesh.

- [ ] Implement the `web-transport` (via `quinn`) server and client in the daemon.
- [ ] Implement mDNS discovery using `mdns-sd` to advertise and browse for `_agentgrid._udp.local` services.
- [ ] Implement manual peer invitation logic: encode connection info (IP, Port, PubKey) into Bech32/Base58 strings.
- [ ] Implement the `ConnectToPeer` gRPC command to initiate a WebTransport connection from an invitation string.
- [ ] **Validation:** Start two daemons locally and verify they discover each other via mDNS and can establish a QUIC connection.

## Phase 5: Pairing & Sync Logic
Implement explicit pairing and the direct synchronization protocol.

- [ ] Implement pairing state management in the daemon (persisted in `redb`).
- [ ] Implement the pairing approval flow (manual approval of discovered peers).
- [ ] Implement the synchronization protocol: State Vector exchange followed by Delta pushing.
- [ ] Implement the "Direct Sync" backend for paired peers.
- [ ] **Validation:** Verify that two nodes, once paired, exchange Loro state vectors and synchronize missing deltas upon connection.

## Phase 6: Integration & Validation
Perform end-to-end tests to ensure system stability and convergence.

- [ ] **Convergence Test:** Two nodes append to the same session-log chunk and converge to the identical state.
- [ ] **Chunking Test:** Verify that exceeding the chunk threshold creates a new deterministic document ID and syncs correctly across peers.
- [ ] **Restart/Replay Test:** Stop a daemon, restart it, and verify it rebuilds state from `redb` and resumes sync without data loss.
- [ ] **Security Test:** Verify that an unpaired node, even if connected via WebTransport, cannot sync session-log data.
- [ ] **Final POC Demo:** A script/command-sequence using the test client to demonstrate the full multi-node sync flow.
