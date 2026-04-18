---
phases: 6
created: 2026-04-17
start_phase: 2
source_files_during_phase_2:
  - claudine/cli/src/argv.rs
  - claudine/cli/src/commands/compose.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages:
  - claudine-cli
---
# Execution Plan — CLI Argv Pre-Processing

Source: [`spec.md`](./spec.md)

Validated against the current implementation in:

- `claudine/cli/src/main.rs`
- `claudine/cli/src/args.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/provider_values.rs`
- `claudine/cli/tests/{command_routing,sequence_cli,wrap_commands,wrap_direct_argv}.rs`

The current tree already has variadic composition positionals and shorthand
setter parsing in `compose.rs` / `sequence.rs`, so this feature should land as
an argv-layer change above clap, not as another round of command-local parsing.

## Phase Index

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Parsing seam, scanner contract, and test homes are locked | none |
| 1 | `argv::normalize` exists and is the single pre-clap entrypoint | 0 |
| 2 | Provider flag/value normalization is canonicalized to `--provider <slug>` | 1 |
| 3 | Composition-only `--` insertion fixes interleaved setter/help parsing | 1 |
| 4 | End-to-end regressions and docs cover the new contract | 2, 3 |
| 5 | Acceptance criteria are verified at package level | 0-4 |

## Phase 0 — Lock The Real Parsing Seam

1. Confirm the three argv ingress points that must stop diverging:
   the `--plain` pre-scan in `main.rs`, the wrapper-aware `parse_cli()` path in
   `main.rs`, and the composition clap surfaces in
   `commands/{compose,sequence}.rs`.
   Observable result: there is one agreed implementation seam above clap, and
   no command-local parser changes are planned beyond downstream cleanup.
2. Define the token-scanning contract for `argv.rs` before editing:
   operate on `Vec<OsString>`, decode UTF-8 opportunistically, stop rewriting at
   the first literal `--`, and treat `COMPLETE` as a hard no-op guard.
   Observable result: the normalizer scope is explicit enough to code without
   re-deciding edge cases mid-implementation.
3. Decide test homes up front:
   unit tests live in `claudine/cli/src/argv.rs`,
   new integration coverage goes in `claudine/cli/tests/argv_normalization.rs`,
   and existing regression suites in `wrap_direct_argv.rs`,
   `wrap_commands.rs`, and `sequence_cli.rs` remain the unchanged-behavior backstop.
   Observable result: every acceptance case has a destination before code moves.

Validation checkpoint:

- `cargo test -p claudine-cli --test command_routing`
- `cargo test -p claudine-cli --test wrap_direct_argv`

## Phase 1 — Introduce `argv::normalize` And A Single Parse Entry

1. Add `claudine/cli/src/argv.rs` and wire `mod argv;` from `main.rs`.
   Start with `pub(crate) fn normalize(raw: Vec<OsString>) -> Vec<OsString>` plus
   private helpers for:
   `completion_mode_active()`, token equality against ASCII flags, first-`--`
   detection, UTF-8 access, and subcommand discovery.
   Observable result: all pre-clap logic has a dedicated module instead of being
   embedded inside `main.rs`.
2. Refactor `main.rs` to collect `std::env::args_os()` once and reuse that same
   vector for:
   the `--plain` pre-scan,
   the call to `argv::normalize(...)`,
   and the final parse path.
   Observable result: there is no second argv read that can disagree with the
   normalized parse path or panic on non-UTF-8 input.
3. Replace `Cli::parse()` / `std::env::args()` usage in `parse_cli()` with a
   `parse_cli_from(argv: &[OsString])` style helper that parses from the already
   normalized argv.
   Observable result: both wrapper and non-wrapper flows see the same normalized
   token stream.
4. Replace the current `raw_args.get(1)` wrapper-subcommand check with the new
   subcommand scanner so global flags before the subcommand do not bypass the
   lenient wrapper path.
   Observable result: wrapper detection and composition-subcommand detection use
   one scanner instead of two inconsistent heuristics.
5. Keep Phase 1 behaviorally neutral apart from the new shared ingress: the
   normalizer should still be a no-op for ordinary inputs until rewrite rules
   land in later phases.
   Observable result: the refactor can be validated independently from the rule logic.

Parallelizable work:

- Steps 1.1 and 1.4 can move in parallel once the helper signatures are stable.
- Step 1.5 is the immediate regression check after the refactor compiles.

Validation checkpoint:

- `cargo check -p claudine-cli`
- `cargo test -p claudine-cli --test wrap_direct_argv`

## Phase 2 — Canonicalize Provider Inputs (Rules 1 And 2)

1. Add the provider-boolean rewrite table in `argv.rs`:
   `--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`,
   `--qwen`, and `--roo` each rewrite to `--provider <Provider::as_slug()>`.
   Observable result: clap sees one canonical provider flag regardless of which
   user-facing sugar token was typed.
2. Implement left-to-right rewrite of provider booleans before the first `--`,
   preserving duplicates exactly as typed.
   Observable result: ambiguous user input turns into duplicate `--provider`
   tokens and clap remains the source of the final error.
3. Implement fuzzy canonicalization for `--provider <value>` and
   `--provider=<value>` using `Provider::fuzzy_match_cli_name`.
   Leave missing, empty, hyphen-prefixed, and unknown values untouched so clap
   keeps its native missing-value and invalid-value errors.
   Observable result: `cl`, `gem`, `oc`, and similar shorthands become canonical
   slugs before clap validation runs.
4. Simplify `SharedComposeArgs::explicit_provider()` so runtime provider
   selection reads the canonical `provider` field rather than re-resolving eight
   boolean fields on every composition path.
   Observable result: the argv layer becomes the sole translation point from
   provider sugar to provider identity.
