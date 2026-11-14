---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-14T10:39:36-08:00'
spec: 2026-11-12-registration/spec.md
implemented: false
description: 'A **feature** review of `2026-11-12-registration/spec.md`'
feature: 2026-11-12-registration/review-5.md
previous: 2026-11-12-registration/review-4.md
---

# Registration review, iteration 5

The feature is **production ready** under this review's criteria. Both findings from review 4 are verified fixed, including every additional instance recorded during the repair. The broader specification review found no new actionable defect or verification-level gap. No human decision is needed.

## Previous Finding Verification

I read review 4 and its repair's Finding Ledger in `log.md` before reviewing the implementation. I also read the earlier reviews and the implementation record. Reviews 2–4 closed the earlier findings; none remains independently open. None of reviews 1–4 contains author rulings. Previously accepted implementation decisions remain accepted.

The outcomes below come from source inspection and fresh executions of the public preparation APIs, shipped CLI paths, and editor protocol tests, rather than completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R4-F1 — Inline source attribution misses rewritten prompts and block errors | R4-F1.1–R4-F1.17, plus the repair ledger's R4-F1.18–R4-F1.26; individually checked below | verified fixed | Claudine's [prompt-origin translation](../../../claudine/lib/src/composition/prepare.rs) recognizes the generated root's stable identity and handles shell, block, collection, table-of-contents, and file-link errors. Darkmatter preserves that identity through rewrites. Fresh library and CLI regressions pass, including local and remote child controls. | None. The additional cases were discovered and repaired during implementation of review 4; they are not new findings or partial repairs in this review. |
| R4-F2 — Claudine skill incorrectly says pattern-key defaults are skipped | R4-F2.1: correct the statement and sweep related guidance | verified fixed | The [Claudine skill](../../../.claude/skills/claudine/SKILL.md) no longer claims that pattern-key defaults are skipped. Simplified Schema's [shared default walk](../../../simplified-schema/lib/src/defaults.rs) visits matching patterns; all 58 required-default tests pass, including the 17 object-child cases. Related guidance describes the implemented behavior. | None. |

The following table accounts for every child row of the R4-F1 ledger. Tests exercise ordinary inline prompts, with one change to the prompt or frontmatter for each error shape. Clean controls retain their expected behavior.

