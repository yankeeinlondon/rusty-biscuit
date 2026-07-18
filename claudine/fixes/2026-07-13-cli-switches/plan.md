---
total_phases: 10
created: 2026-07-16
phase: 1
agent: codex/default
yolo: "true"
---

# CLI Switch Forwarding Execution Plan

This plan implements the functional specification while accounting for the partial Phase 1 implementation already present in the working tree. Existing behavior must be characterized before it is changed so the implementation team can distinguish retained work from gaps and avoid regressing direct wrapper behavior.

## Dependency and parallelism map

| Phase | Depends on | Parallelization |
|---|---|---|
| 1 | None | Sequential baseline |
| 2 | Phase 1 | Sequential contract foundation |
| 3 | Phase 2 | Can run alongside Phases 4 and 6 |
| 4 | Phase 2 | Can run alongside Phases 3 and 6 |
| 5 | Phases 3 and 4 | Sequential integration |
| 6 | Phase 2 | Can run alongside Phases 3–5 |
| 7 | Phases 2–6 | Sequential Phase 1 gate |
| 8 | Phase 2 contract terminology | Can run alongside Phases 3–7 |
| 9 | Phase 8 | Sequential generation work |
| 10 | Phases 7 and 9 | Sequential final integration |

## Phase 1 — Baseline audit and characterization

### Tasks

- [ ] Record the current partial implementation in the phase notes, including the argv partitioner, request-level provider arguments, generic announcement, dry-run redaction, and dormant native-argument classifier; identify each specification requirement that remains incomplete rather than rebuilding completed work.
- [ ] Inventory every construction and consumption site for composition execution requests, child argv, retry/proxy/resume launches, sequence steps, status output, dry-run output, debug traces, metadata, correlated reports, and completion parsing; record the owning module and expected data flow for each site.
- [ ] Run GitNexus upstream impact analysis immediately before editing each affected symbol and capture the direct callers, affected execution flows, and risk level in the implementation notes. Treat `partition_composition_tail` and `build_harness_launch` as known HIGH-risk gates and warn before changing either; rerun analysis if symbol names or boundaries change.
- [ ] Add or confirm characterization tests for direct wrapper argv, composition lifecycle event ordering, terminal exit-code propagation, and exactly-once reporting so later refactors have an observable compatibility baseline.
- [ ] Confirm the existing compiled-binary test helpers can create deterministic fake provider executables on macOS, Windows, and Linux; extend only the helper capabilities required by this feature.

### Validation checkpoint

- [ ] Run `cd claudine && just test` and record the baseline result, including any unrelated pre-existing failures, before production code changes begin.
- [ ] Review the audit against all fourteen acceptance requirements in the specification and confirm every requirement has at least one planned implementation task and one planned validation path.

## Phase 2 — Canonical provider-tail contract and ownership partition

### Tasks

- [ ] Introduce one typed provider-tail descriptor shared across composition paths, containing the exact ordered arguments and whether the tail began implicitly or at an authored `--`; replace parallel `Vec<String>` and boolean fields so source semantics cannot drift.
- [ ] Preserve exact argument order and token boundaries from normalization through launch assembly, documenting and testing the intentional handling of non-UTF-8 `OsString` input at the normalization/partition boundary on supported platforms.
- [ ] Derive the Claudine-owned switch surface from clap metadata for the active composition subcommand instead of maintaining a handwritten list or taking a union across compose and sequence commands.
- [ ] Partition argv before clap parsing so the file operand must appear before the first implicit unowned provider switch, an authored `--` after the file begins an opaque tail, and an authored `--` before the file returns the targeted structural error.
- [ ] Keep Claudine-owned switches owned before an authored boundary even after an implicit provider tail has begun; ensure setter-shaped values following the implicit tail remain provider arguments rather than mutating Claudine state.
- [ ] Preserve clap-compatible handling for long switches, `--name=value`, short switches, clustered/attached short values where supported, help/version behavior, and missing owned-switch values.
- [ ] Require an authored `--` for bare provider operands while preserving the existing multiple-composition-file error for two ordinary positional operands.
- [ ] Add a drift test proving the partitioner’s owned switch surface matches clap’s active command definition and cannot silently diverge when a CLI switch is added or removed.

### Validation checkpoint

- [ ] Run focused unit tests for implicit forwarding, authored opaque tails, pre-file errors, setter ownership, collisions, bare operands, multiple files, help/version, missing values, and non-UTF-8 boundaries.
- [ ] Verify byte-for-byte-equivalent provider argv for representative short, long, equals-form, repeated, and value-shaped tail tokens.
- [ ] Re-run upstream impact analysis after the partition refactor and confirm affected callers and flows match the Phase 1 inventory before proceeding.

