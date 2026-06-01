---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Not ready for production. The iteration fixes several review-2 issues, especially by moving missing-property handling ahead of shell preflight, preserving `inline-compose` prompt-property errors, and strengthening non-interactive sequence aggregation assertions. A new blocker remains: the preflight workaround now validates raw frontmatter before Darkmatter composition, which rejects prompts that the spec explicitly allows to become valid only after templates, environment values, transclusions, or step overlays are applied.

## Findings

### High: schema pre-validation rejects raw frontmatter instead of Darkmatter's effective frontmatter

- Requirement: schema validation must run after Darkmatter composition has produced `PreparedComposition::effective_frontmatter`, because templates, transclusions, sequence overlays, and env-injected values can change frontmatter before validation.
- Implementation: `compose` and `inline-compose` call `pre_validate_with_interactive_collection` immediately after source resolution, before eager target resolution and before `AGENT`/other env overrides are installed for composition. `sequence` also calls `pre_validate_schema` before building the per-step `ComposeOptions` with env overrides.
- Evidence: early direct validation is wired at [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:379) and inline at [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:768). The validator explicitly operates “BEFORE Darkmatter's compose pipeline” and validates `build_effective_instance` from raw frontmatter plus overrides at [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/schema_validation.rs:762) and [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/schema_validation.rs:825). Sequence builds `AGENT` at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/wrap/sequence.rs:799) but then calls `pre_validate_schema` without those env overrides at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/wrap/sequence.rs:809).
- Reproduction: a prompt with `$schema: { runtime_agent: 'enum(goose; required)' }` and `runtime_agent: '{{ env.AGENT }}'` fails before provider launch with `/runtime_agent: "{{ env.AGENT }}" is not one of "goose"` under `claudine compose --goose`, even though the composed frontmatter should see `AGENT=goose`.
- Impact: valid prompts that derive schema-constrained fields from composition context are rejected. This violates the feature's central ordering rule and can affect direct compose, inline compose, and sequence.
- Fix direction: avoid hard validation before composition. The preflight phase needs a schema-tolerant shell-discovery path, or a pre-validation mode limited to cases that are provably composition-independent. Final required/invalid decisions should be made against the same composed frontmatter used by `prepare_*`.

### High: interactive schema UX still lacks the required PTY coverage

- Requirement: missing required values prompt only when `prompt_for_missing` is enabled, stdin and stderr are TTYs, and `--silent` is off; widgets must collect strings, enums, booleans, and numbers with parse-and-retry.
- Verification present: helper/unit tests cover option flags, unsupported shapes, parsing, and rendering snippets. Process tests cover non-interactive `MissingProperties`, but no schema test spawns the CLI under a pseudo-TTY and feeds input through the actual prompt workflow.
- Required level: Level 1 PTY is the minimum for this terminal I/O behavior. If exact status-report styling/glyph rendering is contractual, that output needs Level 2 real-terminal capture.
- Evidence: schema tests are ordinary CLI process tests in [compose_schema_cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/tests/compose_schema_cli.rs:30), and the existing PTY test files do not cover schema prompting.
- Impact: regressions in stdin/stderr TTY gating, `--silent`, prompt cancellation, enum/boolean selection, or numeric invalid-then-valid retry can ship.
- Fix direction: add Level 1 PTY tests for direct compose string, enum, boolean, and numeric retry collection, plus `--silent` and non-TTY denial. Add Level 2 only for styled status output if the glyph/color contract remains acceptance-level behavior.

### Medium: schema-aware completion still lacks command and file-path integration coverage

- Requirement: schema-aware completion applies to `compose`, `inline-compose`, and `sequence`; enum and `file(match(...))` values use Darkmatter completion metadata.
- Verification present: CLI integration covers `compose` property ordering, enum values, and supplied-property filtering.
- Gap: there is still no CLI integration test proving `inline-compose` and `sequence` route through the schema-aware completer, and no CLI integration test for `file(match(...))` value candidates.
- Evidence: schema completion integration assertions in [compose_schema_cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/tests/compose_schema_cli.rs:460) all call `compose`; the existing `completion_inline_compose` and `completion_sequence` tests cover prompt-file discovery, not schema setters.
- Fix direction: add Level 1 completion tests for `inline-compose`, `sequence`, and `file(match('*.ext'))` candidates through the `__complete` surface.

## Test Rigor Classification

- Direct `compose` non-interactive missing required: Level 1 process coverage exists and now asserts typed `CompositionError`.
- `inline-compose` prompt-property precedence: Level 1 process coverage exists.
- `sequence` non-interactive aggregation: Level 1 process coverage exists and asserts per-step aggregation.
- Invalid optional setters: Level 1 unit/process coverage exists.
- Effective-frontmatter-after-composition requirement: missing coverage; add Level 1 tests with templated/env-derived schema values.
- Interactive prompting: strongest coverage is in-process unit tests; needs Level 1 PTY before production readiness.
- Status report styled rendering: semantic unit coverage exists; needs Level 2 only if exact styled output is required.
- Shell completion: partial Level 1 CLI coverage for `compose`; gaps remain for `inline-compose`, `sequence`, and file completions.

## Verification Run

- Manual CLI reproduction confirmed the pre-composition validation bug for `runtime_agent: '{{ env.AGENT }}'`.
- I attempted a second raw JSON Schema reproduction, but stopped it because changing `HOME` caused Cargo to rebuild dependencies under the temp home and exceeded the non-interactive time budget. I did not use that run as evidence.