| Required child | Verification evidence and result |
|---|---|
| R4-F1.1 — Literal directive control | `inline_shell_execution_failure_names_the_prompt_of_the_real_file` passes: real file, `prompt`, no generated line. |
| R4-F1.2 — Frontmatter-only interpolation control | The frontmatter-only case in `inline_shell_failures_after_rewrites_and_in_blocks_name_the_prompt` passes with the same attribution. |
| R4-F1.3 — Earlier body interpolation | The `Update {{ word }}.` case passes: the source identity survives interpolation, and the public error names `prompt`. |
| R4-F1.4 — Earlier structural rewriting | The removed `::block` case passes without publishing a temporary-body line. |
| R4-F1.5 — Command interpolation | The `mkdir {{ word }}` case passes with real source/property attribution. |
| R4-F1.6 — Shell-block command wrapper | Library and CLI cases pass. The outer failure has no generated coordinates; the retained shell cause names `frontmatter.prompt`. |
| R4-F1.7 — Shell block after interpolation | The rewritten-body block case passes with the same outer/inner attribution. |
| R4-F1.8 — Invalid shell-block option | `timeout=bad` passes the attribution assertions in library and CLI tests, while retaining the invalid-timeout reason. |
| R4-F1.9 — Unclosed shell block | Library and CLI cases name the real file and `prompt`, retain the missing-closer reason, and omit generated lines. |
| R4-F1.10 — Approval publication controls | `inline_shell_preflight_names_the_prompt_property_not_a_generated_line` covers interpolated text and invalid block options. Recording-handler coverage verifies the approval request itself. |
| R4-F1.11 — Passive late-value controls | Both `inline_late_value_*` tests pass: the prompt is inspected without executing it, and a failure retains its property attribution. |
| R4-F1.12 — Local child control | The child shell-execution and directive-error tests pass. The child's shell-block option failure retains `child.md:3`. |
| R4-F1.13 — Remote child control | All three `network_transclusion::an_inline_prompt_keeps_a_remote_childs_*` tests pass for shell, block, and collection failures. Remote text and coordinates are not captured by the root translator. |
| R4-F1.14 — Ordinary body/frontmatter controls | `inline_shell_preflight_keeps_body_child_and_frontmatter_origins` passes, retaining authored body, child, and frontmatter-value positions. |
| R4-F1.15 — Warning publication | `inline_prompt_warnings_carry_no_generated_line` passes: root warnings omit generated positions; the local child retains its own line. |
| R4-F1.16 — CLI rendering | The four rewritten-directive/block-error regressions pass through `inline-compose`; errors name `prompt`, omit generated coordinates, and the fake provider never launches. |
| R4-F1.17 — Comments and current documentation | Read the revised `PromptOrigin` contract, Claudine's pre-flight topic, Darkmatter's directive/error topics, and affected skill guidance. They describe stable root identity, property attribution, and preserved child origins. |
| R4-F1.18 — Collections and repeated copies | `inline_collection_and_pairing_failures_name_the_prompt` passes all ten cases, including callback/gate failures and commands inside repeated copies. Local/remote child controls and the CLI collection case pass. Iteration indices remain data positions. |
| R4-F1.19 — Pair-scanner messages | `block_pairs::tests::reasons_are_position_free_and_their_line_is_a_field` passes. Claudine publishes the reason without Darkmatter's separate position note. |
| R4-F1.20 — Table-of-contents links | Root cases before/after interpolation, the local child case, Darkmatter's public document-identity cases, and the CLI case pass. |
| R4-F1.21 — File links | The invalid-depth CLI/library cases pass. Darkmatter's public cases also cover missing directory targets, file-backed documents, local children, and generated identities. |
| R4-F1.22 — Ordinary-file secondary positions | `pairing_errors_keep_their_secondary_lines_for_an_ordinary_file` passes all five rows. The secondary opener/enclosing-block positions remain in Darkmatter's diagnostic note. |
| R4-F1.23 — Tolerated directive warnings | Claudine's tolerated-warning tests and Darkmatter's identity/no-identity controls pass. Root messages contain no generated line; a local child's message keeps its own line and file. |
| R4-F1.24 — File-system construction failure | Inspected the `::file-links` construction-error branch in Darkmatter's `transclusion/engine.rs`: a generated identity selects an empty position suffix. Ordinary documents retain their line. This rare construction failure needs no filesystem-race test to establish that formatting invariant. |
| R4-F1.25 — Self-transclusion ownership control | `inline_prompt_transcluding_its_own_source_is_a_cycle` passes. Cycle detection stops the source from becoming a child before its commands run; discovery offers none of those commands for approval. |
| R4-F1.26 — Child error publication in md | Library and CLI `a_toc_or_file_links_failure_names_the_document_holding_it` cases pass. Structured diagnostics use the child's error document; tolerated warnings preserve it too. |

I also rechecked the earlier repaired classes through the fresh registration, permission, default, and performance-report suites. Foreign event validation, sequence-selected main schemas, configured URL reads, invalid-policy refusal before per-run grants, passive template classification, union/default arbitration, pattern-child defaults, and initialized performance-report nesting remain covered and pass. No earlier finding is reopened.

## Unblocked Findings

None.

## Blocked Findings

None. No item meets the human-stop test, and no later human-review item is warranted.

## Recurrence

No finding recurs in this review. Review 4's source-attribution class is closed by the stable identity and the completed wrapper/publication sweep. Its implementation-discovered siblings are included in the verification table above, rather than silently omitted or reported as new work.

## Specification coverage and verification levels

The review covered the configuration loader and schema admission, permission evaluation and references, locked writes and migration, shared defaults, registration admission/selection/layering, immutable Claudine publication, foreign composition, editor invalidation, and diagnostic publication. I compared the affected documentation with those contracts. The permission-widening workflow retains trusted base-code comparison and a separate approval job; the merge gate remains a fold of job results. The repository-setting activation work already assigned to the author remains an accepted operational decision, not a new code finding.

| User-facing requirement | Required verification level | Evidence and assessment |
|---|---|---|
| Located configuration errors, scope defaults, explicit-empty replacement, union/exclusions, and committed restrictions | L1 public API plus CLI wiring | `config_load`, `config_fields`, `config_permissions`, `config_notices`, references, and widening tests pass. Invalid permissions remain errors before downstream authorization. |
| Preserving saves, repeated setup, cooperating writers, failed replacement, and migration recovery | L1 filesystem/public API plus isolated CLI | Configuration writer, persistence, initialization, migration/receipt, Claudine setup, and YAML conversion suites pass. The writer re-reads under resolved-target locks and validates before replacement. |
| Schema reads/listings, URL consent and redirects, shell rules/templates, and built-in refusal | L1 public API, loopback HTTP, CLI/LSP protocol | Shared permission tests, network tests, Claudine parity, and DMLS classification pass. Passive readers install no effects. |
| Required defaults, strict/soft behavior, invalid values, nested/pattern containers, unions, composition order, prompting, and editor presence | L1 library plus normal CLI/LSP paths | Required-default, composition-default, CLI schema, and editor regressions pass. The minimal-feature suite also passes. |
| Host identification, ambiguity, deduplication, layered imports, foreign runtime separation, publication, and editor changes/deletion | L1 public API plus CLI/LSP protocol | Host registration/selection, installed Claudine registration, foreign composition, and watcher tests pass. Publication verifies immutable member bytes before replacing the stable manifest. |
| Inline prompt attribution after rewrites, directive failures, approvals, and child preservation | L1 public preparation results plus CLI wiring | Every previous ledger case above is checked. These are semantic source-attribution requirements, with no terminal-input encoding or layout requirement. |
| Schema ownership, shipped definitions, and text-only support | L1 source/artifact guards and minimal-feature build/tests | Ownership/artifact checks pass; the Simplified Schema recipe's minimal-feature suite passes all 1,845 tests. |

