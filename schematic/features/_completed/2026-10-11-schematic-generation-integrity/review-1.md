---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-11T22:55:26-07:00"
spec: "2026-10-11-schematic-generation-integrity/spec.md"
feature: "2026-10-11-schematic-generation-integrity/review-1.md"
implemented: true
implemented_by: claude/opus
log: schematic/features/2026-10-11-schematic-generation-integrity/log.md
description: "A **feature** review of `2026-10-11-schematic-generation-integrity/spec.md`"
next_spec: "schematic/features/2026-10-12-schematic-runtime-envelope/spec.md"
next_plan: "schematic/features/2026-10-12-schematic-runtime-envelope/plan.md"
review_stage: human-decision-check
ready: false
findings:
  - "high: carry out ruling on item 1: enforce R13 duplicate method+path as errors and merge the GitHub and Bitbucket duplicate pairs"
  - "high: carry out ruling on item 2: record R30 completion after the author-authorized deletion of schematic/gen/schematic/"
observations: []
human_resolved: true
human_review: true
human_review_items:
  - Decide whether the existing GitHub and Bitbucket duplicate routes receive a temporary exception to required validation errors.
  - Authorize deletion of the obsolete schematic/gen/schematic folder, or explicitly revise the cleanup requirement.
has_blocked_findings: false
blocked: false
human_in_the_loop:
  - |-
      **Item 1 — Decide how existing duplicate routes may be generated until selectors are available.**

      This meets test 2, **Contradicts a decision**. Requirement R13 says: “Two endpoints in one API with the same HTTP method and the same path (compared after normalizing path-parameter names) fail validation.” The implementer instead reports warnings for every such duplicate. The changed premise is that shipped definitions already violate this rule: GitHub's GetGitTree and GetGitTreeRecursive use the same GET route, while Bitbucket's ListDirectoryContents and GetFileContentRaw also share a GET route. They differ in query behavior or response type. Enforcing the rule would stop normal generation; editing definitions or introducing a way to distinguish variants is expressly outside this feature's scope.

      According to the implementation log, the OpenAPI documents retain GetGitTreeRecursive and GetFileContentRaw and omit the other operation on each route. Thus a warning permits incomplete exported descriptions, even though Rust can keep the separate endpoint methods.

      - **Accept a narrow temporary exception for the two existing route pairs (recommended).** Keep those pairs warning-only until the response-model/GraphQL feature provides selectors, while rejecting other same-API duplicate routes as R13 requires. Pros: existing generation continues, newly introduced duplicates remain errors, and no endpoint-definition migration is pulled into this feature. Cons: the two OpenAPI omissions remain an explicitly accepted limitation, and the generator needs a small, temporary exception.
      - **Accept the implemented blanket warning until selectors arrive.** Pros: smallest change and no temporary exception list. Cons: new accidental duplicate routes also pass and can lose operations in exported documents.
      - **Enforce R13 without exceptions now.** Pros: follows the decided rule exactly. Cons: generation of the existing GitHub and Bitbucket APIs fails until their definitions are changed.
      - **Bring forward definition changes or selector support.** Pros: can resolve the underlying ambiguity now. Cons: expands this feature and changes the assigned scope or order of later work; it requires a separate author decision about that scope.

      If you accept the recommendation, record the temporary exception and its removal condition as an author ruling; the next fix cycle can restrict warning severity to those two pairs and document the OpenAPI limitation. No selector or definition edit is authorized by that recommendation. While undecided, acceptance of R13 is blocked and the later selector work cannot safely assume which duplicates this generator rejects.
  - |-
      **Item 2 — Authorize removal of the stale duplicate generated folder, or change the cleanup requirement.**

      This meets test 2, **Contradicts a decision**: requirement R30 requires deleting the obsolete folder, but the implementation log marks it incomplete. The spec separately says its deletion authorization is a request, not a grant, and its current message to the reviewing agent says: “Do not delete it without the author's confirmation.” This is an unresolved authorization, not a request to reaffirm an authorization already granted.

      The folder `schematic/gen/schematic/` contains five tracked files from an old generation run in the wrong directory. The implementer reports that it is neither a workspace member nor a test or CI input. Its presence can mislead someone into editing the wrong generated output. The prior permission layer refused deletion, and the implementer left it intact.

      - **Authorize deletion in this feature (recommended).** Pros: completes the required cleanup; the files remain recoverable from Git history; the reported dependency checks found no consumers. Cons: an agent still needs to perform the deletion and verify that only these five stale files were removed.
      - **Keep the folder and explicitly waive R30.** Pros: no deletion is needed. Cons: the misleading duplicate remains and this feature no longer delivers its agreed cleanup.
      - **Assign deletion to a separate change.** Pros: separates the cleanup from generator work. Cons: changes the spec's statement that this feature alone owns the deletion, and requires explicit ownership and authorization for that later change.

      If you accept the recommendation, authorize an agent to remove only `schematic/gen/schematic/` during the next fix cycle. This does not authorize a commit or changes to other folders. While undecided, R30 remains incomplete. The saved `checks/cargo-machete.txt` reference can stay as historical output under the supplied default because no regeneration recipe was found; recapturing it need not block this deletion.
