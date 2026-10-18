---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Startup omits package and package-area schema roots"
    - "R1-F2 (high): Unlisted Markdown targets never receive on-demand analysis"
    - "R1-F3 (high): A ready full index stays ready after losing a readable document"
    - "R1-F4 (high): Trigger preparation still blocks message dispatch"
    - "R1-F5 (high): Trigger payload failures are missing from failed-trigger counts"
observations: []
human_in_the_loop: null
human-in-the-loop: false
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-17T18:34:54-07:00
spec: 2026-10-17-logging-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/log.md
description: "A **fix** review of `2026-10-17-logging-improvements/spec.md`"
fix: 2026-10-17-logging-improvements/review-1.md
next: 2026-10-17-logging-improvements/review-2.md
---

# Logging improvements: review 1

The fix is **not production ready**. Five findings remain. Most notably,
after a previously indexed document becomes unreadable, heading rename can
return successful edits that omit its references. Startup and lazy analysis
also implement narrower coverage than the specification requires.

## Initial decision review

Two independent agents reviewed the implementation log and the specification
and plan before implementation review began. Neither found an unresolved
design decision requiring a human pause. The plan records author approval of
its eight implementation rules on October 17. Temporary deferrals between
implementation phases were subsequently addressed.

Routine implementation choices include normalizing BOM/CRLF before YAML
parsing, retaining timings on server state, comparing context values for
cache freshness, watching dependency directories, and canceling a shared
index build when its last caller leaves. The log also documents combined
workspace identifiers and link-count conventions. No approval is requested
for these choices.

The log expressly records narrower startup preparation, unchecked targets
outside the workspace, and synchronous registry preparation. Those have
concrete requirements in the approved spec; the findings below request
implementation of those requirements. They do not require another design
decision. Publishing a missing `$schema` error before file listing is a
reasonable small departure: that error comes from direct disk resolution,
and cannot be falsely inferred from an incomplete file list.

The persistent index store, a new scope setting, and unrelated memory
investigation remain explicitly outside this fix. No human review blocks
repair of the findings.

## Findings

### R1-F1 (high): Startup omits package and package-area schema roots

**Authority — spec R3:** “DMLS prepares the union of schema roots applicable
to workspace documents,” using the existing contexts and package catalog.
R8 requires distinct file counts across those searched directories and a
failure summary when applicable scopes fail.

This is incomplete scope preparation: a context for a workspace folder is
treated as sufficient preparation for documents in its descendant packages.
In the dmls package,
[`prepare_workspace_registries`](../../dmls/src/overlay/mod.rs:518), which
prepares the startup schema cache, creates one probe directly in each
workspace folder. It does not prepare the package contexts in its catalog.
Those roots first enter the cache when a document there is analyzed.

**Reproduction:** use a fixture repository with a Cargo workspace member
`area/pkg`, a valid named schema in `area/schemas/area.yaml`, and
`area/pkg/schemas/broken.yaml` containing:

```yaml
kind: trigger-schema
match: {}
```

Initialize DMLS at the repository root without opening a document. The
startup line says `schema registry ready roots=0 scoped_schemas=0
failed_triggers=0 failed_scopes=0`. The public document-context and schema-root
APIs identify both scoped directories as searched roots. Opening a Markdown
document with frontmatter in `area/pkg` finally logs the broken trigger.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Startup package-root preparation | Declared nested package with broken trigger | Root omitted until a package document opens | Prepare and classify it at startup |
| R1-F1.2 | Startup package-area preparation | Same fixture, valid area schema | Area schema omitted from startup totals | Include the area root and named schema |
| R1-F1.3 | Startup readiness and count projection | Same fixture, no open documents | Reports ready and zero failures | Include the scoped failure and counts |
| R1-F1.4 | Tree/home root preparation, control | Existing known-count startup fixture | Correct counts; suite passes | Correct counts, as observed |
| R1-F1.5 | Environment roots and per-document shadowing, control | Existing direct-discovery parity fixture | Cached registry equals direct discovery | Preserve that parity while expanding preparation |

