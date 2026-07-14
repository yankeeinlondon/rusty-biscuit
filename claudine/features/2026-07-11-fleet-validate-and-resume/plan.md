---
agent: codex/
total_phases: 5
created: 2026-07-12
phase: 1
yolo: true
---

# Execution Plan — Fleet Validate-and-Resume Lifecycles

Derived from [`spec.md`](spec.md). This plan graduates the `agent-errors` pilot into
a reusable fleet-research authoring pattern only after its B2 checkpoint records a
positive decision. It deliberately excludes Claudine harness changes: lifecycle
`resume`, late-bound messages, capability gating, attempt budgets, and exhaustion
fallthrough already exist and have L2 coverage.

## Planning assumptions and boundaries

The duplicate `agent` requirement is resolved using its final requested value,
`codex/`. The implementation must copy the pilot's proven findings transport rather
than choose a competing design in advance. The recommended atomic JSON findings file
remains the default only if B1/B2 did not establish another successful transport.

No mass migration of existing `_fleet.md` files is in scope. Existing topics adopt
the shared checks only when each topic is next refreshed. Network-dependent source
liveness remains advisory and must never trigger `resume`.

## Phase 1 — Promotion gate and proven contract

- [ ] Confirm that `claudine/features/2026-07-11-provider-errors-as-data/spec.md` increments B1 and B2 are complete and that the Codex pilot artifacts, validation findings, and lifecycle telemetry are present; record their exact paths in this feature's implementation notes.
- [ ] Review the B2 checkpoint with the named owner and record an explicit **promote**, **revise and rerun**, or **do not promote** decision, including resume count, correction convergence, `max_attempts` fit, and the exhausted-budget outcome. Do not begin Phase 2 without a **promote** decision.
- [ ] Reproduce the pilot's contract from a clean checkout: run the checker against a valid document, an intentionally invalid document, and a corrected document; verify respectively empty/absent findings, deterministic findings, and stale-findings removal.
- [ ] Document the selected contract without redesigning it: checker invocation and arguments, baseline/seed inputs, findings location and schema, atomic replacement behavior, exit-status policy, lifecycle `when:` expression, late-bound resume message, and telemetry fields used to count attempts.
- [ ] Verify that the wrapper agent used for the pilot reports `supports_resume`; record that this is a property of the CLI running the research, not the provider represented by the fleet row.
- [ ] **Validation checkpoint:** demonstrate `invalid output → one resume → corrected output → no second resume`, then demonstrate exhaustion leaves a non-empty machine-readable finding and a non-zero fleet validation outcome for human review.

## Phase 2 — Reusable deterministic checks

- [ ] Create `claudine/docs/research/_checks/` and extract only the pilot-proven, topic-neutral checker code and contract; keep topic-specific coverage policy outside the shared core.
- [ ] Define a deterministic findings record that includes the check identifier, affected field/value, actionable correction text, and document path; make ordering stable so repeated runs over unchanged input are byte-identical.
- [ ] Implement seed/sticky-value preservation using an explicit immutable baseline input, including the update-mode case where selected prior-frontmatter values must survive a refresh unless the topic declares an allowed replacement.
- [ ] Implement provenance coherence for research records: evidence classes that require a source reject empty sources, and seed provenance rejects values absent from the declared seed/baseline.
- [ ] Provide a narrow topic-check extension point for coverage rules, then port the pilot's capacity/overload-or-gap rule through it to prove that topic-specific checks do not get hard-coded into the shared checks.
- [ ] Ensure every check run clears or atomically replaces stale findings before evaluation; a clean run must never consume findings from an earlier attempt, interrupted process, provider, or fleet row.
- [ ] Add fixture-driven tests under the shared-checks area for clean documents, each individual failure class, multiple failures with stable ordering, malformed input, paths containing spaces, interrupted/stale findings, and update-mode baseline preservation.
- [ ] Run the checker tests on macOS and add CI coverage for Windows and Linux using the pilot-selected runtime; do not introduce a Bash-only or platform-specific path contract.
- [ ] **Parallelizable:** after the findings schema is fixed, author the provenance fixtures and baseline-preservation fixtures concurrently; merge only after both suites emit the identical common record shape.
- [ ] **Validation checkpoint:** run the complete fixture suite twice and confirm identical output, no leftover temporary files, and no network access for any blocking check.

## Phase 3 — Canonical fleet authoring recipe