## Phase 3 — Launch assembly, sequencing, and recovery paths

### Tasks

- [ ] Thread the typed provider-tail descriptor through every composition request constructor, inline composition path, sequence iterator, and provider-specific execution request without reconstructing or reparsing it.
- [ ] Seed each child launch from the provider tail before applying Claudine’s resolved entrypoint, model, transport, system-prompt, MCP, and prompt requirements; keep MCP argument injection independently observable.
- [ ] Apply provider capability and conflict validation to the resolved provider while leaving forwarded tokens untouched.
- [ ] Ensure retry and proxy attempts reuse the same immutable launch basis so every attempt receives the exact provider tail once and in the same position.
- [ ] Replace resume’s hardcoded passthrough whitelist with profile-aware, resume-aware argv assembly that carries the typed provider tail and required Claudine safety/transport arguments without duplicating either.
- [ ] Ensure sequence and multi-provider execution apply the same tail to each resolved provider step, while provider-specific entrypoints and required arguments remain step-local.
- [ ] Preserve exit codes, interruption semantics, retry policy, and lifecycle event ordering across the launch-assembly refactor.

### Validation checkpoint

- [ ] Add focused tests for exact child argv ordering and single insertion across direct composition, sequence, retry, proxy, resume, and multi-provider execution.
- [ ] Confirm resume retains arbitrary forwarded switches and values that were previously outside the hardcoded whitelist.
- [ ] Re-run impact analysis for `build_harness_launch` and its replacement boundaries; do not advance if unexpected HIGH/CRITICAL callers or execution flows appear.

## Phase 4 — Shared reporting and redaction

> Parallelizable with Phase 3 after Phase 2 is complete.

### Tasks

- [ ] Define shared provider-tail presentation helpers used by direct wrappers and composition so switch-name extraction, source labeling, status rendering, and redaction have one behavior path without changing direct wrapper argv.
- [ ] Render the implicit-tail INFO message with `TerminalRenderable` components and neutral wording that does not claim the provider switch is unknown; render an authored-boundary tail as opaque.
- [ ] Replace process-global announcement deduplication with execution-scoped state so each distinct provider/tail pair is announced once per command across sequences, loops, retries, and proxies, without leaking state between invocations or tests.
- [ ] Suppress the generic INFO announcement under quiet and silent modes while preserving normal status/event behavior.
- [ ] Apply the existing shared argument redactor to every fuller-value surface, including debug traces, dry-run tables, `AGENT_PARAMS` or equivalent metadata, correlated diagnostic excerpts, and any structured execution diagnostics.
- [ ] Verify redaction affects presentation and metadata only; the actual child process must receive the original unredacted tokens.

### Validation checkpoint

- [ ] Test once-only INFO behavior for repeated attempts and repeated provider/tail pairs, distinct messages for distinct pairs, and quiet/silent suppression.
- [ ] Test that INFO contains switch names but no values and that secrets are absent from debug, dry-run, metadata, and diagnostic output while still reaching the fake provider unchanged.
- [ ] Run direct-wrapper characterization tests and confirm both argv and existing output semantics remain compatible except for intentionally shared classification/reporting behavior.

## Phase 5 — Typed native exit correlation

### Tasks

- [ ] Introduce a typed native-exit result that carries exit status, termination context, and bounded stdout/stderr tails from the terminal attempt without altering the child process lifecycle.
- [ ] Define classifier precedence so stronger causes—missing executable, authentication, timeout/interruption, API failure, and model failure—win over argument rejection.
- [ ] Narrow `ArgumentRejected` recognition to provider-backed signatures and known collision fixtures, considering both stdout and stderr and avoiding generic false positives.
- [ ] Correlate an argument-rejected exit with a non-empty forwarded tail even when the opaque tail contains no switch-shaped token; keep the diagnostic explicitly probabilistic rather than asserting causality.
- [ ] Redact all quoted or excerpted arguments while retaining enough provider-native output to make the report actionable.
- [ ] Wire the shared correlation path exactly once into composition and direct wrapper reporting, replacing the dormant helper path without emitting duplicate reports.
- [ ] Preserve the provider’s exit code, signal/interruption handling, retry decisions, and lifecycle event order after the typed result is introduced.

### Validation checkpoint

- [ ] Add positive provider-specific rejection and collision fixtures, negative near-miss fixtures, stronger-cause precedence fixtures, stdout-only and stderr-only fixtures, and opaque bare-tail fixtures.
- [ ] Assert exactly one correlated report, no secret leakage, unchanged exit semantics, and no correlation when the forwarded tail is empty.
- [ ] Run focused direct-wrapper and composition tests to prove both call the same classifier/reporting path without changing child argv.

