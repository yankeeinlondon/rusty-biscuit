---
 phases: 4
 created: 2026-05-02
 start_phase: 1
 source_files_during_phase_1:
   - biscuit-clipboard/lib/Cargo.toml
   - biscuit-clipboard/lib/src/lib.rs
   - biscuit-clipboard/lib/src/content.rs
   - biscuit-clipboard/lib/src/entry.rs
   - biscuit-clipboard/lib/src/backend.rs
   - biscuit-clipboard/lib/src/history.rs
   - biscuit-clipboard/lib/src/storage.rs
   - biscuit-clipboard/lib/src/error.rs
   - biscuit-clipboard/service/Cargo.toml
   - biscuit-clipboard/service/src/main.rs
   - biscuit-clipboard/cli/Cargo.toml
   - biscuit-clipboard/cli/src/main.rs
   - Cargo.toml
 docs_updated_during_phase_1: []
 docs_created_during_phase_1: []
 skills_files_updated_during_phase_1: []
 packages:
   - biscuit-clipboard
   - biscuit-clipboard-cli
   - biscuit-clipboard-service
 source_files_during_phase_2:
   - biscuit-clipboard/lib/src/lib.rs
   - biscuit-clipboard/lib/src/watcher.rs
   - biscuit-clipboard/lib/Cargo.toml
   - biscuit-clipboard/service/Cargo.toml
   - biscuit-clipboard/service/src/main.rs
   - biscuit-clipboard/service/src/api.rs
   - biscuit-clipboard/service/src/daemon.rs
 docs_updated_during_phase_2: []
 docs_created_during_phase_2: []
  skills_files_updated_during_phase_2: []
  source_files_during_phase_3:
    - biscuit-clipboard/lib/src/config.rs
    - biscuit-clipboard/lib/src/client.rs
    - biscuit-clipboard/lib/src/lib.rs
    - biscuit-clipboard/lib/Cargo.toml
    - biscuit-clipboard/cli/src/main.rs
    - biscuit-clipboard/cli/Cargo.toml
    - biscuit-clipboard/service/src/daemon.rs
  docs_updated_during_phase_3: []
  docs_created_during_phase_3: []
   skills_files_updated_during_phase_3: []
   source_files_during_phase_4:
     - biscuit-clipboard/service/src/api.rs
     - biscuit-clipboard/service/src/daemon.rs
     - biscuit-clipboard/service/Cargo.toml
   docs_updated_during_phase_4: []
   docs_created_during_phase_4: []
   skills_files_updated_during_phase_4: []
   ---

 # Execution Plan: Biscuit Clipboard Kickoff

 This plan details the implementation of a background clipboard service (`clipper`) and its companion CLI (`clip`), as defined in the functional specification.

 ## Phase 1: Foundation & Shared Library
 Focus on initializing the workspace members and implementing the core logic for clipboard observation and history management.

 - [ ] **Step 1: Workspace Initialization**
     - Create `biscuit-clipboard/lib/Cargo.toml`, `biscuit-clipboard/service/Cargo.toml`, and `biscuit-clipboard/cli/Cargo.toml`.
     - Add members to root `Cargo.toml`.
     - Set up base dependencies (`clipboard-rs`, `axum`, `tokio`, `biscuit-hash`, `serde`, `chrono`, `dirs`).
     - *Validation Checkpoint:* `cargo metadata` includes all three new crates.

 - [ ] **Step 2: Core Data Models**
     - Implement `ContentType`, `ClipboardFormat`, `ImageSnapshot` in `lib/content.rs`.
     - Implement `ClipboardEntry` with xxHash-based `EntryId` in `lib/entry.rs`.
     - *Validation Checkpoint:* Unit tests pass for `EntryId` generation and serialization.

 - [ ] **Step 3: Backend Abstraction**
     - Define `ClipboardBackend` trait in `lib/backend.rs`.
     - Implement `SystemClipboard` using `clipboard-rs`.
     - Implement macOS concealed-type detection (`org.nspasteboard.ConcealedType`).
     - *Validation Checkpoint:* A temporary CLI `watch` prototype successfully prints clipboard changes to stdout using the backend.

 - [ ] **Step 4: History & Storage**
     - Implement `History` ring buffer with 1-hour TTL and 2-entry floor in `lib/history.rs`.
     - Implement `Storage` for disk-spill of large entries (>64KB) in `lib/storage.rs`.
     - *Validation Checkpoint:* Unit tests pass for TTL expiration and disk-spill/load cycle.

 ## Phase 2: Background Service (`clipper`)
 Implement the long-running daemon that watches the clipboard and exposes the REST API.

 - [ ] **Step 5: Watcher & Supervisor**
     - Implement `Watcher` running on a dedicated OS thread in `lib/watcher.rs`.
     - Implement `tokio::sync::mpsc` bridge to async history management.
     - Add supervisor logic with exponential backoff and "degraded" state for panic recovery.
     - *Validation Checkpoint:* Service continues to respond to health checks even if the watcher thread is simulated to panic.

 - [ ] **Step 6: REST API implementation**
     - Set up Axum router with endpoints: `GET /health`, `GET /history`, `GET /history/latest`, `GET /history/:id`, `GET /history/:id/content`, `GET /history/:id/thumbnail`, `GET /current`, `POST /set`, `DELETE /history`.
     - Implement `X-Clipper: 1` health fingerprint.
     - Implement transparent disk-spill loading in the `/content` endpoint.
     - *Validation Checkpoint:* `curl` requests to the local port return expected JSON shapes and headers.

 - [ ] **Step 7: Daemon Lifecycle & Process Coordination**
     - Implement `flock`-based PID file (`clipper.pid`) and port file (`clipper.port`) creation in `service/daemon.rs`.
     - Handle termination signals (SIGINT, SIGTERM) for graceful cleanup.
     - *Validation Checkpoint:* Attempting to start two instances of `clipper` fails cleanly for the second one.

 ## Phase 3: CLI Client (`clip`)
 Implement the CLI tool for user interaction and service management. (Parallelizable with Phase 2, Step 6/7)

 - [ ] **Step 8: REST Client & Auto-start Handshake**
     - Implement `ClipperClient` in `lib/client.rs`.
     - Implement the 4-step handshake (Read port file -> PID check -> Health check -> Auto-start with exponential backoff).
     - *Validation Checkpoint:* Running `clip info` when the service is off starts the service automatically and succeeds.

 - [ ] **Step 9: CLI Commands Implementation**
     - Implement `clip get`, `clip set`, `clip info`, `clip clear`, `clip watch`.
     - Implement `clip service {start, stop, status}` commands.
     - *Validation Checkpoint:* `clip set "foo" && clip get` successfully returns "foo".

 - [ ] **Step 10: History JSON Output**
     - Implement `clip history --json`.
     - *Validation Checkpoint:* `clip history --json` outputs a valid JSON array of entries.

 ## Phase 4: Integration & Validation
 Verify the system as a whole and ensure cross-platform readiness.

 - [ ] **Step 11: End-to-End Integration Testing**
     - Script a battery of tests: set text, check history, set image (large to trigger disk spill), check disk spill, clear history.
     - Verify `X-Clipper` prevents port collision false-positives.
     - *Validation Checkpoint:* Integration suite passes locally.