Prepare the catalog-derived union without parsing unopened Markdown. Keep
document-specific payload resolution and shadowing; replacing the five-root
authority with a workspace-wide search path would introduce another defect.
Extend startup tests to include unopened packages and package areas.

### R1-F2 (high): Unlisted Markdown targets never receive on-demand analysis

**Authority — spec R6:** referenced targets are parsed when their headings or
outgoing transclusions are needed; “On-demand targets outside the workspace
remain resolvable through `FileReference` ... and receive dependency
invalidation; the file list must not restrict existing resolution behavior.”

This is an eligibility error shared by all lazy target consumers: membership
in the discovery list is required before a resolved Markdown target can be
parsed. In the dmls package,
[`WorkspaceIndex::needs_parse`](../../dmls/src/graph/invalidate.rs:316),
which decides whether a target needs a disk parse, accepts a new target only
when it is listed. Target traversal also follows graph-node edges only;
resolved file edges cannot supply an external target to the parser.

**Reproduction:** start with the existing file-list session fixture. Write
`target.md` with `# Alpha`, `## Beta`, and a transclusion back to the source.
Open a source linking to `target.md#missing` and transcluding that target.
Repeat with the same target excluded from discovery, then with the editor's
workspace folder narrowed to `docs/` and the target addressed as
`../target.md` inside the enclosing fixture repository. Resolution succeeds
in all cases. Only the listed control produces the missing-anchor and cycle
diagnostics. For the other two, `#beta` hover shows only the path, definition
lands on line zero instead of the heading, and completion after `#` is empty.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Lazy parse eligibility and graph-target extraction | Excluded and outside-folder Markdown targets | Neither target is parsed | Admit resolved Markdown dependencies independently of discovery |
| R1-F2.2 | Fragment-link diagnostics | Both targets, `#missing` | No diagnostic | Report the missing anchor |
| R1-F2.3 | Heading completion | Both targets, cursor immediately after `#` | Empty list | Offer `alpha` and `beta` |
| R1-F2.4 | Link hover | Both targets, `#beta` | Path only | Show target title and Beta heading |
| R1-F2.5 | Heading definition | Both targets, `#beta` | Line zero | Navigate to Beta on line two |
| R1-F2.6 | Transclusion traversal and cycle detection | Both targets transclude the source | No cycle diagnostic | Follow the dependency and report the cycle |
| R1-F2.7 | Target invalidation through watched changes and fallback rescans | Same unlisted targets | No lazy parse/dependency is installed to invalidate; change filtering rejects unlisted paths | Track and refresh the referenced target independently of discovery |
| R1-F2.8 | Listed Markdown control across diagnostics, completion, hover, definition, and cycles | Same content, listed target | Correct diagnostics, headings, and locations | As observed |
| R1-F2.9 | Wiki heading targets, control | Listed wiki candidate in existing heading-completion session | Correct heading completion | Preserve listed-candidate behavior; no broader external wiki-name search is requested |

Keep the discovered workspace list separate from the set of parsed referenced
dependencies. The repair must preserve the rule against parsing arbitrary
`::code` bytes as Markdown. Cover the external target's subsequent change and
deletion through both watcher and save-fallback paths, in addition to its
initial parse.

### R1-F3 (high): A ready full index stays ready after losing a readable document

**Authority — spec R7:** the full index stays current file by file; references,
rename, file rename, and workspace symbols answer from a complete index. “If
a listed Markdown file cannot be read or parsed, report the build failure
and return an error ... rather than silently issuing partial rename edits
or incomplete results.”

This is a stale readiness flag: a disk-read failure removes a document's parse
without withdrawing the assertion that the full index is complete. In the
dmls package,
[`WorkspaceIndex::ensure_parsed`](../../dmls/src/graph/invalidate.rs:332)
removes a failed parse, but the full-index flag remains true. The
[`request prerequisite check`](../../dmls/src/router.rs:1488) therefore
allows every full-index consumer to run against the incomplete snapshot.

