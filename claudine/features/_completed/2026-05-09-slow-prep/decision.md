# Phase 3 Decision: Compose Pass Reuse

## Feature

2026-05-09-slow-prep

## Measurement

The motivating trace (`trace.md`) was captured from:

```sh
RUST_LOG=trace c compose prompts/implement-phase.md \
  plan="features/2026-05-08-expression-syntax/plan.md" \
  -y --claude total_phases=6
```

### Relevant Timing Landmarks

| Time | Event | Duration |
|------|-------|----------|
| `11:17:21.610` → `11:17:21.656` | `biscuit-file` file reference resolution | ~46 ms |
| `11:17:21.659` | First `sniff::filesystem::git::discover` close | ~295 µs |
| `11:17:22.225` | Second `sniff::filesystem::git::discover` close | ~374 µs |
| `11:17:22.922` → `11:17:23.507` | First dynamic model-catalog subprocess (`opencode models`) | ~585 ms |
| `11:17:23.508` → `11:17:24.076` | Second dynamic model-catalog subprocess (`opencode models` for Qwen) | ~568 ms |
| `11:17:24.079` | Third `sniff::filesystem::git::discover` close | ~389 µs |
| `11:17:24.079` → `11:17:24.087` | OS, hardware, repo detection | ~8 ms |
| `11:17:24.089` → `11:17:24.121` | ignore-aware repo walk logs | ~32 ms |
| `11:17:24.239` → `11:17:24.241` | Two `compose_with` runs (preflight + final) | ~2 ms total |

### Shell Preflight vs. Final Preparation

The trace shows the two Darkmatter composition passes (shell-command discovery via `collect_shell_commands()` and final `prepare_direct()`) together consumed approximately **2 ms** at the end of the prep pipeline. Each individual pass is therefore **sub-millisecond**.

This is consistent with the spec's observation:

> "The trace confirms two full composition passes. In the captured run they were sub-millisecond, so this is not the dominant cause here, but it is still duplicate work on every command."

After Phase 1 (elimination of global catalog refresh) and Phase 2 (shared source-root discovery), the remaining prep time is dominated by:

1. File reference resolution (`biscuit-file` workspace discovery) — ~46 ms
2. Dynamic catalog refresh for selected providers (when applicable) — ~500–600 ms per subprocess
3. ignore-aware repo walk during environment detection — ~32 ms

The two-pass composition is **not material** relative to these costs.

## Decision

**Do not add a reuse path between `collect_shell_commands()` and final `prepare_*()`.**

### Reasoning

1. **Measured cost is below the decision gate threshold.** The combined two-pass cost is ~2 ms, which is well under both the 50 ms absolute threshold and the 10 % of remaining prep time relative threshold.

2. **Reuse would introduce correctness risk for marginal gain.** A reusable composed document would need to guarantee:
   - Identical interpolation state (same env vars, same `set` overrides)
   - Identical approved-command set (shell preflight must not change between discovery and final prep)
   - Same source file and transclusion graph
   - Same Darkmatter compose options

   Violating any of these would risk running final preparation with stale or incorrect shell-expansion state. The safety checks needed to guarantee equivalence would likely cost more than the 2 ms being saved.

3. **The two-pass design is intentional and well-understood.** Darkmatter's `collect_shell_commands()` performs a dedicated discovery pass with explicit shell-expansion policy, while `prepare_direct()` / `prepare_inline()` perform the full composition with all transformations. Keeping these separate preserves the security boundary between *discovery* and *execution*.

4. **Future instrumentation is in place.** All major prep phases now have `tracing` spans:
   - `compose_prep.file_reference`
   - `compose_prep.prep_context`
   - `compose_prep.source_repo_root`
   - `compose_prep.selection_config`
   - `compose_prep.installed_clients`
   - `compose_prep.eager_target`
   - `compose_prep.model_catalog`
   - `compose_prep.shell_preflight`
   - `compose_prep.prepare_direct`
   - `compose_prep.prepare_inline`
   - `compose_prep.environment`

   If future traces show the composition-pass cost growing (e.g., due to much larger documents or slower filesystems), the spans make the regression immediately visible, and the reuse path can be revisited with fresh data.

## Equivalence Conditions (Documented for Future Reuse)

Should the measurement change in the future and a reuse path become worthwhile, the following conditions must be met for a preflight-composed document to be safely reused by final preparation:

1. **Same `ComposeOptions`** — all fields (`set_overrides`, `source_file`, `env_overrides`, `pre_approved_commands`) must be bitwise-equal.
2. **Same source `Markdown`** — the source document and any transcluded documents must not have changed on disk.
3. **Same shell-expansion state** — the `pre_approved_commands` set used during discovery must be exactly the set passed to final preparation.
4. **No harness plan changes** — if harness pre/post checks contain shell commands, those must also be covered by the preflight pass.

## Action Items

- [x] Add `compose_prep.model_catalog` tracing span around catalog refresh.
- [x] Add `compose_prep.environment` tracing span around environment detection.
- [x] Add `compose_prep.prepare_inline` tracing spans for inline-compose paths.
- [x] Document decision and equivalence conditions in `decision.md`.
- [ ] Revisit this decision if a future trace shows two-pass composition exceeding 50 ms or 10 % of total prep time.
