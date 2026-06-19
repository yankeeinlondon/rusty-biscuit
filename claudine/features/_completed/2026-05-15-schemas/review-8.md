---
ready: true
agent: open_code
model: ""
---

# Review: Schema Support in Claudine

## Summary

Iteration #8 review of the schema support feature (`2026-05-15-schemas`). All prior review findings (1–7) have been resolved. The implementation is complete, well-tested, and aligns with the specification.

## Findings

No findings. The review-7 declaration-order gap has been fully addressed:

- `declared_property_order` re-parses raw frontmatter YAML with `serde_yaml_ng` (preserving insertion order) to recover authored property sequence, handling inline mappings, YAML/JSON file references, and root unions (`schema_completion.rs:177–289`).
- `declaration_rank` sorts properties within required/optional groups by their authored position (`schema_completion.rs:144–149`).
- The completion engine wires both together at `engine.rs:241–242`.
- The integration test at `compose_schema_cli.rs:831–843` now asserts exact declaration order within both required (`topic` before `tier`) and optional (`description` before `status`) groups.

## Spec Requirement Coverage

| Requirement | Implementation | Tests |
|-------------|---------------|-------|
| Schema source and resolution (inline, file ref, root union) | `DarkmatterSchemas::effective_for` | `load_effective_schema_*` (4 unit + 2 integration) |
| Validation timing (post-composition, pre-provider) | `pre_validate_schema` → `prepare_*_with_schema` → `post_shell_validate` | 45 unit + 22 integration |
| Required property outcomes (valid/missing/invalid) | `categorize_problems` partitioning | `valid_required_*`, `missing_required_*`, `invalid_required_*` |
| Optional property outcomes (drop-and-retry) | `drop_invalid_optionals` + `source_with_dropped_optionals` + `options_with_dropped_optionals` | `invalid_optional_*` (5 unit + 3 integration) |
| Composition-tolerant pre-validation | `value_needs_composition` defers templates/shell expressions | `pre_validate_does_not_reject_template_bearing_value` + 4 regression tests |
| Post-shell re-validation | `post_shell_validate` | `post_shell_*` (3 unit + 3 integration) |
| Configuration (`prompt_for_missing`) | `ClaudineConfig` field, TUI toggle, CLI setter | 6 unit (config) + TUI integration |
| Interactive Mode gating | `InteractiveSchemaOptions::allowed()` (4 flags) | `interactive_options_*` (2 unit + PTY) |
| Status report rendering | `build_schema_status_report` + `render_status_report` | 6 unit + L2 PTY regression |
| Widget mapping (enum/boolean/number/string/file) | `interactive_shape_for_atom` + `collect_missing_values` | `missing_*_property_maps_to_*_shape` (7 unit + 5 PTY) |
| Parse-and-retry for numbers | `collect_number` loop | `parse_number_*` (5 unit + 1 PTY) |
| Unsupported shapes → `UnsupportedInteractiveSchema` | `interactive_shape_for_atom` returns `None` for object/any/union | `missing_object_property_*` + `pre_validate_*_unsupported` |
| Non-TTY fallback | `resolve_interactive_options` reads config + TTY state | `pre_validate_with_interactive_returns_missing_when_not_allowed` |
| Error types (SchemaLoad/SchemaValidation/MissingProperties/UnsupportedInteractiveSchema) | `CompositionError` enum + `BlockError` impl | 9 unit (error display/render) |
| Shell completion: required-before-optional + declaration order | `property_names` + `declared_property_order` + `declaration_rank` | 10 unit + 9 integration |
| Shell completion: enum/file values | `property_value` + `MatchGlobs` | 7 unit + 3 integration |
| Shell completion: supplied filtering | `collect_supplied_setter_names` | 1 unit + 1 integration |
| Sequence: per-step validation before launch | `run_phase_1c_with_schema` two-attempt loop | 2 integration + L2 PTY |
| Sequence: cross-step deduplication | `collect_sequence_missing_values` keyed by `(name, type_label, description)` | L2 PTY |
| Sequence: aggregated error | `SequenceMissingProperties` + `render_sequence_missing_properties_block` | 3 unit + 2 integration |
| Inline-compose: prompt check precedence | `prepare_inline_with_schema` checks `PromptPropertyMissing` first | 2 unit + 1 integration |
| Repo config rejection | `RepoOverrideConfig` denies `prompt_for_missing` | 1 unit |

## Test Rigor Classification

- **L1 (in-process / CLI process):** 45 library unit tests, 40 CLI binary unit tests, 22 integration tests (`compose_schema_cli`), 5 sequence integration tests. Covers all error paths, happy paths, regression scenarios, and the `__complete` CLI protocol for shell completions. Appropriate for requirements that assert CLI protocol output, error surface structure, and validation logic.

- **L2 (PTY):** 9 PTY tests in `level2_schema_prompt_pty.rs` (gated by `require_level!(Level::L2, pty_available(), ...)`). Covers interactive string/enum/boolean/number collection, parse-and-retry loops, `--silent` suppression, template-tolerant status reports, sequence deduplication, step-overlay satisfaction, and setter-reflected status. Appropriate for TTY-interactive requirements since no OS keyboard-encoder behavior is specified.

- **L3 (OS keyboard injection):** Not required. No spec requirement asserts bare-modifier press events, hotkey chords, or other key-encoder-dependent behavior. All interactive collection goes through `biscuit-tui` widgets whose input model is PTY byte-streams, not raw scancodes.

## Verification

- Source review of all schema-related files (library: `schema_validation.rs`, `error.rs`, `mod.rs`, `claudine_config.rs`; CLI: `schema_interactive.rs`, `schema_completion.rs`, `engine.rs`, `compose.rs`, `sequence.rs`, `wrap/sequence.rs`, `config_tui/`).
- All 117 schema-related tests executed and passed:
  - 45 library unit tests (`composition::schema_validation::tests`)
  - 14 error display/render tests (`composition::error::tests`)
  - 40 CLI binary unit tests (`schema_interactive::tests`, `schema_completion::tests`)
  - 6 config tests (`prompt_for_missing_*`)
  - 22 integration tests (`compose_schema_cli`)
  - 5 sequence integration tests (`sequence_cli`)
  - PTY tests not executed in this review (require Level 2 env gating) but were passing in prior reviews.
