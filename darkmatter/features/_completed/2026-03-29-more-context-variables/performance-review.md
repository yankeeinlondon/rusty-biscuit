# Performance Review: Compose Context Variables

## Summary

The lag is real, and it is coming from more than one place.

- Core compose on stdin is already slow enough to be noticeable because runtime context capture is doing several expensive `sniff` probes eagerly.
- File-input compose is substantially slower because CLI reference validation runs before compose and is also triggering extra context capture.
- The biggest correctness/performance issue is that validation is not reusing the already-captured compose context, so the "resolve `ctx` once per compose operation" rule is currently violated on the `md compose <file>` path.

## Quick Measurements

These were spot measurements on `2026-03-29` using the already-built `target/debug/md` binary.

| Command | Wall time |
|---|---:|
| `printf '# test\n' \| target/debug/md compose -` | `1.77s` |
| `cat darkmatter/docs/topics/context-variables.md \| target/debug/md compose -` | `2.97s` |
| `target/debug/md compose /tmp/small.md` | `5.64s` |
| `target/debug/md compose darkmatter/docs/topics/context-variables.md` | `8.16s` |

Interpretation:

- Roughly `~1.8s` is fixed runtime/context overhead even for a tiny stdin compose.
- File input adds another `~3.9s` even for a tiny document, which points directly at pre-compose validation overhead.
- Real documents add a bit more work, but the baseline fixed cost is the main problem.

## What Is Slow

### 1. Reference validation is capturing context again

This is the clearest code issue.

- `darkmatter/cli/src/commands.rs:337-383` runs `md.validate_references(...)` before actual compose.
- `darkmatter/lib/src/markdown/reference/types.rs:430-436` makes `ReferenceGraphOptions::default()` contain a default `ComposeOptions`.
- `darkmatter/lib/src/markdown/compose/types.rs:424-450` means every default `ComposeOptions` eagerly captures runtime context.
- `darkmatter/lib/src/markdown/reference/graph.rs:267-276` builds `EffectiveState` without passing the already-captured context, so `EffectiveStateBuilder::build()` falls back to `ComposeContext::capture()`.
- `darkmatter/lib/src/markdown/compose/state.rs:307-309` is the fallback that triggers the recapture.

Effect:

- `md compose <file>` pays for context capture in validation.
- Validation can pay for it once per graph node, not once per user-visible compose.
- This violates the intended "resolve `ctx` once per compose operation" behavior.

### 2. Context capture is fully serial and includes several heavyweight probes

`darkmatter/lib/src/markdown/compose/context/capture.rs:36-145` runs all of this in sequence:

- `git::detect_git(...)`
- `repo::detect_repo(...)`
- `docs::detect_docs(...)`
- `os::detect_os()`
- `hardware::detect_hardware()`

Even if every probe were individually reasonable, doing them all serially guarantees visible latency.

### 3. Hardware capture is much heavier than compose needs

- `sniff/lib/src/hardware/mod.rs:71-77` always calls `detect_audio_devices()` before building the hardware snapshot.
- That same block documents that CoreAudio init on macOS is still around `~1.5s`.

But compose only uses:

- `memory_total`
- `memory_used`
- `memory_avail`
- `cpu_cores`
- `cpu_arch`
- `gpu`

It does not use:

- audio devices
- storage inventory
- most GPU capability detail

So compose is paying for a broad hardware inventory to populate a tiny summary.

### 4. `detect_docs()` is doing a full repo document scan and re-detecting repo structure

- `sniff/lib/src/filesystem/docs.rs:94-107` calls `detect_repo(&repo_root)` again even though Darkmatter already has `repo_info`.
- `sniff/lib/src/filesystem/docs.rs:141-156` walks the entire repo for `*.md`.
- `sniff/lib/src/filesystem/docs.rs:168-179` reads each markdown file and extracts title, prompt, model, timestamps, hashes, and blast-radius data.

For compose context, Darkmatter only needs:

- README paths
- docs with `blast_radius`
- docs affected by dirty source files
- package scoping

It does not need full document metadata, content hashes, titles, prompts, or last-updated timestamps just to fill `ctx`.

