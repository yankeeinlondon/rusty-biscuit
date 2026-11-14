---
$schema: feature-review.yaml
ready: false
findings:
  - 'R3-F1 (high): Inline shell diagnostics publish generated document positions'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-14T07:41:24-08:00
spec: 2026-11-12-registration/spec.md
implemented: true
log: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/log.md
implemented_by: claude/opus
description: 'A **feature** review of `2026-11-12-registration/spec.md`'
feature: 2026-11-12-registration/review-3.md
previous: 2026-11-12-registration/review-2.md
next: 2026-11-12-registration/review-4.md
---

# Registration review, iteration 3

The feature is not production ready. Both findings from review 2 are verified fixed, including every required instance and the repair ledger's added controls. One new finding remains: inline shell diagnostics give positions in a generated document as though those positions belonged to the user's file. The repair fits the existing design and needs no human decision.

## Previous Finding Verification

I read review 2 and both Finding Ledgers in the repair log before examining the changes. Review 2 closed all eight findings from review 1; neither earlier review has human rulings or a still-open earlier finding to carry. The outcomes below come from source inspection and fresh executions, not the ledger's completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1 — Required defaults are skipped beneath pattern keys | R2-F1.1–R2-F1.4: prefix, suffix, regex, and catch-all beside an unrelated pattern; R2-F1.5–R2-F1.11: literal, catch-all, mixed literal/pattern, array, tuple, reference, and conjunction controls; R2-F1.12–R2-F1.14: CLI validation, composition, and editor diagnostics; added R2-F1.15 controls | verified fixed | Read Simplified Schema's [shared object walk](../../../simplified-schema/lib/src/defaults.rs:357): every matching pattern is visited, including alongside a literal property, and only uncovered keys use the catch-all. Ran all 58 `required_defaults` tests, including all 17 `object_children` cases. The CLI conformance test passes for all three selectors, reports the expected default pointer, and composes `Retries 3`. The DMLS protocol test no longer reports missing `x-build.retries` and still reports its required-without-default control. Explicit values, multiple patterns, invalid children, invalid regexes, and every original container control pass. | None. |
| R2-F2 — Initialized composition can panic when performance reporting is enabled | R2-F2.1–R2-F2.2: initialized direct and inline reports; R2-F2.3–R2-F2.7: ordinary direct/inline, initialized without reporting, sequence, and wrapper-only controls; added R2-F2.8 timing-boundary sweep | verified fixed | Read Claudine CLI's [staged preparation boundary](../../../claudine/cli/src/commands/compose/prep.rs:2094): it closes after initialization, rereading, composition, and request construction. Both initialized CLI regressions exit 0 with composition beneath preparation; the fresh final run measured them at 0.913 s and 0.908 s. Ordinary report/stdout parity, initialized sequence placement, initialization without reporting, wrapper collectors, and reconciliation tests pass in the 116-test CLI run. Inspected the ordinary route, sequence placement, zero-duration wrapper initialization, environment substages, and loop return; none supplies the formerly misplaced child. The reconciliation assertion and its tolerance remain intact. | None. |

The repair also fixed an additional defect it discovered: inline shell discovery inspected the source body rather than the `prompt` that execution composes. Its six CLI regressions and three targeted public-API tests pass: permitted commands run, uncovered commands require approval, source-body commands are ignored, and initialization, sequence, retry, and proxy routes use the right text. That behavior is repaired. R3-F1 concerns the locations attached to that text; it is not a partial repair of either finding above.

## Unblocked Findings

### R3-F1 (high): Inline shell diagnostics publish generated document positions

**Authority — defect:** in the Claudine package, an ordinary `inline-compose` document containing an uncovered shell command in its `prompt` reports `doc.md:10` even when the source file has only eight lines and the command is on line 6. An approved command that fails similarly renders an invented line-10 excerpt and an `unknown` source. These are wrong results for a normal document and command failure, independently of whether this feature introduced the original inline implementation.

**Defect class:** a projection that moves authored frontmatter into a generated Markdown body loses its source identity and exposes the generated body's coordinates as authored file positions.

Claudine's [`document_for_mode`](../../../claudine/lib/src/composition/prepare.rs:697), which supplies the document for inline preparation and discovery, constructs `Markdown::with_frontmatter(frontmatter.clone(), prompt)`. That object contains the prompt twice: once as frontmatter and once as the body. A body's line count consequently starts after the serialized frontmatter. The [single-document approval collector](../../../claudine/lib/src/composition/preflight.rs:100) and [sequence graph collector](../../../claudine/lib/src/composition/preflight.rs:188) publish `entry.origin.line_number()` without translating it. The [late-value validator](../../../claudine/lib/src/composition/prepare.rs:1590) and actual inline composition also carry the generated origin into their errors. The temporary document has no file source, explaining the `unknown` execution context.

Reproduction, with an isolated home containing ordinary default permissions and a local fake Goose executable:

```markdown
---
agent: goose
prompt: |
  Update the body.

  ::shell echo unpermitted
---
# Body
```

