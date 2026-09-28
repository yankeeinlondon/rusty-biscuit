# Content Policy: Evaluation, Baselines, and Renewal

Date: 2026-09-28  
Status: Draft for design review; implementation has not started.

## Purpose

Let Markdown documents declare when they need to be refreshed, archived, or
removed. Provide a reusable Rust library and a CLI that share the same policy
semantics. Evidence should travel with the document through inline values or
frontmatter references, without requiring a sidecar.

This draft separates the agreed model from recommendations awaiting review.
Sections labeled **proposed** are design choices, not implementation commitments.
The current reader-facing description is in
[Policy Evaluation and Renewal](../../docs/topics/policy-lifecycle.md).

## Agreed Model

- Policies are declared in Markdown frontmatter under a configurable key,
  initially `content-policy`.
- Multiple rules combine as OR of triggers. Each rule has an action, defaulting
  to `refresh`.
- Action precedence is fixed: `remove` > `archive` > `refresh`. Declaration
  order has no effect on the winner.
- Baselines can be literal values inside a rule or references to frontmatter
  properties. `@` identifies a property reference.
- Evaluation does not initialize or renew baselines. Recording an update is an
  explicit operation after review or regeneration of the content.
- Renewability belongs to a rule type. `ValidFor` renews its starting date while
  retaining its duration. `ValidUntil` retains its deadline across content
  updates and stays triggered after expiration unless the policy is edited.
- Action and renewability are independent. A renewable rule can request archive
  or removal.
- The library owns policy semantics. Replaceable providers gather evidence;
  bundled integrations are available to library consumers and the CLI.
- Reports retain all triggered rules. Unknown results cannot nominate actions;
  uncertainty that could change the winner is reported explicitly.
- Evaluation reports actions. Execution of refresh, archive, or removal belongs
  to the consuming application.

## Proposed Delivery Scope

The first increment delivers a complete lifecycle for `Evergreen`,
`TimeSensitive`, `ValidFor`, and `ValidUntil`: declaration parsing, validation,
references, evaluation, renewal planning, and CLI reporting. The second adds
`FileChanged` to prove provider injection and persisted fingerprints.

Package versions, symbols, URL/schema changes, and file/program presence rules
follow after their comparison contracts are reviewed. They should not delay the
first increment. Reaper is not a prerequisite for time or file policies.

Automatic content generation, archive destinations, file deletion, a scheduler,
a policy expression language with AND/NOT, sidecar storage, and configurable
action precedence are outside this initial scope.

## Terms

| Term | Meaning |
| --- | --- |
| Rule | A condition that can trigger, such as an elapsed validity interval |
| Policy entry | A rule plus its configured action |
| Baseline | Evidence accepted when content was last updated or reviewed |
| Observation | Current evidence used for a comparison |
| Renewal | Explicitly replacing a renewable rule's baseline after a content update |
| Effective action | Highest-priority action among confirmed triggered entries |

## Declaration Format

The compact and action-bearing forms express the same model:

```yaml
last_updated: 2026-09-28
content-policy:
  - ValidFor(3mo, @last_updated)
  - rule: ValidUntil(2027-01-01)
    action: archive
```

Inline baselines are equally valid:

```yaml
content-policy:
  - ValidFor(3mo, 2026-09-28)
```

### Defaults and references — proposed

- An absent policy key uses the caller's default policy, initially
  `ValidFor(6mo)`. An explicit empty list also uses this default. Use `Evergreen`
  to explicitly request no expiry. A malformed or null declaration is an error.
- `ValidFor(3mo)` resolves the caller's default date property, initially
  `last_updated`. The report identifies the effective rule as defaulted or
  explicitly declared and records the resolved baseline source.
- Initially, `@name` selects one top-level frontmatter property. No nested paths,
  recursive references, environment lookup, or cross-document lookup is implied.
  A referenced string is consumed as a typed value, not reparsed as an expression.
