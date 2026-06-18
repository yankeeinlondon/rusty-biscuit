# Phase 1 — Grounding and inventory

Read-only confirmation of the spec's `file:line` anchors against the live tree, plus a real `--perf`
baseline captured before any change. No source files were modified in this phase.

Tree state: branch `claudine`, built `cargo build -p claudine-cli` clean (37.94s, dev profile).

---

## 1. The six headline-emit sites (TM-1 / affected-code map)

All six sites confirmed present and unchanged from the spec anchors. Each computes a **fresh mid-flight
timer**'s `.elapsed()` as the headline `total`, then `collector.into_report(total)` →
`eprint!(render_perf_report(...))`. This is exactly RC-1: the headline is whatever local timer the site
started, never `process_start`.

| # | File | Line | Local timer | Distinguishing context (routes to one Phase-2 helper) |
|---|------|------|-------------|-------------------------------------------------------|
| 1 | `cli/src/commands/wrap/composition/mod.rs` | **848** | `total_start` (`:790`) | **dry-run-unresolved** — early seam, `request.dry_run && request.resolved_target.is_none()`; no provider selection/preflight |
| 2 | `cli/src/commands/wrap/composition/mod.rs` | **1693** | `total_start` | **dry-run-resolved** — `request.dry_run` after target resolution + harness preflight |
| 3 | `cli/src/commands/wrap/composition/mod.rs` | **1900** | `total_start` | **harness/loop branch** — harness-enabled provider, `harness_perf` set via `set_agent_perf` |
| 4 | `cli/src/commands/wrap/composition/mod.rs` | **2000** | `total_start` | **structured/inline (non-harness)** — `execute_without_harness`, direct + inline modes |
| 5 | `cli/src/commands/wrap/mod.rs` | **334** | `wrapper_start` (`:313`) | **direct provider wrapper** — `run_provider_wrapper`, collector `CommandPerfCollector::new("Wrapper", …)` |
| 6a | `cli/src/commands/wrap/sequence.rs` | **442** | `sequence_start` (`:58`) | **sequence partial/interrupt** — `step_contexts` empty + `total_steps > 0`; calls `set_partial()` |
| 6b | `cli/src/commands/wrap/sequence.rs` | **757** | `sequence_start` | **sequence normal completion** — end of `execute_sequence` |

> The spec counts "six sites"; composition has four (sites 1–4), wrapper one (5), sequence two (6a/6b) —
> seven emit statements total, but the wrapper + sequence share their per-area timer, matching the
> spec's six-source framing (composition `total_start`, wrapper `wrapper_start`, sequence `sequence_start`).

**Common shape** every site reduces to (the Phase-2 `emit_report` helper target):

```rust
let total = <local_timer>.elapsed();
let report = <collector>.into_report(total);
eprint!("{}", crate::perf::render_perf_report(&report));
```

The only per-site variation is *when* it fires and which `collector`/`acc` value is in scope. The
threaded baseline + shared helper collapses all seven to one call taking the collector (the collector
computes the headline from its own baseline at `into_report` time — `total` parameter drops or becomes a
test seam).

---

## 2. `StartupTimings` construction + `process_start` scope (TM-1 carrier)

- `StartupTimings` struct: `cli/src/perf.rs:37-46` (fields `arg_parsing`, `tracing_init`, `config_loading`,
  `pre_dispatch`, `prep_phase`). **No `process_start` field today** — Phase 2 adds the baseline carrier here.
- `process_start = Instant::now()` captured at `cli/src/main.rs:191` (top of `run()`), threaded as a param
  into `async_main` (`:236`) and used for `pre_dispatch = process_start.elapsed()` (`:254`).
- Two `StartupTimings` construction sites, both with `process_start` **in scope** and both setting
  `prep_phase: Duration::ZERO`:
  - **Wrapper path:** `main.rs:264-270` (inside the `wrapper_command` match arm).
  - **General path:** `main.rs:302-308` (composition / other commands).
