---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - "The first review remains unavailable"
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-14T01:02:34-07:00
spec: 2026-10-21-claudine-schemas/spec.md
implemented: false
description: "A **feature** review of `2026-10-21-claudine-schemas/spec.md`"
feature: 2026-10-21-claudine-schemas/review-3.md
previous: 2026-10-21-claudine-schemas/review-2.md
---

# Review 3: Claudine Schemas

The feature is **production ready** within the specification's stated scope. Review 2's editor completion and hover finding is implemented, its complete instance sweep passes, and no new high, medium, or low finding was identified. No additional human design decision is required.

## Previous findings

Review 2 had one unblocked finding: **Trigger schemas do not supply the promised editor completion and hover**. It is resolved.

In the **Darkmatter library**, [EffectiveSchema::property_shape](../../../darkmatter/lib/src/markdown/schemas/mod.rs) now supplies the property definitions used by the editor, layering matching trigger payloads between extensions and the authored schema. Disk and embedded payload resolution retain their resolved object shapes. The authored-only `simplified` field remains separate, preserving decisions about the author's parameters and optional bindings.

In **DMLS**, [the frontmatter provider](../../../darkmatter/dmls/src/providers/frontmatter.rs) and [schema overlay](../../../darkmatter/dmls/src/overlay/schema.rs) use that shared property map. The repair removes their duplicate assembly code. Later trigger declarations and authored redeclarations follow the same whole-property replacement order as validation. DMLS continues to read the bundle from disk and has no Claudine dependency.

The permanent [public language-server tests](../../../darkmatter/dmls/tests/l1/claudine_schema_discovery.rs) reproduce the earlier failing inputs in an isolated repository containing the five shipped schema files. Documents have no authored `$schema` except the explicit override controls. These tests ran in this review:

| Earlier instance | Input or request | Result after repair |
| --- | --- | --- |
| Top-level key completion | `prompt: hi` with partial `mod` | Offers `mode` and `model` |
| Nested parameter completion | `start` list item containing `say`, with partial `te` | Offers `text` |
| Lifecycle item completion | Partial `- sa` | Offers `say`, filtered to the typed prefix |
| Union item completion | Empty `- ` under `start` | Offers action keys and bare verbs, including `stop` |
| Loop key completion | Partial `ma` under `loop` | Offers `max`, without unrelated top-level keys |
| Scalar enum completion | Empty `mode:` | Offers `append` and `replace` |
| Boolean completion | Empty `interactive:` | Offers `true` and `false` |
| Top-level hover | `mode: append` | Describes the enum and its purpose |
| Lifecycle parameter hover | `text` in a `say` item | Describes the required string and identifies the communication alternative by its member verbs |
| Loop hover | `max` under `loop` | Describes its numeric type and iteration purpose |
| Linking hover | `max_turns` | Describes its numeric type and agent-turn purpose |
| Authored-schema controls | Redeclare `mode` as `enum(fast, slow)` | Completion and hover use the authored enum; other trigger properties remain available |
| Existing authored lifecycle controls | Imported lifecycle types in the existing frontmatter provider tests | Nested completion, union item completion, and hover still pass |
| Trigger diagnostics | Typo, valid lifecycle/linking document, and malformed linking value | Typo and malformed value are rejected; valid document is accepted |

Additional controls prove that a nonmatching trigger contributes no editor keys, later matching triggers win shared properties, and trigger declarations outrank extension definitions. The existing schema-change recovery test now also verifies trigger-only completion and hover after an imported schema fails, is repaired, or its directory is recreated. Trigger-declared expression roots no longer receive an undeclared-property warning; expression name completion retains its authored-parameter policy.

The communication hover identifies the category by the alternative's verb list rather than printing the imported type name `communicate`. This is the same rendering as the authored-schema control accepted in review 2; the repair restores that behavior for trigger-only documents.

## Unblocked Findings

None.

## Recurrence

No finding repeats a class from an available earlier review. Review 2's missing editor projection is closed across the sites listed above. The first review remains unavailable, so comparison with that iteration is limited as described below; no earlier findings were invented.

## Observations

### The first review remains unavailable

`review-1.md` is absent from this feature directory, and `git log --all` for its expected path returns no history. Review 2 reported the same limitation. Restore that artifact if it becomes available so its findings and review-chain metadata can be checked; its absence does not block this feature or require a new design approval.

## Verification and scope

The review covered the spec and plan, both implementation logs, the available earlier review, the five schema files, embedded loading and provenance, launch preparation and cleanup, the editor repair, and the associated declared test targets. Requirements were checked at these boundaries:

| Requirement | Appropriate verification level and evidence | Result |
| --- | --- | --- |
| Five files, kinds, exports, action inventory, planned verbs, bundle short forms, closed action maps, open metadata | Level 1: Claudine schema inventory and resolved-shape assertions | Passed |
| Three typed levels and loose bodies below; valid and invalid lifecycle fixtures | Level 1: shipped fixture corpus through disk discovery, embedded loading, Darkmatter validation, parser, and `md schema validate` | Passed |
| Non-null trigger selection, malformed identifying values, inline sequence exclusion, application data | Level 1: fixture verdicts and trigger-selection assertions | Passed |
| Byte-identical embedded files; all imports and exports resolve without a checkout | Level 1: embedded bundle identity and export probes | Passed |
| Disk filename shadowing, ignored file symlinks, relative imports, trigger-envelope reference rejection, embedded fallback, authored precedence, visible discovery failure | Level 1: schema-location and embedded-versus-disk fixtures | Passed |
| Editor discovery, diagnostics, inventory parity, completion, hover, authored overrides | Level 1: public language-server requests and library precedence controls | Passed |
| Launch errors name the malformed key and prevent provider launch; valid launches and authored required-value phases remain intact | Level 1: `compose` and `inline-compose` subprocess tests with isolated fixtures and fake providers | Passed |
| Invalid Claudine controls are retained as errors; authored optional application values still receive cleanup | Level 1: library and CLI launch tests | Passed |
| Initialization precedes the verdict; rejected documents do not dispatch `start`; proxy, sequence, and retry fresh reads use the verdict | Level 1: event-file and provider-recording assertions | Passed |
| Pending values, literal tokens, shared preparation, inline completion, frozen schema and next-run refresh | Level 1: preparation/completion suites and launch trigger tests | Passed |
| Current docs, one schema home, deleted placeholders and old copies | Source/docs inspection; DMLS inventory and `mapping_only_corpus` tests without bless mode | Passed; historical research/spec records remain snapshots |
| One-time resolved payload measurement | Implementation log records 2,497,899 bytes and 48.7 ms on one host; no timing assertion added | Present |

No requirement here depends on terminal rendering, the terminal's input encoder, or OS keyboard injection. Level 2 or Level 3 would not provide stronger evidence for these schema and language-server results. Cross-OS execution evidence remains CI's responsibility and does not block this review.

### Input robustness

The schema-reader matrices use the shipped bundle as the control, change one field per cell, and assert public discovery, trigger selection, or validation results. They cover absent, explicit null, wrong whole-field type, one/every malformed element, empty values, duplicate keys, and trailing content in block YAML and JSON syntax, for disk and embedded origins. The executable tables cover library envelopes and imported definition classes, trigger envelopes, condition/combinator operands, and payload definitions. Controls distinguish missing declarations from explicit empty definitions, and loader failures cannot silently select the embedded fallback.

The review-2 repair introduces no new configuration format or permissive parse path: it retains an already resolved payload's optional SimplifiedSchema projection. A raw JSON Schema payload intentionally has no such projection and still validates. No new reader defect was identified.

### Executed checks

- `cd darkmatter && just test`: **9,997 passed, 10 skipped**, exit 0; no slow reports.
- `cd claudine && just test`: final run **9,063 passed, 9 skipped**, exit 0; no slow reports.
- `cd darkmatter && just lint`: exit 0, including the Zed WASM compile check.
- `cd claudine && just lint`: exit 0.
- `cd claudine && just test claudine_schemas launch_trigger_schemas`: **177 passed**, exit 0; no slow reports.
- `cd claudine && just test-cli document_environment_routes`: **14 passed**, exit 0.
- `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter`: exit 0; **zero stranded tests**.
- `md schema validate claudine/features/2026-10-21-claudine-schemas/review-3.md`, review source-link checks, and `git diff --check`: passed.

The initial Claudine full run overlapped its lint recipe and failed eight environment-capture tests. That lint recipe invokes a CLI test build without `test-fixtures`, sharing the CLI output path with the running fixture-enabled suite. After lint finished, the isolated environment-route tests and the entire full suite passed without source changes. The final evidence above comes from the full rerun with no competing build, rather than treating the first run as green.

The new editor tests are declared by DMLS's consolidated `l1/main.rs`; Claudine's schema and launch tests are likewise declared and selected by live Level-1 recipes. No source implementation was changed for this review. No commit, formatting command, live provider, audio playback, or terminal window was requested or used.