- Missing properties are unavailable evidence and yield `unknown`. A present
  value of the wrong type, an invalid date, or a malformed reference is a
  validation error. Neither causes fallback to a different baseline.
- References occupy typed baseline/deadline positions. They are distinct from
  Biscuit File's file-reference syntax. A future file selector must explicitly
  distinguish a file path from a frontmatter reference.

A structured form is proposed for richer rules. For example, a file baseline
needs the digest algorithm as well as the digest:

```yaml
content-policy:
  - rule:
      type: FileChanged
      file: ./src/config.rs
      baseline:
        algorithm: blake3
        digest: "<recorded digest>"
    action: refresh
```

The digest above is a placeholder, not valid stored evidence. Structured and
compact forms must normalize to the same internal representation. The first
increment need only accept the illustrated time-rule forms; the complete
structured grammar will be settled before provider-backed rules are built.

## Time Semantics — Proposed

The evaluator takes an explicit evaluation time. The CLI captures it once for
the whole document; tests supply it directly.

- Initially accept ISO dates (`YYYY-MM-DD`) and positive integer durations with
  one unit: `d`, `wk`, `mo`, or `yr`. Compound durations and timestamps are deferred.
- Interpret dates at midnight UTC. `ValidUntil(2027-01-01)` triggers at the start
  of January 1, rather than the end of that day.
- `ValidFor` triggers when evaluation time is greater than or equal to its
  computed deadline. Days/weeks are UTC day increments; months/years use calendar
  arithmetic, clamping to the last valid day of the destination month.
- For example, January 31 plus one month becomes February 28 in a non-leap year;
  February 29 plus one year becomes February 28 in the following year.
- A future starting date yields `unknown` with an inconsistent-baseline reason;
  it does not establish freshness. A future `ValidUntil` deadline is normal.

These choices intentionally make expiration independent of the host's timezone,
while leaving date-only boundary behavior visible for review.

## Evaluation Contract

Separate declaration validity from evidence availability. Invalid syntax,
unknown rule/action names, invalid parameters, and wrong-typed resolved values
produce validation diagnostics with entry/property locations. The proposed
behavior is to withhold a usable document verdict when declarations are invalid.

Each valid entry evaluates to `triggered`, `not_triggered`, or `unknown`.
Unknown results carry reasons such as missing baseline, missing provider,
unsupported capability, timeout, or failed observation. A successful observation
of absence is distinct from an observation failure; the rule decides what that
absence means.

All entries are evaluated for a complete report. Do not stop after finding a
trigger: later entries may request a higher-priority action or provide useful
explanations. Identical observation requests can share evidence within a run.

### Aggregation

| Confirmed results | Document status | Effective action |
| --- | --- | --- |
| No triggers, no unknowns | `fresh` | None |
| No triggers, at least one unknown | `unknown` | None |
| Highest triggered action is refresh | `stale` | `refresh` |
| Highest triggered action is archive or remove | `expired` | `archive` or `remove` |

Document status describes confirmed evidence. A separate action-resolution
field reports whether unknown entries could alter the effective action:

| Known triggered action | Unknown entry's action | Resolution |
| --- | --- | --- |
| None | Any | Incomplete |
| Refresh | Refresh | Complete; reasons remain incomplete |
| Refresh | Archive or remove | Incomplete |
| Archive | Refresh or archive | Complete; reasons remain incomplete |
| Archive | Remove | Incomplete |
| Remove | Any | Complete; reasons remain incomplete |

A fully evaluated fresh document has complete action resolution and no action.
Evaluation completeness and action-resolution completeness are separate fields.
Consumers must not interpret a complete winning action as proof that every rule
was evaluated successfully.

### Report shape — proposed

Return document identity, evaluation time, effective/defaulted policy, overall
status, effective action, evaluation completeness, action-resolution
completeness, and a result for every entry. Results include entry location,
action, renewal classification, baseline source/value, current observation when
available, and a reason. Sensitive provider configuration is not report data.

Illustrative JSON for two confirmed triggers:

```json
{
  "status": "expired",
  "action": "remove",
  "evaluation_complete": true,
  "action_resolution_complete": true,
  "results": [
    {"rule": "ValidFor(3mo, @last_updated)", "result": "triggered", "action": "refresh", "reason": "Validity interval elapsed"},
    {"rule": "ValidUntil(2027-01-01)", "result": "triggered", "action": "remove", "reason": "Fixed deadline reached"}
  ]
}
```

This is an excerpt, not a finalized serialization schema.

## Recording Updates and Renewal

Evaluation never writes policy state. Repeated checks against the same evidence
must not make a stale document fresh.

Renewal is an explicit assertion that content has been updated or reviewed using
the replacement evidence. A library operation should produce proposed document
edits for the caller to persist. File persistence can be a shared library helper;
the CLI must not be the only consumer able to renew policies.

### Renewal rules — proposed

- Replace an inline `ValidFor` baseline in place. For `@last_updated`, propose
  an edit to that property while preserving the reference.
- For shorthand/defaulted rules, create or update the configured date property.
  Evaluation alone never creates it.
- Preserve durations, selectors, actions, and nonrenewable deadlines. Renewal
  does not guarantee freshness: `TimeSensitive` still triggers and a passed
  `ValidUntil` still expires the document.
- Let callers supply evidence captured during the content update. A new fetch
  after generation may observe a different version than the content used.
- Consolidate identical writes to a shared property. Reject conflicting writes.
  If a proposed write also changes a nonrenewable rule's referenced deadline,
  report a conflict instead of silently extending that deadline.
- Missing baseline targets can be initialized explicitly. Present malformed
  values need correction; renewal is not a general repair operation.
- Plan the entire requested renewal before writing. If required evidence is
  unavailable, return diagnostics without partially advancing baselines.
- Preserve the Markdown body, unrelated frontmatter, and declaration form.
  Detect intervening document edits before applying a prepared renewal.

A fingerprint baseline must identify its comparison algorithm and scope.
Changing the fingerprint scheme requires explicit recapture; incompatible
fingerprints cannot be treated as proof of content changes or freshness.

## Library Architecture — Proposed

Use one library crate with optional provider integration modules and a separate
CLI crate. Avoid requiring callers to assemble every provider for a date check.

| Layer | Owns |
| --- | --- |
| Declaration/model | Parsing, validation, references, typed policies and baselines |
| Evaluation | Time/version comparisons, result aggregation, precedence |
| Renewal | New baseline interpretation and proposed document edits |
| Provider contracts | Typed requests and observations for external capabilities |
| Bundled integrations | Adapters to existing monorepo packages |
| CLI | Configuration, input/output, invoking evaluation or explicit renewal |

Providers return observations, not final freshness judgments. Introduce only
traits required by implemented policies; do not stub every future capability.
Keep external crate types out of policy declarations where they would couple
storage to a particular implementation. Exact Rust signatures, async boundaries,
feature names, and Markdown/frontmatter parsing reuse await implementation design.

| Integration | Useful capability | Remaining policy responsibility |
| --- | --- | --- |
| Sniff | Registry releases and program discovery | Version comparison and release selection semantics |
| Biscuit File | File references and HTTP fetching | Content comparison, baseline handling, error interpretation |
| Tree Hugger | Symbol selection and source extraction | Define comparison scope and fingerprint selected content |
| Reaper, when available | Extract meaningful page content | Define what content changes invalidate the document |

Tree Hugger's symbol identity alone is not a content-change fingerprint.
A file's modification time is not a portable substitute for its content baseline.
Request credentials and runtime network policy belong to provider configuration.

## Later Policy Decisions

These candidates retain the original idea without claiming settled semantics:

| Candidate | Decision required before implementation |
| --- | --- |
| `SemVerMajorChange` | Package identity, registry, baseline version, release channel; propose a strictly newer major release |
| `SemVerMinorChange` | Propose a newer minor or major release; define prerelease and pre-1.0 behavior |
| `FileChanged` | Propose byte-content fingerprints; define missing/deleted file behavior |
| `SymbolChanged` | File plus qualified selector, ambiguity/deletion handling, signature/body/docs scope |
| `UrlChanged` | Request identity, selected response content, normalization, conditional-response handling |
| `SchemaChanged` | Structural changes versus compatibility-breaking changes; schema dialect and reference scope |
| `WhenFileCreated` / `WhenFileRemoved` | Current presence/absence versus transition since a baseline |
| `ProgramInstalled` / `ProgramRemoved` | Presence versus transition; host/environment identity and discovery scope |

In particular, a missing program on another host should not accidentally prove
that the program was uninstalled on the host used to create the document.

## CLI Contract — Proposed

`policy document.md` prints the evaluation report as JSON. A successfully
produced report exits zero even if it reports stale, expired, or unknown;
validation/input errors exit nonzero. Exact error codes remain a review item.

`policy document.md --is-stale` answers whether any rule has confirmed a need
for action, including expiration. It prints `true` for stale/expired, `false`
for fresh, and no boolean for unknown or invalid input. A known trigger still
prints `true` when action resolution is incomplete. The full report is required
to choose an action. Decide during review whether the flag should communicate
true/false through exit codes as well as stdout.

Renewal command naming and whether applying edits requires a write flag remain
open. The contract requires explicit invocation, previewable edits, and no
automatic refresh/archive/remove execution.

## Acceptance Criteria for Implementation

1. Compact and action-bearing declarations normalize consistently; defaults and
   baseline sources are visible in reports.
2. Inline and referenced dates evaluate equivalently. Missing dates produce
   unknown results, and wrong-typed values produce validation diagnostics.
3. A fixed evaluation time makes time-policy results deterministic, including
   exact deadlines, month ends, leap years, and future baselines.
4. Reordering entries does not change the effective action. All combinations of
   confirmed and unknown actions follow the aggregation tables.
5. Evaluation performs no document writes and never implicitly captures a
   baseline, including on the first evaluation.
6. Renewal changes only the intended baseline values, preserves policy settings,
   consolidates shared writes, and rejects conflicts or incomplete capture.
7. The lifecycle example below passes through both library and CLI behavior.
8. The file increment demonstrates the same lifecycle with a fake provider and
   a bundled file adapter, including read failures and incompatible fingerprints.
9. New crates follow repository package-area conventions and support macOS,
   Linux, native Windows, and WSL2. Implementation follows existing test recipes
   and maintains topic/dependency documentation and applicable skills.

### Lifecycle example

A document is updated on September 28, 2026. It has `ValidFor(3mo, @last_updated)`
with `refresh` and `ValidUntil(2027-01-01)` with `archive`. Under the proposed
date semantics it is fresh before December 28, stale on December 28, and renewed
when an explicit content update sets the baseline to December 29. On January 1,
2027 it is expired with action `archive`. Further content updates do not move
that deadline or clear expiration. Changing the retirement action to `remove`
changes the winning action, while every triggered entry remains in the report.

## Review Questions

1. **Defaults and references:** approve `last_updated`, top-level `@name`, and
   treating an empty policy list like an absent policy? Nested references can
   wait unless existing document metadata needs them.
2. **Dates:** approve midnight UTC expiration, calendar month/year clamping,
   and the initial date-only duration grammar? In particular, should a named
   `ValidUntil` date instead remain valid through that entire day?
3. **Renewal:** approve writing referenced properties with conflict detection,
   and rejecting the whole requested renewal when some evidence is unavailable?
4. **Declaration shape:** approve `{ rule, action }` with a string or structured
   rule, including the baseline object for fingerprint policies?
5. **CLI:** retain `--is-stale` for both stale and expired content, and choose its
   exit-code behavior and the explicit renewal command syntax.
6. **Scope:** approve time policies first and `FileChanged` second before
   expanding into package, symbol, and remote comparisons?

Review these choices before implementation planning and public Rust API design.
