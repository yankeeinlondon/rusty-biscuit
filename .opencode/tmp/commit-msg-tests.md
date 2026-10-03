test: add Phase 1 RED reproductions for agent-text-is-data fix

- add darkmatter/cli/tests/l1/compose_value_provenance.rs with 8 ignored tests (red until phase 2) that pin the post-fix contract for value provenance: data-origin values are not re-read as templates, command-line setters remain templates, and the override channel preserves origin
- add claudine/cli/tests/l1/agent_text_is_data.rs with 17 ignored tests covering the four runtime rows of the spec's "Where agent text re-enters" table - lifecycle render_message, resolve_typed_value, the inline persistence repair/encode path, and the proxy.with merge
- register both new modules from their respective cli/tests/l1/main.rs so the nextest run picks them up once the per-test #[ignore] markers are removed by their phase