### 5. `detect_repo()` is broader than the compose use case

- `sniff/lib/src/filesystem/repo.rs:380-410` runs all workspace detectors and then recursively discovers additional manifests.
- `sniff/lib/src/filesystem/repo.rs:1736-1772` calls `refresh_package_boundaries()`, which rescans each package with `detect_package_languages(...)`.
- `sniff/lib/src/filesystem/repo.rs:1793-1842` shows recursive filesystem walking for manifest discovery.

For compose context, Darkmatter mainly needs:

- monorepo yes/no
- package names and areas
- current package / current package area
- a context-sensitive language and package manager

It does not need the full package-boundary refresh and broad language/framework inventory on every compose.

### 6. `detect_git()` also overfetches for the compose context case

- `sniff/lib/src/filesystem/git.rs:940-946` gathers recent commits, remotes, worktrees, config, branches, and tracking.

For compose context, Darkmatter really needs only:

- repo root
- repo name
- file changes

Everything else is work the compose path does not use.

### 7. Validation duplicates inline-pre compose work

- `darkmatter/lib/src/markdown/reference/graph.rs:726-741` prepares content for validation by running inline-pre compose operations.

That means file-input compose can do interpolation/page blocks/shell-expansion work once for validation and again for actual compose. Even after the context-capture bug is fixed, this is still duplicate hot-path work.

### 8. Context cloning and hashing will get more expensive as `ctx` grows

The new `ctx` map is much larger now.

- `darkmatter/lib/src/markdown/compose/cache/hashing.rs:103-124` clones and normalizes the full context map for hashing.
- `darkmatter/lib/src/markdown/compose/mod.rs:1284-1294` clones options for child compose.
- `darkmatter/lib/src/markdown/compose/mod.rs:1399-1427` clones the context again for code transclusion replacement state.

This is not the main current bottleneck, but it will become more noticeable on large transclusion graphs.

## What Looks Correct

The core compose pipeline itself mostly respects "capture once, reuse across the graph."

- `ComposeOptions::new()` captures once.
- Child compose clones the parent options in `darkmatter/lib/src/markdown/compose/mod.rs:1284-1294`.
- Main compose state-building uses `options.context().clone()` rather than recapturing.

So the main violation is not recursive compose. It is the validation path.

## Recommended Changes

### Priority 0: Fix the recapture bug in validation

1. Capture one `ComposeContext` in the CLI compose command and explicitly share it with both validation and compose.
2. Stop letting `ReferenceGraphOptions::default()` implicitly capture context on its own.
3. In `reference/graph.rs`, always pass the shared context into `EffectiveStateBuilder` with `.with_context(...)`.
4. Add a regression test that proves `md compose <file>` captures runtime context exactly once even when validation runs.

This is the first change I would make.

### Priority 1: Make context capture demand-driven

If the goal is "nearly instantaneous", Darkmatter should not eagerly gather every possible context group for every compose.

Recommended model:

- Parse interpolation expressions and `when=` conditions first.
- Determine which context groups are actually referenced.
- Only capture the needed groups.

Suggested groups:

- `datetime`
- `git_status`
- `repo_structure`
- `documents`
- `os`
- `hardware`

Example:

- If a doc only uses `ctx.today`, do not call Sniff at all.
- If a doc only uses package vars, skip docs and hardware.
- If a doc never references `ctx.*`, skip runtime context capture entirely.

This is the biggest path to "instant" behavior.

### Priority 2: Split Sniff into lightweight compose-oriented APIs

Darkmatter is currently calling broad inventory APIs where it really needs summary APIs.

Good additions in `sniff` would be:

- `detect_git_context(base_dir)` returning only repo root, repo name, and file changes.
- `detect_repo_context(repo_root, base_dir)` returning only monorepo/package/package-area/language/package-manager summary.
- `detect_docs_context(repo_root, packages)` returning only README paths and blast-radius data needed for `docs_*`.
- `detect_hardware_summary()` returning only CPU/memory/GPU summary without audio or storage.

Darkmatter should call those instead of the current broad detectors.

