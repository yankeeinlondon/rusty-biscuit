---
total_phases: 5
created: 2026-08-12
phase: 5
agent: codex/default
yolo: true
packages:
  - darkmatter
  - claudine
---

# Execution Plan: Materialize Unset Optional Parameters

This plan implements the behavior defined in `spec.md`: a top-level, optional,
no-default parameter declared by a document-owned SimplifiedSchema becomes an
explicit `null` binding during composition. The change remains confined to the
schema-validation stage; validation-only APIs stay passive, baseline and
trigger schemas remain policy-only, and undeclared roots continue to fail
closed.

## Completion Notes

- Darkmatter has no public schema-validation operation switch. The regression
  therefore exercises the public no-effective-schema boundary with document,
  baseline, and trigger schemas absent or disabled.
- Darkmatter deliberately defines `null == null` as false. Public expression
  coverage uses `is_null(...)`, truthiness, and type predicates without changing
  that separate, locked expression contract.
- The trigger parser currently recognizes `kind: trigger-schema`; exclusion
  coverage follows that production spelling pending a separate kind-catalog
  migration to `schema-trigger`.

## Phase 1: Lock the Regression and Acceptance Matrix

- [x] **Task 1.1: Add schema-stage regression tests before changing production code.** In `darkmatter/lib/src/markdown/compose/schema_validation.rs`, add tests that demonstrate the current missing binding and cover inline and whole-file referenced SimplifiedSchema declarations, repeat invocation of the schema stage, and the ordering of materialization before coercion and validation. Assert that the first pass inserts JSON `null` and that the second pass is idempotent.

- [x] **Task 1.2: Encode the complete value-preservation matrix.** In the same schema-stage test module, parameterize caller/authored values for explicit `null`, `""`, `false`, `0`, empty arrays, and empty objects; assert each present value survives unchanged and is never replaced by synthesized `null`.

- [x] **Task 1.3: Encode every exclusion as an observable test.** Cover baseline-only properties, trigger-payload properties, raw JSON Schema, root-union document schemas, nested object properties, `required` properties, and properties carrying `default(...)`. Assert that none materialize, that a missing required property still fails validation, and that composition with the normal default Darkmatter baseline does not add unrelated keys.

- [x] **Task 1.4: Add user-visible compose-output regressions.** In `darkmatter/cli/tests/compose_schema.rs`, compose with `--frontmatter`, reparse stdout, and assert an eligible unset parameter serializes as YAML `null` in deterministic declaration order. Add companion cases proving a supplied value wins and a compose operation with no effective schema does not synthesize a key. **Parallelizable:** this task can be written alongside Tasks 1.1-1.3 because it exercises the public CLI boundary rather than the schema-stage unit seam.

- [x] **Task 1.5: Add downstream Claudine regressions for the exact defect.** Use the real `prompts/_implement/implement-plan.md` artifact through the normal `claudine compose` path, with a temporary plan and controlled provider/command doubles, to capture: unset `commit_message` passing lifecycle-shell preflight, a supplied message stamping `git commit -m "chore: x"`, unset/set values selecting opposite `when:` branches at event time, and undeclared shell/guard roots retaining their existing errors. Keep terminal tests inside the existing non-focusing tmux/test-harness path. **Parallelizable:** the Claudine fixtures and assertions can be prepared independently of the Darkmatter unit tests.

- [x] **Validation checkpoint 1:** Run the new targeted tests before implementation. Record that the new eligible-binding and shipped-prompt cases fail for the reported missing-root behavior, while the existing undeclared-root strict-mode tests still pass. Do not weaken assertions to accommodate the current defect.

## Phase 2: Implement Document-Owned Binding Materialization

- [x] **Task 2.1: Centralize SimplifiedSchema property semantics needed by composition.** Add or expose a crate-internal query over `PropertyDef`/`PropertyAtom` that uses the same value-level and array-level constraint rules as SimplifiedSchema conversion to answer whether a top-level property is required and whether it has `default(...)`. Reuse this query from conversion where practical so property unions and array constraints cannot drift between JSON Schema lowering and binding eligibility.

- [x] **Task 2.2: Derive eligible names from resolved schema provenance.** In `darkmatter/lib/src/markdown/compose/schema_validation.rs`, build the candidate list only when `EffectiveSchema::simplified` is `SimplifiedSchema::Single`. Iterate its top-level literal properties in declaration order, require `SchemaOriginKind::Document` or `SchemaOriginKind::ReferencedFile`, and reject required/defaulted definitions. This construction must inherently exclude raw JSON Schema, root unions, nested declarations, baseline winners, and trigger winners; do not infer ownership from nullable JSON Schema shape alone.

- [x] **Task 2.3: Materialize absent eligible bindings at the schema seam.** Immediately after effective-schema resolution and before `coerce_frontmatter_with_pending` and validation, insert `serde_json::Value::Null` only when the frontmatter map does not already contain the key. Preserve authored and caller override values exactly, insert multiple bindings deterministically, and rely on presence checks for second-pass idempotence.

- [x] **Task 2.4: Keep composition and validation boundaries explicit.** Ensure the mutation occurs only when the compose pipeline runs schema validation; do not modify `DarkmatterSchemas::validate`, standalone schema parsing, DMLS, trigger matching, or other read-only validation APIs. Update the behavior-bearing module docs and function comments in `schema_validation.rs` so they describe both coercion and eligible null materialization without stale step ordering.

