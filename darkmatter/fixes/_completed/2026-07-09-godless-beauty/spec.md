---
status: ready for planning and implementation
reviewed: true
review_iterations: 4
created: 2026-07-09
area: darkmatter
packages:
  - darkmatter
  - dmls
  - claudine
inputs:
  - ../../reviews/2026-06-14-god-files/review.md
---

# Godless Beauty — Reducing God-Files and Improving Modularity

**Date:** 2026-07-09
**Area:** primarily `darkmatter/lib`; Improvement 4 intentionally migrates workspace consumers in
`darkmatter/dmls` and `claudine` because it changes the expression-catalog authority.
**Origin:** derived from the 2026-07-01 more-modular brainstorm prompt, which this specification
supersedes.

## Problem

A tree-hugger sweep flags dozens of files in Darkmatter as god-file candidates. Exact line counts
and source locations are intentionally omitted from the requirements because active work changes
them quickly; the named modules and their responsibilities are the stable scope. The worst
offenders mix many unrelated top-level symbols, carry high import coupling, and hide three
distinct diseases that need three distinct cures:

1. **Dead weight** — entire modules that are no longer compiled.
2. **Copy-paste siblings** — parallel files that grew from one template and have already drifted
   behaviorally.
3. **Inline test ballast + missing seams** — files that are ~50–65% inline `#[cfg(test)]` code
   sitting on top of production code whose natural submodule boundaries already exist but were
   never cut.

The six improvements below are independently shippable. The purpose is not to satisfy a line-count
threshold. It is to leave one obvious authority per behavior, make production seams visible, and
reduce the number of unrelated reasons a file changes.

## Goals and non-goals

This work MUST:

- preserve public behavior and public import paths except for the explicitly documented expression
  catalog migration in Improvement 4;
- preserve compose pass order, rendering bytes/snapshots, error/status-block text, cache and shell
  security behavior, demand-driven context capture, and test-level gating;
- keep shared parsing helpers crate-private and return neutral values so `LinkError` and
  `ImageRefError` remain owned by their public types;
- move tests to the module that owns the behavior, without weakening assertions or silently
  changing ignored/serial/Level-2 gates; and
- update comments and documentation whose architectural claims become stale.

This work MUST NOT introduce a generic pass trait, a plugin framework, new user-facing behavior
beyond the UTF-8 parser fix in Improvement 2 and the GPU-only capture fix in Improvement 6, or
opportunistic cleanup in moved code.

For every mechanical relocation, reviewers should compare pre/post test inventories (test names
and counts) in addition to relying on compilation. A move that accidentally drops a test is not
behavior-preserving merely because the remaining suite passes.

---

## Improvement 1 — Delete the dead `markdown/transform/` module

**What:** Remove `lib/src/markdown/transform/` (`mod.rs` and `types.rs`) entirely.

**Evidence it is dead, not merely deprecated:**

- No `mod transform;` declaration exists anywhere in the crate — `markdown/mod.rs` lists every
  sibling module except this one. Zero external references to `TransformReport`,
  `TransformOptions`, `.transform()`, `.transform_with()`.
- It cannot compile if wired in: `transform/mod.rs` declares `mod conditions; mod state;`
  but those files no longer exist in the directory (they migrated to `compose/` during the
  transform→compose rename, commits `e295a4eda` → `aa63ca0f5`).
- Its `render_markdown_transclusion` / `render_code_transclusion` are pre-cache snapshots of the
  live, cache-aware versions in `compose/transclusion/engine.rs`.
- Its `test_stage2_*` tests are stale copies of tests that already run in `compose/tests.rs`
  (for example, `test_stage2_mutual_exclusion_conditions` is identical modulo the API rename).

**Why it matters beyond line count:** the file is a perception trap. It reads as a live serial
pipeline duplicating the compose engine, inflates the god-file count, and any future grep for
transclusion logic finds two "authorities."

**Scope:** pure deletion plus a repository-wide source and documentation sweep for the removed
module and API, including generic references such as `markdown::transform` (not only
`TransformOptions` / `TransformContext`). Where prose uses “transform” in its ordinary English
sense, leave it alone; where it describes the old pipeline, replace it with the live compose
contract. The current exception in `docs/topics/darkmatter-expressions.md` is one known stale
reference.

**Acceptance:** no compiled module or documentation refers to the deleted API, `cargo check -p
darkmatter` remains green, and the package-area `just test` passes. This step must not alter the
live compose implementation or copy tests out of the dead module.

---

## Improvement 2 — Extract shared HTML/Markdown reference parsing from `render/link.rs` and `render/image_ref.rs`

