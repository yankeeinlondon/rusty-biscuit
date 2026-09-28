# Policy Evaluation and Renewal

**Status: planned.** Content Policy is in design review. This page describes the
agreed lifecycle; details labeled proposed have not yet been approved or built.

Use content policies to tell an application when a Markdown document needs a
refresh or should leave active use. A document can carry both kinds of rule:

```yaml
---
last_updated: 2026-09-28
content_policy:
  - rule: ValidFor(3mo, @last_updated)
    action: refresh
  - rule: ValidUntil(2027-01-01)
    action: archive
---
```

`ValidFor` measures an interval from the recorded update date. `ValidUntil` has a
fixed deadline. Updating the first date does not move the second.

## Keep the Baseline with the Document

A baseline records what the content was based on. For a time policy this can be
an inline date, `ValidFor(3mo, 2026-09-28)`, or a reference,
`ValidFor(3mo, @last_updated)`. The `@` prefix means a frontmatter property
reference in that argument position. No sidecar is required.

The proposed shorthand `ValidFor(3mo)` uses a configurable default date property,
initially `last_updated`. The proposed default policy for documents without a
policy is `ValidFor(6mo)`. Missing baseline evidence produces `unknown`; checking
a document does not invent an update date.

Policies live under the `content_policy` key by default (snake_case, like the
repository's other frontmatter keys); a caller can choose a different key.
Some early research notes used a `Duration(3mo)` rule. `Duration` is not a
rule name and is reported as a validation error, so write `ValidFor(3mo)`
instead. The existing notes are planned to be migrated.

Future relative policies need different evidence. A package rule needs the
version used during research; a file rule needs a content fingerprint. A shared
`last_updated` date cannot replace those observations.

## Evaluate Without Renewing

```mermaid
flowchart TD
    D[Read document and policy] --> V{Declaration valid?}
    V -->|No| X[Report validation errors]
    V -->|Yes| B[Resolve existing baselines]
    B --> O[Collect current observations]
    O --> R[Evaluate every rule]
    R --> A[Resolve action and uncertainty]
    A --> P[Return report]
    U[Explicit content update or review] --> E[Supply evidence used for the update]
    E --> N[Plan renewable baseline edits]
    N --> C[Caller applies edits]
    C --> D
```

Evaluation reads existing state. Renewal records a content update or review.
For example, a stale three-month rule can renew when `last_updated` is explicitly
advanced after review. Merely running the evaluator again leaves it stale.

`Evergreen` never triggers and `TimeSensitive` always triggers; neither has a
baseline to advance. `ValidUntil` is nonrenewable: once its deadline passes,
ordinary renewal does not clear it. Changing that deadline is a policy edit.

Proposed renewal behavior preserves references and edits their target properties.
If one property supplies both a renewable baseline and a nonrenewable deadline,
the renewal must surface that conflict rather than silently move the deadline.

Renewal is planned to edit only the bytes of the values it changes, so comments,
quoting, key order, and the Markdown body survive untouched. That precision
limits which YAML shapes it can edit. Renewal refuses, and writes nothing, when:

- the policy is a flow-style list, such as `content_policy: [ValidFor(3mo)]`
- a value it must change is a block scalar or spans several lines
- a value it must change is a double-quoted string containing escape sequences

The error message suggests rewriting the policy as a block list:

```yaml
content_policy:
  - ValidFor(3mo)
```

Evaluation has no such limit and accepts any valid YAML list.

## Choose the Action

A compact rule defaults to `refresh`:

```yaml
content_policy:
  - ValidFor(3mo, @last_updated)
```

An explicit action can request retirement instead:

```yaml
content_policy:
  - rule: ValidFor(3mo, @last_updated)
    action: archive
```

Renewability describes how the rule's baseline can advance. Action describes
what to do when the rule triggers. These are independent choices.

When multiple rules trigger, use **`remove` > `archive` > `refresh`**. For
example, simultaneous refresh and removal triggers produce an effective action
of `remove`. Keep both triggered rules in the report so the decision is
explainable. Declaration order does not affect the result.

The consuming application executes the action. Evaluation itself does not
refresh, archive, or remove a document.

## Understand Incomplete Evidence

A valid rule can be triggered, not triggered, or unknown. A missing baseline or
failed observation is unknown. An invalid rule declaration is a validation error.
Neither should silently become a successful freshness check.

| Evidence | Document status | Effective action |
| --- | --- | --- |
| All rules evaluated; none triggered | Fresh | None |
| No confirmed trigger; some unknown | Unknown | None |
| Highest confirmed action is refresh | Stale | Refresh |
| Highest confirmed action is archive/remove | Expired | Archive/remove |

Unknown rules cannot nominate actions. However, an unknown removal rule could
outrank a confirmed refresh. Report that refresh as the highest confirmed action
and mark action resolution incomplete.

If removal is already confirmed, an unknown refresh cannot change the winner.
Action resolution is complete even though evaluation still has an unknown
result. Consumers need both completeness indicators.

## Check and Renew from the CLI

The planned `policy` command has two subcommands:

```sh
policy check notes.md                  # report (add --plain or --json)
policy check --at 2027-01-01 notes.md  # evaluate as of a chosen date
policy renew notes.md                  # preview the renewal edits
policy renew notes.md --write          # apply them
```

`policy check --needs-action` answers "does this document need action?" by
printing `true` (stale or expired), `false` (fresh), or `unknown` (not every
rule could be evaluated). It exits `0` for all three answers; `1` means an
error such as an invalid declaration or unreadable file, and `2` a usage error.
The answer is printed rather than signaled by exit code because a shell `if`
treats any non-zero exit as false and would quietly read `unknown` as fresh.
Compare the text instead:

```sh
case "$(policy check --needs-action notes.md)" in
  true)    echo "notes.md needs a refresh, archive, or removal" ;;
  false)   echo "notes.md is fresh" ;;
  unknown) echo "notes.md could not be fully evaluated; read the report" ;;
  *)       echo "policy check failed" >&2; exit 1 ;;
esac
```

`policy renew` changes nothing unless `--write` is given. `--on <date>` sets
the update date, which defaults to today (UTC). It exits `0` when a preview is
produced or the edits are written, and `1` on any error: an unsupported YAML
shape, a file that changed between planning and writing, or missing evidence.

## Library Ownership

The library defines policy meaning, interprets baselines, compares observations,
and resolves actions. Providers obtain facts from the outside world. Optional
bundled integrations are planned for existing package, file, symbol, and web
capabilities; callers can substitute their own providers.

For example, a package provider reports a release version. The policy evaluator
decides whether that version crosses the configured major or minor boundary.
Time policies need only the document data and an explicit evaluation time.

The first implementation is planned in two phases: time and constant rules,
then file content changes. Exact date boundaries, structured declaration syntax,
reference traversal, and line-ending handling for file fingerprints remain
under design review.
