---
kind: implementation-log
---

# Documentation audit: 2026-10-31-type-aware-evaluation

Updated **5 existing canonical documents**; created **0 documents**. The
completed spec now lists thirteen canonical documents using implicit
repository-relative paths. Every reference exists under a package area's
`docs/` folder and declares `kind: canonical-documentation`.

## Changes

| Existing document updated | Change |
| --- | --- |
| `darkmatter/docs/topics/darkmatter-expressions.md` | Added an early route into call checks, diagnostic policy, and reports. Corrected the guard example to declare `release_tag` in `$schema` instead of assigning type text as its value; corrected loop syntax and explained that denying the always-false warning blocks composition. |
| `darkmatter/docs/topics/host-functions.md` | Added canonical kind. The registration example now writes its facts artifact before installation consumes the function set; the artifact section explains that ordering. |
| `darkmatter/docs/cli/policy.md` | Added canonical kind and a descriptive top-level title in place of an isolated overview heading. |
| `darkmatter/docs/cli/report-json.md` | Added canonical kind and a descriptive top-level title. |
| `biscuit-file/docs/topics/file-references.md` | Added canonical kind while retaining existing ownership frontmatter. |

## Canonical coverage

| Functionality | Canonical documents |
| --- | --- |
| Effective declaration layers, root unions, nested paths, recursive references, call returns and fallback types | Expression guide; `darkmatter/docs/inline/call-check.md`; `darkmatter/docs/topics/schemas/definition.md` |
| Guard forms, registered facts, binding precedence, mutation invalidation, conservative limits, absence warnings | Expression guide; `darkmatter/docs/inline/interpolation.md` |
| Pure file-reference validation, `is_file_type`, malformed-literal warnings and guarded file parameters | Expression guide; biscuit-file reference guide |
| Host registration, handler/signature parity, generated facts artifacts and incomplete editor analysis | Host-function guide; `darkmatter/dmls/docs/diagnostics.md`; `claudine/docs/topics/schemas.md` |
| Shared diagnostics, strict editor warnings, absence suppression and host globals | Call-check guide; `darkmatter/docs/lsp/configuration.md`; DMLS diagnostics; Claudine schema guide |
| Allow/deny/disable policy, selector identity, precedence, removed flags, stopping before later effects, no rollback | Policy guide; `darkmatter/docs/cli/compose.md` |
| Fixed report shape, null fields, authored source positions, stream isolation, retained warnings, failure output and exit behavior | Report guide; compose command guide; `darkmatter/docs/topics/compose-requests.md` |

The remaining eight referenced pages already contain current explanations and
the requested kind, so they were reviewed without edits in this step. Existing
prose, examples, tables, and diagrams cover the functionality without needing a
new page. Runtime and editor limits remain explicit rather than being described
as stronger static guarantees.

## Validation

Compared the docs with the effective declared-type projection, host-function
registration and artifact APIs, CLI policy registry, and report sink. In
particular, `install_host_functions` takes ownership, while `facts_artifact`
borrows the function set, confirming the corrected example order.

A reference check confirmed all thirteen paths, unique entries, correct kinds,
and the completed spec's byte-for-byte unchanged body. The five edited pages
contain no dated feature/fix identifiers. Relative targets added to the
expression introduction exist. `git diff --check` passed. The concurrent test
agent owns behavioral verification; this docs slice ran no test suite and
changed no source or test code. No skill drift was found in the relevant
current skill guidance.