- `prep_phase` is later filled in `compose.rs` (`:669-671` direct, `:1076-1078` inline) from
  `compose_entry.elapsed()` / `inline_compose_entry.elapsed()`. Sequence never sets it → stays `0µs`
  (visible in the sequence baseline below).

Phase-2 thread: add `process_start: Instant` to `StartupTimings`, populate from `main.rs`'s value at both
construction sites, carry into both collectors, compute headline as `process_start.elapsed()` in
`into_report`.

---

## 3. Collector types + env-setup timers

Two collector types, both in `cli/src/perf.rs`:

| Type | Constructors | Env timer start | Env timer close |
|------|--------------|-----------------|-----------------|
| `CommandPerfCollector` | `new` (`:292`), `new_with_composition` (`:306`) | `env_setup_started_at = Some(Instant::now())` in constructor (`:296`/`:314`) | `mark_env_setup_complete` (`:324`) |
| `SequencePerfAccumulator` | `new` (`:136`) | `env_setup_started_at` in `new` (`:139`) | `mark_env_setup_complete` (`:149`) |

`mark_env_setup_complete` call sites confirmed:
- Wrapper/composition collector: `cli/src/commands/wrap/composition/mod.rs:1524` (after the last substage).
- Sequence accumulator: `cli/src/commands/wrap/sequence.rs:439` (partial path) and `:461` (normal path).
- Spec also cites wrapper `wrap/mod.rs:905`; confirmed — `collector.mark_env_setup_complete()` at
  `wrap/mod.rs:905` inside `run_provider_wrapper_inner` (direct-wrapper env window close).

`CommandPerfCollector` for composition is constructed at `mod.rs:791-798` (`new_with_composition`,
carrying `request.prepared.compose_perf.clone()`), i.e. its env timer starts ~1 line after `total_start`
(`:790`).

---

## 4. Substage checkpoint chain + clock-sharing audit (RC-3 / TR-2 input)

- The substage chain lives in `execute_composition_request_inner`:
  - `total_start = Instant::now()` (`mod.rs:790`); `last_checkpoint = total_start` (`:802`).
  - `record_substage(collector, checkpoint, name)` (`:804-814`) records `checkpoint.elapsed()` then resets
    the checkpoint to `Instant::now()`. Because each call measures the full delta since the previous
    checkpoint with **no internal gaps**, the substages sum to `[total_start … last reset]`.
  - Substage recordings: `target resolution` is the first (via `record_substage` at `:1026`), then
    `header env plan` (`:1082`), `child env build` (`:1118`), `mcp composition` (`:1227`), `argv assembly`
    (`:1357`), `system prompt` (`:1448`), `stream + prompt delivery` (`:1517`).
  - **`mcp composition` zero-row:** at `mod.rs:1230` `collector.mark_substage("mcp composition",
    Duration::ZERO)` is emitted directly (the only substage pushed as an explicit zero rather than a timed
    delta) — confirmed; must remain representable in the new tree.

- **Clock-sharing finding (RC-3 confirmed):** the substage chain and the `environment setup` window are on
  **two different `Instant`s**:
  - Substage chain origin: `total_start` (`:790`).
  - Env-setup window origin: `env_setup_started_at` inside `CommandPerfCollector::new_with_composition`
    (`perf.rs:314`), constructed at `mod.rs:791-798` — microseconds after `total_start`.
  - Closes: last substage resets the checkpoint at `:1517-1521`; `mark_env_setup_complete` reads
    `env_setup_started_at.elapsed()` at `:1524`. The work between `:1521` and `:1524`
    (`validate_argv_flags_before_separator`, the `if let Some(collector)` guard) is in the env-setup window
    but attributed to **no** substage.
  - Net: `Σ substages ≈ [790 … 1521]` while `env_setup = [~791 … 1524]`. They nearly coincide but are not
    the same timer; nothing guarantees the substages carve the parent exactly. TR-2 option (a) =
    make `mark_env_setup_complete` the close of the *same* checkpoint chain so substages reconcile.

---

## 5. P-5a audit — the `compose_entry → execute_composition_request_inner` prep window

