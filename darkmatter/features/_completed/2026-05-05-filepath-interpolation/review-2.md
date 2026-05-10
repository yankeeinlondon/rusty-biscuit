---
ready: false
agent: codex
model: ""
---

# Review 2: Filepath Interpolation

## Findings

### High: Link resolve/normalization still write debug traces to stdout

The implementation contains unconditional `println!` calls in production paths:

- `darkmatter/lib/src/markdown/compose/link_resolve.rs:52`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:54`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:75`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:129`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:139`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:148`
- `darkmatter/lib/src/markdown/compose/link_resolve.rs:152`
- `darkmatter/lib/src/markdown/compose/mod.rs:151`
- `darkmatter/lib/src/markdown/compose/mod.rs:166`
- `darkmatter/lib/src/markdown/compose/mod.rs:176`
- `darkmatter/lib/src/markdown/compose/mod.rs:185`

These are not test-only diagnostics; they run while composing real documents. This corrupts CLI stdout for `md compose`, where stdout is the composed markdown payload. The focused test runs confirmed this: `cargo test -p darkmatter link_resolve -- --nocapture` and `cargo test -p darkmatter --test link_interpolation_integration -- --nocapture` both printed resolver traces such as `Total records extracted`, `Record kind`, and `find_target_range` during successful composition.

**Required fix:** remove these calls or convert them to `tracing::trace!`/`debug!`. Add a CLI Level-1 test with `assert_cmd` that composes a file containing a relative link and asserts stdout contains only the composed markdown, not diagnostics.

**Test-rigor classification:** user-observable CLI output currently has no feature-specific CLI stdout assertion. Existing unit/integration tests are Level 1 but do not verify stdout cleanliness.

### High: HTML attributes with spaces around `=` are skipped entirely

Both link operations return early unless the document contains exact substrings `](`, `href=`, or `src=`:

- `darkmatter/lib/src/markdown/compose/link_resolve.rs:31`
- `darkmatter/lib/src/markdown/compose/link_normalization.rs:41`

But the HTML extractor explicitly supports spaced attributes via patterns like `href = "` and `src = "` in `darkmatter/lib/src/markdown/reference/html.rs:517`. A document containing only `<a href = "./page.md">` or `<img src = "./image.png">` will be skipped before extraction, so the spec's HTML tag/attribute support is incomplete.

**Required fix:** remove the brittle pre-scan or make it match the extractor's accepted syntax. Add tests for `<a href = "...">`, `<img src = "...">`, `<video src = "...">`, and `<link href = "...">` through both Link Resolve and Link Normalization.

**Test-rigor classification:** Level 1 unit tests are sufficient for this path rewriting behavior, but the current Level 1 tests only cover unspaced `href=`/`src=`.

### Medium: ENV-var warnings are emitted twice by the CLI

`normalize_links` now both records a `ComposeWarning` and directly writes a rendered `Status` to stderr:

- `darkmatter/lib/src/markdown/compose/link_normalization.rs:185`
- `darkmatter/lib/src/markdown/compose/link_normalization.rs:189`

The CLI then renders every `report.warnings` entry to stderr again:

- `darkmatter/cli/src/commands.rs:759`

That means an ENV path substitution in `md compose` can produce duplicate user-facing warnings. The prior review's request to make the warning programmatically visible was addressed, but the direct library-side `eprintln!` now conflicts with the CLI's existing warning emission path.

**Suggested fix:** prefer one emission path. The cleaner design is: library records `ComposeWarning`; CLI renders warnings to stderr using `Status`. Add a CLI Level-1 test for an ENV-var substitution that asserts exactly one warning appears on stderr.

**Test-rigor classification:** warning visibility and duplication are CLI-observable; Level 1 `assert_cmd` coverage is appropriate and currently missing.

### Medium: No CLI coverage for the default end-to-end feature

The feature is mostly tested through library unit tests and one library integration test in `darkmatter/lib/tests/link_interpolation_integration.rs`. That verifies the compose API, but it does not verify the binary's observable contract: stdout is the markdown result, stderr contains warnings, and file input gets source-file context from CLI path resolution.

Given this feature changes default composition output, add CLI Level-1 tests for:

- same-repo relative link survives final output as a portable relative path
- transcluded child link is resolved against the child file and normalized against the root file
- ENV-var substitution writes one warning to stderr and no warning text to stdout
- HTML spaced attributes are handled

No Level 2 or Level 3 terminal tests are required for path rewriting itself. If the exact `Status` SGR styling/glyph layout becomes a user-facing requirement, then warning rendering should also get Level 2 terminal capture coverage.

### Low: `operations/link-resolve.md` now links to a deleted sibling doc

`darkmatter/docs/operations/link-normalization.md` was deleted, and `darkmatter/docs/inline/link-normalization.md` was populated. However, `darkmatter/docs/operations/link-resolve.md` still links to `./link-normalization.md`, which no longer exists in that directory.

**Suggested fix:** update the link to the new inline doc or move `link-resolve.md` into the same docs area.

## Verification Matrix

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Markdown links/images resolve to absolute paths during Inline-Pre | Level 1 unit tests in `link_resolve.rs` | Adequate for library behavior |
| HTML `<a>`, `<img>`, media, iframe, link/script imports resolve | Level 1 unit tests | Incomplete: spaced attributes skipped |
| Transcluded child links resolve before insertion and normalize at root | Level 1 library integration test | Adequate for library behavior, missing CLI assertion |
| Same-repo absolute links normalize to relative paths | Level 1 unit/integration tests | Adequate for library behavior |
| Home-dir paths normalize to `~/` | Level 1 unit/integration tests | Adequate, though tests use real `$HOME` |
| Whitelisted ENV paths normalize to `${VAR}` and warn | Level 1 unit tests for content/report, no CLI stderr count test | Incomplete for CLI behavior |
| CLI stdout remains composed markdown only | No feature-specific assertion; tests with `--nocapture` expose debug stdout | Gap; blocks production readiness |

## Tests Run

- `cargo test -p darkmatter link_resolve -- --nocapture` passed.
- `cargo test -p darkmatter link_normalization -- --nocapture` passed.
- `cargo test -p darkmatter --test link_interpolation_integration -- --nocapture` passed.

These runs also exposed the stdout debug output described above.

## Production Readiness

Not ready. The core library behavior is much closer after the first review's fixes, but unconditional stdout diagnostics will corrupt normal CLI output, and the pre-scan skips valid HTML attribute syntax that the extractor itself supports.
