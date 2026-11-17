---
$schema: feature-review.yaml
ready: false
findings:
  - 'R4-F1 (high): Inline source attribution misses rewritten prompts and block errors'
  - 'R4-F2 (low): Claudine skill incorrectly says pattern-key defaults are skipped'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-14T08:33:54-08:00'
spec: 2026-11-12-registration/spec.md
implemented: true
description: 'A **feature** review of `2026-11-12-registration/spec.md`'
feature: 2026-11-12-registration/review-4.md
previous: 2026-11-12-registration/review-3.md
next: 2026-11-12-registration/review-5.md
log: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/log.md
implemented_by: claude/opus
---

# Registration review, iteration 4

The feature is **not production ready**. Every required case from review 3 is verified fixed, including the repair ledger's four additional cases. An expanded sweep found sibling diagnostic failures after a prompt is rewritten and in shell-block errors. One skill statement also contradicts the repaired default behavior. Both findings have bounded repairs within the existing decisions; neither needs a human decision.

## Previous Finding Verification

I read review 3 and the iteration-3 Finding Ledger in `log.md` before checking the implementation. I also read reviews 1 and 2: their findings were closed by the intervening reviews, and none remains open independently. There are no author rulings in those reviews. The following assessment uses source inspection and fresh public-API and CLI executions, rather than the ledger's completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R3-F1 — Inline shell diagnostics publish generated document positions | R3-F1.1–R3-F1.14 from review 3, plus ledger rows R3-F1.15–R3-F1.18; each checked below | verified fixed | The original source-attribution tests pass through public preparation results and the CLI. Read Claudine's [prompt-origin translator](../../../claudine/lib/src/composition/prepare.rs:716), [discovery publication](../../../claudine/lib/src/composition/preflight.rs:280), approval error rendering, and the shared caller paths. | None of the enumerated original inputs. R4-F1 covers newly tested sibling paths: rewritten runtime contexts and error wrappers that the original sweep did not exercise. |

The distinction matters: the original tests remain green; the new failures require additional shapes and error paths. This is not a regression of an original control or a claim that the original tests were ineffective.

| Required child | Fresh verification |
|---|---|
| R3-F1.1 — direct approval | `inline_shell_preflight_names_the_prompt_property_not_a_generated_line` and the direct CLI refusal report the real file, property `prompt`, and no generated line. |
| R3-F1.2 — initialized reread | The initialized public-API shape and CLI route report the same property attribution. |
| R3-F1.3 — loop approval | The loop public-API shape and CLI route retain the property attribution. |
| R3-F1.4 — sequence graph | `an_inline_prompt_step_refusal_names_the_prompt_property` passes through graph approval. |
| R3-F1.5 — retry/resume approval | The retry CLI regression passes after changing command bytes. Resume delegates to the same `preflight_harness_document` and approval projection; it has no separate origin translator. |
| R3-F1.6 — proxy target | The proxy CLI regression names the target's `prompt`. |
| R3-F1.7 — single-line prompt | The single-line public-API refusal reports `prompt`, with line zero internally and no published numeric line. |
| R3-F1.8 — shell-block approval | The block public-API refusal reports `prompt`; execution and parsing of blocks are distinct new sites below. |
| R3-F1.9 — execution failure | The original unchanged-body `mkdir taken` public-API and CLI fixtures name the real source and `frontmatter.prompt`, without an invented line or excerpt. |
| R3-F1.10 — late-value failure | `inline_late_value_failure_names_the_prompt_of_the_real_file` checks the returned error's source and property. Its chained control keeps body line 6. |
| R3-F1.11 — chained control | The original body approval keeps line 6. |
| R3-F1.12 — local child control | Approval and execution tests retain `child.md:3`. |
| R3-F1.13 — frontmatter shell value | The original control retains line 3 and property `value`. |
| R3-F1.14 — expression/missing-file controls | The translator mutates only the two direct shell-expansion error arms; the original expression and missing-file paths are untouched by this repair. |
| R3-F1.15 — approval-handler request | A recording handler receives the real file and `Frontmatter { key: "prompt", line: None }`. No interactive prompt is opened. |
| R3-F1.16 — warnings | `inline_prompt_warnings_carry_no_generated_line` clears the root warning's line and retains the child's line 3. |
| R3-F1.17 — passthrough audit | `harness/audit.rs` reads the real source file, without using the temporary inline body. Its source contract rules out this projection defect. |
| R3-F1.18 — remote child | `an_inline_prompt_keeps_a_remote_childs_shell_failure_its_own` retains the remote child's text and line 1 instead of attributing it to the root. |

