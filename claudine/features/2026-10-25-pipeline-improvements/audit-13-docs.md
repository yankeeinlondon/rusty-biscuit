---
kind: implementation-log
---

# Documentation audit: 2026-10-23-shell-template-approval

Updated **4 existing canonical documents**; created **0 documents**. The
completed spec now lists all four under `canonical_docs` using implicit
repository-relative paths. Its snapshot body is unchanged.

| Canonical document | Requirements covered | Changes in this audit |
| --- | --- | --- |
| `darkmatter/docs/inline/preflight-checks.md` | Policy files and persistence; canonical template syntax; argument-list matching; type coverage and runtime validation; direct dispatch and restricted grammar; prompt choices; missing and guarded values; preapproved membership versus result caching; host APIs | Added canonical kind and an introduction explaining the reusable shape with a concrete example. Included lifecycle command surfaces, corrected quoted literal-brace guidance, clarified exact-choice eligibility and guard refinement, and explained program-option boundaries. |
| `claudine/docs/topics/pre-flight-checks.md` | Document and lifecycle integration; upfront transitive sequence approval; first/later task modes; static position; captured inputs; deferred and guarded values; runtime barriers; closed membership across retries and rereads | Removed stale claims that preapproval skips runtime blacklists. Replaced retired sequence roots and task setup/teardown syntax, included recursive first-task position and sequence shell surfaces, corrected the proxy target's schema example, and distinguished discovery defects from post-approval source changes. |
| `claudine/docs/topics/flow-control/sequences.md` | Sequence execution timing, first-task choice, late inputs and group barriers, container terminal events, current-state substitution and static graph membership | Added canonical kind and clarified that valid first-task placeholders offer shape approval as well as exact approval. Existing late-value, container, and closed-approval explanations remain current. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Byte versus template execution; live and pre-write state; structured dispatch; missing-value and policy refusal; initialization prohibition | Corrected the blanket statement that every `group` shell read is refused: a lexical group read inside a sequence is a late template input; an out-of-group read remains refused. |

No additional canonical page is needed. The two preflight guides are the
obvious entry points and already contain examples, matching tables, prompt
choices, and flow diagrams. The sequence and lifecycle guides explain the
execution-specific rules without delegating to a historical specification.
README overviews already link to the relevant preflight guides and accurately
describe typed-shape approval. The skill summaries inspected for this topic
already describe the current structured-authorization boundary.

## Validation

Compared the prose with Darkmatter's `template/typing.rs`, `canonical.rs`,
`site.rs`, and its lifecycle host guide, plus Claudine's sequence preflight
and existing integrated execution documentation. In particular, the runtime
template path checks membership before `prepare_template`, which performs
value, whole-chain policy, and direct-target checks; the persisted-rule parser
rejects unescaped braces inside quoted literal text.

Automated checks confirmed all four implicit references exist under the
package-area `docs/` folders and declare `kind: canonical-documentation`.
All relative Markdown link targets on those pages exist. No feature/fix
reference was left in these pages. The spec body is byte-for-byte identical
to HEAD, and `git diff --check` passed. Test execution belongs to the
concurrent test agent; this documentation slice ran no test suite.
