---
kind: implementation-log
---

# Documentation audit: 2026-10-18-fm-namespace

Updated **9 existing canonical documents**; created **0 documents**. The
completed spec now lists **13 canonical documents** using implicit
repository-relative paths. Its snapshot body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/darkmatter-expressions.md` | Root precedence; effective `fm` snapshots; dotted/bracket dependencies; ordinary `doc`/`current_env`; migration; passive expression tooling | Added canonical kind; corrected the environment table to per-document capture, documented the explicit body-provider hook exception, and replaced the stale claim that plain `md compose` has no lifecycle globals. |
| `darkmatter/docs/topics/state-management/index.md` | A practical introduction to document, context, environment, lazy, lifecycle, and host state | Replaced stale prose claiming frontmatter is fixed and naming nonexistent `content`/`context`/`tracking` roots. Added a complete example and precise live-snapshot, precedence, and lifetime explanations with links to the detailed guides; set canonical kind. |
| `darkmatter/docs/topics/state-management/context-variables.md` | Captured versus lazy context and reserved-root conflicts | Added canonical kind and repaired its relative namespace-reference link. |
| `darkmatter/docs/inline/fm-interpolation.md` | Frontmatter dependency ordering, literal-bracket keys, partially resolved bare `fm`, cycle behavior, and inserted data | Added canonical kind and removed historical feature references from its frontmatter and drift list. Current functionality was already described. |
| `darkmatter/docs/inline/interpolation.md` | Bare and explicit property lookup, namespace shadowing, and data insertion | Added canonical kind; existing semantics remain current. |
| `darkmatter/docs/topics/compose-requests.md` | One captured environment per document; transclusion reuse; explicit host hook; shell child environments; library-supplied snapshots | Added canonical kind; existing complete API examples and flow diagram remain current. |
| `claudine/docs/topics/composition.md` | Document epochs, retry/resume/loop reuse, fresh proxy/task/leg maps, cross-consumer consistency, approval rechecks, and target identity | Reviewed unchanged; already canonical. |
| `claudine/docs/topics/state-management/context-variables.md` | Claudine context versus document environment; lazy `current`; ordinary `current_env` | Added canonical kind; existing lifetime and namespace explanations remain current. |
| `claudine/docs/topics/configuring-actions.md` | Hook-only `hook` payload root, bare hook fields, missing paths, excluded environment, matcher/condition/template migration | Added canonical kind; existing complete migration table and hook example remain current. |
| `claudine/docs/topics/pre-flight-checks.md` | Approval using the same document environment and rechecking later task values | Added canonical kind; existing policy explanation remains current. |
| `darkmatter/dmls/docs/autocomplete.md` | Passive catalog roots and completion of legitimate ordinary `doc`/`hook`/`current_env` properties | Reviewed unchanged; already canonical. |
| `darkmatter/dmls/docs/hover.md` | Reserved-root precedence and ordinary-property versus `fm.<key>` hover | Reviewed unchanged; already canonical. |
| `darkmatter/dmls/docs/diagnostics.md` | Undeclared-property advisories and passive editor behavior | Reviewed unchanged; already canonical. |

No new page was necessary: the expression reference, request guide, and hook
configuration guide are the main canonical entry points. The rewritten state
introduction makes those guides approachable without requiring prior knowledge
of the monorepo. The fixed reserved catalog remains the four roots `fm`,
`ctx`, `env`, and `current`; lifecycle and host globals are separately declared
for their scopes. The original spec's intermediate ownership descriptions are
not substituted for the integrated implementation.

## Validation

Compared the documentation with the reserved-root catalog,
`fm_namespace` lookup, lifecycle registry generic bindings, frontmatter
interpolation dependency collection, hook dispatch expression/template
adapters, and document environment APIs described by the implementation log.
The concurrent test agent reports no production semantic changes in this slice.

A scripted check confirms all thirteen references exist below package-area
`docs/` folders, use implicit repository-relative paths, and declare
`kind: canonical-documentation`. The spec body is byte-for-byte equal to HEAD.
New relative links and target headings were reviewed, and `git diff --check`
passes. No feature/fix references were introduced into canonical docs. No tests
were run by this documentation agent; the concurrent test agent owns execution.
