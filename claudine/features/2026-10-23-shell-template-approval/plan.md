---
total_phases: 6
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - claudine
    - claudine-cli
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/matching.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/approval.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shell_template_discovery.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/cli/src/approval.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/audit.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/coordinator/tests.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/shell_missing_values.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/docs/topics/pre-flight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/cli-commands.md
---

# Shell Template Approval Implementation Plan

## Phase 1 — Establish Scope and Contracts

### Necessary Rules

Implement reusable typed shell approval across Darkmatter composition and Claudine single-document, sequence, and lifecycle execution. The work spans placeholder-aware parsing, effective-schema lookup, policy persistence and matching, structured approval transport, current-state validation, direct argument dispatch, both approval prompts, and documentation. Success means all 17 specification acceptance criteria have observable coverage, authorization never widens through caching or late substitution, and the existing `just test` and `just lint` gates pass in both package areas. Native recording executables must prove argument boundaries on macOS, Linux, native Windows, and WSL2. The terminal state is implementation complete, ready for review.

1. **Dependency readiness.** `2026-10-16-core-schema-types` must provide its resolved string-like types, canonical options, and public covers relation before dependent implementation begins. Its plan currently records Phase 2 in progress; the presence of alphanumeric source code does not establish completion. Use the dependency's API rather than implementing a second subtype relation here.
2. **No new product rulings.** The specification has no open questions. The rules below resolve implementation ordering and boundaries, not changes to its decisions. Record any subsequently discovered conflict in the implementation log before dependent work proceeds; do not silently amend the specification.
3. **Namespace timing.** Use the current outputs spelling until `2026-10-24-sequence-containers` lands. This feature does not introduce the `seq`/`group` namespace migration, new container surfaces, or group/sequence loops. Carry static position and source ownership through existing group traversal; support nested sequence surfaces only to the extent they exist when this feature starts, and record any acceptance case dependent on a later surface explicitly rather than claiming it passed.
4. **Approval is not availability.** Deferral includes only the outputs accumulator, `loop.*`, and `group.*`, including indirect dependencies. Other statically known `seq` fields remain approval-time inputs. `err`, `timing`, and `current.*` remain refused. A compound expression containing a forbidden root is refused even when another dependency is deferrable. Do not declare Darkmatter-owned reserved roots in Claudine's `BindingView`; runtime lexical availability remains independently enforced.
5. **Two execution modes.** Resolved-byte execution preserves fixed approved bytes and each existing runner. Template execution preserves authored expression nodes and validates original scalar values before conversion and direct dispatch. Exact approvals never cover template-required candidates. Known-value single-document/first-task candidates prefer exact approval, then template, then prefix; an explicit shape choice selects template mode.
6. **Two validation barriers.** Before a sequence step has any shell effects, check all late values used by that step's discovered commands. Check other template values immediately before their command against current state, using the lifecycle pre-write snapshot for `set`. Validate every segment and complete-chain policy before the first segment launches. Earlier steps' effects stand; this feature adds no rollback. Choose and test how the step barrier retains resolved late values so each dispatch does not evaluate a placeholder twice; it must not freeze other mutable event-time values.
7. **Schema provenance.** Only a bare top-level property or `fm.property` uses the owning document's `EffectiveSchema.simplified`, with named types resolved and constraints retained. Other expressions and reserved roots use fallback string conversion. Unsupported declaration types disclose a broad string approval hole while retaining their original validator; raw JSON Schema validation remains effective. Do not add the later expression-type projection.
8. **Operational contracts.** Structured dispatch retains working directory, environment, output routing, deadlines, cancellation, and process-tree cleanup. Reject interpreter-dependent template targets, including implicit script and `.cmd`/`.bat` fallback, before dispatch. Exact-mode grammar and existing explicit approval-flag policy remain unchanged. Initialization and early handler shells stay forbidden under every mode.
9. **Bounded verification.** No performance spike: the specification explicitly rules it out for bounded parsing, matching, and scalar validation. No additional OS benchmark or measurement campaign is scheduled. All new acceptance tests belong in existing L1 suites; no new CI cell. Scripted approval tests use injected input/output, not real terminal focus.
10. **Reader scope.** The changed configuration reader is the existing line-based whitelist/blacklist loader. Its robustness matrix appears below. Authored templates and type expressions receive grammar tests, not JSON/YAML configuration matrices. This feature consumes existing parsed schema/frontmatter data and does not redesign those loaders; if implementation changes a structured reader, add its actual per-format, per-load-bearing-field matrix before making that change.

