---
total_phases: 7
created: 2026-07-19
phase: 1
agent: "opencode/kimi-for-coding/k3"
yolo: "true"
spec: "darkmatter/features/2026-07-13-more-is-more/spec.md"
review: "darkmatter/features/2026-07-13-more-is-more/review-19.md"
iteration: 19
features:
  - "Phase 1[High]: Focused provider failures still have different frontmatter and body semantics"
  - "Phase 2[High]: Production provider construction cannot support the promised self-hosted servers"
  - "Phase 3[High]: The authored query boundary accepts internal fields and silently approximates advertised filters"
  - "Phase 4[High]: Canonical API URLs are not recognized as exact references"
  - "Phase 5[High]: Required Windows and Linux result evidence still does not exist"
  - "Phase 6[Medium]: DMLS emits a document-relative link that does not identify the authored vocabulary file"
  - "Phase 7[Medium]: Provider-supplied link destinations bypass the text-escaping boundary"
packages:
  - darkmatter
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/remote_fetch.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/provider.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
---

# Review Plan 19

Source review: `review-19.md` (7 findings: 5 High, 2 Medium). Source spec:
`spec.md`. Affected packages: `sniff`, `sniff-cli`, `darkmatter`,
`darkmatter-cli`, `dmls`, and downstream `claudine`/`claudine-cli`.

## Sequencing Notes

- GitNexus reports **CRITICAL** upstream risk for `ResolvedRemote` (41 impacted
  symbols, four direct callers, five modules). Phase 2 changes that model and
  therefore runs first among the sniff-side phases, with explicit impact
  re-analysis before and after.
- Phase 3 (flavor-aware filter validation) and Phase 4 (flavor/host-specific
  URL parsing) both depend on the enriched resolution model from Phase 2 —
  they are sequentialized after it. Phase 3 and Phase 4 touch different
  modules (query boundary vs. URL parsing) and may proceed in parallel once
  Phase 2 lands.
- Phase 1 (darkmatter expression-error fatality) is independent of the sniff
  phases and is parallelizable with Phase 2.
- Phases 6 and 7 are small, independent, and parallelizable with everything.
- Phase 5 (cross-OS evidence) is last: it must capture CI runs over the final
  merged tree, and it requires authorization to trigger CI, which prior cycles
  recorded as outside session scope. It is planned here with an explicit
  authorization checkpoint.

## Phase 1 [_High_]: Focused provider failures still have different frontmatter and body semantics

> Performance Gate: false

AC26 requires frontmatter/body availability parity and AC27 requires focused
provider errors to remain actionable. A failing `pr(123)` currently aborts
composition in frontmatter but leaves unevaluated `{{ pr(123) }}` text plus a
warning in the body — pinned by
`body_surface_downgrades_focused_failures_to_warnings`
(`darkmatter/lib/src/markdown/compose/tests/provider_network.rs:601`). The
review-18 implementation deferred this because `ExpressionError::Other` is
deliberately outside `is_authoring_fatal`; the review directs us to fix it by
classifying focused provider errors distinctly rather than by widening `Other`.

- [x] Run GitNexus `impact` (upstream) on `is_authoring_fatal`,
      `cached_provider_query` / `cached_query_error`, and the compose
      frontmatter/body/`$()` evaluation paths; record blast radius before
      editing.
- [x] Introduce a dedicated focused-provider error classification in
      darkmatter's expression layer (e.g. a `ExpressionError::Provider`
      variant or a structured wrapper carrying the sniff
      `FocusedProviderError` kind: not-found, denied host, authentication,
      rate limit, unsupported capability, incomplete domain, transport
      failure). The classification must survive the memoization layer —
      `resolve_ctx.rs` currently stores failures as `String`, which is what
      destroyed the distinction; store the typed error (or a cloneable
      classification token plus message) in the cache slot instead.
- [x] Apply one authoring-fatality rule for the focused-provider
      classification on all three surfaces: frontmatter, body interpolation,
      and `$()`. Body and `$()` must abort composition with the actionable
      error exactly as frontmatter does. Do **not** change the fatality of
      generic `ExpressionError::Other`; `other_is_not_authoring_fatal` must
      keep passing unchanged.
- [x] Verify error messages are not double-prefixed on any surface after the
      reclassification (the review-18 `cached_query_error` adoption behavior
      must be preserved or reimplemented in the typed path).