**What:** `Link` and `ImageRef` are sibling types that grew from one template. Between them they
carry ~900 lines of duplicated helpers, including byte-for-byte identical copies of:
`strip_ansi_sequences`, `base64_encode`/`base64_decode`/`base64_value`, `html_escape`/
`html_unescape`, `normalize_optional`, `normalize_data_key`, and the markdown URL codecs — plus
near-identical copies of `parse_html_attributes`, `find_closing_bracket`, `find_closing_paren`,
`extract_url`, the structured-props tokenizer, and the `MetadataPolicy` enum + env dispatch.

**This is already a correctness problem, not just hygiene:** the copies have drifted. Both title
parsers mix byte offsets with Unicode character access and can corrupt or skip multibyte title
text; the image copy additionally casts UTF-8 bytes directly to `char`. The shared replacement
must scan with `char_indices` or a `Chars` iterator and treat escapes without ever interpreting a
byte as a character. One side also lowercases HTML attribute names with Unicode
`to_lowercase()`, while the other uses `to_ascii_lowercase()`. HTML attribute names are
ASCII-case-insensitive, so `to_ascii_lowercase()` is the canonical behavior for both.

**Target structure:**

```
render/
├── mod.rs
├── reference_parse.rs # HTML attributes, bracket/paren scanners, URL/title codecs,
│                      # structured-props tokenizer and shared normalization helpers
├── metadata_codec.rs  # base64 and generic serde encode/decode, MetadataPolicy
├── link.rs            # slim: LinkType/LinkTarget, popover, per-type drivers
└── image_ref.rs       # slim: image enums, srcset/width hints, per-type drivers
```

Both new modules are `pub(crate)`. Shared scanners return byte offsets only when those offsets are
on UTF-8 character boundaries, or return neutral parsed values. Each caller keeps building its own
`LinkError` / `ImageRefError` with the same caret, message, and `AsBlockError` status-block shape.
The shared structured-property tokenizer accepts a per-type apply closure, so link-only
`prompt`/`target` behavior and image-only `srcset`/width behavior remain in their owning modules.
`metadata_policy` accepts the caller's environment-variable name; `LINK_METADATA` and
`IMAGE_REF_METADATA` remain separate public behavior.

The extraction is behavior-preserving except for the intended UTF-8 title fix. Pin the boundary
with table-driven parity tests that run the same Markdown and HTML cases through `Link` and
`ImageRef`, including non-ASCII title/alt/display text, escaped quotes and backslashes, nested
parentheses, malformed/unclosed input, uppercase ASCII HTML attributes, metadata
inline/strip/lossless modes, and encode/decode round trips. Existing snapshots and status-block
tests must remain byte-identical; add focused regression assertions for the newly corrected
multibyte cases.

**Payoff:** substantial duplicated code is removed and both files become narrower; there is one
correct implementation of each scanner instead of two silently diverging ones. Do **not** merge
the genuinely different halves (popover/`target` handling on links; `srcset`, referrer-policy
suggestion, width hints on images).

---

## Improvement 3 — Move inline test ballast out of god-files and break up `compose/tests.rs`

**What:** Much of the flagged tonnage is not production code at all:

| File | Inline test share |
|------|------------------|
| `layout/page.rs` | ~65% |
| `markdown/cleanup.rs` | ~52% |
| `compose/frontmatter_shell_expansion.rs` | ~52% |
| `compose/expression/functions.rs` | ~49% |

Adopt the convention the codebase already uses in places (`terminal/tests.rs`,
`style/coverage_tests.rs`): a large `#[cfg(test)] mod tests` moves to a sibling file declared as
`#[cfg(test)] mod tests;` (or an explicitly `#[path]`-mapped `tests/` subdirectory). Descendant
test modules retain access to ancestor-private items; shared fixtures inside a test tree use the
narrowest `pub(super)` visibility that works.

Do not move a test twice. The inline tests in `cleanup.rs` and `expression/functions.rs` move
directly into their final domain modules as part of Improvements 5 and 4. This improvement owns
the standalone `compose/tests.rs` and Level-2 split, plus sibling extraction for `layout/page.rs`
and `frontmatter_shell_expansion.rs`, whose production splits are explicitly deferred.

Apply the same medicine to the two dedicated test god-files, splitting **by pipeline stage /
responsibility** rather than one flat file:

- `compose/tests.rs` →
  `compose/tests/{frontmatter.rs, schema.rs, shell.rs, transclusion.rs, caching.rs,
  preflight.rs, rendering.rs}` + a small shared `fixtures.rs` for the temp-repo builders that
  are currently re-declared per test.
