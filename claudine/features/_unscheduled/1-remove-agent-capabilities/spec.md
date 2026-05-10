**What**: Every provider maintains **both** a typed `ProviderInfo` (29 fields) **and** a legacy `AgentCapabilities` tree (~80 fields across 15 nested structs). The `Agent` trait (`agents/model.rs:279`) is a thin wrapper that delegates entirely to `ProviderInfo`. Tests enforce that the two surfaces agree, but the duplication is a maintenance burden and a source of potential drift.

**Value**: Removing `AgentCapabilities` and the `Agent` trait eliminates an entire parallel data surface, the `LazyLock` indirection, and the fn-pointer accessor pattern. `ProviderInfo` becomes the single source of truth.

**Current pain**: Every new metadata field must be added in two places and tested for agreement. The legacy tree uses string-heavy, loosely typed fields (`Vec<&'static str>`) where the typed catalog uses structured enums. The `Agent` trait adds indirection without behavioral abstraction — `agent_for()` literally returns `provider_info()`.

## Design Decisions

### DD-1: Target API surface — structured sub-catalogs on `ProviderInfo`

Migrated `AgentCapabilities` data will be grouped into 3–4 cohesive sub-types on `ProviderInfo` rather than flattened into individual fields or merged into the linking layer's `ProviderCapabilities`.

Planned sub-catalogs (names tentative):

| Sub-catalog | Combines |
|---|---|
| `ModelSelectionSpec` | `cli_flags`, `aliases`, `precedence_order` |
| `ResourcePathSpec` | skill, command, agent, script path discovery |
| `BillingSpec` | billing model data |

Each sub-catalog is independently testable and self-documenting. `ProviderInfo` stays manageable in width.

**Drop-on-removal types** — these `AgentCapabilities` sub-types have no production consumers and are superseded by existing `ProviderInfo` fields or the linking layer's `ProviderCapabilities`. They will be deleted with `AgentCapabilities` rather than migrated:

| Type | Why drop | Already replaced by |
|------|----------|-------------------|
| `FrontmatterContract` | Zero consumers (not even tests read it) | `SkillFrontmatter` in `ProviderCapabilities` |
| `ConfidenceProfile` | Only `gaps` field read, by one parity test; `overall`/`by_area` unread | `ProviderInfo.known_gaps: &'static [KnownGap]` |
| `ConfigFormat` | No production code reads it; format is implicit in path extensions | `ProviderInfo.config_paths` file extensions |
| `LoggingCapabilities` | Only read by parity tests | `ProviderInfo.session_log_paths: &'static [PathTemplate]` |

**Rejected alternatives**:
- **Option A (flatten)**: Would bloat `ProviderInfo` past ~40 fields, harming readability.
- **Option C (merge into linking layer)**: Would couple catalog data to the linking layer, breaking the library's separation of concerns.

### DD-2: Remove the `Agent` trait entirely

The `Agent` trait will be deleted. `agent_for()` will be replaced by direct `provider_info()` calls returning `&'static ProviderInfo`. All consumers will use `ProviderInfo` fields directly.

The `agents` module may serve as a re-export shim during a brief deprecation window, then be emptied or removed. This is an internal library without semver obligations.

### DD-3: Parity test migration — structural invariants + converted snapshots

The 7 parity tests in `provider/tests.rs` (lines 244–616) will be replaced with **structural invariant tests** on `ProviderInfo` itself. Example invariants:

- Every provider with `supports_skills == true` has at least one skill path in `resource_support`.
- Every provider that lists model aliases has a corresponding `ModelSelectionSpec` entry.

The 9 snapshot tests in `agents/tests.rs` will be converted to pin `ProviderInfo` field values directly.

**Key sequencing**: New tests can be written **before** removing `AgentCapabilities`. Old parity tests are only deleted in the final removal PR alongside the struct itself.

## Migration Plan

### Phase 1 — Add sub-catalogs (no removals)

- Define `ModelSelectionSpec`, `ResourcePathSpec`, `BillingSpec` structs.
- Add these as fields on `ProviderInfo`, populated from existing `AgentCapabilities` data.
- Write structural invariant tests on `ProviderInfo`.
- Convert `agents/tests.rs` snapshot tests to pin `ProviderInfo` fields.
- All existing tests continue to pass unchanged.

### Phase 2 — Migrate consumers

- Systematically migrate every consumer of `AgentCapabilities` fields to read from the new `ProviderInfo` sub-catalogs.
- Migrate every consumer of the `Agent` trait to call `provider_info()` directly.
- Remove the `agents` module shim or reduce it to re-exports only.
- Remove feature-flagged compatibility wrappers as each consumer is migrated.

### Phase 3 — Final removal

- Delete `AgentCapabilities` and all nested structs.
- Delete the `Agent` trait.
- Delete the old parity tests in `provider/tests.rs` (lines 244–616).
- Remove the `agents` module entirely (or leave as empty re-export if needed).

## Acceptance Criteria

- [ ] `ProviderInfo` exposes structured sub-catalogs (`ModelSelectionSpec`, `ResourcePathSpec`, `BillingSpec`) covering actively-consumed data from `AgentCapabilities`.
- [ ] Drop-on-removal types (`FrontmatterContract`, `ConfidenceProfile`, `ConfigFormat`, `LoggingCapabilities`) are deleted with `AgentCapabilities` — no migration needed.
- [ ] The `Agent` trait no longer exists. All call sites use `provider_info()` directly.
- [ ] `AgentCapabilities` and all nested structs are deleted from the codebase.
- [ ] The `agents` module is removed or reduced to empty re-exports.
- [ ] Structural invariant tests on `ProviderInfo` exist and pass (covering skills/resource/billing/alias consistency).
- [ ] Snapshot-style tests pin `ProviderInfo` field values directly (no dependency on `AgentCapabilities`).
- [ ] Old parity tests between `AgentCapabilities` and `ProviderInfo` are deleted.
- [ ] No `LazyLock` or fn-pointer accessor patterns remain in the provider catalog path.
- [ ] `cargo test` passes with zero regressions.