- [ ] Update `claudine/docs/research/_TEMPLATE.md` so its required skeleton shows the proven `success`-stack order: run the approved checker, conditionally `resume` with late-bound findings and a bounded `max_attempts`, then raise `error` on the still-present finding after budget fallthrough.
- [ ] In the template, distinguish topic-neutral checks (seed/prior-frontmatter preservation and provenance coherence) from a topic-owned coverage check, and require every new topic to state which sticky fields, seeds, provenance rules, and coverage classes apply.
- [ ] Document findings lifecycle rules in the template: static per-row path, cleanup before checking, atomic write, empty/absent success state, actionable resume text, and persistent machine-visible failure after exhaustion.
- [ ] Document operational constraints in the template: the wrapper must support resume; `shell` values are approved and early-bound; findings read by `when:` and the resume message are late-bound; non-zero checker exits must not stop the stack before `resume`; URL liveness is advisory.
- [ ] Update the topic-recipe section of `claudine/features/_completed/2026-07-02-provider-metadata/spec.md` to point to the shared checks and canonical template, preserving that document's historical decisions while clearly marking this later recipe graduation.
- [ ] Update `.claude/skills/claudine/SKILL.md` with the graduated fleet-research workflow and the shared-check location so the authoritative package skill no longer describes reporting-only lifecycle verification.
- [ ] Add a minimal non-production example/fixture `_fleet.md` that exercises the canonical stack without duplicating the full prose-first research template.
- [ ] **Parallelizable:** once Phase 2 freezes the contract, the `_TEMPLATE.md`, provider-metadata recipe, and Claudine skill updates may be drafted concurrently and reviewed together for terminology drift.
- [ ] **Validation checkpoint:** compose/parse the example and template-derived fixture with Claudine, verify all lifecycle actions pass preflight, and confirm no raw `{{ ... }}` span or stale finding reaches the resume prompt.

## Phase 4 — Incremental topic adoption

- [ ] Retain `agent-errors` as the reference implementation and reconcile its B1 checker/wiring to the shared location without changing its researched output or topic-specific capacity/overload rule.
- [ ] For the first existing topic that is independently scheduled for refresh, identify its immutable seeds or sticky prior-frontmatter fields, provenance-bearing records, and one mechanically decidable coverage class before editing its `_fleet.md`.
- [ ] Retrofit that topic's `_fleet.md` to invoke the shared checks and supply only its topic configuration/coverage rule; do not copy the checker implementation into the topic directory.
- [ ] Run the refreshed topic first with a deliberately failing fixture and then with a valid fixture; verify the same-session correction path converges and that a clean result does not resume.
- [ ] Record the topic's resume count, corrections, false positives, and any rule that required human adjudication; revise the shared pattern only for behavior demonstrated by both the pilot and this second topic.
- [ ] Leave all other existing fleet prompts unchanged and add their candidate checks to the normal refresh checklist rather than opening a migration sweep.
- [ ] **Validation checkpoint:** compare `agent-errors` and the second topic to prove they share invocation/findings semantics while retaining distinct coverage rules and research schemas.

## Phase 5 — End-to-end acceptance and closeout

- [ ] Run a resume-capable wrapper through the canonical example with a scripted bad-first/good-second provider response; assert one resumed invocation, the original session identifier is reused, findings appear in the follow-up prompt, and final findings are empty/absent.
- [ ] Run the same scenario with a provider that never corrects the document; assert attempts stop at the configured budget, the final findings persist, and the post-fallthrough `error` makes the fleet row fail for downstream B3/C1 or human review.
- [ ] Verify unsupported-wrapper behavior is explicit and actionable before a fleet starts; it must not be confused with lack of resume support in the provider being researched.
- [ ] Run the shared-check fixture suite, the package-area `just test`, and `just lint`; use `just test-l2` for the end-to-end lifecycle scenarios when they are housed in the Claudine L2 harness.
- [ ] Review every updated workflow comment and document against the implemented behavior, removing stale reporting-only language and avoiding duplicated field contracts outside `_schema.yaml`.
- [ ] Run `cargo fmt --check` only if Rust files unexpectedly enter scope; do not run write-mode formatting. If implementing the plan reveals missing lifecycle primitives, stop and amend the specification instead of silently adding harness work.
- [ ] **Final validation checkpoint:** map every specification scope item to an artifact or passing scenario—shared checks, findings transport, template recipe, provider-metadata recipe, incremental adoption, convergence, and visible exhaustion—and confirm `git diff` contains no unrelated fleet migrations.

## Completion criteria

The feature is complete when the B2 promotion is recorded, two topics demonstrate the
same reusable contract, the canonical template and authoritative Claudine skill teach
the pattern, blocking checks are deterministic across supported operating systems, and
both convergence and exhausted-budget behavior have observable automated coverage.