- `lib/tests/level2_render_tree_terminal.rs` → keep this top-level integration target as a thin
  module root and place support plus surface modules under
  `lib/tests/level2_render_tree_terminal/`. Use explicit `#[path]` declarations if needed so the
  layout is unambiguous. Split by code panel, images, file links, layout policy/page geometry,
  public entry points, and basic spans. There is no existing shared Darkmatter L2 helper module:
  create a target-local `support/` tree for harness, render-probe, ANSI, and image helpers rather
  than claiming one exists or creating a workspace-wide abstraction.

**Why this is more than cosmetics:** god-file *symptoms* (slow navigation, wide-blast merge
conflicts, 100+ imports) come disproportionately from these test blocks, and shared fixtures
extracted once become reusable — today each mega-test rebuilds its own fixture inline, which is
why the largest blocks in `compose/tests.rs` are 50–90 sloc each.

**Guardrail:** test-relocation changes contain no production behavior changes and no assertion
edits. A diff under `src/` is limited to `#[cfg(test)]` module declarations and removal of the
moved test body. Preserve test names where feasible, `#[serial]` groups, environment guards,
temporary-directory lifetimes, render-probe executable discovery, and
`BISCUIT_TEST_LEVEL_REQUIRED` semantics. Before and after each move, record the relevant nextest
test inventory and require equal test counts; then run the moved target, not only the broad suite.

---

## Improvement 4 — Split `expression/functions.rs` by domain and single-source function registrations

**What (a) — the split:** dispatch is already data-driven, so the free handlers partition along
existing domains:

```
compose/expression/functions/
├── mod.rs           # registration model, registry aggregation, dispatch/accessors
├── args.rs          # arity and value-conversion helpers
├── predicates.rs    # type/value predicates and numeric helpers
├── collections.rs   # first/last/contains/length and list renderings
├── strings.rs       # case conversion, replacement, ensure_* helpers
├── dates.rs         # date formatting/validation and ISO parsers
├── paths.rs         # path projection, basename/dirname/ext/join, file indexes
├── skills.rs        # skill discovery functions
├── markdown_docs.rs # link/load/frontmatter/title/body/schema functions
└── terminal.rs      # terminal rendering function
```

Move each test directly with the handler or registry invariant it protects. Avoid a broad
`use super::*`; each domain should make its real dependencies visible. Shared helpers remain
private to `functions` unless an existing public path requires otherwise.

**What (b) — the single source:** `catalog.rs` currently re-encodes each callable signature as
typed `ParamType`/`ReturnType` descriptors while `PURE_FUNCTIONS`, `FS_FUNCTIONS`, and the lazy
operator lists independently encode runtime names and signatures. Exact-set tests detect drift
after it has been authored; they do not prevent it.

Define one crate-private registration shape:

```rust
struct FunctionRegistration {
    canonical: &'static str,
    aliases: &'static [&'static str],
    descriptors: &'static [ExpressionFunctionDescriptor],
    handler: FunctionHandler,
}

enum FunctionHandler {
    Pure(PureFn),
    Context(FsFn),
    Lazy,
}
```

Each domain module owns a const slice of registrations next to its handlers. Overloads are
multiple descriptors in one registration, not duplicate registrations. Runtime dispatch,
canonical-name enumeration, callable-signature enumeration, DMLS lookup, and the human-facing
catalog all project from the aggregated registrations. `ExpressionFunctionDescriptor` remains
the public, handler-free description type in `catalog.rs`; internal function pointers are never
exposed as documentation API.

**Reader's note — intentional public API migration:** the first draft said “the catalog is a
projection” without accounting for the existing public
`EXPRESSION_FUNCTION_DESCRIPTORS: &'static [ExpressionFunctionDescriptor]`. Domain-owned slices
cannot be flattened into that const slice without re-listing them centrally, which would recreate
the second authority. This reviewed design makes the already-public
`expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor]` accessor the sole
catalog API, backed by one `LazyLock<Vec<_>>`, and removes the raw constant plus the public
`PURE_FUNCTIONS` / `FS_FUNCTIONS` registration tables. This is an intentional source-breaking
change, acceptable before an established user base, and it must be completed atomically across
the workspace.

The migration scope includes:

- Darkmatter's expression evaluator, catalog helpers, generated expression documentation, and
  `crate::catalog` integration;
- DMLS hover/completion/lookups that currently import the raw descriptor constant;
- Claudine library, CLI, tests, and expression-engine/drift documentation that currently name or
  iterate the raw constants; and
- Darkmatter architecture/topic docs and the local `darkmatter` skill, which must teach the
  accessor and domain registration model after implementation.

