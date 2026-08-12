# Baseline: Pre-Feature Lifecycle Notification Behavior

Phase 1 artifact. Records the *current* behavior of top-level
`start` / `success` / `blocked` / `failure` notifications so later phases
can prove they did not regress it.

Captured from `claudine/lib/src/composition/lifecycle.rs` and the
end-to-end execution path in
`claudine/cli/src/commands/wrap/composition/mod.rs`.

## Lifecycle Signal Set

`LifecycleSignal` has exactly four variants today:

| Variant   | `property_name()` | `status_state()`     |
|-----------|-------------------|----------------------|
| `Start`   | `start`           | `StatusState::Info`  |
| `Success` | `success`         | `StatusState::Success` |
| `Blocked` | `blocked`         | `StatusState::Error` |
| `Failure` | `failure`         | `StatusState::Error` |

There is **no** `Initialize`, `Finalize`, or `Loop` variant. These are
the variants Phase 2 must add.

## LifecycleNotification Fields

`LifecycleNotification` carries six optional string fields. It is
`#[serde(deny_unknown_fields)]`, so any other key is a parse error
(routed through `CompositionError::LifecycleInvalid`):

| Field       | Purpose                                                    |
|-------------|------------------------------------------------------------|
| `say`       | TTS via host's speech provider; mutually exclusive with `say_first` |
| `say_first` | TTS via host's speech provider; mutually exclusive with `say` |
| `effect`    | Sound effect name; validated against `playa::SoundEffect::from_name` |
| `message`   | Dispatched through the configured messenger route          |
| `stderr`    | Styled status line to STDERR via `biscuit-terminal::Status` |
| `notify`    | OS desktop notification                                    |

Empty / whitespace-only strings normalize to `None` at parse time.

The lifecycle property is rejected when:

- both `say` and `say_first` are present → `LifecycleSayConflict(property)`
- `effect` is set but not a known sound effect → `LifecycleUnknownEffect(property, name)`

## LifecycleConfig

`LifecycleConfig` carries four optional `LifecycleNotification` slots
matching the four signals above. It is also
`#[serde(deny_unknown_fields)]`. `LifecycleConfig::get(signal)` returns
the notification for that signal or `None`. `is_empty()` returns `true`
when all four slots are `None`.

Parse entry point: `parse_lifecycle_config(frontmatter, source_file)`.

## Audio Phase Ordering

`audio_phases(notification)` builds the ordered audio fan-out. The
ordering rules are:

| Configuration              | Phase 1   | Phase 2   |
|----------------------------|-----------|-----------|
| `say` only                 | Speak     | —         |
| `say_first` only           | Speak     | —         |
| `effect` only              | Effect    | —         |
| `say` + `effect`           | Effect    | Speak     |
| `say_first` + `effect`     | Speak     | Effect    |
| neither                    | (empty)   | —         |

So `say` + `effect` plays the effect first; `say_first` + `effect`
plays speech first. This ordering must survive Phase 4's stack
execution work.

## Per-Signal Emission Order

`emit_signal` (used by both `LifecycleRunGuard` and the deprecated
`emit_lifecycle_signal` free function) emits in this order:

1. **stderr** (always, even on interrupt)
2. short-circuit on `crate::interrupt::interrupted()` — skip the rest
3. **message**
4. **notify**
5. **audio phases** in the order above (TTS config built lazily from
   `GlobalSettings::tts`)

Each audio phase also short-circuits on a fresh interrupt check.

## LifecycleRunGuard Drop Safety Net

`LifecycleRunGuard::drop` re-emits a terminal signal when `start` was
emitted but no explicit terminal was called:

| `start_emitted` | `provider_launched` | Drop emits |
|-----------------|---------------------|------------|
| `false`         | —                   | nothing    |
| `true`          | `false`             | `Blocked`  |
| `true`          | `true`              | `Failure`  |

`emit_terminal`, `emit_blocked_or_failure`, and `defuse` all suppress
the Drop emission. Phase 5 must preserve (or mechanically replace)
this state machine when it adds the new control-flow actions.

## Composition CLI Integration

`claudine/cli/src/commands/wrap/composition/mod.rs` is the single
execution pipeline. The lifecycle-relevant steps are:

1. Build `LifecycleConfig` from prepared frontmatter inside `prepare_direct` / `prepare_inline`.
2. In `execute_composition_request_inner`, load `GlobalSettings` and `RuntimeMessagingSettings` (skipped when `lifecycle.is_empty()`).
3. Construct `LifecycleRuntimeContext` and `LifecycleRunGuard`.
4. Harness plan parse failure or shell-audit denial → `guard.emit_blocked_or_failure()` (no provider invocation).
5. `--dry-run` runs the harness pre-checks then renders; never launches the provider; emits `blocked` on pre-check failure.
6. `run_harness_loop` owns the guard for the live path; the outer guard is `defuse`d.
7. The harness loop emits `Start` immediately before provider invocation and `Success`/`Failure` based on the agent exit code.

This integration shape is what Phase 5's runtime flow changes must extend
without breaking. In particular:

- lifecycle chatter stays off stdout (stderr / messaging / TTS / sound / desktop notification only);
- no provider invocation happens on a `blocked` outcome;
- `LifecycleRunGuard` remains the mechanical safety net for terminal signals.

## Existing Tests That Pin This Baseline

Located in `claudine/lib/src/composition/lifecycle.rs`:

- `parses_valid_lifecycle_config`
- `rejects_both_say_and_say_first`
- `trims_empty_strings_to_none`
- `rejects_unknown_keys`
- `rejects_unknown_effect_name`
- `say_plus_effect_is_valid`, `say_first_plus_effect_is_valid`
- `empty_frontmatter_returns_default`, `non_object_frontmatter_returns_default`
- `null_lifecycle_property_is_skipped`
- `frontmatter_with_non_lifecycle_keys_is_fine`
- `audio_order_*` (five tests covering the full audio phase matrix)
- `status_state_mapping`, `property_names`
- `lifecycle_config_get`, `lifecycle_config_is_empty`
- `guard_emits_start_once`, `guard_drop_emits_blocked_before_launch`,
  `guard_drop_emits_failure_after_launch`, `guard_drop_silent_without_start`,
  `guard_emit_terminal_prevents_drop_emission`

Located in `claudine/lib/src/composition/prepare.rs`:

- `direct_composition_parses_lifecycle_config`
- `inline_composition_parses_lifecycle_config`
- `invalid_lifecycle_config_fails_preparation`
- `malformed_lifecycle_interpolation_fails_preparation`
- `undefined_lifecycle_variable_fails_preparation`
- `lifecycle_variable_defined_in_frontmatter_passes_preparation`
- `lifecycle_fallback_for_undefined_variable_passes_preparation`
- `clean_lifecycle_interpolation_passes_preparation`
- `lifecycle_leak_reported_for_first_field_in_deterministic_order`
- `direct_lifecycle_ctx_agent_uses_env_overrides`

Phase 7 ("Backward Compatibility") must keep every assertion above
passing byte-for-byte or behavior-for-behavior for top-level-only prompts.