Window: `compose.rs` direct `run_compose_inner` `:328` (`compose_entry`) → `:673`
(`execute_composition_request`, after `prep_phase` is stamped at `:669-671`); inline
`run_inline_compose_inner` `:681` (`inline_compose_entry`) → `:1080` (stamped `:1076-1078`). The
darkmatter compose pass that produces `compose_perf` runs **inside** this window (RC-2: `prep_phase ⊇
compose_perf.total`).

Material, non-overlapping units of work in the direct window, with Phase-4 disposition:

| Work unit | Anchor | Cost character | Phase-4 disposition |
|-----------|--------|----------------|---------------------|
| Positional parse, receipt banner, interrupt-guard install, timeout grammar validation, set-override merge | `:330-365` | trivial (µs) | `prep → unattributed` |
| **Source resolution** (`resolve_composition_source` — FileReference resolve + Markdown parse/frontmatter load) | `:368` | material | named Structural child **`frontmatter load`** |
| **Schema-aware pre-validation** (`pre_validate_with_interactive_collection` — schema load + validate; may include interactive prompt = user wait) | `:379-391` | material | named Structural child **`schema validation`** (exclude any interactive wait, or annotate) |
| **Prep context build** (`CompositionPrepContext::new` — single repo-root discovery + selection-config load + installed-provider snapshot) | `:405-409` | material (git/fs) | candidate Structural child **`prep context`** |
| Eager target resolution (`eagerly_resolve_target`) | `:417-427` | small | `prep → unattributed` (or fold into `prep context`) |
| Agent env install | `:429-433` | trivial | `prep → unattributed` |
| `ComposeContext::capture()` + options build (context capture: git/repo/os/hw via sniff) | `:444-455` | material (DM-4 territory — `capture_timings`) | Phase-7/DM-4 attaches `ctx.*`; Phase-4 leaves in `prep → unattributed` or `shell approval` parent |
| **Shell-approval preflight** (`resolve_shell_approvals` — runs a *darkmatter compose pass* to expand/approve `::shell`) | `:457-478` | **material — dominant** in the baseline (ran the `sleep 0.3`) | named Structural child **`shell approval`** |
| `prepare_direct_with_schema` (the metered compose pass → `compose_perf`) | `:506-526` (loop) / `:620-632` (single) | material | becomes **`composition`** child of `prep phase` (not a separate prep child) |

**Key P-5a insight from the baseline:** in the capture, `prep phase = 1.68s` but the metered
`Composition Report TOTAL = 5.9ms`. The ~1.6s gap is the **shell-approval preflight** pass (`:470-478`),
which runs its own un-metered compose to approve the `sleep 0.3` directive — not the final
`prepare_direct_with_schema` pass (whose shells are cache-approved and fast). So `shell approval` is the
single most important named Structural child to add in Phase 4; without it `prep → unattributed` would
carry ~99% of prep. Expect this remainder to exceed the TR-3 display threshold until `shell approval` is
named.

The inline window (`:681-1080`) mirrors the direct one with one extra unit: the `prompt`-property
pre-validation (`:758-782`, `PromptPropertyWrongType` typed guard) before the schema check — trivial,
`prep → unattributed`.

---

## 6. Captured `--perf` baseline (before-image for the Phase-6 snapshot)

Built binary: `target/debug/claudine`. Fixtures under `/tmp/perf-baseline/` (ephemeral; reproduced
inline below). Reports are emitted to **stderr** (G-8 confirmed: stdout held composed content only).
ANSI/OSC8 stripped for readability; the yellow `▌ ` BlockQuote frame is retained verbatim.

### 6a. `claudine compose <fixture> --perf --dry-run --yolo` (the motivating shape)

Fixture: title-only frontmatter + body with `::shell[hostname]` and `::shell[sleep 0.3 && date]`.

