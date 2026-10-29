---
kind: implementation-log
---

# Documentation audit: 2026-10-27-darkmatter-lifecycle-ownership

Updated **6 existing canonical documents**; created **0 documents**. The
completed specification now lists ten canonical documents using implicit
repository-relative paths. All ten declare
`kind: canonical-documentation`; the specification body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/lifecycle/index.md` | Shared grammar and engine ownership, event ordering, host registration and renames, `md` effects and shell invariants, passive surfaces, Rust integration | Added a concise implementation-ownership boundary and clarified stdout redirection under `--output report-json`. Existing examples, diagrams, registration rules, and the planned Claudine request/reply migration remain current. |
| `darkmatter/docs/topics/lifecycle/error-handling.md` | Failure stages, cancellation, error fields, recovery modes, terminal downgrade and finalization | Added canonical kind; distinguished checks performed before the run from checks inside it, with initial provider selection versus active sequence selection as the example. |
| `darkmatter/docs/cli/compose.md` | Public `md compose` invocation, root-only events, supported versus ignored actions, output order, shell reporting and exit behavior | Added canonical kind and an approachable command introduction and title. Existing lifecycle and report-mode sections were reviewed against the host and CLI. |
| `darkmatter/docs/topics/schemas/definition.md` | Compiled baseline from the global catalog's `fm`, reserved imports, lifecycle schema projection, trigger kind, matching, discovery and flags | Reviewed current; already canonical. The current unified `--disable schema-triggers` policy supersedes the snapshot's intermediate flag spelling. |
| `darkmatter/docs/topics/schemas/authoring-schemas.md` | Authored global types, independent `fm.ctx`/`ctx`, availability, explicit catalog registration and schema layering | Reviewed current; already canonical. The guide distinguishes implemented property/path matching from planned predicate families. |
| `darkmatter/docs/topics/schemas/schema-targeting.md` | Trigger selection, AND/OR composition, presence gates, path matching, invalid envelopes, precedence and discovery | Added canonical kind and rewrote the guide around the supported grammar, using complete report and path examples. Planned environment/expression predicates are now a separate labeled section rather than the only worked example. |
| `darkmatter/docs/schemas/darkmatter-schema.md` | Current runtime baseline, embedded imports, lifecycle partials, passive validation and open application data | Reviewed current; already canonical. |
| `darkmatter/dmls/docs/features.md` | Passive editor typing, host catalogs and lifecycle artifacts, shared event/global/action interpretation | Added canonical kind; replaced extension-only deployment wording with schema-trigger, catalog, artifact and explicit-extension guidance, including a link to editor setup. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Claudine host names, outcomes, failure routing, cancellation and recovery policy | Corrected the failure table and introduction so an early provider/schema check does not imply `blocked` or `finalize` before a lifecycle exists. |
| `claudine/docs/topics/schemas.md` | Host globals, compiled runtime versus disk editor schemas, shared verb types, host event names and launch typing | Reviewed current; already canonical. |

No additional page was needed. The shared lifecycle introduction and error
handling guide are the obvious engine entry points; the Claudine guide owns
its host-specific behavior. The trigger topic now explains the implemented
selection behavior independently of the longer planned authoring material.
The docs preserve final integrated behavior, including unified CLI policies,
recursive action schemas, passive call checking, and data-driven editor host
artifacts, rather than restoring intermediate pipeline states.

## Validation

Compared the documentation with `EventRegistry`, `md_host`, the shared
preflight stages and policy routing, Claudine's registered checks and call
sites, the global catalog, and the trigger grammar/matcher/lint. The registry's
explicit late-global list intentionally contains `loop` and `group`; `seq` is
typed through the host catalog, so the separate artifact documentation's list
was correctly left alone. No skill update was needed: the existing skills
already record the ownership and early-check boundary accurately.

Checked all ten canonical references and frontmatter kinds, all relative
Markdown targets in the six edited pages, absence of feature/fix references in
those pages, and the specification body against HEAD. `git diff --check`
passed. This documentation-only slice ran no tests; the concurrent test agent
owns functional verification. A first link-check expression mistook a schema
array spelling inside code for a Markdown link; the final check removes code
before checking link targets and passes.

The test agent confirmed the early-check wording against host-check tests. Its
trigger-migration source guard rejected spelling the retired kind in the new
guide; the text now describes that rejection without reproducing the retired
spelling, preserving the existing explicit negative-fixture allowlist.
