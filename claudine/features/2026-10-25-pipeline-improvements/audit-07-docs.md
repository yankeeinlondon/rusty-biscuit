---
kind: implementation-log
---

# Documentation audit: 2026-10-17-content-policy-base-schema

Updated **4 existing canonical documents**; created **0 documents**. The
completed specification now lists five canonical documents using implicit
repository-relative paths. All five declare `kind: canonical-documentation`.

| Canonical document | Coverage | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/definition.md` | Embedded schema identities, shared preparation, disk equivalence, baseline selection and precedence, custom paths, override behavior, validation results, public library accessors | Clarified that the baseline's list requirement applies to non-null values; explicit optional null is accepted by the schema but rejected by the policy parser. Existing sections already cover preparation errors, no disk/network fallback, no filesystem dependency, CLI selection, origins, and cache reuse. |
| `darkmatter/docs/inline/schema-validation.md` | Default CLI/compose validation, opt-outs, custom baselines, unconfigured library API, pending-value checks, no-insertion semantics | Added a complete schema-free policy example, the valid/invalid command outcome, passive-check explanation, and a link to declaration limitations. |
| `darkmatter/docs/schemas/darkmatter-schema.md` | The shipped baseline's content, ownership, imported editor schema, optionality, document overrides | Added canonical kind; clarified the current global-catalog/partial source and explicit-null behavior; distinguished declaration parsing from evaluation. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Built-in typing outside a checkout, extension/trigger/document precedence, completion, hover, precise diagnostics, dependency watching, template limitation | Reviewed unchanged: current content already covers the feature and has canonical kind. |
| `content-policy/docs/topics/policy-lifecycle.md` | Declaration grammar, editor schema, parser agreement and deliberate differences, reusable artifact, passive validation versus freshness/evidence | Added canonical kind and a complete mixed compact/long-form example with CLI outcomes and opt-out scope. Corrected parser-only rejections previously attributed to evaluation. Clarified optional null and why referencing the whole reusable schema does not type the `content_policy` property. |

No new page was necessary: the policy lifecycle guide owns author-facing
policy declarations, the inline validation guide owns normal CLI usage, and
the schema reference owns embedding and baseline behavior.

## Validation

Compared the documentation with `Policy::from_declaration`, the shipped
Content Policy schema, `fm.yaml`, `schemas::embedded::SHIPPED`, the cached
`ShippedBaseline`, and `schema::validate::load_api`. Later pipeline changes
move the runtime baseline into the global catalog's `fm` definition and use
`--disable baseline-schema`; the documentation describes those integrated
forms rather than restoring the older snapshot's paths and flags.

All five canonical references exist under a package area's `docs/` folder and
have the requested kind. The completed specification's body is byte-for-byte
unchanged from HEAD. Relative links and heading anchors in the three edited
guide/reference pages resolve. `git diff --check` passed. No feature/fix names
or links were added to canonical documentation; generic examples of file-match
patterns in existing schema guides remain examples, not links to specs.

The concurrent test agent owns execution verification and corrected the
corresponding stale comments in `content-policy/schemas/content-policy.yaml`
to attribute passive declaration rejection to the parser. Relevant skill
guidance already distinguishes parser rejection from evaluation and required
no edit in this step.