5. Add dense unit coverage for:
   each provider boolean,
   duplicate provider booleans,
   boolean-plus-`--provider`,
   fuzzy `--provider` value matches,
   pass-through unknown values,
   and non-goals like near-miss flags (`--claud`) staying untouched.
   Observable result: Rules 1 and 2 are locked down before Rule 3 complicates the scan.

Parallelizable work:

- Steps 2.1 and 2.3 can be implemented in parallel inside `argv.rs`.
- Step 2.4 can land as soon as the normalized runtime path is compiling.

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 3 — Insert `--` For Interleaved Composition Setters (Rule 3)

1. Extend the argv scanner with composition-subcommand recognition for
   `compose`, `inline-compose`, and `sequence`, while still honoring root global
   flags before the subcommand and the first user-provided `--` boundary.
   Observable result: Rule 3 is gated to the exact three subcommands named in the spec.
2. Encode the composition flag surface that consumes a following value
   (`--provider`, `--exclude`, `--include`, `--model`, `--output`,
   `--append-system-prompt`, `--replace-system-prompt`, `--timeout`,
   `--operation`, `--set`, `--use`, plus short forms like `-m`, `-o`, `-t`)
   so the scanner can distinguish a flag value from a positional token.
   Observable result: setter insertion logic does not misclassify flag values as
   candidate setters.
3. Implement the Rule 3 state machine after the composition subcommand:
   track whether a positional has been seen,
   track whether at least one flag or flag-value occurred after that positional,
   detect setter-shaped tokens with the same key pattern used by
   `parse_compose_setter`,
   and insert exactly one `--` immediately before the first qualifying setter.
   Observable result: `claudine compose file.md --gemini name=Ken --help`
   becomes a clap-safe argv without changing the common `file.md key=val` case.
4. Keep explicit no-op behavior for:
   existing `--`,
   non-composition subcommands,
   trailing non-setter positionals,
   no-positional-yet cases,
   and non-UTF-8 tokens.
   Observable result: Rule 3 is narrow by construction and cannot bleed into wrapper passthrough.
5. Add unit tests for every Rule 3 case from the spec plus one real-tree
   regression the spec does not spell out explicitly:
   root globals before the subcommand, such as
   `claudine --plain compose file.md --gemini name=Ken --help`.
   Observable result: the implementation matches the documented matrix and the
   actual clap surface in this repository.

Parallelizable work:

- Step 3.2 can be prepared while Step 3.1 finalizes the scanner API.
- Step 3.5 can be written in parallel with Step 3.3 once the state machine shape is fixed.

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 4 — Add End-To-End Coverage And Documentation

1. Add `claudine/cli/tests/argv_normalization.rs` with the three headline
   integration cases from the spec:
   `compose <file> --gemini name=Ken --help`,
   `compose --provider cl <file> --dry-run`,
   and `compose <file> key=val` preserving current behavior.
   Observable result: the feature is proven from the binary entrypoint, not only
   from `normalize(...)` unit tests.
2. Create each integration test’s markdown input in a temp workspace rather than
   introducing a permanent fixture unless a reusable composition fixture already
   emerges naturally during implementation.
   Observable result: the new suite stays local to the feature and avoids
   coupling to unrelated test assets.
3. Add regression assertions for pass-through behavior that the normalizer must
   not change:
   `claudine --version`,
   `claudine --help`,
   `claudine hooks --describe`,
   wrapper passthrough in `wrap_direct_argv.rs`,
   and completion-mode no-op behavior via a scoped `COMPLETE=zsh` unit test.
   Observable result: unchanged commands remain unchanged in the same phase that
   adds the new success cases.
4. Write `claudine/docs/topics/argv-normalization.md` documenting:
   the three rewrite rules,
   the pass-through guarantees,
   the first-`--` stop rule,
   completion-mode no-op behavior,
   and at least one before/after example for each rule.
   Observable result: the normalization contract is discoverable without reading code.
5. If implementation details or public usage text in
   `claudine/docs/topics/composition.md` become inaccurate during the refactor,
   update them in the same change set.
   Observable result: docs do not drift on provider-selection or positional-argument behavior.

Parallelizable work:

- Integration tests and the docs topic can proceed in parallel once Phases 2 and 3 compile.
- Pass-through regression updates can be done independently from the new headline tests.

Validation checkpoint:

- `cargo test -p claudine-cli --test argv_normalization`
- `cargo test -p claudine-cli --test wrap_commands`
- `cargo test -p claudine-cli --test sequence_cli`

## Phase 5 — Final Acceptance Pass

1. Run the full package test suite after the focused checks pass.
   Observable result: unit tests, integration tests, and any existing snapshots
   agree on the final argv behavior.
2. Run the package-level recipe for the area:
   `cd claudine && just test`.
   Observable result: the feature passes the package’s preferred validation path,
   not only ad hoc cargo commands.
3. Manually smoke-test at least these commands with a harmless markdown file:
   `claudine compose @file.md --gemini name=Ken --help`,
   `claudine compose --provider cl @file.md --dry-run`,
   `claudine --plain compose @file.md --gemini name=Ken --help`,
   and `claudine compose @file.md key=val --dry-run`.
   Observable result: the motivating bug, fuzzy provider shorthand, root-global
   flag interaction, and unchanged common case all behave as expected from a user shell.
4. Verify that wrapper passthrough still behaves identically by re-running the
   direct-argv regression tests after the final merge.
   Observable result: normalization fixes composition parsing without corrupting
   wrapped provider argv.

Release gate:

- Do not consider the feature complete until the binary-level help case passes,
  provider fuzzy normalization is canonicalized to a valid slug before clap,
  composition setter insertion is covered by unit and integration tests,
  and pass-through commands remain unchanged under both normal and `COMPLETE`
  environments.