next: "2026-10-11-schematic-generation-integrity/review-2.md"
---

# Review 1: author decisions required

The review stops at the decision check, before comparing implementation with specification. Two items require the author: the change from required duplicate-route errors to warnings, and the unfinished stale-folder cleanup whose authorization remains unconfirmed. The complete context, options, recommendations, and consequences appear in `human_in_the_loop` above, ranked by rework risk.

**Production readiness has not been assessed.** There are no code-review findings because that review has not begun. The `ready` field is `false` because readiness was not assessed (corrected by the author on 2026-10-12; the reviewer originally wrote `true` under the rule that only high findings make it false, which let the repair step treat this stopped review as production ready). `human_resolved: false` and the nonempty `human_in_the_loop` hold the review cycle. No tests were run, implementation code was not reviewed, and the next spec and plan were not refreshed.

## Implementation decisions (informational)

These items come from the spec and implementation log. They do not request confirmation.

| Area | Recorded decision and classification |
| --- | --- |
| Diagnostics | Supplied fields, line rendering, serialization, stderr, and warning exit behavior adopted. Group diagnostics list every contributing API. Output identifiers are `rust`, `openapi`, and `postman`. Additional diagnostic names describe identifier, suffix, name-collision, module-path, filename, schema-registry, and unresolved-operation-link checks; the log's final table omits the last addition recorded in Phase 3. These are inexpensive generator-local choices. |
| Retained inventory | Supplied ownership/current-definition/export checks adopted. An owned `mod.rs` establishes a retained directory module; complete directory ownership is needed before deletion. Unknown or inconsistent inventory fails. Shared compatibility compares `shared.rs` and `ws_shared.rs` byte-for-byte; the next runtime feature already owns replacing this interim check. |
| Publication | Groups and shared files precede prelude and the final `lib.rs` write. Obsolete-path deletions follow that write so replacement content exists before cleanup. The prepare/publish API supports failure testing without a special injection mechanism. Full generation and imports also protect unowned output occupants; export cleanup preserves non-artifact entries. These internal choices do not change later scope. |
| Partial exports | Only selected complete groups are rewritten. Export roots need no retained Rust inventory. Both resolvers produce the same 15 module names for the 18 current APIs; resolver unification remains assigned to later work. |
| OpenAPI | Supplied deterministic operation-name allocation adopted, with numeric suffixes starting at 2. Incoming security names are renamed before merging. These choices support the agreed grouped export behavior. Duplicate routes are the separate author item above. |
| Compile guard | Isolated temporary crates use copied lockfiles, offline locked compilation, TOML-safe local paths, a separate reusable child target, and nextest for fixture execution. Offline workspace update preserves pins instead of generating a new lockfile. Four slow tests cover fixtures, real output/partial regeneration, a compiler-error control, and baseline query behavior. Child builds clear archive execution settings and retain compiler flags/wrapper. The cache-error hint is test-only polish. |
| Test and docs layout | Request-side tests share the compiled fixture and existing modules rather than a new file. Documentation uses three topic pages instead of two. Existing test modules declare the added files according to the log. Test placement, fixture construction, helper names, and page splitting are inexpensive choices. |
| Query split | The supplied five-element request tuple is adopted. A shared helper appends query pairs. Response hooks now receive a path without declared query pairs and a full URL containing them. The log reports no response-hook consumers outside Schematic; this is inexpensive to revisit while the already-planned envelope replacement is pending. |
| Request inputs | Supplied text/binary body types, optional typed header/cookie fields, exact wire names, parameter precedence, cookie encoding, comma-separated header/cookie arrays, repeated query-array keys, and temporary credential setter/environment scan adopted. Literal percent preservation and array commas follow the supplied encoding/array rules and require no reaffirmation. |
| Authentication | Resolution moved into generated shared support to avoid repeated code and incidental module splitting. Explicit credentials suppress environment fallback. Client debug output excludes credential storage. API and endpoint cookies merge by name, as the spec requires. HTTP token validation and content-type validation are local implementation choices; `EnvMapping` remains unchanged. |
| Public generator helpers | Preparation/publication and shared helper APIs were added; unused generator export entry points were removed. The log reports no external callers for removed APIs. Their naming and structure need no author decision in this early repository. |
| Counts and generated diffs | Measured 549 endpoints, 16 multipart endpoints, and 7 header parameters; historical multipart counts do not reconcile. Logged diffs cover Ollama operation names, query splitting, lockfile resolution, shared auth, OpenAI header support, and request-enum documentation. These are factual updates, not scope decisions. |