## Phase 6 — Completion, help, and Phase 1 documentation

> Parallelizable with Phases 3–5 after Phase 2 is complete.

### Tasks

- [ ] Make completion’s tolerant cursor scan consume the same clap-derived owned switch metadata as the partitioner, removing the handwritten value-bearing switch list.
- [ ] Stop Claudine completion after an authored `--` and remain non-failing after an implicit provider tail begins; do not attempt provider-native completion during Phase 1.
- [ ] Preserve composition-file and Claudine setter completion before the provider tail, including incomplete owned switch values and cursor-local partial tokens.
- [ ] Verify help collisions remain Claudine-owned before the authored boundary and are forwarded after `--`, with deterministic behavior for `--help`, `-h`, and their provider-tail equivalents.
- [ ] Generate or test any collision reference from the same clap metadata source so documentation and completion do not establish a second ownership registry.
- [ ] Update CLI pre-parsing, argv normalization, composition, wrapper, and output documentation to describe ownership partitioning, opaque authored tails, setter semantics, redaction, and correlation.
- [ ] Remove stale Rule 3 and synthetic-separator wording from code comments, READMEs, and the Claudine skill documentation, assuming current code behavior is authoritative where comments have drifted.

### Validation checkpoint

- [ ] Add completion tests for pre-tail file/setter suggestions, implicit-tail tolerance, authored-boundary stop behavior, collisions, partial values, and malformed provider tails.
- [ ] Run documentation examples or doctests affected by the terminology changes and confirm no documentation promises Phase 2 metadata enrichment during Phase 1.

## Phase 7 — Phase 1 compiled-binary and quality gate

### Tasks

- [ ] Add compiled-binary integration cases using deterministic fake providers for compose, inline-compose, and sequence headline forwarding, asserting exact argv and setter-shaped tail non-application.
- [ ] Cover authored boundaries, pre-file errors, owned collisions, bare operands, multiple composition files, short/equal forms, and help behavior at the binary boundary.
- [ ] Cover sequence, retry, proxy, resume, and multi-provider propagation, including once-only insertion and provider-specific entrypoint ordering.
- [ ] Cover INFO deduplication and suppression, redaction across every fuller-value surface, typed argument-rejection correlation, stronger-cause precedence, exact-one reporting, and exit-code preservation.
- [ ] Re-run the direct wrapper exact-argv suite to establish the specification’s no-argv-change guarantee.
- [ ] Review production/test placement against the Claudine architecture thresholds, keeping tokenizer tests colocated and moving larger classifier or integration suites to sibling test modules or `claudine/cli/tests` as prescribed.

### Validation checkpoint

- [ ] Run `cd claudine && just test`.
- [ ] Run `cd claudine && just test-l2`.
- [ ] Run `cd claudine && just lint`.
- [ ] Record a Phase 1 acceptance matrix proving requirements 1–13 pass before enabling Phase 2 runtime enrichment.

## Phase 8 — Phase 2 research schema and provider-fleet backfill

> Parallelizable with Phases 3–7 once Phase 2 has fixed the runtime terminology; this work must not affect argv routing.

### Tasks

- [ ] Extend the researched `cli_switches` schema with a canonical flag, explicit `aliases`, a `value_arity` enum of `none`, `one`, `optional`, or `variadic`, and normalized invocation applicability while retaining the human-oriented `value`, `scope`, description, examples, and notes fields.
- [ ] Represent invocation applicability as normalized exact native command paths plus an explicit global scope, allowing a lookup to match the resolved effective entrypoint without interpreting prose.
- [ ] Update the research prompt/instructions to prohibit inferring arity from `value` or notes and to require authoritative provider help or documentation for every canonical flag, alias, arity, and invocation claim.
- [ ] Backfill all ten provider research documents, splitting combined flag spellings into canonical flags and aliases and recording ambiguous or unsupported applicability as unknown rather than guessing.
- [ ] Add schema validation for valid arity values, canonical/alias shapes, normalized invocation paths, and required descriptions while allowing human-oriented fields to remain descriptive.

### Validation checkpoint

- [ ] Validate all ten provider research documents with `md schema validate` using the updated sidecar schema.
- [ ] Add and run positive and negative schema fixtures for aliases, each arity, global and command-scoped applicability, malformed invocations, and prohibited inferred metadata.
- [ ] Review the Codex research entry against authoritative help and confirm `-c` is represented as an alias of `--config` with researched arity and invocation applicability.

