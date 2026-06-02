---
ready: false
agent: codex
model: ""
---

# Review: URL Referencing

## Findings

### High: read-side expression functions are not wired into compose

- Requirement: remote reads must work for URL-typed expression-function arguments such as `frontmatter(url)`, `file_exists(url)`, `markdown_title(url)`, and related read-side functions.
- Current behavior: the root pipeline discovers expression URL arguments and eagerly registers them only when block transclusion is enabled ([mod.rs:488](../../lib/src/markdown/compose/mod.rs#L488)), but the interpolation stage still builds `Evaluator::new(state)` with no remote-aware resolution context ([mod.rs:1241](../../lib/src/markdown/compose/mod.rs#L1241)). `EffectiveState` implements `EvaluationLookup` without overriding `resolution_context()` ([state.rs:297](../../lib/src/markdown/compose/state.rs#L297)), while the expression dispatch only calls filesystem/remote functions when `lookup.resolution_context()` returns `Some` ([expression/mod.rs:583](../../lib/src/markdown/compose/expression/mod.rs#L583)).
- Impact: ordinary composition cannot evaluate these functions at all in the path users exercise through `Markdown::compose_with` or `md compose`. The new helper-level remote tests manually construct a `ResolutionContext` with `remote_fetch: Some(rt)` ([functions.rs:1459](../../lib/src/markdown/compose/expression/functions.rs#L1459)), so they do not prove the actual compose pipeline works.
- Verification level: missing Level 1 compose and CLI coverage. Add tests that run full composition with `{{ frontmatter(https://...) }}`, `{{ markdown_title(https://...) }}`, `{{ markdown_body_empty(https://...) }}`, `{{ validate_schema(https://...) }}`, and `{{ file_exists(https://...) }}` through the same options and CLI flags users rely on. Level 2/3 is not required for these non-terminal input semantics.
- Suggested fix: introduce a compose interpolation lookup/context that carries the source base directory, magic paths, and the run's `RemoteFetchRuntime`, then use it when constructing the evaluator. The same path should preserve existing local filesystem expression behavior.

### High: remote expression discovery is gated behind `BlockTransclusion`

- Requirement: discovery must collect remote URLs from transclusion directives and URL-typed expression-function arguments, then register each unique URL as an eager in-flight slot.
- Current behavior: discovery runs only inside `if options.allow_remote_transclusion && options.is_enabled(ComposeOperation::BlockTransclusion)` ([mod.rs:488](../../lib/src/markdown/compose/mod.rs#L488)). A caller that enables interpolation and disables block transclusion cannot prefetch or use remote expression arguments, even though expression reads are a separate in-scope use case.
- Impact: option-scoped composition breaks a documented feature. It also makes expression remote reads depend on an unrelated transclusion operation.
- Verification level: missing Level 1 option-composition coverage. Add a test with `ComposeOptions::only(&[ComposeOperation::Interpolation])` plus allowed host config and a URL expression. It should fetch and interpolate without requiring `BlockTransclusion`.
- Suggested fix: run expression URL discovery when `Interpolation` is enabled and remote reads are configured, independent of block transclusion. Directive discovery can remain tied to transclusion operations.

### Medium: wildcard host policy contradicts its own contract

- Requirement/safety contract: host policy is the shared SSRF boundary. The docs for `HostPattern::Wildcard` say `*.example.com` matches subdomains but not `example.com` itself.
- Current behavior: `Wildcard` accepts the bare suffix host because `matches()` returns true for `host_lower.eq_ignore_ascii_case(&suffix_lower)` ([fetch.rs:31](../../../biscuit-file/lib/src/file_reference/fetch.rs#L31)). The unit tests encode that broader behavior ([fetch.rs:303](../../../biscuit-file/lib/src/file_reference/fetch.rs#L303)).
- Impact: a policy intended to allow only delegated subdomains also allows the parent domain. That is a security-boundary semantics bug, even if the current CLI exposes exact hosts only.
- Verification level: existing Level 1 tests assert the wrong behavior. Flip the unit/integration expectations so wildcard does not match the bare suffix; keep exact-host coverage separate.
- Suggested fix: remove the exact-suffix branch from `HostPattern::Wildcard::matches()` or change the public contract to say wildcard includes the parent host. For a security primitive, the narrower documented behavior is the safer default.

## Test-Level Summary

- Remote transclusion, remote code inclusion, duplicate single-flight behavior, nested remote transclusion, CLI allow-host denial, CLI caching refresh, and stale fallback have Level 1 coverage through unit/integration/CLI tests.
- Rendered remote links have appropriate Level 1 coverage for preservation and Level 2 coverage for terminal hyperlink styling behavior.
- No Level 3 coverage is required by this spec because it does not define OS keyboard, paste, mouse, or terminal input-encoder behavior.
- The key missing Level 1 coverage is full-pipeline remote expression-function behavior and operation-scoped discovery.

## Production Readiness

Not ready for production. Remote transclusion and cache behavior are close, but read-side URL expression functions are a core in-scope requirement and are not wired through the actual compose pipeline.