### Wave 1 — Contract Review

- [x] **Dependency Gate**
  - Confirm the core-schema dependency's implementation and tests expose the covers relation required by specification section 7; name the API in the implementation log.
  - Inventory today's sequence/group surfaces and map future-name examples to their current equivalents. Record deferred integration cases without expanding this feature's scope.
  - Prerequisite for Phase 2: dependency behavior is available and verified, not merely planned.

- [x] **Boundary Inventory**
  - Trace Darkmatter `compose/preflight/{collect,approval,lifecycle}.rs`, `shell_expansion/{types,tokenize,policy,store,executor,mod}.rs`, inline expansion, shell blocks, and frontmatter expansion.
  - Trace Claudine `composition/preflight.rs`, `sequence/preflight/{mod,shape}.rs`, prepared task/document storage, `prepare.rs`, `types.rs`, CLI compose preparation, lifecycle shell and `set` runners, and `harness/shell.rs`.
  - Identify every byte-only `HashSet<String>` approval, result-cache key, and error-suppression boundary. Record affected consumers and comment drift, especially the current assertion that all sequence commands execute preflight-resolved bytes.
  - This inventory can run concurrently with Dependency Gate; it does not modify shared source.

### Wave 2 — Shared Design

- [x] **Contract Definition**
  - After Wave 1, define the minimal shared Darkmatter representations for authored argument segments, expression spans, resolved declaration/validator, approval type, canonical shape, source owner/site, execution mode, and filled chain with ordered redirects.
  - Define structured membership proof and mode-tagged approval keys scoped to policy root; specify result keys separately using filled argv boundaries, operators, redirects, and execution context. Use biscuit-hash if hashing is needed.
  - Define candidate eligibility, missing/invalid/unsupported-template typed errors, step-barrier ownership, and runner adapters. Keep schema/type/matching/value-validation authority in Darkmatter; retain operational execution ownership in each runner.
  - Produce an acceptance-to-task map and file ownership boundaries for the later parallel waves. Shared `types.rs` and exports have one owner; adapters consume the agreed API rather than editing it concurrently.

**Checkpoint:** Dependency readiness is explicit; both execution modes and validation barriers have documented contracts; each acceptance criterion maps to a task below. No spike is scheduled or repeated later.

## Phase 2 — Build Template and Policy Primitives

### Wave 3 — Shared Template Model

- [x] **Template Grammar**
  - Depends on Phase 1. Extend Darkmatter tokenization to recognize complete expression spans before shell operators/quotes, keeping literals and holes within their containing word.
  - Preserve empty arguments, chain operators, and ordered curated redirections. Reject program-word holes in every chain segment, holes in redirect targets/operators, unsupported pipes/semicolons/background syntax, and command substitution in template mode.
  - Implement the canonical one-line literal encoding and decoding before hole recognition: quotes, backslashes, escaped braces, newline/carriage-return/tab, and rejection of other stored control characters.
  - Add the shared types and exports agreed in Phase 1. Verify native and stored representations round-trip without merging or splitting arguments; unsupported grammar remains eligible for exact mode only where the specification permits it.

### Wave 4 — Independent Policy Work

