---
kind: implementation-log
---

# Documentation audit: 2026-10-21-claudine-schemas

Updated **4 existing canonical documents**; created **0 documents**. The
completed spec now lists six canonical documents with implicit repository-relative
references. The spec's snapshot body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `claudine/docs/topics/schemas.md` | Bundle ownership and file classification, trigger selection, action categories, closed action shapes/open application data, recursive typing, parser-only rules, disk discovery, embedded fallback, editor support, authored overrides | Rewrote the introduction around what an unfamiliar developer can do; corrected initialization/launch ordering and shared parser ownership; completed the deployment example with the host lifecycle artifact; replaced empty CLI reference links with current documentation. Existing canonical kind preserved. |
| `claudine/docs/topics/composition.md` | Effective launch verdict, control ownership versus optional application cleanup, phase differences, shell-free initialization, fresh reads, and frozen completion schema | Reviewed; already current and canonical, no edit needed. |
| `claudine/docs/topics/frontmatter-properties.md` | The frontmatter catalog and application-data boundary | Added canonical kind; clarified separate sequence typing and that an authored application schema can restrict otherwise open application data. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Stack execution, conditional branches, parser restrictions, and event-time evaluation | Reviewed; already current and canonical, no edit needed. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Disk-only editor discovery, diagnostics, completion, hover, and deployment outside the Claudine area | Corrected the stale five-file table to the current seven-file bundle, added sequence files and recursive roles, and distinguished companion host catalog/artifact deployment. Existing canonical kind preserved. |
| `darkmatter/dmls/docs/editors/README.md` | Editor setup without an extension-baseline entry | Added canonical kind; corrected the five-file deployment instruction and described the host catalog/artifact companions. |

The schema and composition guides already provide concrete YAML examples and
Mermaid flows for discovery and launch. No new page is needed: the schema guide
is the obvious entry point and links into the frontmatter, execution, and editor
references. Later recursive/container behavior remains documented as implemented,
rather than restoring the original snapshot's five-file, three-level design.

## Related drift maintenance

Updated `.claude/skills/claudine/cli-commands.md` to remove an obsolete payload-size
claim and the old claim that `break` and `prep` are extra planned inventory entries.
The note now points to the enforced payload-size guard and includes these
implemented verbs in the parser inventory. This skill change is not included in
the canonical-document count. Relevant stale source-comment references were
reported to the test agent, which owns code edits.

## Validation

Compared the documentation with the seven `include_str!` entries and winning-origin
logic in `composition/schema/bundle.rs`, `load_effective_schema`, the three current
trigger files, and the schema-directory README. Checked all six reference paths
and canonical kinds, the CLI reference anchors, and byte-for-byte equality of the
spec body with HEAD. `git diff --check` passed. No new feature/fix references were
added to canonical documentation. Test execution belongs to the concurrent test
agent; this documentation audit did not run test suites.
