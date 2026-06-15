---
ready: false
agent: codex
model: ""
---

# Review: Resolution Context & Token Resolution

## Findings

1. **High: frontmatter read-side functions can fetch remote URLs, contrary to Decision B.**

   The spec explicitly decides that read-side function arguments in frontmatter are local-only: a remote URL argument in frontmatter must fail loudly, including `file_exists(url)`. The implementation wires both frontmatter interpolation passes with `options.expression_resolution_context(&runtime.remote_fetch)` at `darkmatter/lib/src/markdown/compose/mod.rs:626-632` and `:700-706`. That helper attaches `remote_fetch` whenever remote reads are enabled at `darkmatter/lib/src/markdown/compose/types.rs:1215-1219`, and the read-side functions then fetch HTTP(S) content through `ResolutionContext::fetch_remote_text` at `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:53-68` and `load_markdown` at `darkmatter/lib/src/markdown/compose/expression/functions.rs:676-684`.

   This means a frontmatter value such as `status: "{{ frontmatter('https://allowed.example/doc.md', 'status') }}"` can perform a remote read instead of producing the required local-only error. That is a behavior/security contract mismatch, not just a missing test.

   The tests currently lock in the wrong behavior: `remote_frontmatter_expression_reads_url` at `darkmatter/lib/src/markdown/compose/mod.rs:5799-5807` expects remote frontmatter expression reads to succeed. The lower-level tests only prove that a manually constructed local-only `ResolutionContext` fails (`darkmatter/lib/src/markdown/compose/expression/functions.rs:1763-1784`); they do not verify the actual compose frontmatter call sites are local-only.

   **Fix:** add a frontmatter-specific context builder, or call `ResolutionContext::new(base_dir)` plus magic paths without `remote_fetch`, for both frontmatter interpolation passes and `$()` frontmatter condition/branch evaluation if those are intended to be frontmatter-local. Then replace the compose-level remote-frontmatter success test with a failure test that asserts a helpful diagnostic.

2. **Medium: documentation now contradicts itself on whether post-shell frontmatter may use remote URLs.**

   `docs/inline/fm-interpolation.md:140-141` says the frontmatter context is local-filesystem-only, but `docs/topics/darkmatter-expressions.md:404-407` says remote URL arguments are honored in "body interpolation and the post-shell frontmatter pass." The spec's Decision B says frontmatter is local-only now, without limiting that to only the pre-shell pass.

   **Fix:** align the expression topic, inline frontmatter docs, and implementation on one rule. Per the spec, both frontmatter passes should fail loudly for remote URL arguments.

## Test Rigor

- Frontmatter local file read-side functions: **Level 1** unit coverage exists for pre-shell and post-shell passes (`frontmatter_interpolation.rs:879-928`), plus **Level 1 CLI/integration** coverage for the motivating optional `spec: file` workflow (`darkmatter/cli/tests/cli.rs:1406-1465`). This is appropriate; no real terminal encoder/renderer behavior is involved.
- `$()` token ladder and no-command diagnostic: **Level 1** parser/unit coverage is present for literals, command classification, `doc.*`, safe functions, branch diagnostics, preflight enumeration, and selected-branch execution. Level 1 is appropriate.
- Reference graph `when=`, public condition API, claudine loop, and claudine hook read-side functions: **Level 1** coverage is present (`reference/graph.rs:995-1031`, `conditions.rs:799`, `loop_engine.rs:1095-1120`, `dispatch/expression.rs:908-924`). Level 1 is appropriate because these are in-process expression semantics, not terminal behavior.
- Remote-frontmatter Decision B: **coverage is incorrect**. Existing tests assert remote reads work through compose frontmatter, so the strongest verification is Level 1 but pointed at the wrong expected behavior.

## Notes

The main local-path resolution work is otherwise in the right shape: `doc.*` is intercepted before fallback, dependency ordering for `doc.<root>` is tested, the motivating schema/empty optional file path has end-to-end CLI coverage, and `absolute`/`relative` have been removed from remote egress discovery.

This feature is **not ready for production** until frontmatter remote URL handling matches Decision B and the contradictory tests/docs are corrected.
