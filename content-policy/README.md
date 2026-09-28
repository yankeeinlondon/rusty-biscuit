# Content Policy

Content Policy is a planned Rust library and CLI for declaring when Markdown
content needs to be refreshed, archived, or removed. Policies live in the
document's frontmatter and are evaluated against dates or observations of files,
packages, symbols, programs, and web resources.

**Status: design draft.** The library and CLI are not implemented. Examples below
show the proposed interface; details identified as proposals remain open for review.

## A Document's Lifecycle

A document can need periodic refreshes and still have a fixed retirement date:

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

The first rule becomes due three months after the last content update. Updating
`last_updated` renews that interval. The second rule expires at a fixed deadline;
refreshing the content does not extend it.

Multiple rules act as a logical OR of triggers. When several trigger, the
highest-priority action wins:

**`remove` > `archive` > `refresh`**

The report retains every triggered rule and its reason. Evaluation reports the
intended action; the consuming application carries out the refresh, archive, or
removal.

## Declaring Rules

A rule can use a compact string. Its default action is `refresh`:

```yaml
content_policy:
  - ValidFor(3mo, @last_updated)
```

Use the `rule` / `action` form when specifying a different action. The default
frontmatter key is `content-policy`, configurable by the caller. The proposed
default for an absent policy is `ValidFor(6mo)`; callers can replace it.

There are two baseline forms:

```yaml
content_policy:
  - ValidFor(3mo, 2026-09-28)
```

```yaml
last_updated: 2026-09-28
content_policy:
  - ValidFor(3mo, @last_updated)
```

The first stores the date inside the rule. The second references a frontmatter
property using `@`. These forms keep the evidence with the document; a sidecar is
not required. The proposed shorthand `ValidFor(3mo)` references a configurable
default date property, initially `last_updated`.

Dates are enough for time policies. Other relative policies need evidence such
as the package version used during research or a fingerprint of a source file.
A structured rule form for this richer evidence is proposed in the design draft.

## Evaluation and Renewal

**Evaluation reads existing evidence. Renewal records evidence after a content
update.** Checking an old document for the first time must not grant it a new
freshness interval.

A missing baseline produces an unknown result. Recording the first baseline is
an explicit assertion that the content is current. Later renewal updates the
baseline of renewable rules while preserving their settings, such as duration.

| Rule | Behavior | Renewal |
| --- | --- | --- |
| `Evergreen` | Never triggers | No baseline to renew |
| `TimeSensitive` | Always triggers | No baseline to renew |
| `ValidFor(duration, baseline)` | Triggers after the interval elapses | Replace the starting date |
| `ValidUntil(date)` | Triggers at a fixed deadline | Nonrenewable; changing the deadline is a policy edit |
| `FileChanged` | Compare current file content with recorded content | Replace the fingerprint |
| `SemVerMajorChange` / `SemVerMinorChange` | Compare a package's current release with its recorded version | Replace the version |
| `SymbolChanged` | Compare selected symbol content with recorded content | Replace the fingerprint |
| `UrlChanged` / `SchemaChanged` | Compare selected remote content with recorded content | Replace the comparison baseline |

The initial implementation is proposed to cover the four time/constant rules,
followed by `FileChanged`. Package, symbol, and remote policies are later
extensions. File creation/removal and program installation/removal are also
candidates; their state-versus-transition semantics need review.

Renewability belongs to the rule type. Action is independently configurable:
`ValidFor(3mo, @last_updated)` can request archival instead of refresh.

## Reports and Uncertainty

| Document status | Meaning |
| --- | --- |
| `fresh` | Every rule was evaluated and none triggered |
| `stale` | The highest confirmed action is `refresh` |
| `expired` | The highest confirmed action is `archive` or `remove` |
| `unknown` | No rule is confirmed triggered, and at least one cannot be evaluated |

Invalid declarations produce validation errors. Missing evidence, unavailable
providers, and failed observations produce unknown rule results with reasons.
Neither is silently treated as fresh.

An unknown rule cannot nominate an action. If it could require an action above
the highest confirmed action, action resolution is incomplete. For example, a
confirmed refresh plus an unknown removal rule reports `refresh` as the known
action, with an incomplete resolution. The report distinguishes this from a
fully resolved refresh.

## Library and Providers

The library owns parsing, validation, baseline interpretation, comparisons,
renewal planning, and action precedence. Replaceable providers supply facts.
Optional bundled integrations make those capabilities available to library
callers as well as the CLI:

| Capability | Intended integration |
| --- | --- |
| Package releases and program availability | Sniff |
| File reference resolution | Biscuit File |
| Symbol extraction | Tree Hugger |
| HTTP resource fetching | Biscuit File's fetching support, subject to adapter design |
| Extracted web-page content | Reaper, when implemented |

A provider reports a version, presence observation, or content snapshot; the
library applies the policy. Time rules can evaluate without external providers.
The initial architecture uses one library with optional integration modules.
Exact Rust types and signatures will follow design review.

## CLI Direction

Proposed evaluation commands:

```sh
# JSON report with policies, results, reasons, and the effective action
policy document.md

# Whether any rule has confirmed that the document needs action
policy document.md --is-stale
```

The proposed boolean includes expired documents. It must distinguish `false`
from an inability to determine freshness. Renewal will be an explicit operation;
its command syntax and exit codes remain review questions.

See [Policy Evaluation and Renewal](docs/topics/policy-lifecycle.md) for the
planned lifecycle. The active design draft is `2026-09-28-content-policy`.
