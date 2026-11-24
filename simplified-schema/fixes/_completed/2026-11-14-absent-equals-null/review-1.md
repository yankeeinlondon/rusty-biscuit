---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Pattern-keyed objects skip default preparation"
    - "R1-F2 (high): Lifecycle validation accepts inputs the parser rejects"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-15T04:43:00-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/log.md
implemented_by: claude/opus
next: 2026-11-14-absent-equals-null/review-2.md
---

# Review 1: Absent equals null

The fix is **not production ready**. The ordinary presence/default table,
schema migration, and composition's separation of declared names from written
keys work in the tested cases. Two defect classes remain: default preparation
does not traverse pattern-selected objects, and passive lifecycle validation
accepts several realistic inputs that execution parsing refuses. Both have
repairs within the spec; neither waits on an author decision.

## Findings

### R1-F1 (high): Pattern-keyed objects skip default preparation

**Authority: spec.** “Inside an existing object, including an object in a
list, child defaults apply”; “One rule for both schema forms”; and “Differing
defaults for the same property on different arms are a compile-time schema
error.” A declared child remains a declared property when its containing
object is selected by a key pattern. The spec does not exclude such parents.

**Defect class:** the default compiler omits a structural child selector, so
both runtime completion and compile-time conflict detection miss the same
subtrees.

In simplified-schema, [DefaultPlan](../../lib/src/defaults.rs) compiles the
places defaults can occur. `Compiler::is_structural` does not recognize
`patternProperties`; `Compiler::scope` never compiles its value schemas and
also suppresses `additionalProperties` whenever a sibling pattern exists.
`Fill::visit` consequently has no route into those objects. The library can
successfully validate a value while silently failing to apply its declared
default, or accept a schema whose defaults conflict.

This public-API reproduction uses an ordinary settings object:

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let schema = SimplifiedSchema::try_from(
    "types:\n  Settings:\n    enabled: boolean(default(true))\n\
     $schema:\n  '<starting::x->': Settings\n",
).unwrap();
let input = json!({"x-settings": {}});
let result = schema.validate_soft(&input).unwrap();
// Observed: {"x-settings": {}}, defaults: [].
// Expected: {"x-settings": {"enabled": true}}, one default report.
```

Changing the key declaration to `<string>` applies `true` correctly. With
`enabled: null`, the pattern form leaves null unchanged. A raw JSON Schema
equivalent without an optional-null wrapper rejects that null instead of
replacing it. Strict validation must report the same default while returning
the original input borrowed.

The problem also reaches a shipped fixture: Darkmatter's conditional schemas
select `then` and `else` through a key pattern. For
`composed: [{ when: 'true', then: [{ edit: { file: notes.md } }] }]`, the public
`DarkmatterSchemas::validate` report contains only `composed.0.no_error`.
Moving the edit out of the conditional reports its `create_new` and inner
`no_error` defaults as well. The same omission occurs under `else`.

The bounded sweep followed the compiler's selectors through conflict
checking, strict/soft validation, export, and the host's default report:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | Literal `properties` child | Existing object; `enabled` absent/null | Default applied/reported; supplied false preserved | Clean |
| R1-F1.2 | `<string>` / `additionalProperties`, without a sibling pattern | Same object and field shapes | Default applied/reported | Clean |
| R1-F1.3 | `<starting::x->` / prefix pattern | Same object and field shapes | Missing/null child left as written; no report | Fill/report `true` |
| R1-F1.4 | `<ending::-settings>` / suffix pattern | Same object and field shapes | Missing/null child left as written; no report | Fill/report `true` |
| R1-F1.5 | `<pattern::^x->` / regular-expression pattern | Same object and field shapes | Missing/null child left as written; no report | Fill/report `true` |
| R1-F1.6 | Raw `patternProperties` | Same object; absent/null Boolean child | Absent stays absent; explicit null is rejected by both validation modes | Prepare the default before judging |
| R1-F1.7 | Raw `additionalProperties` beside a pattern | Nonmatching `other: {}` with a defaulted child in the catch-all schema | No default/report | Complete the nonmatching object's child |
| R1-F1.8 | Conflict detection beneath a pattern, through raw preparation and SimplifiedSchema export | Named object union with `n: number(default(1))` versus `default(2)` | Validation succeeds; SimplifiedSchema export succeeds | Compile-time conflicting-default error |
| R1-F1.9 | Conflict detection beneath a literal or catch-all child | The same conflicting union | `ConflictingDefaults` | Clean |
| R1-F1.10 | Darkmatter `md-conditional-*` → `then` | Shipped edit fixture moved into `then` | Inner action defaults absent from the report | Report all reachable defaults |
| R1-F1.11 | Darkmatter `md-conditional-*` → `else` | Same edit moved into `else` | Same omission | Report all reachable defaults |
| R1-F1.12 | Array `items`, tuple `prefixItems`, and recursive/local named-type children | Existing object items with defaulted children | Completed by the passing `required_defaults` and `defaults.rs` tests; positional items themselves are not inserted | Clean |

For the affected field's other shapes, false is preserved, `"yes"` is
converted only by soft validation, and native `1`, `[]`, and `{}` are rejected
at `/x-settings/enabled`. An absent parent is not created; null or array
dictionary values are rejected by the declared object type. The empty
dictionary itself is accepted and stays empty. These controls distinguish
the traversal omission from a permissive field validator. The loader's
duplicate-key/trailing-content behavior is covered separately below.

**Recommended repair:** extend the existing compiled plan and walk to follow
matching pattern schemas and the catch-all for nonmatching keys. Reuse the
repository's existing pattern-matching semantics, including literal-key
precedence; avoid another defaulting pass in a host. Compile each pattern's
subtree so its internal union conflicts are checked too. Add public-result
regressions for the affected selectors and the shipped conditional example.
Update the Claudine skill's recorded pattern-default limitation when repaired.

### R1-F2 (high): Lifecycle validation accepts inputs the parser rejects

**Authority: spec and defect.** The lifecycle requirements say “native
numbers do not become booleans,” “soft validation and execution agree after
binding,” “Explicit conflicting placements remain invalid,” and “`set` has
no `no_error` flag.” The additional branch-shape defect is independently
reachable by the plausible incomplete item `{ when: 'true' }`: validation
accepts an item for which the shared parser requires a branch or an action.

**Defect class:** schema admission does not run the authored placement and
shape checks that execution parsing uses, and the md schema bypasses flag
checking for provided verbs whose operands it leaves untyped.

In Darkmatter, [BoundOperands](../../../darkmatter/lib/src/markdown/schemas/bound_operands.rs)
adds passive operand checks to schema validation, but only at annotated
operand sites. The md baseline's `md-expression` pattern types provided
side-effect and expression-function operands as `unknown`, so these sites
do not reach that check. For example:

```yaml
composed:
  - ensure_file: { file: a.md, no_error: 1 }
