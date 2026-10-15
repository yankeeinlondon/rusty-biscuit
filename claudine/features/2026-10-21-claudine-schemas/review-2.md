---
$schema: feature-review.yaml
ready: false
findings:
    - "[high] Trigger schemas do not supply the promised editor completion and hover"
observations:
    - "The previous review is unavailable"
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-14T00:23:17-07:00
spec: 2026-10-21-claudine-schemas/spec.md
implemented: true
description: "A **feature** review of `2026-10-21-claudine-schemas/spec.md`"
feature: 2026-10-21-claudine-schemas/review-2.md
previous: 2026-10-21-claudine-schemas/review-1.md
next: 2026-10-21-claudine-schemas/review-3.md
log: claudine/features/2026-10-21-claudine-schemas/log.md
implemented_by: claude/opus
---

# Review 2: Claudine Schemas

The feature is **not production ready**. Schema validation and launch protection pass their tests, but the editor does not provide the completion and hover that the specification promises.

## Previous findings

The requested `review-1.md` is absent. Searching the checkout and Git history, including all available refs for that path, found no copy. Its `Unblocked Findings` therefore could not be checked, and its `next` and `implemented` properties could not be updated. The implementation log describes the original five implementation phases, but does not contain the missing review's findings. No replacement review was invented.

## Unblocked Findings

### High — Trigger schemas do not supply the promised editor completion and hover

**Authority: spec.** The opening paragraph promises that an author editing a Claudine prompt in DMLS sees “completion and hover for Claudine's frontmatter properties.” The action-type design also says that a single-verb item's “hover names the category.” Neither promise is marked planned or excluded from scope. A prompt with a reachable disk bundle and no authored `$schema` is the normal input this feature introduces. Documenting that it receives diagnostics only does not implement the promised editor behavior.

**Defect class:** schema-derived editor projections omit matching trigger declarations, so the newly supplied Claudine types reach validation but not completion or hover.

In the **Darkmatter library**, [`DarkmatterSchemas::effective_for_with_override`](../../../darkmatter/lib/src/markdown/schemas/mod.rs:554), which assembles the document's effective schema, merges trigger payloads into JSON Schema but retains only the authored declaration in `effective.simplified`. In the **DMLS package**, [`known_shape`](../../../darkmatter/dmls/src/providers/frontmatter.rs:2035), which supplies property types to the editor, combines the baseline, configured extensions, and that authored declaration. It never includes the matched trigger payloads. Removing the old plain Claudine schemas and making the replacement bundle trigger-discovered brings Claudine's promised editor features onto this incomplete path.

#### Complete instance sweep

The reproductions used the existing `claudine_schema_discovery` fixture: a temporary repository with `claudine/lib`, all five shipped schema files copied into `claudine/schemas`, an isolated home, and no extension configuration. Each document had `prompt: hi`, so both applicable triggers could match. Public `textDocument/completion` and `textDocument/hover` requests went through the existing in-memory LSP session. Completion requests targeted the indicated partial key or empty value; hover requests targeted the authored key.

| Site in DMLS | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Top-level key completion | `mod:` with cursor after `mod` | `[]` | Offer `mode` from the trigger payload |
| Nested key completion | `start: [{say: {te: null}}]`, presented as a block mapping with cursor after `te` | `[]` | Offer the required `text` parameter |
| Lifecycle item completion | `start:` followed by `- sa:` with cursor after `sa` | `[]` | Offer `say` |
| Union item completion | `start:` followed by an empty `- ` item | `[]` | Offer action keys and supported bare verbs, including `stop` |
| Loop key completion | `loop: {ma: null}`, presented as a block mapping with cursor after `ma` | `[]` | Offer `max` |
| Scalar enum value completion | Empty `mode:` value | `[]` | Offer `append` and `replace` |
| Boolean value completion | Empty `interactive:` value | `[]` | Offer `true` and `false` |
| Top-level schema hover | `mode: append` | `null` | Describe its enum and purpose |
| Lifecycle parameter hover | `start: [{say: {text: hi}}]`, hovering `text` | `null` | Describe its required string type and action category |
| Loop property hover | `loop: {max: 3}`, hovering `max` | `null` | Describe its integer constraint and purpose |
| Linking property hover | `max_turns: 3` | `null` | Describe its integer type and purpose |
| Authored-schema controls: key, enum, and boolean completion | Same properties explicitly declared in `$schema` | `mode`, `append`/`replace`, and `true`/`false` offered | Clean |
| Authored-schema controls: lifecycle key and union item completion | `$schema: {start: 'stack@../../schemas/lifecycle.yaml'}` | Required `text` offered; empty item gets action suggestions including `stop` | Clean |
| Authored-schema controls: scalar and lifecycle hover | Same authored declarations | Type information returned; `text` also names the matching communication alternative | Clean |
| Trigger-based diagnostics | Existing typo, malformed linking value, and discovery tests | Errors reported from the disk bundle; valid lifecycle/linking fixture accepted | Clean |