**Reproduction:** create `target.md` with `# Alpha` and a closed `source.md`
containing `[t](target.md#alpha)`. Open the target, request workspace symbols
to complete the index, replace the referrer's disk bytes with invalid UTF-8,
and send its watched-file change. Rename Alpha to Beta. The response is a
successful edit of the target heading alone. No error identifies the unreadable
referrer. Restoring the referrer's original bytes and sending another change
still leaves references empty: the path is already listed but no longer has
a parse, so the full-index update does not retry it.

Invalid UTF-8 is the deterministic read-failure seam used by the spec and the
existing full-index tests; ordinary permission/read failures enter the same
branch. No permission manipulation or timing race is needed.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Watched modification | Ready index; existing closed file becomes unreadable | Parse dropped, completeness retained | Withdraw completeness and expose the failure |
| R1-F3.2 | Watched creation | Ready index; unreadable Markdown file joins | New file stays unparsed, completeness retained | Fail complete-index operations until resolved |
| R1-F3.3 | Save-fallback rescan | Ready index; existing closed file becomes unreadable | Same successful incomplete results | Same failure policy |
| R1-F3.4 | Buffer close | Ready index; close an authoritative buffer over unreadable disk bytes | Disk parse disappears, completeness retained | Same failure policy |
| R1-F3.5 | File-list replacement | Ready index; ignore-file event relists an unreadable new file | New file stays unparsed, completeness retained | Same failure policy |
| R1-F3.6 | References | Each of the five transitions above | Successful incomplete response | Error naming the unreadable input |
| R1-F3.7 | Heading rename | Each of the five transitions above | Successful partial edits | Error, no partial edit |
| R1-F3.8 | File rename | Each of the five transitions above | Successful response | Error, no partial edit |
| R1-F3.9 | Workspace symbols | Each of the five transitions above | Successful incomplete response | Error naming the unreadable input |
| R1-F3.10 | Recovery after watched modification | Restore bytes, then send another change | References remain empty | Retry parsing and restore complete answers |
| R1-F3.11 | Initial full build, control | File is unreadable before the first build | Existing test returns an error and retries after repair | Preserve this behavior after readiness too |
| R1-F3.12 | Deletion during build, control | File leaves the discovery list | Existing reconciliation test discards stale work without treating an absent file as unreadable | As observed |

All four consumer requests were exercised after every transition in rows
1–5. Make complete-index readiness depend on current listed parses and read
failures, and reuse the initial-build error/retry policy. Add acceptance tests
for a failure after readiness and for recovery, rather than only testing an
unreadable file before the first build.

### R1-F4 (high): Trigger preparation still blocks message dispatch

**Authority — spec startup lifecycle and R3:** the server continues accepting
open/change, cancellation, and shutdown messages; “Registry updates are
prepared outside the request's provider chain and installed atomically when
ready.” The approved plan additionally makes the overlay registry entry point
a lookup only.

This is synchronous prerequisite work on the message-dispatch thread. In the
dmls package,
[`TriggerRegistries::lookup`](../../dmls/src/overlay/registry.rs:325),
which serves schema registries to the overlay, calls preparation directly
whenever a document's entry is missing, stale, or has another context. That
preparation enumerates roots and loads payloads under the overlay mutex.
Moving only the initial workspace probes to a background thread does not
move later preparation off dispatch; a new document also has a distinct
registry key from the startup probe.

