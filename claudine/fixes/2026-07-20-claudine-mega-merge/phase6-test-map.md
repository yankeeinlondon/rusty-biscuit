# Phase 6 requirement-to-test map

Candidate: `df13f68dd7ad3ef22ef7e324dbdc213ed75afcd6`

Phase 6 changes no runtime behavior, parser, schema, template, prompt, or
configuration artifact. It audits the immutable candidate. Consequently no
new regression test was added and no persisted value requires a new
read/write/read round trip in this phase. The behavioral requirements remain
mapped to the tests introduced or identified in Phases 2–5.

| Phase 6 requirement | Public evidence | Verification level |
|---|---|---|
| Generated provider/catalog data matches shipped definitions | `claudine-gen check` plus generator drift tests | generator and L1 |
| Shipped prompt frontmatter parses and routes through the normal CLI path | `shipped_prompt_corpus_parses_frontmatter`; `level2_lifecycle_shipped_implement_route_matches_direct_run` | passive corpus and CLI L2 E2E |
| Dispatch, composition, diagnostic transport, test placement, and harness call sites retain their accepted seams | `dispatch_inventory`, `composition_seams`, `error_guards`, `test_placement`, `run_harness_loop_call_sites`, `shipped_prompt_route_drift`, and `shipped_prompts` test binaries | L1 integration/source guards |
| Package behavior remains green on the merged candidate | `just test` | package-area L1 |
| Real terminal and lifecycle behavior remains green on macOS | `just test-l2 --no-fail-fast` | package-area L2 |
| Source and documentation policies remain green | `just lint` | package-area lint/source guards |
| Candidate contains both reviewed histories | `git merge-base --is-ancestor <feature-tip> <candidate>` for both frozen tips | structural Git audit |
| Final change surface has no unowned paths or flows | `git diff main...<candidate>`, marker scans, `git diff --check`, and GitNexus `detect_changes` | repository audit |
| Required Linux, Windows, and attended L3 behavior passes | dedicated Linux tmux CI, Windows CI/native runtime, and attended L3 run | native external gates |

## Exact targeted tests added in Phase 6

None. This is an audit-only phase and the candidate is intentionally unchanged.

## Exact targeted checks run

- `claudine-gen check`
- the seven focused test binaries named in the table above: 61 passed and 2
  non-applicable tests skipped
- isolated reproduction of
  `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`

The isolated reproduction failed on all four nextest attempts with the exact
shipped fixture bytes still containing `agent: goose`; the failure stack did
not persist the expected `agent: gemini` before retry. This is a candidate
blocker, not waived audit debt.

## Representation and negative coverage

The inherited mapped suites cover native and quoted YAML values, absent and
present values, list/scalar forms, boundary widths/counts, malformed and
unresolvable references, typed downstream diagnostics, lifecycle state, and
filesystem mutation boundaries. The shipped-prompt corpus and real shipped
route satisfy the passive-artifact and normal-invocation requirements. Phase 6
does not introduce a new representation or persistence boundary.
