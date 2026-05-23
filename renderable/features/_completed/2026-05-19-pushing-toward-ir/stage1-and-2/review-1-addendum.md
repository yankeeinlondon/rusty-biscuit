# Review 1 Addendum — Stage 3 Reconciliation

`review-1.md` was useful as a post-Stage-2 production-readiness sweep, but it
was written before the Stage 3 spec existed. This addendum classifies its
findings against the current Stage 3 direction.

## Should Inform Stage 3

### 1. Make layout-matrix coverage explicitly complete for default cases

Review 1 calls out missing matrix coverage for `OrderedList`, `TextBlock`, and
`Todo`. Stage 3 already has S3-6, but it frames the work as a harness rename
and escape-hatch audit. Tighten S3-6 so the default-case matrix includes every
tree-backed component whose terminal `render()` path is expected to match
direct tree rendering:

- `BlockQuote` default border only
- `Compose`
- `OrderedList`
- `Progress`
- `Section`
- `StatusBlock` default border only
- `Table` default path only
- `TextBlock`
- `Todo`
- `TwoColumn` non-image/default path only
- `UnorderedList`
- `FileSystem` only if S3-1c chooses to flip it to tree

Escape-hatch cases should remain outside the default matrix per the current
S3-6 policy.

### 2. Add `NO_COLOR` to Stage 3 cleanup if it is still failing

The BlockQuote, OrderedList, and Progress reviews all point at the same
tree-renderer risk: commands that moved to the tree path may ignore
`NO_COLOR`. This is not component-specific. Stage 3 should add a small
cross-cutting verification item, probably under S3-7:

- Confirm `Terminal` / CLI detection downgrades color depth when `NO_COLOR=1`.
- Add one CLI integration test through a tree-rendered command such as
  `bt quote` or `bt progress`.
- Fix at the shared detection/rendering layer if the test fails.

If the current code already honors `NO_COLOR`, record that in
`lessons-learned.md` and do not add component-level patches.

### 3. Fold error-fallback policy into the migration checklist

Several reviews flag silent `unwrap_or_default()` in Markdown paths and in-band
browser fallback text such as `[render-tree error: ...]`. Stage 3 does not need
to audit every component's fallback behavior before finishing the structural
projection work, but S3-5 should document the target policy for future
components:

- Infallible trait methods should log `tracing::error!` with component,
  target, and error details.
- Markdown / MarkdownPlus fall back to `String::new()`.
- Browser fragments fall back to an empty fragment, not visible sentinel text.
- Terminal paths may use the sanctioned bespoke fallback only where the
  component has a documented terminal-only escape hatch.

This belongs in `migrate-component-to-ir.md`; broad implementation cleanup can
follow Stage 3.

### 4. Include component documentation drift in S3-5

Review 1 specifically found stale `renderable/docs/components.md` rows for
`BlockQuote` and `StatusBlock`, and the same risk likely exists for other
components. S3-5 should say the migration checklist must include documentation
updates:

- `renderable/docs/components.md`
- per-component docs under `biscuit-terminal/docs/components/`
- CLI examples where `--md`, `--md-plus`, `--html`, or `--example` behavior
  changes

This is aligned with Stage 3's goal of publishing the onward migration recipe.

### 5. Keep FileSystem parity as the gate for the S3-1c decision

The FileSystem review's critical parity finding is already represented by
S3-1c. The useful extra detail is the acceptance shape: compare the bespoke
path to direct tree rendering across documented variants before deciding
whether `FileSystem::render` can flip. If the parity test exposes real gaps,
choose S3-1c outcome (iii) and record the missing renderer capability as the
Stage 4 criterion.

## Good After Stage 3

These findings are still valuable, but they should not block Stage 3's
structural-projection completion unless implementation work touches the same
area anyway.

### Browser and Markdown fallback cleanup

Apply the error-fallback policy across existing components:

- Replace silent Markdown `unwrap_or_default()` sites with `match` plus
  `tracing::error!`.
- Change `BrowserTreeComponent::fallback_fragment` and any direct component
  equivalents to log and return an empty fragment.
- Add focused tests for one Markdown path and the shared browser adapter.

### Level-2 real-terminal smoke tests

