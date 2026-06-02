---
ready: false
agent: codex
model: ""
---

# Review: URL Referencing

## Findings

### High: remote directives created during compose are not fetched

- Requirement: remote HTTP(S) references should work through the same descriptors that drive transclusion, and nested/newly parsed remote references should be registered when discovered.
- Current behavior: eager registration scans only the original document content before the compose pipeline runs ([mod.rs:494](../../lib/src/markdown/compose/mod.rs#L494)-[519](../../lib/src/markdown/compose/mod.rs#L519)). Later, the transclusion preparation step can resolve a post-transform directive to `PreparedTransclusion::RemoteFile` or `RemoteCode` ([mod.rs:1545](../../lib/src/markdown/compose/mod.rs#L1545)-[1567](../../lib/src/markdown/compose/mod.rs#L1567)), but that path does not register the URL. Point-of-use then only calls `get_content()` and fails if the URL was not pre-registered ([mod.rs:1889](../../lib/src/markdown/compose/mod.rs#L1889)-[1900](../../lib/src/markdown/compose/mod.rs#L1900), [mod.rs:2004](../../lib/src/markdown/compose/mod.rs#L2004)-[2015](../../lib/src/markdown/compose/mod.rs#L2015)).
- Impact: valid compose flows that materialize a remote directive before block transclusion, such as interpolation or replacement producing `::file https://...` / `::code https://...`, fail with "URL was not registered for fetching" instead of fetching the allowed URL. This is a functionality gap in the transclusion surface, not just a prefetch optimization miss.
- Verification level: missing Level 1 compose/CLI coverage. Add tests where an earlier compose phase creates a remote `::file` and `::code` directive, with `--allow-host`, and assert that the remote body is included. No Level 2 or Level 3 coverage is required because this is not terminal-emulator or OS-input behavior.
- Suggested fix: when transclusion preparation resolves an allowed remote URL, call `remote_fetch.register_nested(url.clone())` or equivalent before pushing the prepared remote item. Keep the initial eager scan for latency, but make point-of-use robust for URLs discovered after that scan.

### Medium: the remote concurrency cap does not bound spawned work

- Requirement: remote fetches should run as tasks on Darkmatter's shared Tokio runtime, with in-flight work bounded by the configured concurrency cap.
- Current behavior: each unique allowed URL starts a new OS thread and builds a fresh current-thread Tokio runtime ([remote_fetch.rs:237](../../lib/src/markdown/compose/remote_fetch.rs#L237)-[243](../../lib/src/markdown/compose/remote_fetch.rs#L243)). The semaphore limits the HTTP request section ([remote_fetch.rs:244](../../lib/src/markdown/compose/remote_fetch.rs#L244)-[252](../../lib/src/markdown/compose/remote_fetch.rs#L252)), but it does not limit thread/runtime creation.
- Impact: a document with many unique remote references can create many blocked threads even when `--remote-concurrency` is low. That weakens the advertised resource-control behavior and makes the implementation less suitable for large generated documents or CI.
- Verification level: current Level 1 tests prove that a cap of 1 eventually completes multiple fetches, but they do not verify that worker creation is bounded. Add a focused runtime test around a shared async executor or expose internal task accounting sufficient to prove the cap controls spawned work, not just request entry.
- Suggested fix: keep one runtime/client per compose run and spawn async tasks onto it, acquiring the semaphore before the fetch. If the compose pipeline must remain sync at the API boundary, hide the runtime inside `RemoteFetchRuntime` rather than creating one runtime per URL.

## Test-Level Summary

- `biscuit-file` URL classification, host policy, fetch, conditional headers, and blocking fetch have Level 1 coverage. I ran `cargo test --color=never -p biscuit-file --features fetch fetch`; it passed.
- Remote expression functions now have Level 1 full-pipeline coverage for `frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema`, `file_exists`, denied hosts, and interpolation-only operation scope ([mod.rs:5541](../../lib/src/markdown/compose/mod.rs#L5541)-[5647](../../lib/src/markdown/compose/mod.rs#L5647)).
- Nested remote transclusion has Level 1 coverage for remote content that itself contains a remote directive ([mod.rs:5670](../../lib/src/markdown/compose/mod.rs#L5670)-[5695](../../lib/src/markdown/compose/mod.rs#L5695)).
- Rendered remote links have Level 1 CLI preservation coverage and Level 2 terminal styling coverage. No Level 3 coverage is required by this spec because it does not define keyboard, paste, mouse, or terminal input-encoder behavior.
- I attempted `cargo test --color=never -p darkmatter remote_`, but it was still compiling after the non-interactive wait window and was terminated.

## Production Readiness

Not ready for production. The core static remote-read paths are much improved, but remote transclusions introduced by earlier compose phases can still fail at point-of-use, and the fetch runtime does not provide the resource bounds promised by the design.
