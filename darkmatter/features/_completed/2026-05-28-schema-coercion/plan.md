---
phases: 5
created: 2026-05-28
start_phase: 1
---

# Schema-Driven Frontmatter Coercion Execution Plan

## Success Criteria

- [ ] The motivating `claudine compose prompts/implement.md …` invocation succeeds, storing `has_spec` / `has_plan` / `has_review` as real booleans in the composed document.
- [ ] Coercion is driven by a single JSON-Schema-driven path that covers inline `$schema`, baseline-merged fields, raw JSON Schema, and root unions.
- [ ] The coercion matrix is honored exactly: string→boolean (boolish set), string→number (numberlike regex), scalar→string, element-wise typed arrays; `boolish`/`numberlike` normalize their stored value.
- [ ] Ambiguous values (`"yes"`, `"1"` → boolean) and out-of-matrix values still fail with the existing strict `Type` error, unchanged in wording.
- [ ] The compose Schema Validation stage mutates the stored frontmatter so coerced types flow to every downstream stage and the composed output.
- [ ] Values still holding `$(...)` are not coerced at the pre-shell stage and remain deferred; post-shell re-validation coerces them via the same helper.
- [ ] `md schema validate`, the library `DarkmatterSchemas::validate` API, and the compose pipeline agree on validity for the same document.
- [ ] `darkmatter` library and CLI tests pass; no behavior change is mixed into comment-only edits.

## Phase 1: Discovery And Coercion API Design

- [ ] Re-confirm the type-checking and value shapes by reading `darkmatter/lib/src/markdown/schemas/mod.rs` (`EffectiveSchema`, `validate_with_positions`, `frontmatter_as_json`), `schemas/validate.rs` (`collect_problems`, `collect_root_union_problems`, `wrap_arm_as_root_schema`, `classify_kind`), and `schemas/simplified/convert.rs` (numberlike/boolish fragment shapes).
- [ ] [parallelizable] Inventory the exact JSON Schema fragment shapes that `convert.rs` emits for `boolean`, `number`/`integer`, `string` (incl. `date`/`datetime`/`time`/`url`/`email`/`file` formats), `boolish`, `numberlike`, arrays (`items`), and root unions (`anyOf`) — these are the patterns the recognizer must match.
- [ ] Define the coercion API surface before coding: a `CoercionTarget` enum (`ToBoolean`, `ToNumber`, `ToString`, `Array(Box<CoercionTarget>)`), a `coercion_target(property_schema: &Value) -> Option<CoercionTarget>` recognizer, and the pure entry point `coerce_frontmatter(json_schema: &Value, instance: &Value) -> CoercionOutcome { value, changed }`.
- [ ] Fix the accepted-form constants as single sources of truth: the boolish string set `{true,false,True,False,TRUE,FALSE}` and the numeric regex `^-?\d+(\.\d+)?$`, reusing or sharing the definitions already used by `convert.rs` rather than duplicating them.
- [ ] Decide module placement: add `darkmatter/lib/src/markdown/schemas/coerce.rs` and register it in `schemas/mod.rs`.
- [ ] Validation checkpoint: write down, in code-level terms, the union algorithm (per-arm coerced candidate → first arm that validates wins) and the rule that the pre-shell compose stage skips `$(...)`-bearing values.

## Phase 2: Core Coercion Engine