The suggested Level-2 tests for `Compose`, `TextBlock`, `Todo`, and
`StatusBlock` are useful regression coverage, but they are expensive relative
to Stage 3's goal. Do them as a follow-up testing pass, prioritizing:

1. `StatusBlock` border glyph/color smoke test.
2. `TextBlock` SGR style smoke test.
3. `Todo` state marker smoke test.
4. `Compose` only if CLI behavior is still important after checking whether a
   first-class `bt compose` command exists.

### `render_html_page` wrapper tests

Thin tests for `render_html_page` on `Section`, `TwoColumn`, `StatusBlock`, and
`UnorderedList` are reasonable low-risk coverage. They are not necessary for
the Stage 3 structural migration.

### FileSystem functional improvements

Keep these as a FileSystem follow-up, not Stage 3 work:

- Real `.gitignore` / ignore-rule integration.
- Permission-error placeholder nodes.
- `--links` CLI flag.
- Root path canonicalization cleanup.
- More visible warning when `render_tree()` is called before the tree is built.

If S3-1c chooses to flip `FileSystem::render` to tree, only the subset needed
for parity should move into Stage 3.

### Table uniform-alignment parity

`uniform_alignment` being ignored by the tree renderer is potentially a real
bug, but it is independent of structural projection. Handle it after Stage 3
unless S3-6 default matrix coverage catches it in a default-path row. If it is
only triggered by a non-default table option, track it as a table-specific
follow-up.

### Small component edge-case tests

The extra tests for empty quotes, Markdown escaping, underline variants, narrow
terminal widths, `Todo::from_prose`, and HTML-sensitive content are worthwhile
hardening. They should be batched into a post-Stage-3 component test sweep.

## No Longer Worth Carrying Forward

### Reintroducing `render_bespoke` for components that already retired it

Review 1 recommends adding or preserving bespoke hooks for `Compose` and some
fully migrated components so parity tests can compare historical output. Stage
3 makes the opposite call: retire unnecessary `render_bespoke()` hooks and use
the tree as canonical. Do not reintroduce bespoke hooks solely to support a
historical comparator.

### Large bespoke-vs-tree parity expansions for components whose hook will be retired

Detailed parity expansion for `OrderedList`, `Progress`, `Section`,
`TextBlock`, `Todo`, and `UnorderedList` has diminishing value if S3-4 removes
their public bespoke hook. Preserve structural tests, direct tree-render tests,
and layout-matrix default cases instead.

### Central `KNOWN_DRIFT` ledgers for retired comparators

`KNOWN_DRIFT` remains useful for retained escape hatches and the layout matrix,
but not for components whose old bespoke comparator disappears. Do not create
new drift ledgers just to memorialize output differences from a path Stage 3
intends to delete.

### Component-specific `NO_COLOR` fixes

The Review 1 per-component `NO_COLOR` suggestions should collapse into one
shared terminal detection/rendering fix. Avoid post-render stripping in
individual commands unless a command has a genuinely unique output path.

### Compose parity file with historical baseline

A `compose_parity.rs` file may still be useful for structural projection tests,
especially the S3-2 fixture table. The parts of the Review 1 recommendation
that require a restored `Compose::render_bespoke()` baseline should be dropped.

### `render_bespoke` visibility changes to `pub(crate)`

The current Stage 3 spec already corrected this: retained escape-hatch hooks
stay `#[doc(hidden)] pub` because integration tests live outside the library
crate. Do not pursue the Review 1 `pub(crate)` direction.

## Suggested Stage 3 Spec Delta

If editing `stage3-spec.md`, the useful minimal deltas are:

1. In S3-6, require default-case matrix coverage for all applicable
   tree-backed components, not just a rename/audit of the harness.
2. In S3-7, add a `NO_COLOR` verification/fix item for tree-rendered CLI
   commands if still failing.
3. In S3-5, add fallback-policy and documentation-update bullets to
   `migrate-component-to-ir.md`.
4. In S3-1c, mention that the FileSystem parity gate should use the variants
   from `FileSystem-spec.md` and record any missing tree-renderer capability
   as the Stage 4 acceptance criterion if the flip is deferred.