```

`DarkmatterSchemas::validate` reports valid; the public md lifecycle parser
refuses the numeric flag. The same result occurs for `abs: { x: 3,
no_error: 1 }`. This affects the passive validation verdict used by the CLI
and editor, even though the parser correctly prevents execution.

Separately, the newly added `no_error: literal(false)` conditional fields
admit an authored false flag that the parser refuses beside `then`.
Admitting a completed false default is necessary; accepting that authored
placement is not. Long-form and sibling schemas also judge duplicate flags
independently, and the md category admits a null flag beside `set`.

The sweep used shipped-schema validation and public lifecycle parsing, with
the same one-field/action edits in both hosts. “Accept/reject” below means
schema accepts and parser rejects; the parser's refusal is the expected
contract, so both should reject the authored input.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | md `md-expression` side-effect operands | `ensure_file` long form with numeric `no_error` | Accept/reject | Both reject |
| R1-F2.2 | md `md-expression` function operands | `abs` long form with numeric `no_error` | Accept/reject | Both reject |
| R1-F2.3 | md `md-conditional-1`, `-2`, `-3` | `when` + `then: [stop]` + authored `no_error: false`, each named type tested | All three schema types accept; shared parser refuses this placement | Both reject |
| R1-F2.4 | md `md-initialize-conditional-1`, `-2`, `-3` | Same authored conditional | All three schema types accept; same parser refusal | Both reject |
| R1-F2.5 | Claudine `conditional`, `initialize-conditional`, `cancelled-conditional` | Same item in `start`, `initialize`, `cancelled` | Accept/reject in all three families | Both reject |
| R1-F2.6 | md short/long action categories and generated long-form projection | Both sibling and inner `no_error: false`, with `edit`, `info`, and `shell` controls | Accept/reject | Both reject duplicate authored placements |
| R1-F2.7 | Claudine category/long-form composition | Both flags on `info`, in all three event families; existing `ensure_file` duplicate-placement test | Accept/reject | Both reject duplicate authored placements |
| R1-F2.8 | md `set` category | `set: { answer: 42 }` beside `no_error: null` | Accept/reject | Both reject the unsupported authored flag |
| R1-F2.9 | md item/conditional admission | `{ when: 'true' }` without `then` or a sibling action | Accept/reject | Both reject the incomplete item |
| R1-F2.10 | Claudine item/conditional admission | Same incomplete item in `start`, `initialize`, `cancelled` | Accept/reject in all three families | Both reject |
| R1-F2.11 | Typed md `info`/`edit` and generated `ensure_file-long`; Claudine's typed generated actions | Numeric flag | Both reject, with a located schema problem | Clean |
| R1-F2.12 | Shared flag binder, sibling flag and bundle paths | Omitted/null flag, false, Boolean-like `"yes"` | Both accept applicable placements; existing tests verify the converted/completed parse | Clean |
| R1-F2.13 | Conditional authored-origin handling | Valid conditional with absent/null flag; authored true flag | Both accept absent/null and reject true in both hosts | Clean |
| R1-F2.14 | Completed default provenance | `{ edit: { file: notes.md } }` with filled inner/sibling false flags and original authored values | Parser preserves the authored parse; does not report a duplicate | Clean |

The flag-shape column was also exercised through the public md APIs on
`ensure_file`: absent, null, false, and `"yes"` pass both; native `1`,
`"maybe"`, `""`, `[]`, and `{}` all incorrectly pass the schema and are
refused by the parser. Boolean flags have no collection-element shapes.
For the conditional row, its valid control and null/false/true variants
were tested in both hosts. Existing edit tests cover missing/null/blank
mandatory operands, wrong argument counts, wrong flag containers, and
unknown keys; they pass and should remain unchanged.

The existing tests
`an_authored_false_flag_beside_then_passes_the_schema_and_fails_the_parser`,
`two_authored_flag_placements_pass_the_schema_and_fail_the_parser`, and
`a_generated_long_form_no_error_follows_the_flag_rule_where_the_verb_is_typed`
currently preserve some of these gaps rather than enforce agreement. The
Claudine placement tests do the same for duplicate flags. Passing those tests
does not meet the spec's parity requirement.

**Recommended repair:** extend the shared passive lifecycle checks to judge
authored flag placement and branch shape before completion obscures origin.
Ensure md's provided but operand-untyped verbs still receive the shared flag
check. Preserve permissive handling of genuinely unprovided verbs and the
parser's existing refusal rules. Reuse these checks for both hosts rather
than adding a large set of special JSON Schema alternatives. Turn the
gap-pinning tests into agreement regressions, retain completed-default
provenance controls, and update docs that describe these gaps as current
limitations. No change to lifecycle execution order is needed.

## Implementation decisions (informational)

Two agents independently read the spec, implementation log, and implementation
record before the code comparison. The log's `decisions_for_review` list is
the implementer's flagged inventory; there is no separate “Decided during
implementation” heading. The following records its choices, including the
ones already authorized by the spec. None needs a human ruling or a separate
post-cycle sign-off.

1. **Phase ordering:** migration, null-carrier readers, revision bumps, and
   artifact regeneration moved into Phase 1 to keep shipped schemas parseable
   after grammar removal. Null replacement in composition moved into Phase 2.
   Keep these implementation-order choices.
2. **Envelope recognition:** untagged `$schema: null` is plain YAML; a tagged
   schema may use its `types` table instead. Empty trigger carriers are
   refused, omitted/null carriers remain matcher-only, and an empty ordinary
   schema export remains valid. These are consistent, reversible reader choices.
3. **Constraint migration:** named `model-override` unions lost `non-null`;
   Boolean settings lost it; repository event/matcher/steering mappings gained
   `not-empty`. Thus empty override mappings are refused. Keep the spec's
   type-based migration and inherited-null behavior.
4. **Presence and phases:** launch preserves the declared value type while
   deferring omitted non-eager required keys, replacing older null-acceptance
   tests. This is explicitly required. The separate `declared-required`
   meta-type option still refuses null-admitting declarations, preserving its
   stated contract and kind-claim checks.
5. **Declaration content checks:** `schema`/`type-definition` `not-empty`
   lowers to text/mapping/list keywords, with only the mapping keyword under
   `mapping` to preserve diagnostic ranges. `yaml`/`json` content checks judge
   text, so `"{}"` has content. Keep these choices.
6. **Consumer adjustments:** `claudine init` omits events with no actions
   instead of writing newly invalid empty lists. The shared operand check
   rejects `edit: null` without requiring `edit` beside unrelated verbs.
   Both are necessary consumer repairs.
7. **Conversion and authored data:** the conversion-only early-success check
   remains after composition prepares defaults. `EffectiveSchema::data_copy`
   converts a completed candidate, then withdraws inserted defaults/restores
   replaced nulls so setters do not persist defaults. Keep these safeguards.
8. **Default-plan choices:** any requiring declaration enables insertion of a
   null default; equal defaults are checked against every declaring property
   definition; reference-only cycles fail during compilation. These are
   reasonable choices within the default contract.
9. **Redundant work removed:** Claudine's separate launch-default helper was
   deleted. Expression typing recognizes a non-null literal default on an
   optional property; unresolved template defaults do not qualify. Keep both.
10. **Conditional compatibility:** conditional arms admit a completed false
    flag so defaults from other union arms do not invalidate them. Keep that
    purpose; repair authored admission under R1-F2.
11. **Authored origin:** Claudine derives it from source frontmatter plus proxy
    overlays and layered overrides. Sequence task stacks are treated directly
    as authored because they are not completed first. Null flags count as
    omitted on accepting actions; `set` retains its exception. Keep these rules.
12. **Configuration representation:** the implementation plan chose keyed
    route/action payloads to give provider defaults separate paths. Private
    adapters preserve runtime enum representations; payloads are closed,
    keys use snake_case, flat YAML receives a migration example, and legacy
    JSON5 is upgraded. This is a concrete way to preserve provider capabilities
    under the spec's global union-default rule; recommend keeping it.
13. **Override behavior:** repository null messaging overrides now inherit,
    so the TUI's repository “disabled” option was removed. No replacement off
    switch was added. Null-as-unset reading also covers TTS and model entries;
    saving avoids previously unwritten null keys. These follow the ruling.
14. **Obsolete safeguards:** the shipped-schema prohibition on
    `unknown(required)`/`any(required)` and unreachable null-error messages
    were removed. The new presence contract makes those guards obsolete.
15. **Lifecycle order:** `initialize` still precedes validation, and refusal
    tests put effects in `composed`. The spec explicitly preserves which events
    execute; its input-matrix default is not grounds to reorder initialization.
16. **Number loading:** YAML overflow spellings remain text; content-based
    raw-schema loading still uses YAML syntax, while `.json` loaders enforce
    JSON. A broader loader change broke pinned behavior and was reverted.
    Keep the behavior; the number docs already explain the distinction.
17. **Error variants:** property-level conflicting defaults remain
    `SchemaError::Convert`; object-arm/raw conflicts use `ConflictingDefaults`.
    Both are compile-time errors, so unifying the variants is unnecessary here.
18. **Documentation choices:** validation.md owns the full contract table;
    other pages link to it. Old anchors were renamed/removed, migration notes
    remain, and the repo skill was split into focused pages. Claudine's
    configuration example is explicitly uncompiled. These are inexpensive
    editorial choices and need no sign-off.
19. **Known gaps:** recording lifecycle schema/parser disagreements did not
    discharge their repair; they are R1-F2. The pattern-default limitation
    recorded in the Claudine skill is R1-F1.
20. **Verification choices:** the stale doctest import was corrected; tests
    use passive/fake services and hermetic CLI launches. Leaving cross-OS proof
    to CI and omitting L2/L3/browser runs for these requirements is appropriate.

Deferred work was considered individually:

- The lifecycle disagreements are ordinary repair work under R1-F2; requiring
  artifact regeneration is not a reason to defer them.
- Initial interpolation of an entire `loop` value may lose authored flag
  provenance. The log records this limitation; the added preparation test
  verifies an authored mapping, not that transformation. No additional
  realistic failing workflow was established here, so this is not a finding.
- Empty nested property names cannot be represented by the default-report
  path and remain skipped. This unusual input does not justify extending path
  syntax in this review.
- A new repository messaging off switch would be additional scope; null
  inheritance is already decided. No new setting is requested.
- There are no dedicated tests solely for clearing repository actions or
  converting an empty legacy event list. Existing schema and conversion
  boundary tests enforce the validators involved; no separate gap is found.
- Compiling the labeled library documentation example, consolidating conflict
  error variants, and changing raw-text syntax admission are unnecessary here.
- Linux, native Windows, WSL2, and Messenger's research-enabled consumer were
  not run by the implementer in Phase 6. Cross-OS evidence is outside review
  readiness; the final source fix was a doctest import. Lifecycle write-boundary
  changes remain in the following spec, as explicitly required.
- Intermediate phases recorded slow/failing runs. Later full gates supersede
  those intermediate states; the review's own slow-test assessment is below.

## Verification and input robustness

Verification used Level 1 public APIs, hermetic CLI processes, fake lifecycle
services, and existing editor-protocol tests. These requirements concern data
validation, conversion, and passive/execution agreement; they do not assert
terminal rendering, keyboard encoding, or GUI interaction. No higher-level
terminal test is needed.

| Requirement | Strongest relevant Level 1 evidence | Assessment |
|---|---|---|
| Presence/default table, strict borrowing, soft copies, falsey values, null defaults, stability | `required_presence`, `required_defaults`, `defaults.rs` tests | Ordinary selectors pass; pattern selectors fail R1-F1 |
| Nested parents, arrays, named/imported/recursive types, unions, pending templates | `required_defaults`, default-engine tests, host preparation tests | Covered apart from R1-F1 |
| Export versus library default application; raw JSON Schema assertions | Plain-validator test, raw-schema matrices, identity/artifact tests | Covered apart from pattern conflict detection |
| Trigger presence, envelope null carriers, removed constraint, extended `not-empty` | Trigger tests, `envelope_null_carriers`, `not_empty_values`, interpolation test | Pass |
| Declared but unwritten parent/child values; no synthetic keys or source mutation | `absent_property_contract`, CLI compose/frontmatter tests | Pass |
| CLI validation, composition, whole YAML, editor diagnostics | Darkmatter CLI suites, whole-YAML tests, DMLS suites | Existing cases pass; shared validation defects remain |
| Lifecycle flags, operands, bundles, defaults and authored origin | Parity tests, fake editor/executor tests, CLI refusal tests, review probes | R1-F1/R1-F2 |
| Claudine override inheritance, keyed config, key counts, failed-conversion publication | Config schema/wire/keyed-entry/JSON5 and CLI wiring tests | Pass |

For file/configuration inputs, the intended invalid-shape outcome is a
located error and no publication. Existing initialization order is preserved.
The documented exceptions include absent/null inheritance, default filling,
and explicitly allowed empty values. The review checked field matrices at the
shared validator and document shapes at each loader rather than multiplying
CLI processes per field.

The inspected matrices cover defaulted scalar/object/list fields; envelope
metadata and schema carriers; lifecycle flags/operands; keyed actions/routes;
and repository override fields. YAML duplicate keys, trailing content, document
markers and null spellings, JSON file syntax/numbers, Markdown frontmatter
loading, and legacy JSON5 have public-result tests plus representative CLI
wiring. The affected rows and columns are expanded in the findings above.

The requested smell search found intentional `Option<T>`/null-as-default
deserialization in configuration overrides, where the new contract equates
absence and null. Collection members are deserialized into typed values before
null entries are dropped. The default compiler's `filter_map` reads schema
metadata; its unrecognized structural selector is accounted for in R1-F1.
No additional silent wrong-type fallback was established.

Review runs on macOS:

| Command | Result |
|---|---|
| `simplified-schema/`: `just test` | 2,237 feature-enabled tests and 1,878 minimal tests passed; none reached 5 s |
| `darkmatter/`: `just test` | 16,657 passed, 10 recipe skips; none reached 5 s |
| `claudine/`: `just test` | 12,164 assertions passed, 15 recipe skips; one test required the load assessment below |
| Public-API review probes | Pattern selectors/conflicts and both hosts' lifecycle verdicts reproduced the findings; temporary probe sources removed afterward |
| `just lint`, all three areas | Passed, exit 0, including Darkmatter's Zed WASM check |
| Implementer's recorded final doctest and tier checks | Passed; zero stranded tests in each area |
| `git diff --check` | Passed |

Claudine's `claudine_schemas::shared_payload_properties_have_identical_types`
took **5.202 s** in the package run. `uptime` showed load averages
**61.18 / 35.77 / 23.68**, above the **16 cores** reported by sniff.
The exact test remeasured alone at **4.513 s**. It therefore clears the stated
loaded-host allowance; no slow-test finding remains. This is not an
unqualified claim that the package run contained no slow tests.

Test modules are registered in the consolidated L1 binaries; the added test
names and required features are selected by the canonical recipes. The
implementer's tier checks also report zero stranded tests. No production code
was changed during this review, and no terminal/editor/browser was opened by
the review probes.

## Implementation record corrections

Corrected the log's obsolete claims that the overflow-number docs were left
unchanged and that loop preparation had no end-to-end test. Also narrowed its
“null flags are omitted everywhere” claim to preserve the `set` exception.
Corrected the overflow-doc claim in implementation.md. Both records contain
dated notes with the verification. These are record corrections, not findings
or human-review items.

This is the first review. There are no carried findings or recurrence items.
