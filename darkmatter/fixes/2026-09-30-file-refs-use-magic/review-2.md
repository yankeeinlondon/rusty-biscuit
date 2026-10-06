---
$schema: feature-review.yaml
ready: false
findings:
    - title: Claudine schema completion still resolves committed prompt paths by joining directories
      priority: high
    - title: The parity matrix substitutes composition for completion and omits configured CLI roots
      priority: medium
human_review: true
human_review_items:
    - |-
        Review the completeness of the reference tests before starting another automated fix cycle. The first review required every entry point to exercise the shared reference contract, but the new test labeled as completion executes composition instead, and configured-root cases remain excluded from the md command. Confirm that each test calls the feature it names and that every required case either runs or has an explicitly agreed contract change. This repeated coverage problem is why human review is requested; the implementation findings themselves can be fixed without a new design decision.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-01T23:33:22-07:00
spec: 2026-09-30-file-refs-use-magic/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-30-file-refs-use-magic/log.md
description: "A **fix** review of `2026-09-30-file-refs-use-magic/spec.md`"
fix: 2026-09-30-file-refs-use-magic/review-2.md
previous: 2026-09-30-file-refs-use-magic/review-1.md
next: 2026-09-30-file-refs-use-magic/review-3.md
---

# Review 2

This fix is **not production ready**. Claudine now compiles, and the previous editor and preflight regressions pass their new tests. Actual shell completion still disagrees with composition about committed prompt references, however. The parity test conceals that disagreement by running composition under the completion entry-point name. The configured magic-root requirement also remains incomplete for `md`.

## Previous findings

The first review contained five actionable findings and no blocked findings. There was consequently no blocked item to unblock before this implementation.

| First-review finding | Verification in this review | Status |
|---|---|---|
| Claudine cannot compile against the new context APIs | Claudine library, CLI, and generator test targets compile; all three context guards and its composition parity process test pass | Compilation and construction migration fixed; completion behavior has the separate defect below |
| Editor Markdown links reject existing targets outside the workspace index | Narrower-workspace protocol test, graph tests, and editor parity runner pass | Fixed for the swept forms and navigation surfaces |
| Editor directive resolution treats fallback chains as one filename | Composition, preflight, and editor fallback-chain tests pass, including suppression and selected-target navigation | Fixed |
| Preflight omits code and table-of-contents targets | Restored consumer rows and existing/missing-target tests pass | Fixed |
| Parity matrix omits cases and changes the tree-escape failure | CLI tree-escape rows now expect `InvalidReference`; former Windows home-case exclusions are removed; configured-root rows run through library/editor/Claudine composition | Partially fixed; actual completion and configured CLI roots remain uncovered |

## Unblocked Findings

### High — Claudine schema completion still resolves committed prompt paths by joining directories

**Defect class:** a reference reader bypasses the shared file-reference grammar, rejecting supported prefixes and applying repository fallback to explicitly relative paths.

In the claudine-cli package, [resolve_prompt_path](../../../claudine/cli/src/completion/schema_completion/mod.rs) locates the prompt whose schema supplies setter suggestions. It still checks an absolute path, then joins the input onto the launch directory and repository root. It never resolves that input through `FileReference`. Both schema-value and property-name suggestions use this reader. Building a proper context afterward in [load_effective_schema](../../../claudine/cli/src/completion/schema_completion/mod.rs) cannot repair the earlier failure to locate the prompt.

This affects selecting a magic prompt and then completing one of its schema properties. The positional completer emits `@prompt.md`, but committing that exact token makes the schema completer return no suggestions. The comment claiming that selected magic references have been rewritten to concrete paths is stale; the shipped positional command and existing completion-to-composition tests preserve the sigil.

**Reproduction:** reproduce the shared parity fixture's topology with a temporary Git repository containing `area/lib` and `area/cli` workspace members and a separate fixture home. Put identical prompts at `repo/prompt.md`, `repo/prompts/prompt.md`, `home/prompt.md`, and `home/.claudine/prompts/prompt.md`. Their inline schema declares `zebra: enum(red, blue)` and `apple: enum(one, two)`. Invoke the compiled binary directly, without shell expansion:

```text
claudine __complete --current 3 -- claudine compose @prompt.md zebra=
claudine __complete --current 3 -- claudine compose @prompt.md z
claudine compose --dry-run @prompt.md zebra=red apple=one
```

