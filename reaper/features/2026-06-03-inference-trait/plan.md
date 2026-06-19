---
agent: open_code
phases: 4
created: 2026-06-11
start_phase: 1
phase_1_completed: 2026-06-11
phase_2_completed: 2026-06-11
phase_3_completed: 2026-06-11
phase_4_completed: 2026-06-11
source_files_during_phase_1:
  - biscuit-contract/lib/Cargo.toml
  - biscuit-contract/lib/src/lib.rs
  - biscuit-contract/lib/src/inference.rs
  - Cargo.toml
source_files_during_phase_2:
  - biscuit-contract/lib/src/inference.rs
source_files_during_phase_3:
  - biscuit-contract/lib/Cargo.toml
  - biscuit-contract/lib/src/inference.rs
source_files_during_phase_4: []
docs_updated_during_phase_1: []
docs_updated_during_phase_2: []
docs_updated_during_phase_3: []
docs_updated_during_phase_4:
  - biscuit-contract/README.md
  - biscuit-contract/docs/dependencies.md
  - docs/dependencies.md
docs_created_during_phase_1:
  - biscuit-contract/README.md
  - biscuit-contract/docs/dependencies.md
docs_created_during_phase_2: []
docs_created_during_phase_3: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_1: []
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3: []
skills_files_updated_during_phase_4:
  - .claude/skills/biscuit-contract/SKILL.md
source_code:
  - biscuit-contract/lib/Cargo.toml
  - biscuit-contract/lib/src/lib.rs
  - biscuit-contract/lib/src/inference.rs
  - Cargo.toml
documentation:
  - biscuit-contract/README.md
  - biscuit-contract/docs/dependencies.md
  - docs/dependencies.md
packages:
  - biscuit-contract
---

# Inference Trait Contract — Execution Plan

This plan converts the functional specification for `biscuit-contract` into a
sequence of concrete, observable tasks. It delivers only the shared contract
crate and its L1 tests; provider and consumer integrations are explicitly out
of scope for this work.

---

## Phase 1 — Scaffolding and Workspace Integration

**Goal:** The `biscuit-contract` package area exists, is a workspace member, and
compiles as an empty crate.

**Parallelizable:** Tasks 1.2, 1.3, 1.4, 1.5, 1.6, and 1.7 can be done in any
order once 1.1 is complete.

- [x] Create the directory layout `biscuit-contract/lib/src/` and
  `biscuit-contract/docs/`.
- [x] Create `biscuit-contract/lib/Cargo.toml` named `biscuit-contract`, version
  `0.1.0`, depending only on `async-trait`, `serde_json`, and `thiserror`.
- [x] Add `biscuit-contract/lib` to the root `Cargo.toml` workspace members.
- [x] Create `biscuit-contract/lib/src/lib.rs` with a public `inference` module.
- [x] Create `biscuit-contract/lib/src/inference.rs` as an empty placeholder.
- [x] Create `biscuit-contract/README.md` with a one-paragraph summary and a
  note that provider implementations are follow-up work.
- [x] Create `biscuit-contract/justfile` that imports the repository's shared
  recipes and exposes `test`, `lint`, `build`, and `doctest`.
- [x] Create `biscuit-contract/docs/dependencies.md` listing the three allowed
  dependencies and stating the forbidden dependency classes.
- [x] Run `cargo metadata --no-deps --format-version 1` and confirm
  `biscuit-contract` appears in the workspace.

### Validation Checkpoint

- [x] `cargo check -p biscuit-contract` passes.
- [x] `cargo metadata --no-deps` lists `biscuit-contract`.

---

## Phase 2 — Core Contract Implementation

**Goal:** All public types and the `InferenceAdapter` trait are implemented
exactly as specified, with object-safe `Send + Sync` async bounds.

**Parallelizable:** The enum and struct definitions (2.1 through 2.8) can be
authored independently; 2.9, 2.10, and 2.11 depend on those types being present.

- [x] Implement `InferencePriority` enum with variants `Cost`, `Latency`,
  `Quality`, and `Balanced` (default), plus `Debug`, `Clone`, `Copy`, `Default`,
  `PartialEq`, `Eq`.
- [x] Implement `ReasoningEffort` enum with variants `None`, `Low`, `Medium`
  (default), and `High`, plus `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`,
  `Eq`.
- [x] Implement `InferenceProfile` struct with `priority: InferencePriority` and
  `reasoning: ReasoningEffort`, plus `Debug`, `Clone`, `Copy`, `Default`,
  `PartialEq`, `Eq`.
- [x] Implement `InferenceOutput` enum with `Prose` and `Structured { schema:
  Value }`, plus `Debug`, `Clone`, `PartialEq`.
- [x] Implement `InferenceData` enum with `Prose(String)` and
  `Structured(Value)`, plus `Debug`, `Clone`, `PartialEq`.
- [x] Implement `InferenceMetadata` struct with optional `provider`, `model`, and
  `agent` strings, plus `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`.
- [x] Implement `InferenceRequest` struct with `prompt`, `output`, and `profile`,
  plus `Debug`, `Clone`, `PartialEq`.