- [x] Replace the asymmetry-pinning test
      `body_surface_downgrades_focused_failures_to_warnings` with parity
      cases. For each failure kind — not-found, denied host, authentication,
      rate limit, unsupported capability, incomplete domain, transport
      failure — assert identical fatal behavior on frontmatter, body, and
      `$()` surfaces (Wiremock-backed, `#[serial(provider_transport)]` where
      an override is installed).
- [x] Keep the spec's narrower bullet ("focused errors are never replaced
      with empty values") asserted on all three surfaces.
- [x] Validation checkpoint: `just test` and `just lint` in `darkmatter/`
      green; `cargo nextest run -p darkmatter` focused provider suite green.

**Acceptance criteria**: an identical failing provider call produces the same
fatal, actionable error on frontmatter, body, and `$()`; the seven failure
kinds each have a three-surface parity test; no generic `Other` fatality
change; the old asymmetry test is gone; darkmatter `just test`/`just lint`
green.

## Phase 2 [_High_]: Production provider construction cannot support the promised self-hosted servers

> Performance Gate: false

The provider scope includes self-managed GitLab and Gitea/Forgejo
(spec.md:914), but `ResolvedRemote` derives `ApiFlavor` from hostname patterns
only and stores only `host_str()` — scheme and port are lost
(`sniff/lib/src/filesystem/git/remote_resolver.rs:126,139`;
`sniff/lib/src/filesystem/git/types.rs:419`). Client construction hard-codes
HTTPS and rejects `Unknown` (`sniff/lib/src/remote/focused.rs:892`), and the
bounded `remote_vendor()` probe result is not reused by `pr*`/`cicd*`.

- [ ] Run GitNexus `impact` (upstream) on `ResolvedRemote`,
      `canonical_api_base`, `FocusedProviderClient::new`, and
      `remote_vendor`; confirm the CRITICAL blast radius (41 symbols) and
      enumerate the four direct callers before editing. Report to the user.
- [ ] Extend `ResolvedRemote` to carry the normalized endpoint origin
      (scheme, host, non-default port), the detected `ApiFlavor` (and server
      version where the probe obtained it), and the derived API base URL.
      Preserve existing behavior for the well-known SaaS hosts
      (github.com, gitlab.com, bitbucket.org, codeberg.org).
- [ ] Route the allowlisted bounded server-discovery probe (already used by
      `remote_vendor()`) into provider client construction so an ambiguous
      self-hosted host is classified once and the result is reused by
      `pr*`/`cicd*` — not probed twice, not silently `Unknown`.
- [ ] Update `canonical_api_base` / `FocusedProviderClient` construction to
      honor the resolved scheme and non-default port instead of hard-coding
      `https://` on a bare host.
- [ ] Decide and document the exact-host allowlist policy for ports (an
      allowlisted `host` entry must match host **and** effective origin, or
      the policy must state explicitly how ports are treated); encode the
      decision in the denial error message so it stays actionable.
- [ ] Add Level-1 tests that pass through the production-equivalent
      classification and endpoint derivation — no test-only flavor/API-base
      override — for: self-managed GitLab, Gitea, Forgejo, each including a
      non-default port case; plus a negative case proving a probe-failing
      ambiguous host remains a focused error rather than `Unknown`.
- [ ] Keep `FocusedProviderClient::with_api_base` working for the existing
      darkmatter test-transport override.
- [ ] Validation checkpoint: `just test` and `just lint` in `sniff/` green;
      darkmatter `just test` green (downstream consumer of the changed
      model); `cargo check --all-targets -p claudine -p claudine-cli` green.
- [ ] Run GitNexus `detect_changes` after the edits and confirm only the
      expected symbols/processes are affected.

**Acceptance criteria**: a self-managed GitLab/Gitea/Forgejo remote on a
neutral hostname (e.g. `git.company.com`) and a non-default port resolves to
the correct flavor and API base through the production path; the vendor probe
result is shared between `remote_vendor()` and provider queries; scheme/port
are preserved end-to-end; sniff and darkmatter suites green with no
test-only overrides in the new cases.

## Phase 3 [_High_]: The authored query boundary accepts internal fields and silently approximates advertised filters

> Performance Gate: false

Depends on Phase 2 (flavor-aware validation needs the detected flavor). The
spec exposes only the canonical keys and forbids ignored fields and
approximations (spec.md:1268, spec.md:1805/D25). Darkmatter currently
deserializes the caller's object directly into Sniff's transport types, so
internal fields (`descending`, `cursor`) are authorable and `cursor` is
silently ignored (`pull_requests.rs:56`, `cicd.rs:88`;
`sniff/lib/src/remote/types.rs:317,575`).

