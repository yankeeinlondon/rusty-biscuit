# Content Policy

Content Policy is a planned Rust library and CLI for declaring when Markdown
content needs to be refreshed, archived, or removed. Policies live in the
document's frontmatter and are evaluated against dates or observations of files,
packages, symbols, programs, and web resources.

**Status: planned.** The design is agreed, but the library and CLI are not
implemented. Examples below show the planned interface.

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
refreshing the content does not extend it. Dates take effect at 00:00 UTC, so
the named day is not included: the document is archived for all of January 1,
2027.

Multiple rules act as a logical OR of triggers. When several trigger, the
highest-priority action wins:

**`remove` > `archive` > `refresh`**

The report retains every triggered rule and its reason. Evaluation reports the
intended action; the consuming application carries out the refresh, archive, or
removal, and decides for itself how to behave while a document is stale.

## Declaring Rules

A rule can use a compact string. Its default action is `refresh`:

```yaml
content_policy:
  - ValidFor(3mo, @last_updated)
```

Use the `rule` / `action` form when specifying a different action. The default
frontmatter key is `content_policy`, in snake_case like the repository's other
frontmatter keys, and the caller can configure a different one. A document
without a policy always gets a default policy: `ValidFor(6mo)` unless the
caller replaces it. A caller can replace the default but not remove it.

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
not required. The shorthand `ValidFor(3mo)` references a configurable
default date property, `last_updated` unless changed. A reference names one
top-level property; a dotted path such as `@review.last_checked` is an error.

Dates are enough for time policies. Other relative policies need evidence such
as the package version used during research or a fingerprint of a source file.
A file rule keeps its fingerprint in a frontmatter property of its own, which
the rule references:

```yaml
config_fingerprint: blake3-lf:9f2c41…e7
content_policy:
  - FileChanged(src/config.rs, @config_fingerprint)
```

Content Policy is also planned to ship a schema for `content_policy` entries, so
editors running DMLS (Darkmatter's language server) can complete rule names and
flag a mistyped rule or action as you write.

## Evaluation and Renewal

**Evaluation reads existing evidence. Renewal records evidence after a content
update.** Checking an old document for the first time must not grant it a new
freshness interval.

A missing baseline produces an unknown result. Recording the first baseline,
which `policy renew` does for any baseline that is missing, is an explicit
assertion that the content is current. Later renewal updates the
baseline of renewable rules while preserving their settings, such as duration.
Tools that already bump `last_updated` when they change content, such as
Darkmatter's `md hash`, renew every rule that references `@last_updated`; the
`policy renew` command is the explicit route and also handles inline dates.

| Rule | Behavior | Renewal |
| --- | --- | --- |
| `Evergreen` | Never triggers | No baseline to renew |
| `TimeSensitive` | Always triggers | No baseline to renew |
| `ValidFor(duration, baseline)` | Triggers after the interval elapses | Replace the starting date |
| `ValidUntil(date)` | Triggers at a fixed deadline | Nonrenewable; changing the deadline is a policy edit |
| `FileChanged(path, @property)` | Compare current file content with the fingerprint in `property` | Replace the fingerprint |
| `SemVerMajorChange` / `SemVerMinorChange` | Compare a package's current release with its recorded version | Replace the version |
| `SymbolChanged` | Compare selected symbol content with recorded content | Replace the fingerprint |
| `UrlChanged` / `SchemaChanged` | Compare selected remote content with recorded content | Replace the comparison baseline |

The initial implementation is planned in two phases: the four time/constant
rules, then `FileChanged`. Package, symbol, and remote policies are later
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
Exact Rust types and signatures will follow implementation design.

## CLI Direction

The planned CLI is `policy`:

```sh
# Report with policies, results, reasons, and the effective action
# (terminal-formatted by default; --plain or --json for scripts)
policy check document.md

# Whether any rule has confirmed that the document needs action:
# prints true, false, or unknown
policy check --needs-action document.md

# Preview the edits that record a content update, then apply them
policy renew document.md
policy renew document.md --write
```

`--needs-action` includes expired documents, and it prints `unknown` rather
than `false` when freshness cannot be determined. Both commands exit `0` when
they produce their output and `1` on an error, so scripts read the printed
answer, not the exit code.

The policy key, default policy, and default date property are set with
`--key`, `--default-policy`, and `--date-property`, each of which falls back to
an environment variable before the built-in value.

See [Policy Evaluation and Renewal](docs/topics/policy-lifecycle.md) for the
planned lifecycle, including a scripting example and renewal's limits.