No new acceptance requirement depends on physical key events, terminal-emulator layout, or browser interaction. There is no L2/L3 mismatch. Reviewed regression sources are compiled by declared targets and selected as L1 tests; `just check-tier-coverage claudine darkmatter` reports no stranded tests. Cross-OS proof remains with CI and does not determine readiness in this review.

## Input robustness matrix

The default outcome is a located error for invalid configuration shapes, with no authorization or publication from that invalid input. The specified exceptions remain explicit: absent settings use schema defaults; authored empty lists replace a scope's defaults; invalid registration entries are diagnosed and omitted independently of valid permissions; a refused manifest publishes no partial profile.

I inspected the shared admitted-YAML loader, configuration field admission, permission-reference reader, manifest admission, and Claudine's schema-checked YAML and temporary JSON5 conversion loaders. Fresh public-result suites cover absent, explicit/implicit null, wrong whole-field types, mixed and wholly invalid collection elements, and empty values. Shared validators have matrix coverage with controls for their callers. Document-loader tests reject duplicate keys, invalid/trailing YAML, and multiple YAML documents; JSON5 conversion covers its distinct syntax and refusal behavior. Positive fixtures show that the edited fields affect the returned configuration or published profile.

The requested smell search included `#[serde(default)]`, `Option<T>`, `filter_map`, `unwrap_or_default()`, and `.ok()`. No new silent fallback was established. Manifest deserialization follows closed-schema admission and supplies no field defaults. Claudine's presence-sensitive repository values are checked before deserialization. Registration's permissive candidate-path enumeration is for watching failed dependencies; actual member admission independently refuses malformed or unresolved references. The new prompt/directive handling is text grammar and error projection, outside the configuration matrix.

## Verification performed

Fresh canonical recipe runs on this macOS host:

- `darkmatter/`: `just test config_ request_config registration host_selection host_registration_watches composition_defaults foreign_document_compose compose_foreign_documents required_defaults network_permission permission_parity shell_permission_classification directive_error_document pairing_errors report_diagnostics collection_blocks::provenance` — **515 passed**.
- `darkmatter/`: `just test block_pairs::tests context_identity error_snapshots::page_block error_snapshots::toc_linking schema_ownership shipped_artifact_is_fresh` — **58 passed**.
- `claudine/`: `just test-library inline_shell inline_late_value inline_prompt inline_collection inline_toc inline_directive inline_tolerated network_transclusion an_inline_prompt_step_refusal` — **38 passed**.
- `claudine/`: `just test-library config permission installed_registration darkmatter_setup` — **728 passed**.
- `claudine/`: `just test-cli inline_prompt_shell_preflight wrap_perf sequence_perf` — **37 passed**.
- `simplified-schema/`: `just test required_defaults` — **58 passed**, followed by the recipe's minimal-feature suite, **1,845 passed**.
- Repository root: `just check-tier-coverage claudine darkmatter` — exit 0, **zero stranded tests**.

Every selected test completed below five seconds. The repair's two recorded loaded-run cases (6.43 s and 7.85 s, load about 28 on 16 cores) have isolated measurements of 1.88 s and 3.20 s, satisfying the supplied loaded-host rule. No outstanding slow-test failure was found in the reviewed evidence. The existing linker unwind-table warning did not prevent compilation or execution.

No production code or tests were edited for this review. No formatter, commit, real provider, terminal window, or browser was used.

## Implementation record accuracy

The verified behavior agrees with the iteration-4 ledger and completion report. I found no newly false behavioral statement in `log.md` or `implementation-log.md` requiring an in-place correction. Earlier dated corrections remain historical evidence of the behavior observed before the corresponding repairs.
