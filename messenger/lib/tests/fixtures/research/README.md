# Research Contract Fixtures

Sanitized, local-only fixtures for the provider research contract
(`messenger/docs/research/platforms/_schema.yaml` and its sibling schemas).
None of these files is research. Hosts use `*.example.com`, identifiers are
placeholders (`USER_ID`, `MESSAGE_ID`), and bodies carry no message content.
`tests/research_corpus.rs` validates every file here with Darkmatter's library
and, with the `research` feature, through `messenger::research`'s typed loader
and semantic rules; it proves validation is passive and scans the corpus for
secrets and control sequences. `tests/research_validation.rs` pins targeted
behaviors with variants of these fixtures.

| Directory | Contents | Expectation |
|---|---|---|
| `contract/` | One fixture per contract representation (constraints, units, knowledge states, versions, formatting and text bindings, images, attachments/addressing/receipts, attribution, location, expression, delivery controls/eligibility/rate limits, gaps), overrides, mappings, a ten-source roster at the cap, and the four schema pilots ported to v1 (`pilot-*.md`, the v1 freeze gate) | Schema-valid and semantically valid |
| `interaction/` | Inbound scope, companion combinations, all four question kinds, and form packaging, with sanitized answer payloads in `interaction_fixtures` | Schema-valid and semantically valid |
| `diagnostics/` | Response envelopes, error records, and sanitized error fixtures: HTTP-200 application failures, plain-text errors, nested field errors, local SDK validation, success warnings, unknown codes, changed envelopes, ambiguous timeouts | Schema-valid and semantically valid |
| `negative/schema/` | One structural defect per file | Schema validation fails; the `# expect-problem:` header names the JSON Pointer (or message text) that must be reported |
| `negative/semantic/` | One semantic defect per file that SimplifiedSchema cannot express | Schema-valid by design; the Rust rule in the `# expect-rule:` header (and the file-stem prefix, `sr-unique--…`) rejects it, and every finding carries that rule |

Extra headers on `negative/semantic/` fixtures:

- `# validate-scope: accepted`: the file is clean as a fragment and is
  rejected only when judged as accepted research (full roster coverage,
  investigated gaps).
- `# expect-ineligible: <fact> <reason>`: the file validates, but the fact
  never reaches the executable projection (`sr-enforceable--*` and the
  secondary-only evidence fixture).

Cross-file fixtures (overrides, mappings) resolve their facts against the
companion documents `contract/constraints-field.md` (Discord) and
`contract/bindings-mode-switch.md` (Telegram). `contract/overrides-valid.yaml`
and `sr-override--expired-override.yaml` carry the real `target_hash` of
`c.fx.content.service_max` and the real schema fingerprint: editing that fact,
`_schema.yaml`, or `_types.yaml` makes them stale, which is the rule working;
refresh the hashes in the same change.

Criterion 1's positive roster case is the shipped
`messenger/docs/platforms.yaml` itself, which the corpus test validates and
checks against `ProviderKind::as_str()`.

Every rule code used here is defined in
`messenger/docs/research/platforms/_rules.md`; the corpus test fails when a
fixture names an undocumented rule.

Relative `$schema` paths point at the shipped schemas, so the fixtures always
exercise the real contract rather than a copy.
