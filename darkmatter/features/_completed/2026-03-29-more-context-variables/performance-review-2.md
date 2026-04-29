# Performance Review 2: Compose Context Variables

## Summary

The major performance recommendations from `performance-review.md` have been implemented in part:

- validation now tries to share compose context
- context capture is demand-driven
- hardware capture uses a lightweight summary API
- docs detection can reuse precomputed package info
- context cloning is cheaper because `ComposeContext` is `Arc`-backed

The remaining issues are mostly in the "last 20%" category: a few hot-path regressions are still present, some of the lightweight APIs are still wrappers over broad scans, and the new performance behavior is not strongly protected by tests.

I ran `cd darkmatter && just test` and the full Darkmatter library + CLI suite passed.

I also did two spot measurements on `2026-03-29` with `target/debug/md`:

| Command | Wall time |
|---|---:|
| `printf '# test\n' \| target/debug/md compose -` | `0.96s` |
| `target/debug/md compose /tmp/dm-small.md` | `2.65s` |

That is a large improvement over the original review, but tiny file compose still carries meaningful fixed overhead.

## Findings

### 1. The CLI compose path still performs redundant eager context captures

Severity: High

The shared-context refactor is only partial.

`darkmatter/cli/src/commands.rs` now captures one `shared_context`, but then it still does:

- `ComposeOptions::new().with_context(shared_context.clone())` for validation
- `ComposeOptions::new().with_context(shared_context)` for compose

`ComposeOptions::new()` eagerly calls `ComposeContext::capture()` in `darkmatter/lib/src/markdown/compose/types.rs`, so both callsites still pay for an extra runtime capture before the shared context is swapped in.

Relevant code:

- `darkmatter/cli/src/commands.rs:357-359`
- `darkmatter/cli/src/commands.rs:395`
- `darkmatter/lib/src/markdown/compose/types.rs:418-449`

This means the "capture once and reuse it" rule is still violated on the CLI file-input path.

### 2. Demand-driven capture only scans the root document, which breaks transcluded child documents

Severity: High

The new demand-driven capture is based only on the root document text:

- `ComposeContext::capture_for_content(base_dir, md.content())`

That works only if every needed `ctx.*` key appears in the root content. It does not account for transcluded children.

Observed behavior:

- direct file containing `{{ ctx.os }}` renders `macOS`
- root file containing only `::file child.md` with child content `{{ ctx.os }}` renders blank output

The transcluded case also emits a misleading warning:

- `warning[context]: Document defines ctx keys that collide with runtime context; runtime values take precedence`

Relevant code:

- `darkmatter/cli/src/commands.rs:337-345`
- `darkmatter/lib/src/markdown/compose/mod.rs:1276-1303`
- `darkmatter/lib/src/markdown/compose/mod.rs:1466-1474`
- `darkmatter/lib/src/markdown/compose/state.rs:344-357`

This is the most important correctness gap introduced by the optimization work.

### 3. The demand-driven scanner is still heuristic and overcaptures on unknown keys

Severity: Medium

The original recommendation was to determine required groups from parsed interpolation expressions and `when=` conditions. The current implementation uses a raw substring scan:

- it scans any `ctx.` occurrence in the raw document
- it returns `ContextGroup::all()` for any unknown key

Relevant code:

- `darkmatter/lib/src/markdown/compose/context/capture.rs:111-152`

Consequences:

- a typo like `{{ ctx.oss }}` forces full capture
- a user-defined nested `ctx` key forces full capture
- `ctx.` text inside prose or code fences can trigger unnecessary work

This is a reasonable first pass, but it is still only a partial implementation of Priority 1.

### 4. The docs path is still a full-repo markdown scan, not a compose-focused summary path

Severity: Medium

`detect_docs_with_packages()` removes the redundant `detect_repo()` call, but it still:

- walks the entire repo for `*.md`
- reads every markdown file fully
- extracts title, prompt, model, timestamps, hashes, and other metadata

Relevant code:

- `darkmatter/lib/src/markdown/compose/context/capture.rs:245-253`
- `sniff/lib/src/filesystem/docs.rs:144-218`

That means Priority 2 / Priority 4 were only partially implemented. The compose path still pays for metadata it does not use.

### 5. Git and repo capture still rely on broad inventory APIs

Severity: Medium

The compose capture path still calls:

- `git::detect_git(base_dir, false, 10)`
- `repo::detect_repo(root)`

Relevant code:

- `darkmatter/lib/src/markdown/compose/context/capture.rs:193-224`

Those APIs still gather much more than compose needs:

- git: recent commits, remotes, worktrees, config, branches, tracking
- repo: workspace detector fanout, manifest discovery, package-boundary refresh, language scanning

Relevant sniff code:

- `sniff/lib/src/filesystem/git.rs:940-946`
- `sniff/lib/src/filesystem/repo.rs:380-410`
- `sniff/lib/src/filesystem/repo.rs:1736-1795`

So the lightweight compose-specific git/repo APIs proposed in the first review still do not exist.

### 6. Validation still duplicates inline-pre work

Severity: Medium

The context-sharing fix addressed one major source of duplicated cost, but validation still runs Inline Pre work separately before actual compose:

- `prepare_content()` runs text replacement, page blocks, interpolation, and shell expansion
- actual compose then runs those stages again

Relevant code:

- `darkmatter/lib/src/markdown/reference/graph.rs:724-749`
- `darkmatter/lib/src/markdown/reference/graph.rs:237-240`

This means Priority 5 is still open even after the context reuse work.

## Coverage Gaps

### 1. No integration coverage for demand-driven capture across transclusions

There is no test proving that a root document with no `ctx.*` references still composes correctly when a transcluded child is the first place that uses a runtime context group.

That gap allowed the regression in Finding 2 to ship with a green suite.

### 2. No focused tests for the scanner itself

`darkmatter/lib/src/markdown/compose/context/capture.rs` has only date/time helper tests. I did not find focused tests for:

- `scan_needed_groups()`
- unknown-key fallback behavior
- false positives from raw `ctx.` text
- code-fence / prose / directive edge cases

### 3. No tests guarding "capture once" at the CLI boundary

The suite does not assert that `md compose <file>` captures runtime context exactly once on the success path.

Given the current `ComposeOptions::new().with_context(...)` pattern, this should be locked down with an explicit regression test or instrumentation hook.

### 4. No hash regression tests for the new context hashing boundary

I did not find direct tests showing that:

- `effective_state_hash()` ignores `ctx`
- `context_hash()` excludes the intended volatile keys
- stable context changes still affect cache identity

`darkmatter/lib/src/markdown/compose/cache/hashing.rs` has broad hash tests, but not for this new performance-sensitive split.

### 5. No focused tests for the new lightweight sniff helpers

I did not find dedicated tests for:

- `sniff::hardware::detect_hardware_summary()`
- `sniff::filesystem::docs::detect_docs_with_packages()`

Those APIs are now performance-critical for compose and should have direct unit coverage.

## Ergonomics and Performance Suggestions

### 1. Add a non-capturing `ComposeOptions` constructor for shared-context callsites

The current `ComposeOptions::new().with_context(...)` pattern is too easy to misuse.

Better options:

- `ComposeOptions::from_context(context)`
- `ComposeOptions::new_without_capture()`
- `ComposeOptions::new_with_context(context)`

Any of these would eliminate the redundant-capture footgun.

### 2. Make demand-driven capture transclusion-aware

Two viable approaches:

- do a pre-pass over the root plus transcluded children before capture
- or lazily hydrate missing context groups on first lookup

The second option is more ergonomic long-term because it keeps "capture once" semantics while avoiding root-only scan bugs.

### 3. Split Sniff further into summary DTOs for compose

The next meaningful performance step is still the one from the first review:

- `detect_git_context(...)`
- `detect_repo_context(...)`
- `detect_docs_context(...)`
- possibly a narrower docs/frontmatter scan that skips title/hash/prompt/model extraction

Right now the compose code is still paying for broad inventory structs.

### 4. Stop propagating materialized `ctx` through child external state

`build_child_external_state()` currently clones the full `state.data()` object, which includes materialized `ctx`.

That is both:

- a correctness problem, because it can emit false collision warnings
- an ergonomics problem, because child state inheritance becomes harder to reason about

At minimum, `ctx` should be stripped before child external state propagation.

### 5. Reuse validation preparation or make it optional on the hot path

If validation stays synchronous in `md compose <file>`, it should reuse already-prepared Inline Pre content where possible.

If that reuse is awkward, a pragmatic alternative is:

- fast compose path by default
- explicit validation mode when the user wants it

## Verification

I ran:

```bash
cd darkmatter && just test
printf '# test\n' | /usr/bin/time -p target/debug/md compose -
printf '# test\n' > /tmp/dm-small.md
/usr/bin/time -p target/debug/md compose /tmp/dm-small.md
```

I also manually verified the transclusion regression described in Finding 2 by composing:

- a direct file containing `{{ ctx.os }}`
- a root file that transcludes a child containing `{{ ctx.os }}`

The direct case rendered `macOS`; the transcluded case rendered empty output and emitted a context-collision warning.