**Reproduction:** use the in-memory public server session and a tracing
subscriber that records the current thread ID for every event. Initialize an
empty schema directory, create a malformed trigger, send its watched-file
event, and open a Markdown document with frontmatter. Then change the trigger
and repeat through watched changes, YAML save, and configuration reload. Each
registry-load failure is emitted on the same thread as the corresponding
notification completion. Source inspection confirms the load runs inline
before the handler can return to its message loop. This proves the execution
placement without asserting a machine-dependent delay.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | First document registry | Open frontmatter document after startup | Registry preparation executes on dispatch | Prepare off dispatch; wait only the dependent work |
| R1-F4.2 | Watched schema reload | Change envelope and send event | Diagnostics refresh prepares on dispatch | Background preparation and atomic installation |
| R1-F4.3 | YAML save reload | Save changed trigger input | Diagnostics refresh prepares on dispatch | Same asynchronous prerequisite |
| R1-F4.4 | Configuration/context reload | Send configuration change with an open consumer | Next schema assembly prepares on dispatch | Same asynchronous prerequisite |
| R1-F4.5 | Save-fallback rescan | Rescan invalidates all registries | Refresh reaches the same inline lookup; code invariant | Background preparation before dependent refresh |
| R1-F4.6 | Schema-dependent request builder and trigger-authoring overlay | Missing/stale registry | Both call the same inline preparation entry point; code invariant | Lookup installed/prepared state without doing the load in the provider path |
| R1-F4.7 | Startup preparation | Initial workspace probes | Worker is off dispatch, but holds the same overlay mutex across preparation | Preserve background execution without blocking dispatch on that long-held lock |
| R1-F4.8 | Ready unchanged lookup, control | Existing read-counter acceptance test | Zero enumeration/read/hash/scan work | Preserve this behavior |

The common request builder serves document symbols, definitions, document
links, references, highlights, folding ranges, hover, semantic tokens,
completion, code actions, and formatting. Each inherits the same prerequisite
path; the trigger-authoring overlay is a second direct caller. Cancellation
cannot be processed while either inline load holds dispatch.

Use the asynchronous prerequisite pattern already present for full-index
builds: prepare without holding the overlay lock across I/O, install a
compatible result atomically, and refresh or resume dependent work. Verify
that a deliberately paused schema preparation still permits an edit,
cancellation, and shutdown to be handled. Existing cached-lookup counter
tests do not establish that responsiveness requirement.

### R1-F5 (high): Trigger payload failures are missing from failed-trigger counts

**Authority — spec R3/R8:** a trigger schema “claims `kind: trigger-schema`
and loads”; “Add `failed_triggers` for claimed files that fail to load.”
Registry elapsed time and preparation include trigger payload loading.

This is classification before the operation that determines success. In the
dmls package,
[`classify`](../../dmls/src/overlay/registry.rs:66), which assigns each
root file's logging category, labels a parseable envelope as a successful
trigger before its payload is resolved. The
[`summary`](../../dmls/src/overlay/registry.rs:531) reads that category
without incorporating payload-load failures. Consequently a missing payload
produces `trigger_schemas=1 failed_triggers=0 failed_scopes=1`, despite the
trigger failing to load.

**Reproduction:** initialize a fixture with one trigger whose match requires
`title` and whose `$schema` is `payload.yaml`. Keep the envelope unchanged and
vary only the payload or its dependency. Capture the startup registry line.
The following cases were each run through the public server session.

| Instance | Site | Shape tested | Observed trigger/failed-trigger counts | Expected |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Envelope classification, clean control | Valid envelope and valid payload | 1 / 0; ready | 1 / 0; ready |
| R1-F5.2 | Envelope failure classification, clean control | `match: {}` | 0 / 1; failed scope | 0 / 1; failed scope |
| R1-F5.3 | Payload-file load | Missing `payload.yaml` | 1 / 0; failed scope | 0 / 1; failed scope |
| R1-F5.4 | Payload parser | Payload contains `$schema: [unclosed` | 1 / 0; failed scope | 0 / 1; failed scope |
| R1-F5.5 | Payload import resolution | Property uses `Missing@./missing.yaml` | 1 / 0; failed scope | 0 / 1; failed scope |
| R1-F5.6 | Payload example resolution | Property uses `date(example(../missing.json))` | 1 / 0; failed scope | 0 / 1; failed scope |
| R1-F5.7 | Startup failure-line and count projection | Each failure above | Failure variant and `failed_scopes` are correct, but payload failures retain the successful-trigger category | Preserve failure-line selection and correct the file categories |