Run `claudine inline-compose --goose doc.md` without a terminal. It exits 1 and reports that `echo unpermitted` at `doc.md:10` requires approval. The source command is on line 6. Change only the command to `false` and approve `exact false` in user `permissions.shell-commands`: it correctly refuses the failed execution, but labels its origin `line 10`, names its context `unknown`, and shows the generated body as a source excerpt. Approval and execution decisions are correct; their source descriptions are not.

The sweep copied the existing inline fixtures, changed the prompt command or its permission, and ran the shipped CLI. The retry and proxy controls use local providers only. The late-value case uses the public library result. All probe additions were removed after collecting evidence.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | Direct inline approval | Eight-line reproduction above; uncovered command at line 6 | Refusal names `doc.md:10` | Name the authored command or `prompt` property; never claim a generated line belongs to the file |
| R3-F1.2 | Initialized reread and approval | Same prompt plus `initialize: [{stdout: initialized}]` | Refusal names line 12; command remains on line 6 | Same source-attribution rule after rereading |
| R3-F1.3 | Inline loop approval | Same prompt plus `loop: {until: 'false', max: 1}` | Refusal names line 13; command is on line 6 | Same rule for loop preparation |
| R3-F1.4 | Sequence graph approval | One inline document step, same uncovered prompt command | Refusal names the step file at line 10 rather than line 6 | Attribute the step's authored prompt |
| R3-F1.5 | Retry/resume's shared harness approval | Existing retry fixture: first attempt's `echo attempt-first` is permitted; retry changes `word` to `second` | Retry correctly requires approval, but names line 16 rather than authored line 7 | Preserve prompt origin across the fresh attempt; resume uses the same `preflight_harness_document` projection |
| R3-F1.6 | Proxy target approval | Existing successful source redirects to an inline target with the same uncovered command | Refusal names `target.md:10` rather than line 6 | Attribute the target's authored prompt |
| R3-F1.7 | Single-line prompt projection | `prompt: '::shell echo unpermitted'` on line 3 | Refusal points to line 5, the source's `# Body` | Identify the prompt property or its proven authored location |
| R3-F1.8 | Shell-block command projection | Prompt contains `::shell-block`, command on line 5, then `::end-block` | Refusal names line 9 in an eight-line file | Attribute the prompt's command, without generated block coordinates |
| R3-F1.9 | Actual inline shell execution failure | Prompt command `false`, permitted by `exact false` | `unknown` context, generated excerpt, `Origin: line 10` | Real source plus prompt origin; omit a numeric location if it cannot be proven |
| R3-F1.10 | Passive late-value validation | Existing late-program fixture: prompt contains `::shell {{ last(seq.outputs) }} probe` | Public `ShellExpansionFailed` carries `Body { line: 9 }` and `SourceContext { absolute: "unknown", ... }`; authored command is line 5 | Preserve prompt origin in the returned error as well as formatted output |
| R3-F1.11 | Chained-document approval control | Same uncovered directive as the real body on line 6 | Refusal names line 6 | Same; clean |
| R3-F1.12 | Transcluded child control | Inline prompt includes `child.md`; child's uncovered directive is on line 3 | Refusal names `child.md:3` | Same; clean; keep child origins when fixing the root projection |
| R3-F1.13 | Frontmatter shell-value control | Inline source has `value: $(echo unpermitted)` on line 3 | Refusal names line 3 | Same; clean for this fixture |
| R3-F1.14 | Prompt expression and missing-file controls | Replace prompt directive with `{{ length(true) }}`, `{{ 1 / 0 }}`, or `::file ./missing.md` | Expressions identify the `prompt` property without invented lines; missing-file error names the reference without invented coordinates | Same; clean for this defect class |

The remaining sequence JIT validation path calls `approve_document_shell` with the same explicit inline mode; its projection is the shared site covered above, not another origin translator. Staged, loop, retry/resume, and proxy callers likewise supply no independent mapping that repairs the origin before publication. There is no need for separate fixes in each caller.

**Recommended repair:** keep the shared text projection, but retain that its generated body came from the source's `prompt`. Apply that origin consistently to discovered commands, passive shell-validation failures, and execution errors. The smallest acceptable repair names the real file and `prompt` property and suppresses unproven numeric positions and excerpts; an exact mapping for YAML block scalars is not required to remove the harm. Preserve genuine child-file and ordinary frontmatter origins. Add public-result assertions for discovery, late-value validation, and execution, with a few CLI wiring checks; extend the existing route tests rather than adding another process matrix. Do not change authorization decisions or disable the guard that rejects commands absent from preflight.

The repair log already notes the incorrect inline line numbers, but sets them aside because the spec does not explicitly require inline line mapping. That does not close this finding: the review instructions also cover wrong results from realistic inputs in existing code. The recommendation above avoids turning this into a general source-map project.

## Blocked Findings

None. R3-F1 has a bounded repair within the existing contract. No decision requires a human stop or later human review.

## Recurrence