- [ ] Define authored request DTOs in darkmatter (one for PR queries, one for
      CI/CD queries) containing exactly the catalog v1 vocabulary, with
      `deny_unknown_fields`, and translate them into Sniff's internal
      paging/query types at the boundary. Authored `descending`, `cursor`,
      or any non-vocabulary key must fail with an actionable authoring
      error naming the offending key.
- [ ] Make capability validation flavor-aware before any I/O: a `stage`
      filter against a flavor with no stage value must return an explicit
      unsupported-filter error, not `[]` (`focused.rs:94,922`).
- [ ] Retain the workflow definition path in the CI projection and match the
      `workflow` filter exactly against definition ID, name, **or** path —
      not only parent name/native ID (`focused.rs:1042`).
- [ ] Fix `sort: "provider-default"`: when selected, neither sort nor reverse
      the provider's order — the current `descending: true` default must not
      apply (`focused.rs:1026`).
- [ ] Add negative tests: authored `descending`/`cursor` rejected by name;
      flavor-specific `stage` (supported flavor honors it, unsupported
      flavor errors); `workflow` matched by path and by definition ID;
      `provider-default` preserves provider order in both provider orderings.
- [ ] Validation checkpoint: `just test`/`just lint` green in both `sniff/`
      and `darkmatter/`.

**Acceptance criteria**: only the canonical vocabulary is authorable;
internal keys are rejected with actionable errors; every canonical filter is
either honored exactly or fails explicitly per flavor before I/O; the four
approximation bullets from the review each have a failing-then-passing test.

## Phase 4 [_High_]: Canonical API URLs are not recognized as exact references

> Performance Gate: false

Depends on Phase 2 (host/flavor-specific parsing needs the enriched
resolution model). The spec defines provider URLs as canonical web **or**
API URLs (spec.md:911, spec.md:1797/D23), but parsing is shape-based for web
routes only (`focused.rs:753,809`); API URLs are rejected or decoded into
the wrong namespace, and percent-encoded GitLab project paths are not
decoded.

- [ ] Implement host/flavor-specific parsers for both route families per
      provider: GitHub (`/pull/` web, `/repos/{o}/{r}/pulls/{n}` API),
      GitLab (`/-/merge_requests/` web, `/api/v4/projects/{id}/merge_requests/{n}`
      API), Bitbucket (`/pull-requests/` web, `/2.0/repositories/{w}/{r}/pullrequests/{n}`
      API), Gitea/Forgejo (`/pulls/` web and API), and the corresponding web
      and API job routes for `cicd()`.
- [ ] Percent-decode repository identity where the provider requires it
      (GitLab `group%2Fproject`); retain scheme and port from Phase 2 so
      self-hosted API URLs resolve against the correct origin.
- [ ] Reject malformed API URLs with focused errors that name the URL form
      problem; never decode into the wrong namespace/repository silently.
- [ ] Extend `sniff/lib/tests/focused_provider.rs` beyond the web-only cases
      at line 114: positive and malformed API-URL cases for every supported
      provider and for both PR and job references.
- [ ] Validation checkpoint: `just test`/`just lint` in `sniff/` green;
      darkmatter `just test` green.

**Acceptance criteria**: the three API-URL examples quoted in the review
(GitHub `/repos/acme/project/pulls/7`, GitLab
`/api/v4/projects/group%2Fproject/merge_requests/8`, Bitbucket
`/2.0/repositories/acme/project/pullrequests/10`) each resolve exactly;
malformed cases error with focused messages; web-URL behavior is unchanged.

## Phase 5 [_High_]: Required Windows and Linux result evidence still does not exist

> Performance Gate: true

AC16/AC29 require macOS, Windows, and Linux compile checks to pass. Review 18
delivered the CI configuration (three-OS Sniff matrix with
`--all-targets --features remote`, downstream Claudine macOS/Windows checks)
but no run was triggered; configuration is not a passing result. This phase
requires no source changes — only CI execution and, if a leg fails, a fix
(which would then spawn its own phase/task).

- [ ] Authorization checkpoint: obtain explicit approval to trigger CI
      (prior cycles recorded this as outside session authorization). Without
      it this phase cannot start; state that plainly rather than simulating
      evidence.
