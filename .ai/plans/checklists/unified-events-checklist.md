# Unified Events Conformance Checklist

Source plan: `.ai/plans/2026-02-18.unified-events.md`
Design authority: `claudine/docs/unified-events.md`

## Baseline Gate Snapshot (2026-02-19)

- [x] `cargo test -p claudine`
  - Result: pass
  - Tests: 293 unit + 1 doctest passed (`0 failed`)
- [x] `cargo clippy -p claudine -- -D warnings`
  - Result: pass

## Verification Snapshot (2026-02-19 Run 2)

- [x] `cargo test -p claudine`
  - Result: pass
  - Tests: 260 unit + 1 doctest passed (`0 failed`)
- [x] `cargo clippy -p claudine -- -D warnings`
  - Result: pass
- [x] `cargo test -p claudine-cli`
  - Result: pass (`14 passed, 0 failed`)
- [x] `just -f claudine/justfile test`
  - Result: pass (library + CLI suite)

## Public Events Surface Expected to Change

Current exports from `claudine/lib/src/events/mod.rs` with downstream impact assessment:

- [x] `AgenticEvent` (canonicalized + `#[non_exhaustive]`; file renamed `agentric` -> `agentic`)
- [x] `Provider` (added `RooCode`, `#[non_exhaustive]`, provider table updates)
- [x] `EventAction` -> `HookAction` (runtime rename completed)
- [x] `LogTarget` shape change (`File` / `Server` canonical shape)
- [x] New canonical types: `Mapper`, `CompiledMapper`, `HookResponse`, `HookDecision`, `ResolvedHook`
- [x] Config structs using actions (`EventBinding`, `HookerConfig`, `GlobalSettings`) use canonical action schema

## Downstream Consumer Hotspots

Primary downstream consumers to keep compiling while refactoring:

- [x] `claudine/cli/src/commands/hooks.rs` (updated to `HookAction` + 8 providers)
- [x] `claudine/cli/src/commands/init/mod.rs` (defaults/build path uses `HookAction`)
- [x] `claudine/cli/src/commands/init/prompts.rs` (interactive action creation uses canonical variants)
- [x] `claudine/cli/src/commands/dry_run.rs` (action rendering + adapter parse path updated)
- [x] `claudine/cli/src/commands/handle.rs` (dispatch contract updated to `DispatchOutcome`)
- [x] `claudine/lib/src/config/*` provider configurators (Roo added; non-exhaustive matches handled)
- [x] `claudine/lib/src/linking/capabilities.rs` + `paths.rs` provider arrays (8-provider coverage)

## Phase Status Tracker

- [x] Phase 0 completed
- [x] Phase 1 completed
- [x] Phase 2 completed
- [x] Phase 3 completed
- [x] Phase 4 completed
- [x] Phase 5 completed
- [x] Phase 6 completed
- [x] Phase 7 completed
- [ ] Phase 8 completed
- [x] Phase 9 completed
- [ ] Phase 10 completed
