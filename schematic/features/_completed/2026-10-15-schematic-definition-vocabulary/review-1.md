---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-22T02:07:08-07:00"
spec: 2026-10-15-schematic-definition-vocabulary/spec.md
feature: 2026-10-15-schematic-definition-vocabulary/review-1.md
implemented: false
description: 'A **feature** review of `2026-10-15-schematic-definition-vocabulary/spec.md`'
ready: true
readiness_assessed: false
review_scope: human-decision-triage-only
findings: []
observations: []
recurrence: false
human_review: true
human_resolved: true
has_blocked_findings: false
blocked: false
human_in_the_loop:
  - |-
    Approve the source and precedence of credentials when one endpoint replaces the API's usual authentication. For example, most endpoints may use a bearer token while one needs a separate Basic username and password. The agent implemented endpoint-specific environment mappings despite the specification reserving that choice for the author. It uses per-call credentials, then compatible client credentials, then environment values; incompatible per-call credentials or missing replacement credentials fail before sending. Explicit anonymous calls bypass replacement authentication.

    - Recommended: accept endpoint-specific mappings and the implemented precedence; explicitly decide whether anonymous calls may bypass a replacement policy. This supports independently configured endpoints without moving a secret into a different authentication mechanism.
    - Reuse only compatible API mappings: smaller definition surface, but endpoints cannot name independent credentials.
    - Require explicit runtime credentials: avoids endpoint environment mappings, but environment-only clients cannot call replacement endpoints.
  - |-
    Decide whether generated Rust clients may omit optional query parameters they cannot serialize. The agent chose a warning and omission for optional unsupported inputs, and a generation error for required ones. Fifteen OpenAI parameters lose their setters, including several include, metadata, order, and variant parameters. The log reports that their former JSON-string serialization produced incorrect requests; omission still removes caller capabilities.

    - Recommended: accept the explicit limitation for this feature, retain named diagnostics, and schedule support only for parameters callers need.
    - Implement the affected shapes before release: retains those capabilities, but adds importer and serialization work.
    - Reject generation even for optional unsupported inputs: makes the limitation unavoidable, but prevents generating otherwise usable OpenAI clients.
  - |-
    Approve the version-1 portable definition bundle layout. The spec proposed separate bindings and schemas maps. The agent instead stores ordered schema components, including their Rust bindings, inside api.schemas; the envelope contains format_version, api, and optional models. This preserves registration order and avoids storing the same facts twice, but changes the proposed interchange format.

    - Recommended: accept the implemented layout before another tool starts depending on version 1.
    - Restore separate maps: follows the proposed layout, but requires consistency rules and a way to preserve meaningful component order.
  - |-
    Approve the OpenAI importer correction beyond the listed definition-adoption scope. The agent changed the default for form-style explode to true and regenerated 279 declarations, preserving repeated query pairs such as project_ids=a&project_ids=b. It also adjusted the historical comparison to allow exactly that change. Separately, regeneration added 287 deprecated: false fields, and Basic-auth migrations required mechanical assertion edits in Bitbucket, EMQX, and Eversolo.

    - Recommended: accept the importer correction and mechanical changes as part of this migration; the log says the old default would send incorrect array requests after query fidelity landed.
    - Separate the importer correction into its own scoped change: provides independent review, but this feature must wait for it to preserve those requests.
  - |-
    Approve the OAuth client-authentication behavior change. The agent made the previously ignored client_auth setting control requests: ClientSecretBasic sends credentials in an authorization header; ClientSecretPost sends them in the body; None sends the client ID and withholds any configured secret. This changes behavior beyond rearranging the grant configuration types, although it makes the configured choice effective.

    - Recommended: retain the implemented behavior and its reported mock-server checks.
    - Preserve the former always-Basic behavior temporarily: reduces behavioral change here, but leaves a public setting ineffective and requires a separate correction.
  - |-
    Decide whether to retain both descriptive rate-limit and retry metadata. GitHub adopts rate-limit headers; generated documentation, OpenAPI extensions, and Postman descriptions read the metadata. The runtime deliberately performs no throttling or retries. Retry guidance has no shipped provider adoption, only fixtures.

    - Recommended: keep the adopted rate-limit metadata and retain retry guidance only if describing future provider policies is useful now.
    - Drop retry guidance until a provider needs it: reduces unused public vocabulary and maintenance.
    - Keep both as implemented: avoids another vocabulary change, with the cost of maintaining retry fields without a shipped adopter.
human_review_items:
  - Resolve the six decisions in human_in_the_loop; each item gives the implemented behavior, alternatives, and recommendation. Record the chosen behavior under Human rulings before resuming implementation review.
---

This review stops for author decisions. Two independent reviewers read the specification and implementation log. Both identified a consequential unresolved authentication choice; the log also records capability changes and departures that deserve approval before a code review proceeds.

No implementation-to-specification comparison, reproductions, test audit, or test execution was performed. There are no code findings or finding IDs. Production readiness has **not been assessed**. Under the requested rule that only a high finding makes `ready` false, `ready: true` records an empty finding inventory; it is not a production-readiness endorsement. The nonempty `human_in_the_loop` and `human_resolved: false` halt the review process. The spec's `completed` flag follows the requested `ready` metadata rule and likewise must not be treated as an implementation-review verdict.

## Decisions requiring author review

The six frontmatter items are the decision record for this pause. Their supporting records are:

| Decision | Why it matters | Evidence |
| --- | --- | --- |
| Endpoint credential sources and anonymous bypass | Determines which credentials an endpoint accepts and when a request may proceed without them. | [Spec: credential-source question](spec.md:265), [spec: author resolution required](spec.md:306), [log: recommendation adopted](implementation-log.md:562), [log: runtime behavior](implementation-log.md:654), [log: anonymous override](implementation-log.md:673). |
| Unsupported optional query inputs | Fifteen OpenAI parameters disappear from the Rust client, even though other parts of the client remain usable. | [Log: omitted query shapes and parameters](implementation-log.md:904). |
| Portable bundle layout | Establishes the file shape other tools will consume. | [Spec: proposed envelope](spec.md:253), [log: actual envelope and rationale](implementation-log.md:817). |
| OpenAI import and mechanical source changes | Goes beyond the named provider adoptions and changes the historical comparison. | [Log: importer correction](implementation-log.md:903), [log: explicit confirmation request](implementation-log.md:1059). |
| OAuth client authentication | Changes where client secrets are sent, and whether they are sent at all. | [Log: newly honored authentication setting](implementation-log.md:660). |
| Descriptive rate-limit and retry metadata | Rate-limit metadata has a shipped adopter; retry guidance currently has none. Neither executes a retry policy. | [Spec: review requested](spec.md:241), [log: consumers and keep recommendation](implementation-log.md:1058). |

The specification says its open-question recommendations require author resolution. The log initially identifies credential sources as unresolved, then adopts the recommendation as a default. Its final claim that no ruling remains is therefore insufficient evidence of author approval. This is a reason for this decision pause, not a code defect finding.

## Other recorded decisions

The following choices are informational unless the author wants to change them. They do not require reaffirming already settled requirements.

- Typed paths reuse parameter definitions; deprecation is a Boolean; Basic authentication stores named environment fallback lists; OAuth configuration is separated by grant; structural composition extends the existing descriptor. These generally follow authorized planning defaults.
- Required query values become constructor and endpoint-method arguments, without additional call-object setters. Requests with required fields lose `Default`; pagination-managed inputs remain supplied by the pager. The specification already settles the required-field and pager behavior.
- The runtime request representation changes to preserve encoded query bytes, and the final runtime compatibility version changes from 4 to 5. This supersedes the earlier phase's decision to leave it at 4.
- Bundle loading has explicit null exceptions for grant-inapplicable OAuth fields, rejects unknown members, and validates references without fetching external files. Canonical writing preserves array order. Shipped bundles are generated in tests rather than all committed; one fixture is committed.
- An identically named, bound, and structured composition descriptor cannot be distinguished from one originating in another registration context. The log records that limitation.
- Exported authentication scheme names, Postman variable suffixes, common path declarations, and repeated per-request metadata were chosen deterministically. Repeating metadata in every request's documentation can be reduced later if the author finds it distracting.
- The log lists replaced OAuth tests, rewritten wire-baseline cases, dependent migrations in homelab and sniff, and the generated GitHub test migration. Those claims are recorded here without verification.

See the [log's consolidated agenda](implementation-log.md:1053) for the complete inventory and its phase-by-phase evidence. Earlier phase deferrals for bundles, query fidelity, documentation, and validation are subsequently reported complete; they are not outstanding work merely because an earlier checkpoint deferred them.

## Remaining limitations and deferred scope

These are reported limitations, not findings from a code review:

- Device-code execution remains absent from schematic-oauth, although definitions can describe that grant; existing execution flows reject it. Optional authentication and consolidation of the two authentication configuration sources also remain outside this feature.
- Importing OpenAPI deprecation flags remains outside the implemented scope. Generated OpenAI parameters receive false values.
- The log leaves pre-existing duplicate EMQX path parameters and unbuilt generator source files unchanged. It also records that module documentation omits replacement-specific environment variables while endpoint/request documentation contains them.
- The log reports macOS and cross-host L1 checks; some fresh-generation, compile-fail, generated-schema, and dependent checks ran only on macOS. Cross-OS proof is not a readiness blocker in this review, and those reports were not independently verified.

## Human rulings

Ruled by the author on 2026-10-22. Each recommended option is accepted; the implementation stands unchanged.

1. **Endpoint credential sources:** accept endpoint-specific environment mappings and the implemented precedence (per-call, then compatible client credentials, then environment; incompatible or missing replacement credentials fail before sending). An explicit anonymous call may bypass a replacement policy, as implemented, because the caller chose it.
2. **Unsupported optional query inputs:** accept omission with named diagnostics for this feature; required unsupported inputs stay generation errors. Support for an omitted parameter is scheduled only when a caller needs it.
3. **Bundle layout:** accept the implemented version-1 envelope (`format_version`, `api` with ordered schema components and their bindings, optional `models`).
4. **OpenAI importer correction and mechanical changes:** accept the form-style explode default of true, the 279 regenerated declarations, the narrowed historical-comparison allowance, the `deprecated: false` fields, and the Bitbucket/EMQX/Eversolo assertion edits as part of this migration.
5. **OAuth client authentication:** retain the implemented behavior; `client_auth` now controls where client credentials are sent.
6. **Rate-limit and retry metadata:** keep both as implemented. Removing retry guidance would cost another vocabulary change; it can be dropped later if it stays unused.

A subsequent review must still perform the implementation comparison and verification omitted by this decision-only review.
