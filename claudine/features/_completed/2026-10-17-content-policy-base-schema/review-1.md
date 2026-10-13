---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Disabling triggers skips validation of shell-resolved policy values"
    - "high: Rule completion drops six shipped suggestions"
    - "high: Diagnostic projections discard policy instance paths"
observations:
    - Shell-validation documentation omits the trigger-dependent behavior
    - An unchanged reference-parity test exceeded the Level 1 time budget
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-12T19:00:28-07:00"
spec: 2026-10-17-content-policy-base-schema/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/features/2026-10-17-content-policy-base-schema/log.md
description: "A **feature** review of `2026-10-17-content-policy-base-schema/spec.md`"
feature: 2026-10-17-content-policy-base-schema/review-1.md
next: 2026-10-17-content-policy-base-schema/review-2.md
---

# Review of Content Policy Base Schema

The feature is **not production ready**. Its embedded baseline, ordinary
validation, schema precedence, and passive policy checks work, but three
acceptance gaps remain. None requires a human decision to implement the
specified behavior.

## Findings

### high: Disabling triggers skips validation of shell-resolved policy values

**Authority — spec:** Acceptance criterion 6 requires that “authored pending
expressions are checked after resolution.” The CLI compatibility section
also says that `--no-trigger-schemas` “keeps its separate existing meaning.”
Turning off trigger discovery must therefore leave the baseline's checks
active. This feature makes the existing conditional validation path reachable
for schema-free policy declarations through its new baseline property.

**Defect class:** Final validation of a resolved value depends on whether an
unrelated schema source is enabled.