Completion returns no candidates; composition succeeds. Changing only the committed prompt to `prompt.md` produces `zebra='red'`, `zebra='blue'`, and `zebra=`. Repeat with every reference form below, from the repository root and nested package, and with `inline-compose` and `sequence` completion.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `compose` schema-value completion | Committed `&prompt.md`, `^prompt.md`, `@prompt.md`, `~/prompt.md`; both launch directories | No enum candidates; composition accepts all four forms | Locate the prompt through the shared context and offer its enum candidates |
| `inline-compose` schema-value completion | Same forms and launch directories | Same missing suggestions | Resolve the committed prompt through the shared reader |
| `sequence` schema-value completion | Same forms and launch directories | Same missing suggestions | Resolve the committed prompt through the shared reader |
| Property-name completion in all three commands | Same committed prefixed prompts; cursor partial `z` | No `zebra=` suggestion | Locate the same schema and offer `zebra=` |
| Schema-value completion in all three commands | `./prompt.md` from `area/lib`, with only the repository-root copy present | Offers the root prompt's enum candidates; runtime composition rejects the explicit relative path | Do not search the repository root for `./`; no candidates from that other prompt |
| Schema-value and property-name controls | Bare `prompt.md`, absolute prompt path; `./prompt.md` from repository root | Correct suggestions; clean | Keep these results |
| Positional prompt completion | `compose @prompt` at repository root | Emits `@prompt.md`; clean | Preserve and accept the emitted token in subsequent schema completion |
| Composition execution | All four prefixed forms, bare and absolute controls, both launch directories | Resolves successfully; clean | Keep these results |
| Shared completion-context builder | Package/area projection test and construction guard | Passing; clean | Retain the prepared context while fixing its consumers |

The source sweep followed both consumers of `resolve_prompt_path`: effective-schema loading and authored property-order recovery. Both fail at that same prompt lookup for prefixed committed arguments; there is no command-specific reader to fix separately. The order reader also textually joins a referenced schema in [referenced_schema_keys](../../../claudine/cli/src/completion/schema_completion/keys.rs). Include that sibling in the migration and verify its supported schema shapes through public suggestions. No separate ordering regression is asserted here: external-schema probes did not produce suggestions in their positive control, so they do not establish one.

Resolve committed prompt arguments with the built completion context and preserve explicit-relative semantics. Update the stale reader comment. Add actual `__complete` process tests for name and value suggestions in all three commands, including a round trip from an emitted magic token and the nested-package explicit-relative negative control.

### Medium — The parity matrix substitutes composition for completion and omits configured CLI roots

**Defect class:** verification substitutes a different entry point or omits required inputs while reporting the agreed reference matrix as covered.

The new claudine-cli [parity runner](../../../claudine/cli/tests/l1/entry_point_parity.rs) dispatches `EntryPoint::ClaudineCompletion` to `supplied_value`, which runs `claudine compose --dry-run <document> target=<value>`. It never calls `__complete`. That usefully checks caller-supplied composition values, but cannot prove completion's reference behavior. The first finding demonstrates a failing actual completion while this runner passes.

The shared Darkmatter [entry-point table](../../lib/tests/common/entry_point_parity/mod.rs) also uses `UNCONFIGURED` for all three `md` variants, deliberately removing the extra configured-root form. The implementation log records that `md` has no configuration mechanism for extra roots. That explains the gap, but does not fulfill the unchanged requirement that a configured magic path run at every entry point.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Claudine completion matrix dispatch | All caller-value rows, root and package launches | Calls composition; passes without exercising completion | Invoke the completion producer and verify its output, with execution as a separate round-trip check |
| Actual Claudine completion | Committed prefixed prompt references from the first finding | Missing suggestions while the named parity row passes | Matrix must detect the disagreement |
| `md compose` document matrix | Configured extra magic-root form | Excluded by `UNCONFIGURED` | Exercise the configured-root contract through the binary |
| `md schema validate` document matrix | Same form | Excluded by `UNCONFIGURED` | Exercise its configured-root contract |
| `md` caller-argument matrix | Same form, both launch directories | Excluded by `UNCONFIGURED` | Exercise its configured-root contract |
| Compose pipeline, preflight, schema validation, DMLS projections, Claudine composition | Configured-root form | Executable rows pass; clean | Keep these rows |
| CLI tree escapes and home/reference rows | Restored rows in current process runner | Passing on this host; old Windows filtering removed in source; clean | Keep the specified failures and live rows |
| Editor graph and code-action census | File, code, table of contents, schema file, Markdown link | Explicit rows and appropriate no-action assertions exist; clean | Keep the complete consumer census |