- [x] **Type Projection**
  - Depends on Wave 3. Resolve bare properties against the owning effective schema, including child schemas, inheritance, local setters, and resolved named types.
  - Keep original validators and constraints separate from persisted approval types. Implement scalar fallback conversion, the broad-string approval exception, and covers matching through the dependency API.
  - Test every covers-table row and provenance/fallback case, including numeric and union declarations, container rejection, numeric values under alphanumeric declarations, and unsupported narrower pairs even when identical.

- [x] **Policy Persistence**
  - Depends on Wave 3; can run concurrently with Type Projection in `store.rs` and store tests using the shared type interface.
  - Load and append `template` rules beside unchanged exact/prefix rules; canonicalize parsed shapes/options for deduplication without persisting runtime values or document-local named definitions.
  - Skip malformed template lines with file-and-line warnings, including constraints with or without a semicolon, unknown options, unresolved names, unsupported rule types, and program-word holes. A malformed line must not invalidate unrelated valid rules.
  - Preserve current policy-root resolution. A failed append remains typed policy I/O failure; it cannot become persistent success or implicit once approval.
  - Walk the policy robustness matrix through public composition approval results and captured diagnostics; retain explicit grammar round-trip tests.

### Input Robustness Matrix

The supported policy format is UTF-8, line-based text, read independently from `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist`. Its load-bearing fields are the rule keyword and its payload (command, executable, or template shape). The matrix targets new template entries; exact/prefix compatibility is covered separately. `S` means skip the malformed line and warn with file/line; `A` means the control whitelist approves the target; `D` means the control blacklist denies it; `N` means this entry contributes no decision. Null and collection syntax below are literal text, not typed JSON/YAML values.

| Shape / fixture edit | Whitelist keyword | Whitelist payload | Blacklist keyword | Blacklist payload |
| --- | --- | --- | --- | --- |
| Control: unedited persisted template | A | A | D | D |
| Absent: omit field | S → N | S → N | S → N | S → N |
| Explicit null: replace field with `null` | S → N | Literal fixed program `null`; cannot cover control command | S → N | Literal fixed program `null`; cannot deny control command |
| Wrong type, whole field: collection syntax | S → N | S → N for malformed template syntax | S → N | S → N for malformed template syntax |
| Wrong type, one element: replace one hole type with `123` | Not a collection field; malformed keyword S → N | S → N for entire line, no retained holes | Not a collection field; malformed keyword S → N | S → N for entire line, no retained holes |
| Wrong type, every element: replace all hole types with `123` | Not a collection field; malformed keyword S → N | S → N for entire line | Not a collection field; malformed keyword S → N | S → N for entire line |
| Empty: empty field / blank document | S → N / no rules | S → N / no rules | S → N / no rules | S → N / no rules |
| Duplicate: repeat same rule line | Set semantics; A once | Parsed-shape deduplication; A once | Set semantics; D once | Parsed-shape deduplication; D once |
| Trailing or invalid content | Extra token in keyword is malformed: S → N | Valid extra argument changes shape and gives N; invalid syntax gives S → N | Extra token in keyword is malformed: S → N | Valid extra argument changes shape and gives N; invalid syntax gives S → N |

Missing policy files intentionally define an empty rule set; missing template payloads do not. Whitespace-only/comment-only files intentionally contain no entries. A repeated rule is a valid set duplicate, not a forbidden mapping key. A separate malformed trailing line is skipped with a warning while prior valid entries remain; additional complete valid lines are valid document content. Unreadable files and invalid UTF-8 produce policy-I/O failure rather than empty rules.