```
▌ Performance (elapsed 71.1ms)
▌
▌ CLI Overhead
▌   pre-dispatch:         66.9ms
▌   prep phase:            1.68s
▌   arg parsing:          55.7ms
▌   config loading:       33.3ms
▌   tracing init:          4.3ms
▌   environment setup:    71.1ms
▌   ════════════════════════════
▌   TOTAL:          1.81s
▌
▌ Composition Report
▌   frontmatter interpolation:        9µs
▌   schema validation:               15µs
▌   frontmatter shell expansion:    241µs
▌   effective state build:           25µs
▌   text replacement:                 1µs
▌   page blocks:                     73µs
▌   interpolation:                   11µs
▌   shell expansion:                318µs
▌   shell blocks:                    29µs
▌   link resolve:                   2.7ms
▌   transclusion parse:             118µs
▌   transclusion prepare:             3µs
▌   transclusion resolve:             0µs
▌   transclusion apply:               0µs
▌   cleanup:                        697µs
▌   normalization:                   24µs
▌   link normalization:             1.5ms
▌   ═════════════════════════════════════
▌   TOTAL:                   5.9ms
▌
▌ Agent execution skipped (dry run)
```

Reproduces all three motivating defects:
1. **Headline ≠ sections:** `(elapsed 71.1ms)` vs CLI Overhead `TOTAL: 1.81s` — ~25× off (RC-1).
2. **Rows don't visibly sum:** `arg parsing`/`config loading`/`tracing init` are diagnostic sub-buckets
   silently excluded from the `TOTAL` (RC-4).
3. **Double-count:** `prep phase 1.68s` ⊇ the compose work, yet `Composition Report` is a peer section
   (RC-2). (Here the metered compose is only 5.9ms because the costly `sleep` ran in the un-metered shell
   preflight — itself the P-5a finding above.)

### 6b. `claudine sequence <fixture> --perf --dry-run --yolo` (two scalar steps)

```
▌ Performance (elapsed 1.00s)
▌
▌ CLI Overhead
▌   pre-dispatch:           1.7ms
▌   prep phase:               0µs
▌   arg parsing:            1.5ms
▌   config loading:         848µs
▌   tracing init:           157µs
▌   environment setup:    972.2ms
▌   ═════════════════════════════
▌   TOTAL:         973.9ms
▌
▌ Composition Report
▌   …(17 stage rows; link resolve 954µs, link normalization 779µs dominate)…
▌   ═════════════════════════════════════
▌   TOTAL:                   2.4ms
▌
▌ Agent execution skipped (dry run)
```

Sequence specifics confirming TM-3 / the `prep phase: 0µs` gap: sequence never stamps `prep_phase`, so all
per-step compose + render work is swept into `environment setup` (972.2ms). The headline (`1.00s`) happens
to be near the env-setup-dominated TOTAL because `sequence_start` (`sequence.rs:58`) fires early, but the
`Composition Report` (2.4ms) still overlaps env-setup as a peer. Phase 3's `steps` Structural subtree and
the `sequence orchestration` vs `unattributed` decision (TM-3) target exactly this.

### 6c. Wrapper variant

**Not captured.** A direct provider wrapper (`claudine claude --perf …`) launches the real provider CLI,
requiring an installed + authenticated agent and network egress — unavailable in this non-interactive
session (and `--help` returns before any perf emit). The wrapper emit path (site 5, `wrap/mod.rs:334`) is
structurally identical to the composition sites and is covered by the Phase-6 integration coverage task;
its headline source (`wrapper_start.elapsed()`) was confirmed by code inspection above.

---

## Validation checkpoint

- [x] Six (seven-statement) emit sites listed with distinguishing context and their shared Phase-2 helper shape.
- [x] `StartupTimings` (`perf.rs:37`) + both `main.rs` construction sites (`264-270`, `302-308`) confirmed,
      `process_start` in scope at both, `prep_phase` provenance traced.
- [x] Both collector types, their env-setup timer start/close, and all `mark_env_setup_complete` call sites confirmed.
- [x] Substage chain (`:804-814`, zero-row `:1230`) confirmed on a **separate** clock from env-setup (RC-3).
- [x] P-5a prep-window work units enumerated with Phase-4 disposition; `shell approval` flagged as the
      dominant must-name child (else a >99% prep remainder).
- [x] Real baseline captured (compose dry-run + sequence dry-run) reproducing all three defects; wrapper
      variant documented as environment-blocked.