Do not leave a compatibility constant with a changed `LazyLock<Vec<_>>` type under the old name;
that looks source-compatible but changes coercion requirements unpredictably. A direct compiler
error plus one documented accessor is the clearer pre-release migration.

**Registry invariants:** tests MUST prove canonical names are unique, aliases do not collide with
another canonical name or alias, every registration has at least one descriptor, every descriptor
signature's leading name equals its registration's canonical name, descriptor signatures are
unique, overloads share one handler, and each handler kind dispatches through its intended path.
Retain behavioral tests for aliases, arity, local-only/remote path rules, injectable date behavior,
and lazy `and`/`or` evaluation. Regenerate and diff the expression-function documentation as part
of this improvement.

**Payoff:** adding a function or overload changes one domain module; the descriptor/dispatch drift
class is eliminated by construction rather than caught by parity tests.

---

## Improvement 5 — Turn `markdown/cleanup.rs` into a `cleanup/` pass pipeline

**What:** the file is already architected as two sequential phases of independent passes —
Phase A event-stream transforms (`Vec<Event>` → `Vec<Event>`: emphasis preservation, empty-fence
language tagging, table alignment) and Phase B string post-passes over one `&mut String`
(placeholder restoration, list spacing/markers/indentation, blockquote fixes, bracket
unescaping), orchestrated by `cleanup_content_internal`. The only cross-pass state is
the emphasis placeholders and extracted list markers. Cut along those seams:

```
markdown/cleanup/
├── mod.rs        # public API + the two-phase orchestrator (pass order stays explicit here)
├── emphasis.rs   # preserve_original_emphasis + restore/unescape passes (the one coupled pair)
├── tables.rs     # align_tables_in_stream, CellWidthCalculator, process_single_table
├── lists.rs      # markers extract/restore, normalize_list_spacing, fix_list_indentation
├── blockquote.rs # fix_blockquote_formatting
├── brackets.rs   # unescape_brackets
└── reflow.rs     # strip_incidental_newlines, reflow_to_width, wrap_text, HTML-block state
                  # (LineMetadata / NewlineBoundary / ReflowPrefix — a self-contained engine)
```

**Why:** 72 top-level symbols and depth-7 nesting in one file make the pass ordering — the one
thing that actually matters here — invisible. After the split, `mod.rs` reads as the pipeline
manifest, and the reflow engine (which `md clean --fixed-width` and DMLS formatting both route
through) becomes an owned, findable unit instead of functions interleaved among table passes.
No trait abstraction is warranted (Rule 2): the passes have two different shapes and a fixed
order; plain functions in submodules are enough.

`markdown::cleanup` and every re-exported cleanup entry point remain source-compatible. Keep the
complete pass order in one plainly readable function in `mod.rs`; submodules must not call the
next pass implicitly. Move characterization tests with each pass, but retain end-to-end tests in
`cleanup/tests/` that pin ordering-dependent behavior (emphasis placeholders through reflow,
list markers inside blockquotes, tables adjacent to lists, fenced/indented code protection, HTML
blocks, CRLF input, Unicode display width, and trailing-newline modes). DMLS formatting must stay
byte-equivalent to `md clean`; its existing parity test is an acceptance gate for this split.

---

## Improvement 6 — Split `compose/context/capture.rs` along its own `ContextGroup` seam

**What:** the per-group split is already latent in the code: `ContextGroup` enumerates `DateTime,
Repo, FileChanges, Languages, Documents, Os, Hardware, Gpu, Agent`; `for_key` maps variables to
groups for demand-driven capture; and most groups already have a dedicated `populate_*` function.
What keeps the file a god-file is the large `ContextCapture::new` probe orchestrator sitting above
unrelated populate bodies.

```
compose/context/capture/
├── mod.rs       # public(crate) facade and capture/population sequencing
├── groups.rs    # ContextGroup, all(), for_key(), demand-scan invariants
├── snapshot.rs  # ContextCapture::new and concurrent sniff probe orchestration
├── datetime.rs  # pure values + owned key list
├── repo.rs      # repo/monorepo/dependency values + owned key list
├── changes.rs   # file/package changes + owned key list
├── languages.rs
├── docs.rs      # docs/skills values + owned key list
├── host.rs      # OS/hardware/GPU values + owned key lists
└── agent.rs     # pure environment reads + owned key list
```

**Why:** each group has a different external dependency (sniff git, sniff filesystem, env, GPU
probe thread) and a different failure/latency profile; per-group files make that ownership
explicit. Each population module owns a `KEYS` slice (or an equivalent `owns_key` function), and
`ContextGroup::for_key` delegates to those definitions instead of repeating one giant match in
`groups.rs`. Adding a generated context value then changes the authored base schema/catalog source
and its capture domain, not three distant Rust lists.