### Priority 3: Parallelize independent capture work

After the git probe determines repo membership/root, the remaining work should not stay serial.

Suggested flow:

1. Run `detect_git_context(base_dir)` first.
2. In parallel:
   - repo/package summary
   - docs summary
   - OS summary
   - hardware summary

Implementation options:

- `rayon::join`
- `std::thread::scope`
- a small fixed fanout, not an unbounded threadpool

Parallelism will not save an overbroad design, but it will materially reduce visible latency once the probes are trimmed.

### Priority 4: Remove redundant repo work from docs detection

At minimum:

1. Change `docs::detect_docs(...)` to accept already-computed package info.
2. Do not call `detect_repo()` again inside doc detection.
3. Avoid reading and hashing full markdown files if compose only needs README/blast-radius information.
4. Read only enough frontmatter to answer the compose context queries.

### Priority 5: Keep validation off the critical path, or make it reuse compose work

For the `md compose <file>` UX, validation is currently part of the perceived lag.

Options:

1. Keep validation enabled, but make it reuse the already-captured context and already-prepared content.
2. Run validation after output emission when errors are only warnings/allowed.
3. Add a fast-path mode that skips validation entirely.
4. Cache validation graph preparation separately if it must remain synchronous.

Even after fixing recapture, validation is still doing duplicate inline-pre work today.

### Priority 6: Make `ComposeContext` cheap to clone

The current map-backed context is getting large enough that clone cost matters.

Recommended changes:

- Back `ComposeContext` with `Arc<ComposeContextInner>`.
- Precompute a stable context hash once at capture time.
- Precompute the `ctx` JSON object once instead of cloning maps during hashing.

This is a second-order optimization, but it will help on deep transclusion graphs.

## Diagnostics and Tracing I Would Add

Right now there is almost no timing instrumentation around the hot path.

### Tracing spans

Add spans around:

- `compose.command`
- `compose.validation`
- `compose.context.capture`
- `compose.context.git`
- `compose.context.repo`
- `compose.context.docs`
- `compose.context.os`
- `compose.context.hardware`
- `compose.inline_pre`
- `compose.transclusion`
- `reference.graph.build`

Each span should include:

- `base_dir`
- `source`
- `doc_count`
- `package_count`
- `dirty_file_count`
- `ctx_keys_requested`
- `ctx_reused=true/false`

### Slow-path warnings

Emit warnings when any capture stage exceeds a threshold, for example:

- `>50ms` for git
- `>100ms` for repo/docs
- `>250ms` for hardware

### User-facing timing output

Add an opt-in mode such as:

- `md compose --timings`
- `md compose --timings=json`

Useful fields:

- `validation_ms`
- `context_capture_ms`
- `context_git_ms`
- `context_repo_ms`
- `context_docs_ms`
- `context_os_ms`
- `context_hardware_ms`
- `inline_pre_ms`
- `transclusion_ms`
- `inline_post_ms`
- `total_ms`

### Explicit reuse counters

Track and print:

- how many times runtime context was captured
- how many times it was reused
- how many reference-graph nodes were built
- how many times validation built `EffectiveState`

This would make regressions obvious immediately.

### Benchmarks and regression tests

Add tests/benches for:

- tiny stdin compose
- tiny file compose
- file compose with validation enabled
- compose with many transclusions
- compose with docs/hardware groups disabled vs enabled

And add a regression test that asserts:

- context capture count is `1` for one compose command
- validation does not trigger extra capture

## Recommended Order of Work

1. Fix validation so it reuses one captured context.
2. Add timings/tracing so improvements are measurable.
3. Split out lightweight Sniff summary APIs.
4. Make context capture demand-driven by referenced `ctx` groups.
5. Parallelize the remaining summary probes.
6. Make context storage/hash cheap to clone and reuse.

## Bottom Line

If the requirement is "nearly instantaneous", the current eager full-context model cannot get there.

The fastest path forward is:

- fix the validation recapture bug first
- stop using broad Sniff inventory APIs for compose context
- only capture the context groups the document actually references
- instrument the path so future regressions are obvious
