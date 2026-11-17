---
$schema: feature-review.yaml
ready: false
findings:
    - "R3-F1 (high): TOC fragment validation rejects anchors generated from raw headings"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-16T00:09:35-08:00"
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/log.md
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-3.md
previous: 2026-11-14-file-interpolate-option/review-2.md
next: 2026-11-14-file-interpolate-option/review-4.md
---

# Review 3: File interpolation option

The feature is **not production ready**. The original cases in the previous
review are repaired. One remaining sibling in TOC reference preparation gives
a false missing-heading error through the public API and the shipped CLI.
It needs no author decision: preserve the existing raw-heading contract and
make validation follow it.

## Previous Finding Verification

The required instances are the previous review's tables **and** the child
rows added to its repair ledger in [log.md](log.md#finding-ledger). The second
ledger has 40 child rows: 11 for R2-F1, 13 for R2-F2, and 16 for R2-F3.
There are no author rulings in earlier reviews, no blocked findings, and no
still-open finding from review 1 to carry forward. Review 2 verified all four
review-1 findings fixed; the interpolation regressions remain passing here.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1: Shell discovery skips grandchildren with different inherited values | R2-F1.1–.6: file/prologue/epilogue/nested-expression descendants, resolved targets and pre-approved execution; .7–.9: direct local/remote overlays, settings and cycle controls; added .10–.11: remote grandchildren and other deduplication decisions | **verified fixed** | `visited_key` hashes starting frontmatter, the actual request fingerprint and discovery-only inherited state. The source-only ancestor marker still bounds cycles. All seven `inherited_values` tests pass, including command equality and execution with only discovered approvals. `set_overlay`, local/remote both-setting tests and growing-value cycle tests pass. Other deduplication sets operate on final command bytes or within one fixed parent; they do not reuse recursive discovery across differing inherited values. | None. |
| R2-F2: Reference preparation loses inherited and resolved frontmatter state | R2-F2.1–.8: inherited file/prologue/epilogue state, resolved `when=`, validation/CLI gates and controls; added .9–.12: templated frontmatter targets, shared-child preparations, accumulated heading snapshots and other consumers; added .13: TOC raw-versus-prepared heading comparison | **partially fixed** | Original .1–.8 and added .9–.12 are verified fixed: graph children use `markdown_child_options`; graph conditions and frontmatter targets read the prepared document. The public-result `reference_inheritance` tests and three CLI missing-link tests pass. The snapshot retains both copies' slugs. Added .13 is not clean: a target's own replacement changes the validation slug even under `interpolate=false`; temporary public-API and CLI probes reproduce the false rejection below. | The original cases are repaired. The remaining TOC sibling from added ledger row .13 is inventoried as **R3-F1**, with its own child IDs, rather than duplicating the repaired cases. |
| R2-F3: Replacement options are omitted from child preparation and cache identity | R2-F3.1–.7: cached/uncached objects and precedence, preflight, graph and CLI; .8–.11: remote file/url execution and discovery; .12–.13: code and loaders without directive replacement; added .14–.16: repeated-site identity, TOC inclusion-map isolation and remote frontmatter-reference isolation | **verified fixed** | `directive_child_request` applies the directive replacement and folded prose setting before local/remote preparation and identity capture. The composed-child key includes the derived request and retains the parent identity for cycle termination; `parameters_identity` includes replacement payload/mode. The `replace_options` tests pass for both orders, cache modes, approvals, graph links, remote children, code, TOC one-off isolation and remote prologue isolation. The CLI replacement-reference regression and existing cycle diagnostics pass. Frontmatter references and nested composition derive with default directive options; raw HTTP caches remain value-independent. | None of these replacement-option instances. R3-F1 is the sibling involving the TOC target's own transformations, not a regression of the repaired one-off-map isolation in .15. |

Evidence is in the Darkmatter library's
[inclusion tests](../../lib/src/markdown/compose/tests/file_interpolate.rs),
the CLI's [publication and reference tests](../../cli/tests/l1/compose_file_interpolate.rs),
and the [reference snapshot](../../lib/src/markdown/reference/snapshot.rs).
The R2-F2 assessment is deliberately partial: the repair's original tests
pass, but its added claim of TOC agreement does not.

## Unblocked Findings

### R3-F1 (high): TOC fragment validation rejects anchors generated from raw headings

**Authority — defect:** In the Darkmatter library, TOC generation and graph
reference generation read a target's raw headings, but fragment validation
checks those generated anchors against transformed headings. An ordinary
`md validate refs --fragments` therefore rejects a link that the library itself
generated. This is reproducible with frontmatter replacement, a templated
heading, or a heading inside a false conditional block. The existing raw-source
contract is explicit in the library's
[TOC module](../../lib/src/markdown/compose/toc_linking/mod.rs:1); this finding
does not impose a new interpretation of Markdown or request different TOC output.

**Defect class:** A generated reference and the validation decision consuming
it use different representations of the same target document. This is a
remaining sibling of R2-F2's reference-preparation mismatch and R2-F3's TOC
preparation instance.

Use these three files:

```md
<!-- root.md -->
::file ./parent.md interpolate=false
```

```md
<!-- parent.md -->
::toc-linking ./toc.md
```

```md
---
replace:
  WORD: World
---
# Toc

## Hello WORD
```

The last block is the complete `toc.md`. Normal
`md compose root.md --output json` exits 0 and emits:

```md
- [Hello WORD](./toc.md#hello-word)
```

But `md validate refs root.md --fragments --output json` exits 1 with
`MissingFragmentTarget`, reporting that `#hello-word` does not exist. The
graph prepared the target into `## Hello World` and retained `hello-world`
instead. Removing `interpolate=false` does not fix it: replacement remains
active in either mode. Changing the target to `name: World` plus
`## Hello {{ name }}` reproduces it with normal prose, while false correctly
suppresses that particular transformation. A false block around `## Hidden`
reproduces it even under false: the generated `#hidden` is checked against a
prepared document from which the heading was removed.

The sweep followed the generated anchor through both generators, target
preparation, stored heading identity and both public validation entry points.
The inclusion loaders were checked separately because the previous repair
changed their request derivation.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | Composition TOC generator and graph's `generate_toc_link_references` | Target's own replacement, heading template or removed block | Clean: output and synthesized reference both name the raw heading (`hello-word`, `hello-name`, `hidden`) | Preserve the existing raw-heading generation |
| R3-F1.2 | Graph TOC target preparation through `accepted_child_options` | Own replacement with normal/false prose; template with normal prose; false conditional block under false | Preparation changes/removes the heading used by .1 | The generated TOC reference must be validated using its raw-heading contract |
| R3-F1.3 | `PreparedHeadingSnapshot::record` and validation's heading-cache seed | Replacement target reached as a TOC child | Stores/seeds only the prepared `hello-world` slug; no raw heading source distinguishes this reference | Retain or select the raw heading set for synthesized TOC references without changing ordinary link validation |
| R3-F1.4 | Body-file inclusion and public `validate_references` | Three-file fixture above; both prose modes | `MissingFragmentTarget #hello-word` | Accept the generated raw anchor |
| R3-F1.5 | Prologue inclusion of the TOC holder | Parent frontmatter `prologue: ./toc_holder.md`, holder contains the same TOC directive, false ancestor | Same false rejection | Same acceptance |
| R3-F1.6 | Epilogue inclusion of the TOC holder | Same fixture with `epilogue:` | Same false rejection | Same acceptance |
| R3-F1.7 | Public `validate_references_with_graph` | .5/.6, freshly built graph with identical request options | Same false rejection as one-step validation | Same acceptance, retaining graph freshness checks |
| R3-F1.8 | CLI validation gate | Three-file fixture, normal compose followed by `validate refs --fragments` | Compose exits 0; fragment validation exits 1 with the raw anchor in its located diagnostic | Validation accepts the TOC-generated anchor |
| R3-F1.9 | Plain-heading control and suppression control | Plain heading; separately heading template under false | Clean: no missing-fragment issue | Preserve both controls |
| R3-F1.10 | Ordinary explicit cross-document link, outside the graph | `[Hello](./toc.md#hello-world)` to the replacement target | Clean through both validation APIs: uses prepared standalone target headings | Keep ordinary links' existing prepared-heading behavior |
| R3-F1.11 | Validation without fragment checking | Replacement fixture, fragment checking disabled | Clean: no issue, matching ordinary compose's default gate | Preserve the opt-in check |
| R3-F1.12 | TOC target with an inclusion's one-off replacement | Existing `a_toc_linking_target_takes_no_one_off_map_from_the_inclusion` | Clean: raw `heading-word` anchor is accepted because the inclusion map is cleared | Preserve the R2-F3.15 repair |

The graph does not fetch remote TOC targets, and the TOC directive has no
`interpolate` or replacement option of its own. Standalone fragment-target
preparation is the clean control in .10, not another raw-heading consumer.
Both validation entry points reach the same validation engine after the
prebuilt path's compatibility check; .7 confirms that this check does not
remove the mismatch.

**Recommended repair:** Keep TOC generation on raw source, and validate
synthesized TOC references against the same raw target heading set. The
existing `toc_synthesized` attribute already distinguishes those references.
Reuse the run's loaded target rather than rereading it. Keep prepared headings
for ordinary links and actual inclusion children, and preserve accumulated
slugs for repeated inclusions. A small distinction at the generated-reference
validation boundary is sufficient; no new public syntax, escaping layer or
change to TOC output is needed. Cover the shared decision in-process and keep
one spawned CLI regression. Do not disable fragment checking to hide it.

## Blocked Findings

None. The raw-heading contract supplies a recommendation within existing
behavior. No author ruling is needed before repair.

## Recurrence

R3-F1 is an **unchanged sibling missed by the earlier sweep**, not a regression
caused by the latest repair. Review 2's R2-F2 covered reference preparation;
R2-F3 covered TOC preparation retaining an inclusion's replacement option.
The repair correctly cleared that option and added a passing test, but its
added ledger row R2-F2.13 classified the target's own transformations as clean
without verifying generated anchors through fragment validation. The sweep
should also have checked the target's own replacement, heading interpolation
and conditional heading removal, then followed their slugs through the
snapshot and both validation entry points. Those sites are inventoried above.

This class has a concrete boundary: synthesized TOC references use raw target
headings; ordinary links keep their existing prepared-heading contract.

## Verification and requirement coverage

All requirements in this specification need **L1** verification: string
composition, passive analysis, public API results and ordinary CLI output.
None depends on terminal rendering, browser behavior or keyboard encoding.

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Boolean/default/null/quoted/repeated values and located invalid-value errors | L1 parser, library/editor agreement and spawned CLI diagnostics | Passes |
| False preserves known/unknown/malformed/bad-call examples and triple braces; overrides code-block interpolation | L1 public composition under deny-all, including fenced/indented code | Passes |
| Frontmatter and supplied overrides still resolve; conditions, collections and directive inputs execute | L1 public output, command approval equality, inherited-value tests and pre-approved execution | Original cases pass |
| Descendant inheritance, parent/sibling independence, both orders and cache modes | L1 public composition, cache counters, identities and cycle controls | Passes |
| Prologue/epilogue, remote Markdown and nested composition inherit false | L1 temporary files, allowed loopback HTTP and public discovery/execution | Passes |
| Inactive read/context/remote/command samples cause no effects or approvals | L1 read sets, mock request counts, capability/probe and approval results | Passes |
| Replacement, exclusion, links and wrappers still work without activating examples | L1 public composition, graph/validation and normal CLI | Replacement and original link cases pass; TOC fragment comparison fails as R3-F1 |
| Editor completion/hover/validation and independent child analysis | L1 public DMLS providers and semantic-token tests | Passes |
| Frozen caller-skill publication, colliding name, deny-all guard and passive shipped-source check | L1 spawned ordinary compose command and corpus parser | Passes |

Reviewer runs:

- `just test interpolate`: **247 passed**, none at or above five seconds.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. Library
  tests are compiled by the existing module; CLI/DMLS files are declared by
  their existing `l1` targets, with recipe-enabled features.
- `just test` in `darkmatter/`, after removing all reviewer probes: **16,978
  passed, 10 skipped**, exit 0; **none at or above five seconds**. Nextest
  took 74.814 seconds; the recipe took 1 minute 34 seconds including build.
- Temporary L1 probes used the public composition/reference APIs and the
  CLI's `CliProcessFixture`. They reproduced R3-F1 and its clean controls;
  assertions expecting correct behavior intentionally failed. All probes
  were removed before the final area run. The CLI failure took 0.259 s;
  in-process probes stayed below 0.1 s.

The input robustness matrix does not apply: these changes concern Markdown
directive/expression grammar, not a file-format or configuration loader.
The recorded implementation lint result was inspected; this review makes no
new lint or cross-OS execution claim. Review 1's repaired tooling-speed
finding was already verified closed in review 2, and that unchanged tooling
was not rerun here.

## Implementation-record correction

Corrected `log.md` in place, dated **2026-11-16**, at R2-F2.13 and the two
summaries repeating its claim. It incorrectly said raw and prepared TOC
headings agreed for every input governed by this feature and treated the
remaining difference as needing a design decision. The replacement and
conditional-heading probes under false, both public validation APIs and the
normal CLI show otherwise. The correction includes the actual behavior and
verification method. It also corrects that ledger's current disposition
totals to 28 fixed, 11 clean and one unresolved child row.

These record corrections are not separate findings or human-review items.
The specification body and review 2's `implemented`/`implemented_by` claims
are preserved. Its `next` link and the spec's review count are updated.