I also reran the required-default suite and initialized performance-report regressions from review 2. Pattern-key defaults, union controls, strict/soft results, editor checks, and both initialized report paths pass. No earlier finding needs to be reopened.

## Unblocked Findings

### R4-F1 (high): Inline source attribution misses rewritten prompts and block errors

**Authority — defect:** a normal Claudine `inline-compose` prompt containing `Update {{ word }}.` before a permitted failing shell command still reports `unknown` and `Origin: line 11` for a nine-line source file. A failing command inside `::shell-block` reports block/command lines 13/14 for an eleven-line file. These are wrong source descriptions for ordinary inputs, in code inspected during this review. This finding extends the source-attribution class of R3-F1 to newly checked sibling paths.

**Defect class:** source attribution is inferred from mutable temporary text or applied to only some error wrappers, so later diagnostics expose temporary-body coordinates instead of the source's `prompt` property.

In the Claudine package, [`PromptOrigin::shows_generated_text`](../../../claudine/lib/src/composition/prepare.rs:744) recognizes a file-less context only when its text equals the original temporary body or reconstructed document. Body interpolation and conditional-block removal change that text before a shell fails. The [decision in `shell_error`](../../../claudine/lib/src/composition/prepare.rs:778) then declines to translate a genuine root error. This same decision handles the location-bearing shell-expansion error variants, so changing the command's failure reason cannot repair the source identity.

The [error dispatcher](../../../claudine/lib/src/composition/prepare.rs:799) also ignores `ComposeFailed(ShellBlock(...))` and `ComposeFailed(PageBlock(...))`. Darkmatter's [`ShellBlockError`](../../../darkmatter/lib/src/markdown/compose/shell_blocks/types.rs:98), used to report block parsing and execution failures, carries its own line/excerpt fields; command failures additionally contain a shell-expansion error. Those outer coordinates must be handled as well as the inner cause. An unclosed shell block reaches the common page-block parser before shell execution and has the same attribution problem.

Reproduction using an isolated home, user permission `exact mkdir taken`, an existing directory named `taken`, and the existing fake Goose fixture:

```markdown
---
agent: goose
word: taken
prompt: |
  Update {{ word }}.

  ::shell mkdir taken
---
# Body
```

`claudine inline-compose --goose doc.md` exits 1 before any provider launch. Its error contains:

```text
unknown
Command: mkdir taken
Origin: line 11
```

The authored command is on line 7. Replacing only `Update {{ word }}.` with `Update.` makes the existing translator work. Replacing the directive with a `::shell-block` containing the same command fails attribution even without interpolation. A mistyped `timeout=bad` and a missing `::end-block` also produce unqualified temporary locations.

The bounded sweep used the existing inline fixture, changed its prompt or one frontmatter value, and checked the returned public errors. Four representative forms also ran through the CLI. The instance table includes controls and the later decisions that publish the location.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R4-F1.1 | Direct shell-error translation, control | Literal prompt, literal failing command | Real file and `prompt`, no numeric line | Same; clean |
| R4-F1.2 | Frontmatter-stage control | Unrelated `title: '{{ 1 + 1 }}'`, literal prompt body | Real file and `prompt` | Same; clean |
| R4-F1.3 | Runtime context recognition after body interpolation | `Update {{ word }}.` before literal `mkdir taken` | Public error has `unknown`, `Body { line: 12 }`; the smaller CLI fixture reports line 11 | Real source and `prompt`; no unproven line or excerpt |
| R4-F1.4 | Runtime context recognition after structural rewriting | True `::block` removed before the same command | Public error has `unknown`, body line 14 | Same source-attribution rule after block removal |
| R4-F1.5 | Runtime context recognition after command interpolation | `::shell mkdir {{ word }}`, `word: taken` | Public error has `unknown`, body line 12 | Same source-attribution rule for the filled command |
| R4-F1.6 | Shell-block command-error wrapper | Literal prompt and failing command in a block | Public outer error retains block/command lines 14/15 and a generated excerpt; inner source is `unknown`; smaller CLI fixture reports 13/14 | Attribute root block and cause to `prompt`, suppress unproven outer and inner positions |
| R4-F1.7 | Shell-block command-error wrapper after rewriting | Same block after `Update {{ word }}.` | Same untranslated outer and inner locations | Same rule after rewriting |
| R4-F1.8 | Shell-block parse-error wrapper | `::shell-block timeout=bad` | Public and CLI error report line 3 without source or property | Name real source and `prompt`; a temporary-body line must not masquerade as an unqualified source position |
| R4-F1.9 | Shared block-pair parser for shell blocks | Remove `::end-block` from the same prompt | Public `PageBlock` error has `unknown`, opening line 3; CLI names file `unknown` | Identify the source's `prompt`, without an invented file line |
| R4-F1.10 | Approval publication: bytes and candidates | Interpolated prompt or bad block option with uncovered `mkdir taken` | Refusal correctly has real file, line 0, property `prompt` | Same; clean; discovery is not the failing runtime context decision |
| R4-F1.11 | Passive late-value validation | Original late-program rejection and expanded literal-command controls | Original error is correctly attributed; expanded literal inputs remain accepted without execution | Same; clean; passive acceptance is not proof of runtime attribution |
| R4-F1.12 | Local transcluded child | Child's failing command | Child file and line 3 retained | Same; clean |
| R4-F1.13 | Remote transcluded child | Remote child's failing command | Child text and line 1 retained | Same; clean; do not restore the earlier heuristic that rewrote every `unknown` context |
| R4-F1.14 | Ordinary body and frontmatter-value controls | Chained command and inline `value: $(...)` | Authored body/key locations retained | Same; clean |
| R4-F1.15 | Warning publication | Root and local-child unknown-option warnings | Root has no numeric line, child retains line 3 | Same; clean |
| R4-F1.16 | CLI error rendering | Interpolated directive, block command, invalid block option, unclosed block | Each exposes the public error's temporary coordinates or missing source; fake provider never launches | Publish the corrected source/property for each error form |