## Phase 9 — Catalog types and generator projection

### Tasks

- [ ] Add leaf catalog vocabulary for CLI switch value arity and serializable switch metadata, keeping shared enums in `claudine-catalog-types` and provider lookup structures in the Claudine library.
- [ ] Extend `ProviderInfo` with generated CLI switch metadata and update registry parsing, provider-source coercion, and Rust/catalog emitters end to end.
- [ ] Validate canonical flags and aliases, non-empty descriptions, explicit arity, normalized invocations, and canonical/alias uniqueness within an invocation scope; permit the same spelling only when applicability is provably disjoint.
- [ ] Sort emitted switch and alias metadata deterministically so regeneration is stable across operating systems and source ordering.
- [ ] Add generator and library lookup tests for global, exact-command, disjoint-scope, duplicate, ambiguous, and unknown cases, including Codex `-c` resolving to canonical `--config`.
- [ ] Regenerate every committed provider `data.rs` file and catalog artifact with the repository generator rather than editing generated files by hand.

### Validation checkpoint

- [ ] Run `cargo run -p claudine-gen -- generate --yes` from the repository root and inspect the generated diff for only expected provider metadata changes.
- [ ] Run `cargo run -p claudine-gen -- check` and require a clean drift report.
- [ ] Run the catalog-types, Claudine library, generator, and registry-coverage tests, including all negative validation fixtures.

## Phase 10 — Phase 2 runtime enrichment and final acceptance

### Tasks

- [ ] Add a read-only metadata lookup keyed by resolved provider and normalized effective native invocation, returning known canonical, unknown, or ambiguous results without participating in argv ownership or routing.
- [ ] Enrich implicit-tail INFO and argument-rejection diagnostics with researched canonical spelling and description when the lookup is unambiguous; use neutral unknown wording otherwise and keep authored-boundary tails opaque.
- [ ] Route direct wrappers and composition through the same lookup and presentation path while preserving each provider step’s effective entrypoint in multi-provider sequences.
- [ ] Add invariant tests proving metadata additions, removals, aliases, arity changes, ambiguity, and invocation-scope changes cannot alter the forwarded child argv.
- [ ] Update provider metadata, CLI behavior, and troubleshooting documentation to distinguish structural Phase 1 routing from advisory Phase 2 enrichment.
- [ ] Complete a cross-platform review of argv token preservation, executable fixtures, path handling, output capture, and deterministic generation for macOS, Windows, and Linux; rely on CI for unavailable hosts.
- [ ] Run GitNexus change detection against `main` before any commit and verify only expected symbols and execution flows changed; investigate any unexpected process impact before handoff.

### Validation checkpoint

- [ ] Run `cargo run -p claudine-gen -- check`.
- [ ] Run `cd claudine && just check`.
- [ ] Run `cd claudine && just test`.
- [ ] Run `cd claudine && just test-l2`.
- [ ] Run `cd claudine && just lint`.
- [ ] Run `cd claudine && just doctest`.
- [ ] Run `cargo fmt --check` as a read-only diagnostic; do not run formatting in write mode.
- [ ] Complete the final acceptance matrix for all fourteen specification requirements, including direct-wrapper compatibility and the invariant that Phase 2 metadata never changes argv.

## Acceptance traceability

| Specification requirement | Implementation phases | Primary validation |
|---|---|---|
| 1–6: routing, boundaries, ownership, and operands | 2 | Unit and compiled-binary cases in Phases 2 and 7 |
| 7: sequence/retry/proxy/resume/multi-provider | 3 | Launch assembly tests and Phase 7 L2 cases |
| 8–9: INFO behavior and redaction | 4 | Presentation, secret-leak, and Phase 7 L2 cases |
| 10: typed correlated native error | 5 | Classifier precedence and exact-one report cases |
| 11: shared direct-wrapper path without argv change | 4–5 | Baseline and Phase 7 direct-wrapper exact-argv suite |
| 12: completion behavior | 6 | Cursor and compiled completion cases |
| 13: documentation and Rule 3 revision | 6 | Documentation review and doctests |
| 14: researched metadata and Codex alias | 8–10 | Schema, generator, lookup, and argv-invariance tests |

## Completion criteria

- [ ] All phase checkpoints pass, all fourteen acceptance rows have recorded evidence, and no unresolved HIGH or CRITICAL impact warning remains.
- [ ] The implementation changes only the intended Claudine symbols, generated provider metadata, tests, and documentation; unrelated working-tree changes remain untouched.
- [ ] No production routing decision depends on researched alias or arity metadata, and direct wrapper child argv remains identical to its Phase 1 baseline.