No finding repeats an earlier review's defect class. R2-F1's default-container traversal and R2-F2's timing boundary are fixed. Review 1's R1-F8 concerned lost schema-default attribution for an inserted value, with no fabricated authored line; those instances remain fixed. R3-F1 concerns authored prompt text copied into a generated Markdown body and the resulting false numeric locations. It is a newly reviewed source-projection defect, not an unchanged instance or regression of the earlier default repair. The repair log's separate observation of this problem was never a previous review finding.

## Input robustness and verification levels

For configuration and manifest loading, the default invalid-input outcome remains a located error without authorization or effects. The intentional exceptions are unchanged: absent settings receive their schema defaults; explicit empty policy lists replace defaults; a malformed registration is diagnosed and omitted while independent valid permissions remain usable.

I inspected the admitted-YAML loader, Darkmatter configuration validators and reference loader, manifest admission, and Claudine's schema-checked loader. Fresh public-result suites exercise absent, explicit null, wrong whole-field types, mixed and wholly invalid collection elements, and explicit empty values. Loader cases cover duplicate keys, trailing or invalid YAML, and multiple documents. The configuration and registration controls establish positive loads. Claudine's JSON5 migration cases cover its distinct legacy loader. The load-bearing parse-smell check found no new permissive fallback: manifest fields have no serde defaults; configuration iteration follows admission and preserves invalid-entry diagnostics; Claudine deserializes after its schema check. No new reader finding is warranted. Markdown prompt syntax and schema grammar are outside the configuration-shape matrix.

| User-facing requirement | Appropriate verification level | Evidence and assessment |
|---|---|---|
| Configuration defaults, shape errors, scope replacement/union, committed policy, references, migration, and preserving writes | L1 public API and isolated CLI | Configuration, permission, persistence, migration, and host configuration suites pass. |
| File reads/listings, shell rules/exclusions/blacklist, network consent and redirects, passive editor classification | L1 public API, local HTTP fixtures, CLI and LSP protocol | Permission/network/editor suites pass; the new inline discovery tests prove the executed text is audited. Location errors are R3-F1. |
| Required defaults, strict/soft results, nesting/unions, composition reads, prompting, and editor presence | L1 shared-library plus CLI/LSP paths | All R2-F1 instances and added controls pass; no missing verification level. |
| Host identification, manifest publication, layering, foreign runtime separation, shared events, and editor invalidation | L1 public API and CLI/LSP protocol | Host registration/selection, published Claudine profile, foreign composition, and editor watch suites pass. |
| Initialized performance reporting | L1 isolated CLI with fake provider | Both affected routes and ordinary/sequence/wrapper controls pass; R2-F2 is fixed. |
| Physical key behavior or real-terminal layout | No new such requirement in this spec | Configuration and editor semantics need neither terminal focus nor OS input injection. No L2/L3 mismatch found. |

The relevant test files are declared by their consolidated binaries. Feature-dependent CLI fixtures are selected with `test-fixtures`, which the recipes and CI metadata enable. The named regressions run at L1; none was silently excluded by a higher-tier name.

## Verification performed

- `simplified-schema/`: `just test required_defaults` — **58 passed**; the recipe's full minimal-feature run — **1,845 passed**.
- `darkmatter/`: `just test required_defaults complete_an_entry_a_pattern_key config_fields config_load config_references registration host_selection host_manifest config_writers config_migration network_permission foreign_document_compose compose_foreign_documents shell_permission_classification` — **262 passed**.
- `darkmatter/`: `just test config_ request_config host_registration host_selection host_registration_watches composition_defaults foreign_document_compose network_permission shell_permission_classification` — **337 passed**.
- `claudine/`: `just test-library installed_registration network_transclusion config permission` — **724 passed**.
- After removing the temporary CLI probes, `claudine/`: `just test-cli wrap_perf sequence_perf inline_prompt_shell_preflight compose_initialize perf::` — **116 passed**.
- After removing the temporary library probe, `claudine/`: `just test-library inline_shell_preflight inline_late_value` — **3 passed**.
- Temporary isolated CLI probes recorded the source-position failures and controls above. The first eight-command probe took 2.266 s; the nine-test route sweep's longest test took 1.088 s; the corrected shell-block probe took 0.583 s. The public late-value probe took 0.064 s. These probes collected results; their harnesses completing successfully does not mean their observed diagnostics were correct.

All recorded tests in these review runs were below five seconds. The previous repair's three loaded-run measurements between five and ten seconds have isolated measurements below five seconds and the recorded load/core evidence required by the review rule; no unresolved speed failure was found. Cross-OS results remain with CI and do not determine this review's readiness.

Existing test files were restored byte-for-byte after probing. This review makes no production or permanent test changes, runs no formatter, creates no commit, launches no real agent, and opens no terminal or browser window.

## Implementation record accuracy

No new behavior statement in the implementation records was verified false. The log accurately records the inline location problem; its decision to exclude it from review scope is addressed by R3-F1 rather than rewritten as a factual correction. Earlier dated corrections remain intact.