- [x] **Validation checkpoint 2:** Run the Phase 1 Darkmatter unit tests and the existing schema-validation/coercion tests. Confirm positive cases now pass, exclusions remain absent, required failures are unchanged, and two schema passes yield byte-for-byte equivalent effective frontmatter.

## Phase 3: Verify Darkmatter Public Composition Behavior

- [x] **Task 3.1: Complete CLI serialization coverage.** Make the Phase 1 CLI tests pass without output-string replacements: `md compose --frontmatter` must expose the synthesized key as a real null value, preserve caller values and ordering, and avoid baseline/trigger key flooding.

- [x] **Task 3.2: Protect strict-root and interpolation semantics.** Run and, only where coverage is missing, extend subtree strict-mode and interpolation tests to prove an eligible declared root resolves as null/empty downstream while a genuinely undeclared root still reports `unknown root`. Include `is_null(value)`, truthiness, and empty-string distinction assertions at the public composition boundary; preserve Darkmatter's separate contract that `null == null` is false.

- [x] **Task 3.3: Protect the pipeline boundary before schema validation.** Add or retain a regression showing first-pass frontmatter interpolation cannot rely on a not-yet-materialized optional parameter, while body interpolation and later compose stages can. This makes the deliberate pipeline boundary observable and prevents a future accidental stage reorder.

- [x] **Validation checkpoint 3:** From `darkmatter/`, run the targeted CLI tests, then `just build` and `just test`. Confirm validation-only tests show no mutation and no snapshots change beyond composed frontmatter that now honestly contains eligible null bindings.

## Phase 4: Verify Claudine Preflight and Lifecycle Semantics

- [x] **Task 4.1: Prove the shipped prompt passes preflight when unset.** Finish the L2 regression using the repository’s real `prompts/_implement/implement-plan.md`, omit `commit_message`, and assert composition reaches the controlled provider rather than failing in `resolve_lifecycle_shell_commands`. Keep all work in an isolated temporary repository and replace side-effecting lifecycle commands with existing cross-platform test doubles or harness interception.

- [x] **Task 4.2: Prove supplied command text remains exact.** Run the same path with `commit_message='chore: x'` and assert the approved/resolved lifecycle command is exactly `git commit -m "chore: x"`; also assert the unset run does not execute that branch. Do not change Claudine production interpolation or preflight code unless the Darkmatter binding fails to flow through the normal preparation path.

- [x] **Task 4.3: Prove event-time guard selection in both directions.** Using controlled dirty-file state, assert unset/null selects `ctx.dirty_files && !commit_message`, supplied text selects `ctx.dirty_files && commit_message`, and neither evaluation raises an undefined-variable error. Assert branch effects or recorded commands, not only expression return values.

- [x] **Task 4.4: Reconfirm fail-closed behavior.** Run existing and targeted Claudine tests for undeclared roots in lifecycle `shell:` interpolation and `when:` guards. The error text/code and no-side-effect behavior must remain unchanged, demonstrating that the known-root set grew only through materialized document declarations.

- [x] **Validation checkpoint 4:** From `claudine/`, run the targeted preflight/lifecycle unit tests and the targeted shipped-prompt L2 test, then `just test` and `just test-l2`. Report any environment-gated L2 skips explicitly; terminal or browser windows must not gain focus.

## Phase 5: Documentation, Cross-Package Gates, and Closeout

- [x] **Task 5.1: Document the public composition contract.** Update `darkmatter/docs/inline/schema-validation.md` to state that composition materializes only missing, optional, no-default top-level bindings owned by an inline or referenced SimplifiedSchema; document the exclusions and the fact that validation-only APIs and composition with no effective schema remain non-mutating.

- [x] **Task 5.2: Keep agent guidance synchronized.** Update `.claude/skills/darkmatter/schema.md` and, only if its summary contract needs clarification, `.claude/skills/darkmatter/SKILL.md` with the post-schema binding behavior, provenance boundary, and pipeline position. **Parallelizable:** documentation can be drafted after Phase 2 semantics stabilize while Phases 3-4 tests run.

- [x] **Task 5.3: Run final package-area gates.** From `darkmatter/`, run `just build`, `just test`, and `just lint`. From `claudine/`, run `just build`, `just test`, and `just lint`, plus the targeted `just test-l2` regression. Do not run `cargo fmt`; review formatting only in touched files and preserve macOS, Windows, and Linux compatibility in tests and implementation.

- [x] **Task 5.4: Audit scope and observable output.** Review `git diff` to confirm production behavior changed only at Darkmatter’s schema-composition seam, Claudine changes are regression coverage unless a demonstrated integration defect required otherwise, validation-only paths remain passive, and no unrelated user changes were touched. Re-run the exact reproduction with no `commit_message` and the supplied-message variant, and record both outcomes.

- [x] **Validation checkpoint 5:** Map every specification success criterion to a passing named test or command result: original shipped-prompt reproduction, supplied message, both guard branches, unchanged typo failure, positive materialization/idempotence/value preservation, all source/schema exclusions, no-effective-schema behavior, and default-baseline non-flooding. The fix is complete only when this matrix and all package gates pass.