These are one missing projection, not separate defects per property. The sweep covers top-level and nested keys, action unions, scalar candidates, and schema hover across composition, lifecycle, loop, and linking declarations. Expression-property name completion intentionally uses authored parameters rather than every baseline property; changing that separate policy is not requested here.

Carry matched trigger property definitions into the editor's existing schema-shape path, using the same shadowing and per-property precedence as validation. Preserve authored overrides and the distinction between authored parameters and host controls; do not require authors to duplicate the bundle in `$schema` or add a Claudine dependency to DMLS. Add Level-1 public LSP assertions for trigger-only completion and hover, with authored override controls. No real terminal or keyboard injection is needed to verify these protocol results.

## Recurrence

No earlier review was available for comparison, so recurrence could not be established. `recurrence: false` records that no repeat was identified, not that the previous review was verified. The implementation log already acknowledges this limitation as its editor completion/hover open item; that implementation note is not an earlier review or authorization to narrow the spec.

## Observations

### The previous review is unavailable

Restore the original `review-1.md` when it becomes available so its findings can be compared and its requested `next` and `implemented` metadata can be updated. This missing artifact does not create another code finding or a human design approval requirement.

## Verification and scope

| Requirement group | Appropriate level and evidence | Result |
| --- | --- | --- |
| File kinds, exported action types, inventory parity, closed actions, open metadata, three typed levels | Level 1: Claudine schema inventory and fixture tests; Darkmatter CLI schema validation | Passed |
| Trigger selection and editor/CLI passive verdict parity | Level 1: discovered-schema fixture suite and parity tests | Passed |
| Embedded identity/imports; disk precedence, shadowing, symlinks, authored overrides, visible malformed-trigger failures | Level 1: Claudine embedded-bundle tests and Darkmatter bundle tests | Passed |
| Launch rejection, fake-provider non-launch, optional cleanup policy, initialization ordering, proxy/sequence/retry preparation, frozen inline completion | Level 1: `launch_trigger_schemas` and existing schema preparation/completion tests | Passed |
| DMLS discovery, event/verb inventory, linking diagnostics | Level 1: DMLS public LSP diagnostics and inventory tests | Passed |
| DMLS trigger-only completion and hover | Level 1 public LSP requests are sufficient; permanent tests assert diagnostics only | Missing behavior and coverage; finding above |
| Schema migration and current documentation | Source/docs inspection; `mapping_only_corpus` runs without bless mode | Migration passes; docs accurately acknowledge the editor limitation, which still conflicts with the spec |
| One-time resolved size/time measurement | Implementation log records 2,497,899 bytes and 48.7 ms on one host; no timing assertion | Present |

The input robustness matrix was reviewed for the changed schema readers: library `kind`, `types`, imported exports, trigger `kind`, `match`, `$schema`, condition/combinator operands, and payload definitions. Tests start from the shipped bundle, change one field, retain positive/violating controls, and exercise absent, null, whole-field and element type errors, empty values, duplicates, and trailing content in block YAML and JSON syntax for disk and embedded origins. Catalog/union semantics and deliberate discriminator non-classification are recorded in the implementation log. No additional reader defect was found; the editor finding concerns projections, not configuration parsing.

Executed on macOS:

- `cd claudine && just test`: 9,063 passed, 9 skipped; exit 0, with three slow reports while the package suites ran concurrently. A focused run of the changed schema and launch suites below checked their timing without the other package's load.
- `cd darkmatter && just test`: 9,987 passed, 10 skipped; exit 0.
- `cd claudine && just lint`: exit 0.
- `cd darkmatter && just lint`: exit 0.
- `cd claudine && just test claudine_schemas launch_trigger_schemas`: 177 passed, no slow reports; exit 0.
- `md schema validate claudine/features/2026-10-21-claudine-schemas/review-2.md` and `git diff --check`: passed.
- Temporary Level-1 LSP probes through the declared DMLS `l1` target confirmed the complete instance table. The probes were removed after collecting their results; no implementation or permanent test source was changed.

The schema suites are declared in their consolidated `main.rs` targets and have ordinary L1 names. `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter` both found zero stranded tests. This feature has no terminal rendering or keyboard behavior that needs Level 2 or Level 3. Cross-OS proof is left to CI and does not block this review. No commit, source formatting change, live provider, audio playback, or terminal window was made.