Direct, initialized, loop, sequence, retry/resume, and proxy callers share the temporary-document builder and error translation. The original route tests above pass; there is no independent runtime mapper in a caller that fixes the new failures. Within this boundary, the current unclosed-block route produces `PageBlock`, rather than the `ShellBlockError::Unterminated` variant; the reproduction checks the route a caller actually reaches. Other location-bearing shell-expansion variants share `location_mut` and the same context-recognition decision.

**Recommended repair:** give the temporary root a source identity that survives body changes, using the existing source/origin mechanisms, and handle the root-owned block-error wrappers before publication. Preserve local and remote child identities. Keep the earlier minimum contract: real source plus `prompt`, suppressing numeric positions and excerpts that cannot be proven. No YAML scalar source map, new escaping layer, or per-route process matrix is needed. Extend the public-result tests with rewriting and block errors, and keep a few CLI checks. Update the affected comments and current documentation to describe the resulting coverage.

### R4-F2 (low): Claudine skill incorrectly says pattern-key defaults are skipped

**Authority — spec, “Diagnostics and documentation” completion criterion:** “the configuration, permissions, defaults, host registration, and affected skills describe the implemented behavior.” The [Claudine skill's configuration guidance](../../../.claude/skills/claudine/SKILL.md:59) still says, “The default engine never fills values under `patternProperties`,” and uses that assertion to explain its explicit event/provider keys.

That is false after the R2-F1 repair. Simplified Schema's [object default walk](../../../simplified-schema/lib/src/defaults.rs:383) visits every matching pattern, and all 17 `object_children` cases in the freshly passing required-default suite confirm the behavior through public validation results.

Remove or correct the obsolete explanation. Keeping Claudine's explicit keys is reasonable and requires no schema refactor. This is a documentation defect, not a reason to restore the old default limitation.

## Blocked Findings

None. Both findings can be repaired within the specification and existing decisions. No item meets the human-stop test, and no later human-review item is needed.

## Recurrence

R4-F1 repeats the source-projection class of **review-3.md, R3-F1 — Inline shell diagnostics publish generated document positions**. It is a sibling set missed by that sweep: runtime contexts after body interpolation or block removal, and the `ShellBlock`/common block-parser wrappers. The repair added a text-equality decision that works for the original unchanged-body failure but does not survive the stages that change that text. Its wrapper dispatch also omits block errors. The previous block instance tested approval, not a block command failing or its syntax being rejected.

The original ledger's eighteen cases are fixed; the new finding does not undo those checks. This review carries the full newly checked set and clean controls in one class finding. A stable source identity and bounded wrapper handling address the causes without a new general source-mapping requirement. R4-F2 is newly found documentation drift after R2-F1, rather than a recurrence of the repaired traversal defect. Accordingly, `recurrence` is `true` because of R4-F1.

## Input robustness and verification levels

The configuration/manifest default remains: invalid shapes produce located errors and cannot authorize effects. Exceptions defined by the spec are unchanged: absent properties receive schema defaults; explicit empty policy lists replace their scope's defaults; a bad registration is diagnosed and omitted while independent valid permissions remain usable.

I inspected the shared admitted-YAML loader, configuration field admission, referenced permissions, manifest admission, and Claudine's schema-checked loader. The fresh suites check absent, null, wrong whole-field types, mixed/wholly invalid collection members, and explicit empty values through their public results. Validators shared by several fields are tested once as a matrix, with wiring controls for the other fields. The YAML loader cases cover duplicate keys, invalid/trailing content, multiple documents, and YAML's implicit null. Claudine's migration tests cover its distinct JSON5 loader and conversion behavior. Positive configuration and manifest fixtures establish that the edited fields are load-bearing.

The parse-smell search found no new permissive reader defect. Manifest serde reads have no field defaults and follow closed-schema admission. Configuration string iteration follows admission; invalid registration entries retain findings instead of disappearing silently. Claudine's presence-sensitive repository fields are checked before deserialization. Its optional YAML parsing for an editor diff is followed by the locked validating writer, rather than being an authorization fallback. Markdown prompt syntax and block parsing are text grammar, outside this matrix.

| User-facing requirement | Required level | Evidence and assessment |
|---|---|---|
| Config defaults, located errors, replacement/union, committed restrictions, references, migration, and preserving writes | L1 public API plus isolated CLI wiring | Configuration, permissions, writer, reference, setup/migration, and widening-comparison cases pass. |
| File reads/listings, shell rules and blacklist, URL consent/redirects, and passive editor classification | L1 public API, loopback HTTP, CLI/LSP protocol | Shared permission and network suites pass, including invalid-config refusal. New source-attribution failures are R4-F1. |
| Required defaults, strict/soft/nested/union behavior, composition order, prompting, and editor presence | L1 library plus normal CLI/LSP paths | Required-default and composition/editor cases pass; the inaccurate skill statement is R4-F2. |
| Identification, layering, immutable publication, foreign runtime separation, shared events, and editor invalidation | L1 public API plus CLI/LSP protocol | Registration/selection, installed Claudine registration, foreign compose, and watcher cases pass. |
| Initialized performance reporting | L1 CLI with fake provider | Direct, inline, sequence, ordinary, and wrapper report controls pass. |
| Physical keys or real-terminal layout | No such new acceptance requirement | No L2/L3 verification mismatch. These semantic diagnostics can be checked without taking terminal or browser focus. |

The relevant test sources are compiled by their declared consolidated targets. The CLI recipe enables `test-fixtures`; the reviewed regressions have ordinary L1 names and were actually selected in the runs below.

## Verification performed

After restoring the temporary probes:

- `claudine/`: `just test-library inline_shell inline_late_value inline_prompt_warnings an_inline_prompt_step_refusal installed_registration network_transclusion config permission` — **736 passed**.
- `claudine/`: `just test-cli inline_prompt_shell_preflight wrap_perf sequence_perf` — **30 passed**.
- `darkmatter/`: `just test config_ request_config registration host_selection host_registration_watches composition_defaults foreign_document_compose compose_foreign_documents required_defaults network_permission permission_parity shell_permission_classification` — **426 passed**.
- `simplified-schema/`: `just test required_defaults` — **58 passed**, followed by the recipe's minimal-feature suite, **1,845 passed**.

All selected tests were below five seconds. The repair's recorded loaded-run measurements of 6.44 s and 8.29 s have the required load/core evidence and isolated measurements of 1.76 s and 3.14 s; those recorded cases are cleared under the supplied rule. No new slow-test finding was observed. Cross-OS evidence remains with CI and does not determine readiness here.

Temporary public-API probes exposed the expanded error shapes in 0.112 s and recorded discovery/late-validation results in 0.086 s. The four-form CLI probe took 1.629 s and failed its attribution assertions with the output quoted above. An earlier version of the API probe compared a canonical path to its unresolved spelling; that assertion was corrected before accepting its results. The unchanged-body and frontmatter-only controls then passed, while the rewritten-body and block-error failures remained.

All probe additions were removed and their source files restored byte-for-byte. No production code or permanent tests were changed; no formatter, commit, real provider, terminal window, or browser was used. Existing passing suites do not cover the new failing shapes and do not make the feature ready.

## Implementation record correction

Corrected the iteration-3 progress statement in `log.md` that claimed translation “at every site,” and added a dated note describing the rewritten-body and block-error exceptions and the public-API/CLI evidence. The original passing ledger cases remain valid. This factual record correction is not a separate finding or a human-review item. No newly false statement requiring another correction was found in `implementation-log.md`.