- [ ] Implement `coercion_target` recognizing every fragment from the Phase 1 inventory, including the numberlike/boolish `anyOf` shapes; return `None` for `object`, `any` (`{}`), bare `enum` without a `type`, and any unrecognized shape.
- [ ] [parallelizable] Implement scalar coercion: string→boolean (boolish set only), string→number (regex only), and scalar→string (`number`/`boolean` → canonical string). Leave `null`, arrays, objects, and ambiguous strings (`"yes"`, `"1"`) untouched.
- [ ] [parallelizable] Implement element-wise array coercion driven by the `items` target, recursing through the scalar rules; an uncoercible element leaves that element untouched (validation reports it).
- [ ] Implement the non-union object pass: for each instance property with a recognized target, attempt coercion and replace on success; track whether anything changed.
- [ ] Implement the root-union pass: for each `anyOf` arm in index order, build a coerced candidate, strict-validate it against the arm (reuse `wrap_arm_as_root_schema` + the validator cache), commit the first arm that validates; if none validate, return the instance unchanged.
- [ ] Ensure idempotence: coercing an already-correctly-typed value is a no-op and reports `changed: false`.
- [ ] Validation checkpoint: unit tests in `coerce.rs` for every matrix row, every "never coerced" ambiguous case, typed arrays (including a mixed array with one uncoercible element), a non-union object, and a three-arm union mirroring `prompts/implement.md` (assert the first validating arm's coercions are committed).

## Phase 3: Library Validation Integration

- [ ] Wire `coerce_frontmatter` into `EffectiveSchema::validate_with_positions`: coerce a working copy of the instance using `self.json_schema`, then validate the coerced copy so the `ValidationReport` reflects post-coercion validity. No document mutation on this path.
- [ ] Confirm `EffectiveSchema::validate` (the position-less convenience wrapper) inherits the same behavior through `validate_with_positions`.
- [ ] Verify baseline-merged fields are coerced: because coercion reads `json_schema` (post-merge), a baseline-declared `boolean`/`number`/`string` field coerces even though the document `simplified` AST does not contain it.
- [ ] Verify raw-JSON-Schema documents (where `simplified` is `None`) still coerce, since the path never consults the AST.
- [ ] Validation checkpoint: add tests under the `schemas` module proving a document that previously produced a `Type` problem now validates after coercion, for both an inline `$schema` and a baseline-merged field; and that an ambiguous value still reports the `Type` problem.

## Phase 4: Compose Pipeline Write-Back

- [ ] Change `compose/schema_validation::run` to take `&mut Markdown`; update the call site at `compose/mod.rs` (passes `self`, already `&mut`).
- [ ] In `run`, after resolving the effective schema, build the instance, call `coerce_frontmatter`, and write coerced top-level properties back into `markdown.frontmatter_mut().as_map_mut()` so downstream stages and the composed output see the real types.
- [ ] Skip coercion for any value that still contains `$(...)` (reuse / align with `value_needs_shell_expansion`), preserving the existing deferral contract; such values are neither coerced nor errored at the pre-shell stage.
- [ ] Validate after write-back (idempotent) and keep the existing deferral filter so only composition-independent problems are reported pre-shell.
- [ ] Confirm the post-shell re-validation path (the consumer's `prepare_*_with_schema`, and any second darkmatter validation pass) coerces via the same `coerce_frontmatter` helper, so shell-produced typed values (e.g. a `number` from `$(...)`) are coerced consistently after expansion.
- [ ] Update the existing `schema_validation.rs` unit tests that assumed strict rejection of coercible values (e.g. `document_schema_wrong_type_fails` uses `spec: 42` against `string` — under the new rules this coerces to `"42"`); adjust expectations and add coercion-positive cases without weakening the genuinely-invalid cases.
- [ ] Validation checkpoint: add a compose-level test asserting that a document with `has_spec: "{{spec ? true : false}}"` and a boolean schema arm composes successfully and that the resulting frontmatter holds a real boolean; assert `$(...)`-bearing typed values remain deferred.

## Phase 5: End-To-End Verification, Docs, And Drift

- [ ] Reproduce the original failing invocation against a fixture equivalent to `prompts/implement.md` and confirm it now composes with coerced booleans.
- [ ] [parallelizable] Add or update Rustdoc for the new `coerce.rs` public items and the changed `validate_with_positions` behavior, following the repo convention (summary first; `Examples`/`Returns`/`Errors` only when useful; no `# H1`).
- [ ] [parallelizable] Update `darkmatter/docs/topics/schema-definition.md` to document default-on coercion, the coercion matrix, the ambiguity rules, and the `boolish`/`numberlike` normalization change.
- [ ] [parallelizable] Update the `darkmatter` skill (`.claude/skills/darkmatter/`) where it describes schema validation / the compose Schema Validation stage, and regenerate the skill `hash:` frontmatter with `md hash <file>` for any edited skill file.
- [ ] Review all edited comments/docs for drift, especially the compose-pipeline stage comments in `compose/mod.rs` and `schema_validation.rs` that describe pre-shell validation behavior.
- [ ] Run focused `darkmatter` library tests for the `schemas` and `compose` modules.
- [ ] Run focused `darkmatter` CLI tests for `md schema validate` and compose.
- [ ] Run the package-level `darkmatter` validation recipe per the repo testing skill, without running `cargo fmt` unless explicitly requested.
- [ ] Validation checkpoint: verify `git diff` contains only intended coercion, validation, compose, test, and documentation changes, with no unrelated formatting or comment-only cleanup mixed into behavior changes.