Keep passive root-file classification separate from context-bound load
outcomes, and include the latter when computing the required counts. Preserve
the distinct-file counting rule and shadowing behavior. Add payload-failure
controls to the startup log tests; their current malformed-envelope example
exercises only the already-correct path.

## Verification and coverage

`just test` in `darkmatter/` passed: **10,235 tests passed, 10 skipped**,
66.262 seconds. `just check-tier-coverage darkmatter` reported **zero
stranded tests**. No formatter, commit, editor window, or terminal window was
launched.

Temporary tests were compiled through the declared DMLS L1 integration target
and run with nextest. They used the existing isolated in-memory server fixture,
short-lived directories, captured logs, and real protocol requests. They
recorded current public results rather than claiming the defective behavior
was correct. Their source and module registration were removed after review;
only this review and the spec's review-iteration metadata remain changed.
The reproduction descriptions and instance tables above are the repair inventory.

| User-facing requirement | Appropriate verification | Evidence and outcome |
| --- | --- | --- |
| Unrelated malformed YAML stays harmless; claimed YAML gets located syntax errors | L1 public parser, discovery, CLI, and protocol | Existing claim, incident, embedded, corpus, CLI, and encoding tests pass |
| All applicable startup schema roots and accurate registry totals | L1 server session and captured logs | Existing tree/home controls pass; scoped-root and payload-failure probes expose R1-F1/R1-F5 |
| Cached registries avoid I/O and recover on input events | L1 work counters and server sessions | Existing ready-cache/invalidation tests pass; off-dispatch preparation remains incomplete under R1-F4 |
| File listing honors ignore/include/exclude rules and does not parse unopened Markdown | L1 filesystem fixture and parse counters | Existing discovery and startup tests pass |
| Referenced-target diagnostics, heading navigation/completion, and cycles | L1 server session | Listed controls pass; unlisted targets expose R1-F2 |
| Complete references/rename/symbols, cancellation, reconciliation, and error recovery | L1 server session with deterministic worker gates | Existing initial-build tests pass; failures after readiness expose R1-F3 |
| Startup/rescan/build fields and 500 ms request threshold | L1 captured logs and injected clock | Existing tests pass; startup coverage/count defects described above remain |
| Logs stay off the stdout protocol stream | L1 headless subprocess | Existing stdio routing tests pass |

No changed requirement needs a terminal emulator or OS keyboard injection;
L2/L3 runs would not strengthen these protocol and log checks. Cross-OS proof
is left to CI and does not affect readiness in this review.

### Input robustness

The changed file-reader boundary is the shared trigger-envelope classifier.
The configuration parser itself did not change. Existing public classifier
tests cover absent, explicit null (including YAML `kind:`), wrong scalar or
collection type, and empty `kind` values: each is unclaimed. A valid exact
claim is the positive control. Duplicate keys, trailing invalid content, and
a second YAML document produce the dedicated YAML error when the fallback
claim is present, and remain unclaimed otherwise. Valid quoted-key/JSON
forms, BOM/CRLF, multiline scalars, and nested/nonexact nonclaims are covered.
These tests passed in the full run.

The grep for defaults and swallowed parse errors found no new
`serde(default)`, `unwrap_or_default()`, `.ok()`, or element-filtering fallback
in that classifier. Its `Option` payload field and match/payload validation
precede this fix; no absent/null policy change is inferred for them. The
explicit error-to-unclaimed branch implements R1's required recovery policy.
No input-shape finding was identified at this boundary.

## Observations

None.

## Prior findings

This is review iteration 1. There are no previous review findings or partial
repairs to classify.
