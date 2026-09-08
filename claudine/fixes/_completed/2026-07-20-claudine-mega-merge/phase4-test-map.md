# Phase 4 requirement-to-test map

Phase 4 changes generated evidence, documentation, and drifted source comments.
It does not change runtime behavior, a parser, a schema, a template,
a prompt, configuration semantics, or persistence. Consequently no new
behavioral regression test is required; the existing public and passive guards
below are the concrete acceptance tests for every reconciled claim.

| Phase 4 requirement | Public or passive verification |
|---|---|
| The checked-in dispatch inventory is exactly reproducible | `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`; the test exercises both source trees, rewrites the shipped artifact, and compares it with the committed corpus. |
| Every generated provider artifact matches its owning inputs | `cargo run -p claudine-gen -- check` and `cargo nextest run -p claudine-gen --test drift`. The drift suite covers the complete shipped provider corpus rather than selected files. |
| Source-scan, dispatch, and test-placement scope remains intact | `cargo nextest run -p claudine-cli --test error_guards --test test_placement`, `cargo nextest run -p claudine-cli --test composition_seams`, and `cargo nextest run -p claudine --test boundary_lint`. |
| All ten provider booleans normalize through the public CLI contract | `argv::rule1_provider_bool::tests::rule_1_rewrites_every_provider_boolean`; its input is derived from every compiled catalog entry and its dependent output is the canonical `--provider <slug>` argv pair. |
| Completion exposes every compiled wrapper in catalog order | `completion::root_menu::tests::full_menu_wrappers_are_catalog_derived_in_display_order`, `completion::engine::tests::run_with_context_emits_full_root_menu_on_bare_tab`, and the real CLI-path test `completion_contract::complete_subcommand_drives_dynamic_completion`. These cover the cold root invocation and downstream rendered candidates. |
| Provider adapter and system-prompt documentation matches the catalog | `provider::tests::serialized_field_list_matches_catalog`, `provider::tests::adapter_detect_exercise_all_providers`, and the provider generator drift corpus. The catalog is the delivery source of truth, including unsupported modes. |
| Composition, lifecycle, file-reference, schema, Sequence Plus, and typed-error ownership claims remain mechanically enforced | `composition_seams`, `boundary_lint`, `error_guards`, `test_placement`, and `scripts/check-lifecycle-doc-facets.sh`. Existing subsystem tests are then included again by the required `just test` area gate. |
| The Linux L2 workflow description matches the shipped workflow | Read-only comparison with `.github/workflows/claudine-tests.yml`, plus `scripts/check-lifecycle-doc-facets.sh` for the lifecycle acceptance facets. No L2 behavior changed in this phase. |

## Representation and negative coverage

The phase introduces no new input grammar. Existing tests retained by the area
gate cover the relevant variants cited by the reconciled docs: bare versus
explicit-relative and magic `FileReference` values, absent/null/append/replace
system-prompt modes, malformed mode values, missing versus discovered prompt
files, supported versus unsupported provider delivery, and completion with and
without repository configuration. Persistence behavior is unchanged, so no new
read/write/read round trip applies.

## Targeted tests added

None. Adding tests for unchanged behavior would duplicate the catalog-derived
and passive-corpus guards above.