Retain supplied-value composition tests under an accurate name and implement the missing completion runner. Complete the specified CLI configured-root capability and its rows, or obtain an explicit contract amendment; excluding rows is not that amendment. No new terminal tier is needed to exercise these reference APIs.

## Blocked Findings

None. Human review is requested because the coverage defect recurs, not because either finding needs permissions, unavailable hardware, or an unresolved implementation decision.

## Recurrence

The second finding repeats **review-1.md — “The parity matrix omits required cases and changes a required failure.”** That fix should have swept the actual invocation behind every entry-point variant and the configured-root row at every CLI route. It restored several missing consumers but left all three `md` variants without configured-root cells and added a Claudine completion variant that invokes composition.

This review swept every shared entry-point variant, each runner's dispatch, CLI form filters, and corresponding actual completion commands. The tables include both remaining omissions and clean siblings. `recurrence: true` requests that the automated review/fix loop stop for a human coverage check before continuing. The remaining behavior and verification defects determine readiness; the human-review request is a separate process requirement.

## Verification and requirement coverage

This fix changes filesystem resolution, composition, CLI results, and language-server protocol responses. **Level 1 is appropriate for each requirement.** There are no new physical-key, terminal-layout, or browser requirements needing Level 2 or Level 3. Completion was invoked directly through the shipped process protocol; no terminal windows were opened or focused.

| Requirement | Verification level and evidence | Result |
|---|---|---|
| Original preflight regression, package lookup, common reference forms and source-opening boundaries | Level 1 library and CLI parity runners | Passing current rows |
| Required contexts, one builder, permitted process-state reads | Level 1 guards in Darkmatter, CLI, DMLS, Claudine library/CLI, generator | Passing |
| Invalid context rejection and one environment for references/expressions | Level 1 request-context builder and request-epoch tests | Passing |
| Preflight file/code/table-of-contents consumers | Level 1 public matrix and existing/missing-target tests | Passing |
| Editor context reuse, invalidation, failure recovery and untitled buffers | Level 1 protocol tests | Passing |
| Editor narrow-workspace and fallback-chain behavior | Level 1 protocol projections and graph tests | Passing |
| Claudine composition and completion parity | Level 1 composition runner and direct completion probes | Composition passes; actual completion fails the cases above |
| Configured extra roots at every entry point | Level 1 public/API rows and CLI row inspection | Required `md` cells absent |
| Declared targets, enabled features, active tiers and repository-input declarations | Cargo manifests, `l1/main.rs` modules, source-input metadata, tier checks | New permanent tests compile and run; no stranded tiers |

Successful local commands:

- Darkmatter: `just test entry_point_parity` — 12 passed; `just test repository_contexts` — 11 passed; `just test request_context` — 22 passed; `just test context_construction_guard` — 4 passed; `just test graph` — 197 passed.
- Claudine: `just test entry_point_parity` — 1 passed; `just test context_construction_guard` — 3 passed; `just test completion_resolution_round_trip` — 3 passed; `just test completion` — 518 passed. The last filter includes tests unrelated to this fix; its count does not establish missing matrix coverage.
- Repository: `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine` — no stranded tests.

These were targeted runs, not a full-suite or lint result. One initial attempt to combine filters with `|` was interpreted by the recipe's shell and failed; the relevant filters were rerun individually. Cross-OS evidence is left to CI and is not a readiness finding. Unchanged messenger and generator behavior was not independently re-reviewed beyond the generator guard run.

The configuration-field robustness matrix is not applicable: this fix changes context ownership and reference resolution, not deserialized configuration fields. The added directive-chain reader handles textual Markdown targets; fallback order, terminal suppression, invalid nonterminal suppression, and alternative spans have grammar and public-result tests. This review does not expand scope to existing manifest or frontmatter deserialization rules.

The two ambient-state sites assigned to the following glob implementation remain outside this review's scope. No production code or tests were modified for this review. Temporary process fixtures are outside the repository; completion observations are retained in `/tmp/file-refs-review2-completion-results.json` on this host. Only this review and the requested review/spec metadata are review output.