Use one table-driven matrix test for this format, parameterized over whitelist and blacklist, starting from a file produced by the real persistence API. Apply one edit per cell, include the positive control, and observe public approval/denial plus warning/failure. Place valid exact/prefix sentinel lines alongside malformed template rows to prove they survive. Audit touched reader paths for `#[serde(default)]`, absent/null-collapsing `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and swallowed `.ok()` parses; document legitimate absence-to-empty behavior only where the format defines it.

### Wave 5 — Policy Integration

- [x] **Policy Matching**
  - Depends on both Wave 4 tasks. Match whitelist templates by argument positions, literals, holes, operators, ordered redirects, and covers; match blacklist templates by shape regardless of hole types.
  - Add blacklist matching against filled argv, including resolved-byte candidates without authored holes. Retain exact and prefix rules and existing flag policy.
  - Check fixed program/literal positions at preflight without deleting holes or shifting subcommand positions. Check complete filled chains before any segment starts.
  - Table-test quoting/whitespace equivalence and refusal on changed literals, extra arguments, moved holes, changed operators, and redirect ordering. Prove `git {{ string }}` cannot authorize a runtime `reset` or evade an exact blacklist.

**Checkpoint:** Canonical round trips, covers cases, malformed policy handling, and whitelist/blacklist matching pass. No unresolved shape/type contract blocks downstream consumers. Acceptance criteria 3–5 and the primitive portions of 13, 14, and 17 are covered.

## Phase 3 — Preserve Discovery and Authorization

### Wave 6 — Discovery and Approval

- [x] **Discovery Provenance**
  - Depends on Phase 2. Retain authored expressions, spans, owner schema, source site, and declared late dependencies throughout Darkmatter's condition-blind discovery, including transcluded children and frontmatter commands.
  - Track indirect deferred dependencies through derived frontmatter/task parameters without evaluating them or running effects. Separate host-declared deferral from runtime lexical availability using the existing binding/discovery seams.
  - Check approval-time presence before any effects: null/absent produce one typed missing-value error; empty string, zero, false, and present fallback expressions succeed. Preserve parse/unavailable-binding errors and non-shell null rendering.
  - Preserve static includes/generated-content restrictions, pending shell-dependency refusals, and effect-dependent expression refusals. Add no executable discovery path.

- [x] **Approval Transport**
  - Depends on Phase 2 and the Wave 2 interface; can run concurrently with Discovery Provenance in approval/cache/prepared-option files. The discovery owner publishes candidates through the agreed interface.
  - Replace byte-only pre-approved sets with explicit resolved-byte/template approvals across Darkmatter options, Claudine preparation/service layers, prepared graphs, lifecycle shell values, and CLI handoff/re-audit paths.
  - Implement candidate-specific choices, precedence, typed decision variant, mode/policy-root/type-tagged cache keys, and existing concurrent reservation/failure-release semantics.
  - Require proof of membership in the original authored approved candidate before filling runtime values; never insert freshly resolved bytes to manufacture membership. Ensure persistence failure releases waiters and leaves no successful cached approval.
  - Keep result caching separate, keyed by filled structured commands and execution context. Changed values rerun; changed types rematch; changed constraints revalidate.

**Checkpoint:** Integrate Wave 6 and compile all affected consumers. In-memory contract tests prove effect-free discovery, missing-value timing, ownership, mode isolation, policy-root isolation, and failed-reservation recovery. Fix drifted comments asserting every approval is a string of resolved bytes. Acceptance criteria 1, 12, 15, and 16 have their discovery/transport foundations.

## Phase 4 — Execute Against Current State

### Wave 7 — Independent Runtime Integration

- [ ] **Shared Validation**
  - Depends on Phase 3. Implement Darkmatter's shared template resolver/validator and structured filled-chain dispatch interface, consumed by all adapters.
  - Evaluate each placeholder once, validate its original value and constraints, convert supported scalars with the existing conversion engine, and concatenate within one argument. Reject null, arrays/objects, and embedded NUL before spawn.
  - Validate complete chains and runtime blacklists before their first segment. Enforce approved membership before authorization can reach result-cache lookup or spawning.
  - Wire body directives, shell blocks, and frontmatter expansion; preserve stage order. Keep typed authorization/input failures unsuppressible by shell fallback, `when_error`, or transclusion notices; diagnostic text names expression/type/location without exposing values.
  - Build reusable portable native recording/child fixtures with the Test Toolkit and runtime binary lookup for downstream tests. Pin executable resolution to a direct-dispatch path that refuses interpreter-dependent targets without fallback.

- [ ] **Sequence Position**
  - Depends on Phase 3; can run concurrently with Shared Validation using the agreed runtime API, in sequence preflight/shape/prepared-task code.
  - Carry static first-task eligibility recursively: a first action qualifies, all legs of a first group qualify, a first nested sequence qualifies only through its own first child. Later descendants stay templates; skipped predecessors and retry/resume do not change position.
  - Treat container terminal events and all `group.*` readers as templates. Preserve source owner and task parameters when preparing child documents and task stacks.
  - Introduce the step-entry late-value barrier before any step shell commands run, including commands on conditional branches. Retain parse-only documents but invalidate runtime-filled prepared compositions; use current task/inherited state for later preparation.
  - Keep approval-time presence checks for known inputs in later tasks and defer only allowed dependencies. Remove only the specified late-root `SequenceShellLateBinding` refusals.

### Wave 8 — Runner and Prompt Adapters

These tasks depend on both Wave 7 tasks. They can run concurrently with separate file ownership; shared interface changes go through the Phase 1 contract owner.

- [ ] **Sequence Dispatch**
  - Add structured template mode to `composition/sequence/task/shell.rs` and its calling task runner, using the shared validation/membership/policy path.
  - Preserve native process-tree establishment and teardown, stream/capture separation, timeout budgets, volume caps, cancellation, and bounded reader settlement. Direct dispatch must not bypass Unix process-group or Windows Job Object ownership.
  - Test chain short-circuit behavior after whole-chain validation, argument boundaries, deadline/cancellation cleanup, and a blocked later chain segment preventing all spawns.

- [ ] **Lifecycle Dispatch**
  - Add template mode to lifecycle shell and whole-value `$(...)` set execution. Route single-document compose/inline-compose/wrapper preparation and loop/retry/resume/proxy re-audits through structured approvals.
  - Evaluate ordinary holes against current event state and set holes against the existing pre-write snapshot; retain approval-time captured `ctx.*`/`env.*` snapshots.
  - Preserve resolved-byte commands exactly, update the disabled runner to refuse every structured path, and preserve initialization/early-handler prohibition and unsuppressible typed failures despite `no_error` or timeout fallback.
  - Test reusable shape approval across three loop iterations and refusal of changed exact bytes on iteration 2, plus lifecycle mutation visibility and changed constraints.

- [ ] **Approval Prompts**
  - Update `darkmatter/cli/src/approval.rs` and Claudine's installed handler in `lib/src/harness/shell.rs` to display expression/type, original-validator disclosure when broader string approval is used, and only eligible choices.
  - Implement all persistence rows from specification section 8, chain executable lists, typed write failure, no-terminal refusal, and once semantics for each mode.
  - Render with biscuit-terminal components. Use injected scripted input/output for tests; no real providers, focused windows, or interactive test requirements.

**Checkpoint:** End-to-end stub runs exercise all shell surfaces in both modes, native argv boundaries, current state, fixed first-task bytes, and step-entry failure before effects. Approval-time and runtime blacklists remain effective through cache/pre-approved paths. Acceptance criteria 1–2 and 6–17 can now be verified at public boundaries.

## Phase 5 — Verify Acceptance and Portability

### Wave 9 — Independent Acceptance Suites

- [ ] **Execution Corpus**
  - Depends on Phase 4. In Darkmatter-owned test files, cover the store/matching/covers corpus, missing-value behavior, type provenance, direct argv substitution, ordered redirections, and all invalid grammar/target cases.
  - Assert `a b`, shell-looking values, quotes, backslashes, Unicode, newlines, percent/exclamation signs, empty words, adjacent holes, and NUL through native recorded argv or explicit pre-spawn refusal. Avoid tests that merely assert a parser result.
  - Verify original declared-value validation and secret-free error chains, not just the top-level message. Include malformed persisted constraints with and without semicolons and the policy matrix.

- [ ] **Orchestration Corpus**
  - Can run concurrently with Execution Corpus in Claudine-owned test files. Use `CliProcessFixture`, private CWD/home, fake providers, and child-local `PLAYA_DRY_RUN`/private spool; assert no unintended publication or leaked children.
  - Cover the spec's late-output and build/clean scenarios, no-terminal failure before step 1, later known-value template approval, caller null failures, and missing/invalid late values preventing an earlier command in the same step from running.
  - Cover changed task/event/child values, indirect deferral, static group/nested positions, terminal events, skipped predecessors, retry/resume, and parse-cache reuse boundaries available in today's traversal.
  - Verify unchanged exact/prefix corpus, forbidden roots, initialization, approval/result cache separation, concurrent approval recovery, and runtime blacklist denial after cache hits.

### Wave 10 — Validation Gate

- [ ] **Package Gates**
  - After Wave 9, run `just test` and `just lint` from `darkmatter/` and `claudine/`; use nextest through these recipes, never `cargo test`. Do not run `cargo fmt` or commit.
  - Use targeted tests while developing, then the required area gates once integrated. Compile affected reverse consumers of changed public APIs according to actual blast radius; do not run a workspace-wide suite or add CI coverage cells.
  - Exercise the portable native argument and runner-contract cases on macOS, Linux, native Windows, and WSL2 through existing cross-check mechanisms. Load the OS skill and select configured build hosts; cross-compilation alone does not prove argv behavior.
  - Record exact environment, test identity, evidence, and any unresolved blocker. Missing platform evidence or a later-surface dependency remains visibly unmet rather than silently waived. Keep existing CI scheduling unchanged.

**Checkpoint:** Map each of the 17 criteria to passing test identities and platform evidence, or an explicit unresolved dependency/blocker. No failure has been suppressed, no real provider launched, and no terminal/browser gained focus.

## Phase 6 — Document and Hand Off

### Wave 11 — Documentation and Audit

- [ ] **Public Documentation**
  - Depends on implemented behavior and Phase 5 findings. Update `darkmatter/docs/inline/preflight-checks.md`, `claudine/docs/topics/pre-flight-checks.md`, and the sequence/lifecycle topic pages, both area READMEs, and both area skills.
  - Explain exact versus shape choices with compact examples, type matching/disclosure, missing versus empty values, first versus later task approval, late-value timing, direct-dispatch restrictions, canonical policy syntax, blacklists, and authorization versus result caching.
  - Add a Mermaid approval-to-runtime flow showing the first-task branch and the step-entry/command-entry barriers. Describe current behavior without linking to or naming feature/fix snapshots in topic docs.
  - Update behavior-changing symbol comments, especially approved-byte invariants in sequence preflight/task runners. Update dependency documentation only if crates actually changed; remove any planned markers for behavior now shipped.

- [ ] **Acceptance Audit**
  - Can run concurrently with Public Documentation against separate review/log files. Audit all 17 criteria and all nine requirements against implementation, tests, matrix, typed diagnostic propagation, and recorded OS evidence.
  - Review each robustness finding across its whole row and column; search for bypasses that interpolate then tokenize, manufacture exact approvals, drop partial argument positions, re-read ambient environment, reuse filled prepared state, or swallow policy/validation failures.
  - Record departures and outstanding dependency cases in `implementation-log.md`, leaving the finalized specification unchanged. If findings require code changes, rerun the affected tests/gates before marking completion; documentation alone cannot close missing behavior.

- [ ] **Review Handoff**
  - Depends on both Wave 11 tasks. Verify changed files are scoped to this feature, all required evidence is recorded, and plan checkboxes reflect actual completion.
  - Report implementation complete, ready for review only when all required behavior and evidence are satisfied. Leave the feature in its active directory; the author closes review and moves it to `_completed`.

**Checkpoint:** Current documentation agrees with tested behavior, public API/comment drift is resolved, and the review package contains explicit acceptance and portability evidence without changing the specification snapshot.
