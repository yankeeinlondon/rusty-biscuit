---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### High: Structured extraction accepts the first JSON value instead of enforcing a single JSON value

The spec requires structured output extraction to tolerate surrounding prose or code fences, but says "the result must be a single JSON value" and invalid JSON must be `InvalidResponse`, never silent success ([spec.md:264](spec.md:264), [spec.md:269](spec.md:269)). The current fallback extraction path returns the first balanced object or array span it can parse ([structured.rs:65](../../../claudine/contract/src/structured.rs:65), [structured.rs:66](../../../claudine/contract/src/structured.rs:66)), and `balanced_span` stops as soon as that first span closes ([structured.rs:102](../../../claudine/contract/src/structured.rs:102), [structured.rs:137](../../../claudine/contract/src/structured.rs:137)).

That means an assistant response such as `{"name":"Ada"} {"name":123}` or a fenced block followed by another JSON value can succeed if the first value satisfies the schema, even though the provider did not return a single structured value. The L1 structured tests cover valid JSON, fenced JSON, invalid JSON, schema violation, and prose ([tests.rs:442](../../../claudine/contract/src/tests.rs:442), [tests.rs:453](../../../claudine/contract/src/tests.rs:453), [tests.rs:465](../../../claudine/contract/src/tests.rs:465), [tests.rs:475](../../../claudine/contract/src/tests.rs:475), [tests.rs:487](../../../claudine/contract/src/tests.rs:487)), but none cover multiple JSON values or trailing JSON after a parsed span.

Impact: a consumer requesting structured output can receive a successful `InferenceData::Structured` from an ambiguous or malformed model response. This weakens the adapter-side validation guarantee and can hide provider behavior that should be surfaced as `InvalidResponse`.

Fix by making extraction prove uniqueness: after parsing a whole response, fenced block, or balanced span, reject any non-whitespace/non-prose content that contains another parseable JSON value, or more simply make the permissive fallback fail when another top-level balanced object/array remains outside the selected span. Add L1 tests for adjacent JSON values, fenced JSON plus another JSON value, and prose containing two balanced JSON values.

## Verification Level Assessment

| Requirement | Required level | Strongest present | Assessment |
|-------------|----------------|-------------------|------------|
| Object-safe `Arc<dyn InferenceAdapter>` | L1 | L1 | OK |
| Prose responses through fake runner | L1 | L1 | OK |
| Structured response success and schema validation | L1 | L1 | Partial: schema validation exists, but uniqueness of the extracted JSON value is not enforced |
| Invalid structured responses map to `InvalidResponse` | L1 | L1 | Gap: invalid JSON, schema violation, and prose are covered; multiple JSON values are not |
| Claude session planning: stream output, guard prompt, strict MCP, shadow HOME, env allowlist | L1 + real_ | L1; real_ tests exist but were skipped in this run | Mostly OK, live provider not verified here |
| Codex session planning: `exec --json`, TOML-quoted guard override, read-only sandbox, isolated CWD/HOME, env allowlist | L1 + real_ | L1; real_ tests exist but were skipped in this run | OK at L1, live provider not verified here |
| Tool-free execution and post-hoc tool rejection for enabled providers | L1 + real_ per enabled provider | L1 for Claude/Codex; real_ tests skipped in this run | OK at L1, live provider behavior not verified here |
| Terminal rendering / keyboard behavior | L2/L3 only if specified | Not applicable | No terminal UX requirement in this spec |

## Tests Run

- `cargo test -p claudine-contract --color=never` passed: 37 unit tests, 4 `real_provider` tests, and 2 doctests.
- The `real_provider` tests returned early because `CLAUDINE_CONTRACT_REAL=1` was not set, so this run did not exercise live Claude Code or Codex binaries.

## Ready For Production

No. `ready` is `false` because the structured-output path can silently accept ambiguous multi-value assistant output despite the spec requiring a single JSON value. The prior Codex guard-encoding issue appears fixed at L1.