## Deferred work and verification limits

- Three older ignored end-to-end tests and the `cargo test`-based `test-e2e` recipe remain because removal authorization was not confirmed. Their cleanup is polish under this decision screen, rather than a change to later scope; this does not endorse the recipe or assess the new guard's coverage.
- Postman header/cookie parameter support remains unimplemented with per-output diagnostics. Existing component-link merge gaps, nested query-array compilation, and small-client split-threshold fragility are recorded limitations. Their classification as code findings or observations belongs to the pending implementation comparison.
- Requiredness, parameter style, explode, enum fidelity, nine embedded-query path migrations, module-resolver unification, feature flags, broader redaction, runtime compatibility markers, response envelopes, and selectors remain assigned to later features. Query keys can still appear in request URLs and HTTP errors until the already-assigned redaction work; that assignment needs no reaffirmation here.
- The log reports passing fresh-output checks on macOS, native Windows, and WSL2. Linux's native check failed because its offline cache lacked `cc v1.2.52`; cache population is an automation task, not human-only evidence. No hosted CI run contains this implementation. The log also records incomplete CI completion-accounting/evidence-reuse integration; the pending code review must reconcile that report with the final acceptance ledger. No CI scope change is proposed or authorized by this review.
- All eight dependents reportedly passed tests; no dependency was added and no dependent source was edited. These reports were not independently verified in this decision-only review.
- No live account, device, credential, or intrinsically human-only verification is required. The saved Cargo-machete report stays unchanged under its supplied default.

## Human rulings

### Item 1

Ruling (2026-10-12): **R13 is enforced as written, with no exceptions, and the
REST selector design is withdrawn.** The reviewer's recommended exception is
rejected: its removal condition ("until selectors arrive") could never be met,
because the planned selector was a request-body field and both duplicate pairs
are `GET` requests.

1. **An endpoint is `method + path`, permanently.** The uniqueness key stays
   `(method, normalized_path)`; no selector, parameter, or parameter value ever
   joins it. Same-API duplicates are validation errors (`Severity::Error`);
   `DUPLICATE_METHOD_PATH_SEVERITY` and its warning behavior are removed. The
   D36 forward hook described in this spec is withdrawn (this spec stays as a
   snapshot; the departure is recorded here and in the implementation log).
2. **Each endpoint has one request type and enumerates its possible
   responses.** When an endpoint can return more than one shape (for example
   JSON or an SSE stream depending on `stream`, or a directory listing or file
   bytes depending on what a repository path names), the definition declares
   every shape and the generated success type is a tagged enum. Cause and
   effect ("returned when `stream=true`") is documentation only; the client
   never routes on request values. An undeclared response is an error.
3. **Responses must be distinguishable on the wire.** Validation fails when
   two declared shapes cannot be told apart by status code, `Content-Type`, a
   JSON discriminator field, or JSON shape (array versus object).
4. **Parameter location is mechanics, not identity.** Query, body, and header
   inputs behave the same in the definition model; each parameter declares
   its allowed location, and a body input on `GET` fails validation.
5. **This feature's fix cycle** (authorized here, overriding the spec's
   prohibition on touching endpoint literals for exactly these two edits):
   - GitHub `GetGitTree` and `GetGitTreeRecursive` merge into one `GetGitTree`
     with an optional `recursive` query parameter (both returned
     `GitTreeResponse`, so `recursive` is an option, not a variant).
   - Bitbucket `ListDirectoryContents` and `GetFileContentRaw` become one
     source endpoint returning `Binary`, which is truthful for both a
     directory and a file until the response-model feature gives it a
     `Directory | File` enum.
   - Remove the warning severity, regenerate with `just generate`, update the
     docs and skills that describe the warning, and log the departure.
     Nothing outside `schematic/` uses any of the four endpoints.
6. **Later features** are amended to match:
   `2026-10-14-schematic-definition-api` merges the seven OpenAI
   `?beta=true` endpoints into their non-beta twins with an optional `beta`
   query parameter (gated on a beta/non-beta schema comparison) and adds
   parameter locations; `2026-10-16-schematic-response-model-graphql` drops
   selectors, turns the Anthropic and Ollama streaming pairs into single
   multi-shape endpoints, adds the JSON-discriminator and JSON-shape tags,
   upgrades Bitbucket, and adds the OpenAI `GetResponse` SSE shape;
   `2026-10-18-schematic-fidelity-polish` drops REST selector export and
   import. WebSocket message selectors are a separate mechanism and are
   unaffected.

### Item 2

Ruling (2026-10-12): **Deletion authorized (option 1).** `schematic/gen/schematic/`
(five tracked files) is removed, completing R30. `checks/cargo-machete.txt`
stays unchanged as historical output. Performed during the decision session
with `git rm -r`; not committed.
