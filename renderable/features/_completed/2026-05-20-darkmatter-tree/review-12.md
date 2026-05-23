---
ready: false
agent: codex
model: ""
---

# Review 12

## Findings

### High: parity helpers discard fold diagnostics, so the suite cannot identify fold-vs-render mismatches

`darkmatter/lib/tests/render_tree_parity.rs:278` and `darkmatter/lib/tests/render_tree_parity.rs:288` intentionally bind fold diagnostics to `_diags` and return only the `Document`. Every downstream helper (`tree_html`, `tree_terminal`, `tree_html_spanned`, `tree_terminal_spanned`) therefore renders a document even if the fold emitted `Unsupported`, `Lossy`, or `Structural` diagnostics.

This violates the diagnostic-model requirement that fold diagnostics and render diagnostics stay separately visible, and it weakens DMTR-5's requirement that parity failures be specific enough to identify fold vs renderer mismatch. A future change could introduce a fold-time lossy conversion while preserving the visible tokens asserted by the parity tests; the parity suite would still pass.

Verification level: Level 1 exists for output tokens, but diagnostics are not verified at any level in the parity drivers.

Recommendation: make the parity fold helpers return `(Document, Vec<Diagnostic>)` or a small local pipeline result, assert expected diagnostic emptiness for equivalent fixtures, and assert expected diagnostics explicitly for accepted divergence fixtures.

### Medium: file-backed Markdown sources are downgraded to virtual sources

`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:219` documents that a file-backed Darkmatter document maps to `SourceDescriptor::File`, but `derive_source` always returns `SourceDescriptor::Virtual` at `entrypoints.rs:232`, even for `ComposeSource::File`.

That loses source kind in every `SourceLocation` emitted through `to_render_document`, which is exactly the path diagnostics and downstream source-aware tools will consume. The render tree already has `SourceDescriptor::File { path }`, so this looks like an implementation miss rather than a model limitation.

Verification level: Level 1 tests cover source byte ranges, but I did not find a test that pins the source descriptor variant for file-backed Markdown.

Recommendation: return `SourceDescriptor::File { path: path.to_path_buf() }` for `ComposeSource::File`, keep URL/unknown as `Virtual`, and add an entry-point unit test that resolves the registered source descriptor.

### Medium: benchmark baselines are marked stale for corrected production-shaped paths

DMTR-6 requires benchmark commands and baseline numbers before migration tuning/cutover decisions. `baselines.md` includes numbers, but it also says key groups need re-capture after implementation fixes:

- `renderable/features/2026-05-20-darkmatter-tree/baselines.md:58` says `terminal_no_color` numbers should be re-captured because previous numbers measured TrueColor tree work.
- `baselines.md:72` says `large_code_block` terminal tree numbers must be re-captured because the old baseline measured the plain-fence fallback instead of the wired `TerminalCodeRenderer`.
- `baselines.md:77` says the HTML code-block tree numbers must also be re-captured after browser code-renderer wiring.

The later tables do not clearly state that those stale measurements were replaced for the corrected code-renderer and no-color paths. As written, the feature's benchmark evidence is internally contradictory, so it should not be considered production-ready for any target whose performance depends on those corrected paths.

Verification level: benchmark harness exists, but baseline evidence for the corrected paths is not clearly current.

Recommendation: re-run `cargo bench -p darkmatter --bench migration_parity` after the latest code-renderer/color-depth wiring, update the affected tables, and remove or rewrite the stale-baseline warnings.

## Verification-Level Summary

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Mark/dim fold shape and source ranges | Level 1 unit tests in `fold.rs` / `span.rs` | Appropriate for tree shape and byte-range policy. |
| Mark/dim terminal styling | Level 2 WezTerm tests in `level2_render_tree_terminal.rs` | Appropriate when run with WezTerm available; default skip means CI must set `DARKMATTER_LEVEL2_REQUIRED=1` to enforce it. |
| HR attribute terminal styling | Level 2 WezTerm test | Appropriate when enforced. |
| Raw HTML safe HTML default | Level 1 entry-point tests | Appropriate; this is rendered string policy, not terminal encoder behavior. |
| Parser-option divergences | Level 1 structural/output tests | Appropriate for parser/fold behavior. |
| Fold/render diagnostic separation in parity | Not verified | Gap; see high finding. |

## Notes

The requested `root` skill is not present in the local skill catalog for this session, so I used the repo-level instructions and the required `renderable` skill instead.

I attempted `cargo test -p darkmatter --test render_tree_parity --color=never`, but the workspace was still compiling dependencies after about a minute. Because this is a non-interactive session with explicit no-hang constraints, I stopped relying on that run and completed the review from static inspection.