`sniff` remains the authority for repository, filesystem, OS, and hardware discovery; this split
must not replace it with direct platform commands or platform-specific filesystem assumptions.
Preserve the existing dependency-aware concurrency: repo discovery precedes documentation
detection, independent probes overlap, diagnostics and timing labels remain stable, and no probe
runs unless its group is requested.

**Intended correctness fix:** today `for_key("gpu")` requests only `ContextGroup::Gpu`, and
`ContextCapture::new` performs that probe, but the facade inserts `gpu` only as part of
`populate_hardware`, which runs only for `ContextGroup::Hardware`. A document that references only
`ctx.gpu` therefore pays for the probe and still receives no value. Extract `populate_gpu` from
`populate_hardware` and invoke it for `ContextGroup::Gpu`; requesting `ctx.gpu` must not also force
the CPU/memory probe. This correction is part of Improvement 6 and must have a focused
GPU-only-group regression test that injects or constructs capture data without depending on the
host having a GPU.

Tests must also prove that each generated context descriptor maps to exactly one capture group
(with backward-compatible aliases covered by an explicit allowlist), unknown keys map to none,
content with no relevant `ctx.*` reference performs only the documented local datetime work, and
macOS/Windows/Linux behavior continues to flow through `sniff`.

---

## Follow-on candidates (deliberately out of scope here)

Worth doing, but lower leverage or better bundled with already-planned work:

- **`layout/page.rs`** — after Improvement 3 strips the 65% test share, split the render surface
  into `page/terminal.rs` (`render`, `apply_row_decoration`) and `page/browser.rs`
  (`render_to_browser`, `wrap_browser_html`) impl-block moves; the builder stays in `mod.rs`.
- **`reference/graph.rs::build_node` (430 sloc)** — extract per-source handlers
  (`directives.rs`, `toc_linking.rs`, `file_links.rs`, `frontmatter_refs.rs`), keeping the
  recursion skeleton in `graph.rs`.
- **`transclusion/engine.rs`** — `resolve_prepared_transclusion` (332 sloc) and
  `prepare_block_transclusions` (248 sloc) branch per directive kind; split along
  file/url/code/file-links once Improvement 1 has removed the shadow copy.
- **`frontmatter_shell_expansion.rs`** — the parse layer (lines ~133–820, spanned AST + `$()`
  grammar) can move next to its consumers in `shell_expansion/` as `frontmatter_parser.rs`;
  hold until the stalled real-errors Phase-2 cascade lands to avoid churning the same lines.

## Sequencing

1. Improvement 1: delete the dead module and repair stale architecture documentation.
2. Improvement 2: extract shared reference parsing and land the isolated UTF-8 fix with its own
   regression tests.
3. Improvement 3: split the standalone test targets and extract only the inline suites whose
   production modules are not otherwise split here.
4. Improvements 5 and 6: split cleanup and context capture independently, moving their tests
   directly to final homes.
5. Improvement 4: perform the cross-workspace registry/catalog migration last, after the
   lower-risk file moves have reduced merge pressure.

Improvement 2's bug fix must not be hidden in a purely mechanical commit. Likewise, Improvement
4's public migration must be atomic across Darkmatter, DMLS, and Claudine; do not leave the branch
in a state with two catalog authorities.

## Verification and definition of done

Every improvement is complete only when its focused tests pass and the package-area recipes are
green. Use nextest, never `cargo test`, and do not run write-mode `cargo fmt` as part of this work.

- For test-only moves, compare `cargo nextest list -p darkmatter` (and the named integration
  target where applicable) before and after, then run the moved target.
- After every improvement, run `just test` and `just lint` from `darkmatter/`.
- After Improvements 2 and 3, and at final closeout, run `just test-l2`; these steps touch real
  terminal rendering and the Level-2 target layout.
- After Improvement 4, run checks/tests for `darkmatter`, `dmls`, `claudine`, and `claudine-cli`,
  then regenerate the expression documentation and require a reviewed diff.
- At final closeout, run `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine -p
  claudine-cli`, the full package-area `just test`, `just test-l2`, and `just lint`, and a
  repository-wide search proving removed module/constant names survive only in historical
  specifications or migration notes.

The final diff must show no public rendering, compose, cleanup, context, or expression behavior
change beyond the specified UTF-8 correction, GPU-only capture correction, and documented
source-level expression-catalog API migration. No new production file should merely replace one
god-file with another: each domain module must have one coherent reason to change, while
orchestration remains visible in its facade.