- [ ] Trigger the Sniff matrix: `workflow_dispatch` on `test.yml` (or a PR
      to `main`) covering `sniff-cross-platform` on macOS, Linux, Windows —
      including the `--all-targets --features remote` compile step and the
      provider Wiremock suites.
- [ ] Trigger the darkmatter full-OS legs (`darkmatter-tests.yml`) and the
      Claudine `cross-platform-check` job (push touching `claudine/**` or
      dispatch).
- [ ] Capture the run URLs/IDs and results into `log.md` as the AC16/AC29
      evidence record.
- [ ] If any Windows/Linux leg fails: resolve the failure in source (new
      task(s) under this phase) and re-run until green. A Windows failure
      must be fixed, not absorbed by the `continue-on-error` advisory job.
- [ ] Validation checkpoint: green runs recorded for all three OSes across
      the Sniff, Darkmatter/DMLS, and downstream Claudine scope.

**Acceptance criteria**: named, linked green CI runs on macOS, Windows, and
Linux for the affected scope exist in `log.md`; any Windows failure
encountered was resolved in source; no advisory/`continue-on-error` result
is counted as evidence.

## Phase 6 [_Medium_]: DMLS emits a document-relative link that does not identify the authored vocabulary file

> Performance Gate: false

The catalog links to `darkmatter-expressions.md#provider-query-vocabulary`
and DMLS copies it verbatim into hover/completion
(`dmls/src/overlay/expressions.rs:408`), but the active document can live
anywhere, so the relative target is not anchored to `darkmatter/docs/topics/`.

- [ ] Choose the resolution strategy: (a) rewrite the catalog documentation
      target into an absolute workspace-rooted `file://` URI at the DMLS
      response boundary, or (b) embed the compact vocabulary table directly
      in the hover. Prefer (a) if the LSP client contract supports file
      links; fall back to (b). Document the choice.
- [ ] Implement the rewrite/embedding in `expressions.rs` without touching
      the generated-catalog source of truth (the YAML stays
      workspace-relative; the DMLS overlay resolves it).
- [ ] Replace the raw-substring assertion at `expressions.rs:711` with a test
      at the LSP response boundary verifying the emitted target resolves to
      the generated topic file and anchor regardless of the active
      document's location.
- [ ] Validation checkpoint: `just test`/`just lint` in `darkmatter/` green
      (dmls included).

**Acceptance criteria**: hover/completion emitted from an arbitrary document
location carries a resolvable link (or embedded vocabulary) pointing at
`darkmatter/docs/topics/darkmatter-expressions.md#provider-query-vocabulary`;
the boundary test proves it.

## Phase 7 [_Medium_]: Provider-supplied link destinations bypass the text-escaping boundary

> Performance Gate: false

Both formatters interpolate `web_url` directly into the Markdown destination
(`pull_requests.rs:75`, `cicd.rs:107`); a `)` breaks the deterministic
projection and a non-HTTP scheme becomes an unsafe downstream link.

- [ ] Validate normalized record URLs at the Sniff provider boundary: parse
      each response URL field, require an HTTP(S) origin consistent with the
      resolved provider's policy (host/origin from Phase 2), and surface a
      focused error for malformed, cross-host, or non-HTTP values.
- [ ] Serialize the validated destination safely in the darkmatter formatters
      (percent-encode or otherwise neutralize characters that terminate a
      CommonMark link destination, consistent with the shared text escaper
      from review 18).
- [ ] Extend the hostile-provider Level-1 fixture beyond title text:
      malformed URL, cross-host URL, and non-HTTP scheme cases for both PR
      and CI/CD projections, asserting the composed document parses as
      CommonMark+GFM with the canonical destination intact.
- [ ] Validation checkpoint: `just test`/`just lint` green in `sniff/` and
      `darkmatter/`.

**Acceptance criteria**: no provider-supplied URL reaches the Markdown output
unvalidated; the three hostile URL classes are covered by tests on both
surfaces; legitimate provider URLs render byte-identically to before.

## Parallelization Summary

| Track | Phases | Constraint |
|-------|--------|-----------|
| A | 2 → {3, 4} | 3 and 4 parallel after 2 |
| B | 1 | Independent; parallel with A |
| C | 6, 7 | Independent; parallel with A/B (7 prefers Phase 2 landed for origin policy) |
| D | 5 | Last; after A–C merge; needs CI authorization |