- [x] Implement `InferenceResponse` struct with `data` and `metadata`, plus
  `Debug`, `Clone`, `PartialEq`.
- [x] Implement `InferenceErrorKind` enum with all nine variants specified, plus
  `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`.
- [x] Implement `InferenceError` struct with `kind`, `message`, and
  `retry_after`, deriving `Debug`, `Clone`, `PartialEq`, `Eq`, and using
  `thiserror` for `std::error::Error` and `Display`.
- [x] Implement the `InferenceAdapter` trait with `#[async_trait]`, `Send +
  Sync` supertrait, and a single `infer` method taking `InferenceRequest` and
  returning `Result<InferenceResponse, InferenceError>`.
- [x] Add a compile-time assertion or doc test proving `Arc<dyn
  InferenceAdapter>` is constructible.
- [x] Ensure no `Serialize`/`Deserialize` derives are added to request or
  response types.

### Validation Checkpoint

- [x] `cargo check -p biscuit-contract` passes.
- [x] `cargo doc -p biscuit-contract --no-deps` builds without errors.
- [x] A temporary `let _: Arc<dyn InferenceAdapter>` compiles.

---

## Phase 3 — Deterministic L1 Tests

**Goal:** The contract crate has tests covering object safety, prose and
structured inference, profile defaults, error categories, and invalid-response
semantics.

**Parallelizable:** Each fake adapter and its corresponding test can be written
independently; the object-safety test and profile-defaults test can also be
written independently.

- [x] Create a fake prose adapter that returns `InferenceData::Prose` and
  populates `InferenceMetadata`.
- [x] Create a fake structured adapter that validates the schema is a non-null
  object and returns `InferenceData::Structured` matching the request.
- [x] Create a fake variant-mismatch adapter that returns
  `InferenceData::Prose` for a structured request.
- [x] Create a fake schema-violation adapter that returns structured JSON that
  does not satisfy the provided schema.
- [x] Write a test proving a fake adapter can be stored in and called through
  `Arc<dyn InferenceAdapter>`.
- [x] Write a test covering a prose request/response round-trip through the trait
  object.
- [x] Write a test covering a structured request/response round-trip through the
  trait object.
- [x] Write a test asserting the default values of `InferenceProfile`.
- [x] Write a test constructing and pattern-matching every
  `InferenceErrorKind` variant.
- [x] Write a test asserting a response-variant mismatch is reported as
  `InvalidResponse`.
- [x] Write a test asserting a deliberate schema violation is reported as
  `InvalidResponse`.
- [x] Run `just test` (or `cargo test -p biscuit-contract`) and confirm all
  tests pass.

### Validation Checkpoint

- [x] `cargo test -p biscuit-contract` passes with 100% of tests green.
- [x] `cargo test -p biscuit-contract --doc` passes.

---

## Phase 4 — Documentation, Tooling, and Drift Maintenance

**Goal:** The package area is fully documented, wired into repository tooling,
and the local skill catalog and dependency docs are updated.

**Parallelizable:** README, justfile, dependencies.md, skill-catalog update, and
root dependency-doc update can all proceed in parallel once the API is stable.

- [x] Complete `biscuit-contract/README.md` with the package purpose, a minimal
  usage example using `Arc<dyn InferenceAdapter>`, and a link to the spec.
- [x] Finalize `biscuit-contract/justfile` so `just test|lint|build|doctest`
  delegate correctly and reuse shared recipes.
- [x] Complete `biscuit-contract/docs/dependencies.md` with rationale for each
  allowed dependency and an explicit prohibition list.
- [x] Add `biscuit-contract` to the root `justfile` curated area list if it is
  intended to participate in root lifecycle commands.
- [x] Update `.claude/skills/` (or the local skill catalog) with an entry for the
  new shared package area, per repository drift-maintenance rules.
- [x] Update root `docs/dependencies.md` (or the appropriate root dependency
  document) to list `biscuit-contract` and its allowed dependencies.
- [x] Run `cargo check --workspace` and confirm no new warnings originate from
  `biscuit-contract`.
- [x] Run `cargo clippy -p biscuit-contract -- -D warnings`.
- [x] Run `cargo test -p biscuit-contract` one final time.
- [x] Verify `Cargo.toml` does not contain Tokio, an HTTP client, a JSON Schema
  engine, or any consumer/provider crate.

### Validation Checkpoint

- [x] `cargo check --workspace` passes.
- [x] `cargo clippy -p biscuit-contract -- -D warnings` passes.
- [x] `cargo test -p biscuit-contract` passes.
- [x] README, justfile, and dependencies.md are complete and consistent with the
  spec.
- [x] Skill catalog and root dependency docs reflect the new package area.

---

## Dependency Order Summary

1. **Phase 1** must complete before Phase 2 can compile.
2. **Phase 2** must complete before Phase 3 tests can reference the full API.
3. **Phase 3** must complete before Phase 4 final documentation examples can be
   verified.
4. Within each phase, tasks marked parallelizable may be executed concurrently
   by separate implementers.