In the Darkmatter package, the
[compose pipeline](../../../darkmatter/lib/src/markdown/compose/pipeline/mod.rs#L402)
runs full validation after shell expansion only when `options.trigger_schemas`
is true. Otherwise it checks projection stability and separately validates
values carrying typed shell suffixes; ordinary shell output escapes the
baseline check.

Reproduction uses an isolated directory, fixture home, and cleared schema
environment. Write this document and a fixture-owned
`.darkmatter-shell-whitelist` containing `exact echo words`:

```yaml
---
content_policy: "$(echo words)"
---
Body
```

`md compose --frontmatter doc.md` exits 1 with an array-type error.
`md compose --frontmatter --no-trigger-schemas doc.md` exits 0 and emits
`content_policy: words`. That is a resolved, invalid declaration. No trigger
file exists in either run; the baseline is the only effective schema.

The sibling sweep used the same document with one value change per row:

| Site / resolution route | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Compose, triggers enabled | Direct `$(echo words)` output | Exit 1, array-type error | Reject; clean |
| Compose, triggers disabled | Direct `$(echo words)` output | Exit 0, emits invalid string | Reject |
| Compose, triggers enabled / disabled | `$(cat policy.txt)`, file contains `["Duration(3mo)"]` | Enabled rejects; disabled emits the JSON text as a string and exits 0 | Reject in both modes; shell stdout is not an array |
| Initial validation, both modes | Literal `content_policy: words` | Exit 1 | Reject; clean |
| Template validation, both modes | `content_policy: "{{ value }}"`, `value: words` | Exit 1 | Reject; clean |
| Deferred template pass, both modes | `value: "$(echo words)"`, `content_policy: "{{ value }}"` | Exit 1 | Reject; clean |
| Typed shell-value backstop, triggers disabled | `$(echo words)::ok`, `::exit-code`, and `::result` | Each exits 1 with schema failure | Reject boolean, number, and object respectively; clean |

For the affected `content_policy` field, the remaining robustness cells are
covered by the passing declaration corpus: absent is valid and stays absent;
whole-list null is accepted under the specified optional-property convention;
literal string/number/object values reject; mixed valid/invalid items and
entirely invalid items reject without filtering; empty lists/maps reject;
duplicate keys and invalid frontmatter fail loading. The failure is the
wrong-type whole-field cell **after direct shell resolution with triggers
disabled**, rather than a permissive YAML parser.

The new Darkmatter
[compose-contract tests](../../../darkmatter/lib/tests/l1/content_policy_compose_contract.rs#L104)
test templates, but omit this shell/trigger combination. Run the final
baseline/document validation independently of trigger discovery and add a
hermetic Level 1 subprocess case with the fixture-owned approval above. This
needs no terminal window, new parser, or broader policy evaluation.

### high: Rule completion drops six shipped suggestions

**Authority — spec:** Acceptance criterion 2 requires that DMLS “completes
supported rule forms and actions” and says to “Preserve branch descriptions
and shipped suggestions through embedding.” The shipped artifact supplies
seven rule suggestions; ordinary completion exposes only one.

**Defect class:** Suggestion filtering validates union candidates against the
single alternative that stores them rather than the effective rule type.

Content Policy's
[editor schema](../../../content-policy/schemas/content-policy.yaml#L19)
stores all seven candidates on the `Evergreen` alternative. In the Darkmatter
package,
[suggestions_for_def](../../../darkmatter/lib/src/markdown/schemas/simplified/query.rs#L157),
the public API used to build value completion, isolates each suggestion-bearing
alternative before applying its lint. Six candidates are discarded by the
`^Evergreen$` pattern. The embedded typed schema preserves the strings, but
that alone does not satisfy editor completion.

I copied the real schema into a temporary disk fixture and compared it with
the embedded baseline through the built DMLS stdio interface. Both documents
used the new completion test's block-list layout. The public Darkmatter query
APIs were also exercised against the embedded baseline:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Darkmatter `suggestions_for_path` | Embedded `content_policy` declaration | `Evergreen` only | All seven shipped rule candidates |
| Darkmatter `suggestions_for_def` | Embedded compact-item definition | `Evergreen` only | All seven candidates |
| Darkmatter `suggestions_for_def` | Embedded long-form `rule` definition | `Evergreen` only | All seven candidates |
| Built DMLS, embedded baseline | Blank compact item after a valid item | `Evergreen`, `rule`, `action` | Seven rule candidates plus the long-form keys |
| Built DMLS, disk-imported policy schema | Same blank compact item | `Evergreen`, `rule`, `action` | Same complete set |
| Built DMLS, embedded / disk source | Blank long-form `rule:` value | `Evergreen` only | Seven rule candidates |
| Built DMLS, embedded baseline | Blank `action:` value | `refresh`, `archive`, `remove` | Same three actions; clean |
| DMLS Level 1 protocol test | Missing long-form key after its sibling | Correct `rule` or `action` key | Correct remaining key; clean |
| DMLS Level 1 protocol test | Hover on compact and long-form rules | Shipped branch descriptions retained | Retain descriptions; clean |

The seven expected candidates are `Evergreen`, `TimeSensitive`,
`ValidFor(3mo)`, `ValidFor(3mo, @last_updated)`,
`ValidFor(3mo, 2026-09-28)`, `ValidUntil(2027-01-01)`, and
`FileChanged(src/config.rs, @config_fingerprint)`.

The new DMLS
[completion test](../../../darkmatter/dmls/tests/l1/content_policy_baseline.rs#L70)
asserts only `Evergreen` at both affected cursor positions. The standalone
binary test likewise does not demand the remaining candidates. Extend those
assertions to the shipped candidate set. Correct the query/schema interaction
without introducing a second completion grammar; simply moving a separate
`suggest(...)` list onto every alternative conflicts with the current
one-list-per-property rule. This is a usability gap rather than incorrect
policy validation, but the explicit acceptance criterion makes it blocking.

### high: Diagnostic projections discard policy instance paths

**Authority — spec and implementation plan:** Acceptance criterion 2 asks for
diagnostics identifying the offending item or field with an “instance path
such as `/content_policy/1/rule`” and its source position. The plan's
“Successful completion” paragraph explicitly says: “Invalid entries report
their instance path and source position.” The internal reports meet this
contract; two user-facing projections do not.

**Defect class:** Projection of a structured validation problem loses its
instance path while retaining its position.

In the Darkmatter package, the
[compose error renderer](../../../darkmatter/lib/src/markdown/errors/blocks.rs#L708)
reduces `/content_policy/1/rule` to its first segment. In the DMLS package,
the [diagnostic projection](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs#L314)
emits the validator message and range, but neither the message nor structured
diagnostic data carries the instance path. Both implementations predate this
feature; the new baseline makes them the default diagnostic routes for
schema-free policy lists, which this acceptance criterion explicitly covers.

I ran all six required rejection shapes through the built `md` commands and
the built DMLS stdio interface. Every route rejects them and locates the right
node. The paths expected from these fixtures are:

| Shape | Expected instance path | Source position observed |
| --- | --- | --- |
| Valid first item, `Duration(3mo)` second item | `/content_policy/1` | Line 4, column 5 |
| Unknown long-form action | `/content_policy/0/action` | Line 4, column 13 |
| Long-form entry without `rule` | `/content_policy/0` | Containing mapping, line 3, column 5 |
| Long-form `rule: null` | `/content_policy/0/rule` | Line 3, column 11 |
| Empty list | `/content_policy` | Line 2, column 17 |
| Wrong list shape, `content_policy: words` | `/content_policy` | Line 2, column 17 |

The sibling projection sweep checked that same set at every boundary:

| Site | Shapes tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Library baseline validation | All six, through passing corpus tests | Full paths, kinds, baseline origins, and node positions retained | Retain; clean |
| Library compose error | Invalid second item, plus passing compose tests | Structured `ValidationProblem.path` retained | Retain; clean |
| `md schema validate --format json` | All six, direct subprocess reproduction | Full paths and positions emitted | Retain; clean |
| `md schema validate`, pretty output | All six, direct subprocess reproduction | Full paths without leading `/`, plus positions | Retain; clean |
| `md compose`, error output | All six, direct subprocess reproduction | Only `content_policy` displayed, plus positions | Display the item/field path for nested problems |
| DMLS published diagnostics | All six, direct stdio reproduction | Correct node ranges and problem codes; path absent from message and data | Preserve the instance path alongside the existing range |

This does not require changing how positions are found. Keep the existing
typed reports, ranges, and exit codes; retain the full path in the terminal
target and the editor diagnostic projection. The new
[compose subprocess assertions](../../../darkmatter/cli/tests/l1/standalone_baseline.rs#L139)
currently require only `content_policy` and line/column, and the new DMLS
assertions check ranges without checking retained paths. Extend these existing
Level 1 checks rather than adding another synthetic payload matrix.

## Verification and coverage

Review scope was the feature's implementation commits from the Content Policy
schema change through `a8a9316d6`, excluding the unrelated Claudine schema-plan
commit. Production source was left unchanged. Tests are declared in the
consolidated `l1` targets, included by their `main.rs` files, and selected by
the package-area recipes. `just check-tier-coverage darkmatter` found no
stranded tier tests.

| User-facing requirement | Strongest relevant verification | Result |
| --- | --- | --- |
| Valid policy and policy-free documents in library, validate, compose, and DMLS | Level 1 public APIs, `md` subprocesses, in-memory LSP, built DMLS stdio | Pass |
| Invalid rules/actions/list shapes and required long-form fields | Level 1 shared block/flow corpus and standalone binaries | Reject correctly; diagnostic projection finding above |
| Embedded imports, errors, shared caches, and installed-binary operation | Level 1 preparation tests, same-process identity check, isolated standalone subprocesses | Pass |
| Disk/embedded equivalence and definition isolation | Level 1 equality, import mutation, and baseline/trigger/document/editor layering tests | Pass |
| CLI precedence, truthy spellings, conflicts, opt-outs, quiet/JSON/exit codes | Level 1 fixture-owned CLI subprocesses | Pass |
| Editor rule/action/key completion and rule hover | Level 1 LSP and built-server protocol tests | Rule suggestions incomplete; other checks pass |
| Passive validation and optional-property behavior | Level 1 bytes/frontmatter checks and effect counters | Pass |
| Pending-value validation after resolution | Level 1 templates and direct shell reproductions | Trigger-disabled direct shell route fails |

Level 1 is appropriate throughout: these requirements concern schema data and
protocol results, and the spec expressly requires no real-terminal or browser
tier. No editor or terminal window was activated.

The input robustness tables cover YAML declaration fields `content_policy`,
`rule`, and `action`, plus the baseline `$schema`, policy `kind`/`types`, named
definitions, and long-form declarations. Disk and embedded preparation share
the schema mutations and positive controls; public validation exercises the
declaration cases. Optional whole-list null and the documented calendar,
overflow, path, and cross-entry limitations have separate parser/schema
expectations. No new silent parser-default defect was found.

Executed on macOS:

- `just test` in `darkmatter/`: 9,737 passed, 11 skipped; one unchanged test
  exceeded five seconds, recorded below.
- `just test` in `content-policy/`: 176 passed, 4 skipped.
- `just lint` in both areas: passed, including Content Policy's dependency
  boundary check and Darkmatter's Zed extension compile check.
- Direct isolated `md` and DMLS reproductions for the findings, and public
  Darkmatter suggestion-query calls.

Cross-OS proof remains the existing CI responsibility and does not affect this
review's readiness decision.

## Observations

### Shell-validation documentation omits the trigger-dependent behavior

The new paragraph in Darkmatter's
[schema-validation page](../../../darkmatter/docs/inline/schema-validation.md#L342)
says `md compose` does not check shell output again and that an invalid
shell-produced policy passes. The ordinary command actually rejects it;
the acceptance failure requires `--no-trigger-schemas`. The implementation
log makes the same overbroad claim. Code was treated as authoritative: the
review's reproduction and finding distinguish both routes. Correct the
description alongside the pending-value repair so readers do not infer the
wrong default behavior.

### An unchanged reference-parity test exceeded the Level 1 time budget

Darkmatter's unchanged
[reference-parity suite](../../../darkmatter/lib/tests/l1/entry_point_parity.rs)
included `darkmatter_entry_points_agree_on_every_reference_part_01`, which
passed in 5.505 seconds during the full run. That exceeds the repository's
five-second Level 1 budget. This test was not added or changed by the feature;
the observation does not block this review or request another performance
workload in this cycle.
