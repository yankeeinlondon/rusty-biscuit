---
feature: 2026-04-25-repo-pr
review: review-1.md
created: 2026-04-26
phases: 4
packages:
  - schematic-definitions
  - schematic-schema
  - sniff
  - sniff-cli
---

# Review Remediation Plan: `sniff repo pr`

This plan addresses the gaps identified in `review-1.md` for the `sniff repo pr`
feature. Each phase is self-contained: it ends with passing tests and zero
clippy warnings/errors in any package the phase touches.

## Conventions

- Use targeted `cargo` invocations with `-p <pkg>` flags. **Never** `cargo build`
  / `cargo test` at the repo root (per CLAUDE.md / MEMORY.md).
- `schematic/schema` is excluded from the workspace. Address it with
  `--manifest-path schematic/schema/Cargo.toml`.
- `schematic/schema/src/*.rs` is **generated** from `schematic/definitions`.
  After changing definitions, regenerate via:
  ```bash
  just -d schematic generate-one gitea
  just -d schematic generate-one github
  # (or `generate-all` to regenerate every API)
  ```
  Verify regenerated files compile with
  `cargo check --manifest-path schematic/schema/Cargo.toml`.
- Pre-existing local edits in this worktree (`schematic/define/src/headers.rs`,
  `schematic/schema/src/gitea.rs` `eprintln!` debug line, edits to
  `prompts/greet.md`, `docs/knowledge/commits.md`, `.claude/skills/sniff/SKILL.md`)
  are unrelated to this review. Either revert them before starting, or keep them
  isolated in their own commit. Do **not** mix them into review-remediation
  commits.
- Lint cleanup for the entire `sniff` package area (lib + cli) is the
  developer's responsibility, regardless of authorship. End every phase by
  running `just -d sniff lint` and resolving any warnings.
- Existing `unsafe { std::env::set_var }` patterns in
  `sniff/lib/tests/remote_providers.rs` are a known wart; do not refactor them
  in this plan unless they break.

---

## Phase 1 — Schema expansion: GitHub & Gitea labels (and confirm Bitbucket fields)

**Goal:** Make the schematic schema expose the data the providers need so that
`PullRequestInfo.labels` is populated for GitHub and Gitea (review item 1) and
so the Bitbucket follow-up implementation in Phase 2 has a verified field
inventory.

This phase only touches the schema layer. No `sniff` provider behavior
changes yet. Existing sniff tests will still pass because providers continue
to ignore the new fields until Phase 2.

### Files to change

- `schematic/definitions/src/github/types.rs`
  - Extend `pub struct PullRequestSummary` (around line 226) with:
    - `pub labels: Vec<Label>` (use existing `Label` type from
      `schematic/definitions/src/github/types.rs`; same type already used by
      `IssueSummary`). Wrap with `#[serde(default)]` so existing fixtures
      without a labels array still deserialize.
  - Add a `pull_request_summary_with_labels_deserialization` test next to the
    existing `pull_request_summary_deserialization` (line 908) using
    `"labels": [{"name": "bug"}]` and asserting `pr.labels[0].name == "bug"`.
- `schematic/definitions/src/gitea/types.rs`
  - Extend `pub struct PullRequestSummary` (around line 217) with:
    - `pub labels: Option<Vec<Label>>` (Gitea types are universally optional;
      reuse the existing Gitea `Label` type if present, otherwise add a small
      `pub struct Label { pub name: Option<String>, pub color: Option<String> }`
      with `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`).
    - `#[serde(default)]` on the field.
  - Add a Gitea `pull_request_summary_with_labels_deserialization` test
    mirroring the GitHub one.
- `schematic/definitions/src/{github,gitea}/mod.rs`
  - If a new `Label` type was introduced, register it in the schema registry
    alongside `PullRequestSummary` (look for the existing
    `.register::<PullRequestSummary>("PullRequestSummary")` lines, mod.rs:76 /
    mod.rs:85). Re-export from `prelude.rs` if needed.
- **Bitbucket — no schema change required.** The existing
  `schematic/definitions/src/bitbucket/types.rs` `PullRequest` struct
  (line 330) already has a `pub description: Option<String>` field. The list
  endpoint just doesn't populate it. Confirm this by reading the file; do not
  modify the schema. Phase 2 handles Bitbucket via a follow-up
  `GetPullRequest` call, not a schema change.

### Step-by-step tasks

1. Read `schematic/definitions/src/github/types.rs` lines 220–295 to confirm
   the existing `Label` type definition shape (used by `IssueSummary`).
2. Add the `labels: Vec<Label>` field to GitHub's `PullRequestSummary` with
   `#[serde(default)]`.
3. Add a unit test (next to existing `pull_request_summary_deserialization`,
   ~line 908) covering the new field.
4. Repeat steps 1–3 for Gitea (`schematic/definitions/src/gitea/types.rs`).
   Note: Gitea types are pervasively optional, so use
   `Option<Vec<Label>>` plus `#[serde(default)]`. If Gitea has no existing
   `Label` type, add one.
5. If a new Gitea `Label` was introduced, register it in
   `schematic/definitions/src/gitea/mod.rs` and re-export it via
   `schematic/definitions/src/prelude.rs` if other crates rely on the prelude.
6. Regenerate the schema crate from definitions:
   ```bash
   just -d schematic generate-one github
   just -d schematic generate-one gitea
   ```
   Or run `generate-all` if the developer prefers a single sweep.
7. Inspect the regenerated `schematic/schema/src/github.rs` and
   `schematic/schema/src/gitea.rs` to confirm the new field appears on the
   regenerated `PullRequestSummary` structs. Commit the regenerated files
   alongside the definition changes.

### Tests to add or update

- `schematic/definitions/src/github/types.rs` — new
  `pull_request_summary_with_labels_deserialization` unit test.
- `schematic/definitions/src/gitea/types.rs` — new
  `pull_request_summary_with_labels_deserialization` unit test.
- Existing `pull_request_summary_deserialization` tests must still pass
  unchanged (proves `#[serde(default)]` keeps the field optional).
- `schematic/schema` has its own `tests/` directory; run those to ensure
  regenerated code still compiles and any drift checks pass.

### Verification

```bash
# Definitions: types + registry
cargo test -p schematic-definitions
cargo clippy -p schematic-definitions -- -D warnings

# Generation: confirm regen runs cleanly
just -d schematic generate-one github
just -d schematic generate-one gitea

# Optional: full schematic test sweep
just -d schematic test
just -d schematic lint

# Schema crate (out-of-workspace): regenerated code must compile
cargo check --manifest-path schematic/schema/Cargo.toml
cargo test --manifest-path schematic/schema/Cargo.toml
cargo clippy --manifest-path schematic/schema/Cargo.toml -- -D warnings

# Sniff still builds and tests against the regenerated schema
cargo build -p sniff
cargo test -p sniff
just -d sniff lint
```

### Phase 1 done when

- New `labels` field exists on regenerated `PullRequestSummary` for GitHub and
  Gitea.
- New unit tests pass; existing `pull_request_summary_deserialization` still
  passes.
- All commands above exit zero with no clippy warnings.

---

## Phase 2 — Provider behavior: labels, Bitbucket body, deferred-credentials path

**Goal:** Use the new schema fields to populate
`PullRequestInfo.labels` for GitHub and Gitea (review item 1), populate
`body` for Bitbucket via a follow-up call when verbose data is requested
(review item 2), and refactor providers so missing credentials no longer
short-circuit before attempting an unauthenticated request (review item 3).

### Files to change

- `sniff/lib/src/remote/provider.rs`
  - No trait signature change is required for labels/body — `PullRequestInfo`
    already has those fields.
  - **Optional enhancement (recommended):** add an additional method
    `async fn get_pull_request(&self, owner: &str, repo: &str, number: u64)
    -> Result<PullRequestInfo, SniffError>;` with a default implementation
    that returns `Err(SniffError::RemoteApi { ... status: 501 ... })`.
    Bitbucket overrides it. (Decision deferred to the implementer; if the
    "verbose-only follow-up" approach is chosen the trait must expose this.)
- `sniff/lib/src/remote/github.rs`
  - In `list_pull_requests` (around line 321), replace the hardcoded
    `labels: Vec::new()` with
    `labels: pr.labels.into_iter().map(|l| l.name).collect()`.
  - Refactor `map_schematic_error` (around line 81): treat
    `SchematicError::MissingCredential` and
    `SchematicError::AuthenticationRequired` as
    `SniffError::MissingCredentials` **only** when called from a context
    that has already failed an unauthenticated attempt. The cleanest pattern:
    add an internal helper `fn unauthenticated_client(&self) -> GitHub` that
    returns a `GitHub` client variant with no credentials and no env_auth.
    Then in `list_pull_requests` (and any other public methods this review
    flags), wrap the call:
    ```text
    match self.client.request(req.clone()).await {
        Err(SchematicError::AuthenticationRequired { .. })
        | Err(SchematicError::MissingCredential { .. }) => {
            // Retry once with the explicit unauthenticated client
            self.unauthenticated_client().request(req).await
        }
        other => other,
    }
    .map_err(map_schematic_error)
    ```
    Then in `map_schematic_error`, the `MissingCredential` arm is only
    reachable when *both* attempts failed (or when an explicit-only API
    rejected anonymous use), which is correct per spec section 5.
  - The schematic `GitHub` client supports `variant().env_auth(vec![]).build()`
    (see `schematic/schema/src/github.rs` lines 1837 / 2007 / 2125).
    `env_auth(vec![])` produces a client whose env-fallback list is empty,
    which together with no explicit headers produces an unauthenticated
    request.
- `sniff/lib/src/remote/gitea.rs`
  - In `list_pull_requests` (around line 426), replace
    `labels: Vec::new()` with
    `labels: pr.labels.unwrap_or_default().into_iter()
        .filter_map(|l| l.name).collect()`
    (or equivalent if the regenerated Gitea `Label` field shapes are
    different).
  - Apply the same unauthenticated-fallback refactor to `map_schematic_error`
    / `list_pull_requests` as for GitHub. Note Gitea's existing 401 mapping
    is more eager than GitHub's: it converts 401 directly to
    `SniffError::MissingCredentials` (gitea.rs:167). Per spec, a 401 response
    is the correct moment to surface missing credentials, so retain that
    behavior — but only after the anonymous attempt has been made.
- `sniff/lib/src/remote/gitlab.rs`
  - GitLab already populates labels (mr.labels at gitlab.rs:385); no labels
    change needed.
  - Apply the same unauthenticated-fallback refactor. Note that GitLab's 401
    mapping (gitlab.rs:124) currently maps to `MissingCredentials` — this is
    correct per spec for the post-attempt path.
- `sniff/lib/src/remote/bitbucket.rs`
  - Decide between options A and B for body:
    - **Option A (recommended): document the limitation.** Leave
      `body: None` in `list_pull_requests` and add a doc comment explaining
      that Bitbucket's list API does not return PR descriptions. Update the
      verbose CLI rendering (`sniff/cli/src/output/remote.rs`) to fall back
      gracefully when `body` is `None` (it likely already does — confirm).
    - **Option B: follow-up fetch when verbose.** Add an
      `async fn get_pull_request(&self, ws, slug, id)` that hits
      `GetPullRequestRequest` (already exists in the schema —
      see `schematic/definitions/src/bitbucket/mod.rs` and
      `schematic/schema/src/bitbucket.rs:379`). The trait must add the
      method (see optional change in `provider.rs` above). The CLI must
      orchestrate the follow-up only when `verbose > 0`.
  - **Default to Option A** unless the implementer has time for Option B
    in this phase. Option B can be a Phase 5 follow-on.
  - Map `priority`/`kind` to `labels`: **NOT applicable to Bitbucket PRs.**
    Inspection of `schematic/definitions/src/bitbucket/types.rs:330` shows
    that `priority` and `kind` exist on `Issue` (line 605) but **not** on
    `PullRequest`. The review's suggestion was based on a mistaken read.
    Add a brief comment in `bitbucket.rs::list_pull_requests` (next to
    `labels: Vec::new()`) explaining: "Bitbucket Cloud's PR API does not
    expose labels, priority, or kind — those fields exist on Issues only."
  - Apply the same unauthenticated-fallback refactor as the other providers.
    Bitbucket uses Basic auth (`BITBUCKET_USERNAME` +
    `BITBUCKET_APP_PASSWORD`), so the unauthenticated variant is
    `Bitbucket::with_base_url(&base_url)` followed by
    `.variant().env_auth(vec![]).build()`.
- `sniff/lib/src/error.rs` — review variants
  - The current `SniffError::MissingCredentials` is the right error to surface
    *after* an authenticated retry fails. No new variant is required.
  - **Optional:** if the implementer wants to differentiate
    "401 from authenticated request" vs "anonymous request rejected, supply
    credentials", consider whether `InvalidCredentials` is the right variant
    for the former (it already is). Document the decision in a comment near
    the refactored `map_schematic_error`.

### Step-by-step tasks

1. (Trait, optional) Add `get_pull_request` to `RemoteRepoProvider` with a
   default `Err(...status:501...)` implementation. Skip this step if
   sticking with Option A for Bitbucket body.
2. GitHub: add `unauthenticated_client()` helper, populate `labels`, wire
   the retry-on-missing-credential pattern in `list_pull_requests`.
3. Gitea: same as GitHub, accounting for `Option<Vec<Label>>` and
   per-label `Option<String>` name.
4. GitLab: same retry-on-missing-credential pattern; no labels change.
5. Bitbucket: same retry-on-missing-credential pattern; add the explanatory
   comment about labels/priority/kind. If implementing Option B for body,
   override `get_pull_request` to call `GetPullRequestRequest` and map
   `pr.description` to `body`.
6. Update inline rustdoc on `map_schematic_error` in all four files to
   describe the new contract: `MissingCredentials` is only emitted *after*
   the anonymous retry fails or when the API explicitly demands creds for a
   private resource.

### Tests to add or update

Phase 2 makes provider behavior changes; tests live in Phase 3 (so the
test-coverage step gets its own self-contained, focused phase). However,
this phase must still keep existing tests green:

- `sniff/lib/tests/remote_providers.rs` — existing tests must still pass.
  The GitLab assertion at line 731–732 (already verifies labels + body
  populated) must continue to pass unchanged.
- The GitHub `list_pull_requests_success` test at line 270 currently
  asserts `prs[0].labels.is_empty()` — this assertion is **wrong** after
  Phase 2 because the fixture at line 88 already includes
  `labels: [{"name": "enhancement"}, {"name": "help wanted"}]`. Update
  this assertion in Phase 3 alongside the other test work; for Phase 2,
  temporarily either:
  - leave it (Phase 2 will go red on this single test), and rely on Phase 3
    to make it green again — but document that Phase 2 ends with this one
    failing test, OR
  - update the assertion as part of Phase 2 (cleaner; recommended).
  **Preferred:** change the assertion in Phase 2 to
  `assert_eq!(prs[0].labels, vec!["enhancement".to_string(), "help wanted".to_string()])`
  so Phase 2 ends fully green.
- The Bitbucket `list_pull_requests_success` assertion at line 1521–1522
  (`assert!(prs[0].labels.is_empty()); assert!(prs[0].body.is_none());`) is
  correct under Option A; leave it. If Option B is chosen the body
  assertion needs Phase-3 reworking.
- The Gitea `list_pull_requests_success` assertion at line 1114
  (`assert!(prs[0].labels.is_empty())`) — the fixture at line 991 has
  `"labels": []`, so this assertion stays correct after Phase 2 (empty
  schema-backed `labels` deserializes to an empty vec). Phase 3 will add a
  separate fixture/test that exercises a non-empty Gitea `labels` array.

### Verification

```bash
# Sniff lib must build, test, and lint clean
cargo build -p sniff
cargo test -p sniff
cargo clippy -p sniff -- -D warnings

# Sniff CLI also has to keep building (it depends on sniff::remote)
cargo build -p sniff-cli
cargo test -p sniff-cli
just -d sniff lint
```

### Phase 2 done when

- Labels populate end-to-end for GitHub, GitLab, Gitea (Bitbucket continues
  to return empty with the explanatory comment).
- All four providers attempt anonymous on `MissingCredential` /
  `AuthenticationRequired` and only surface
  `SniffError::MissingCredentials` after the anonymous retry fails or
  after a real 401/403 response.
- `cargo test -p sniff` passes (with the GitHub assertion update above).
- `just -d sniff lint` reports zero warnings.

---

## Phase 3 — Tests: labels, body, and unauthenticated-fallback coverage

**Goal:** Backfill the gaps the review called out under "Test Coverage"
(review item 4) so the new provider behavior is regression-protected.

### Files to change

- `sniff/lib/tests/remote_providers.rs` — extend tests for all four providers
  and add a new `unauth_fallback_tests` module.

### Step-by-step tasks

1. **GitHub label coverage**
   - In `mod github_tests`, ensure the existing
     `list_pull_requests_success` assertion now reads:
     ```text
     assert_eq!(
         prs[0].labels,
         vec!["enhancement".to_string(), "help wanted".to_string()]
     );
     ```
     (If Phase 2 already updated it, just confirm.)
   - Add a `list_pull_requests_no_labels` test using a fixture with
     `"labels": []` and assert `prs[0].labels.is_empty()`.
   - Add a `list_pull_requests_omitted_labels` test using a fixture that
     omits the `labels` key entirely (proves `#[serde(default)]` works).

2. **Gitea label coverage**
   - Add a fixture variant `gitea_pull_requests_with_labels_fixture()` that
     includes
     `"labels": [{"name": "feature"}, {"name": "good first issue"}]`.
   - Add `list_pull_requests_with_labels` test asserting the labels round-trip
     into `PullRequestInfo.labels`.
   - Keep the existing `list_pull_requests_success` test (with empty labels)
     to assert the no-labels path still works.

3. **GitLab label & body coverage (already partially present)**
   - Strengthen the existing `list_pull_requests_success` (around line 727)
     to also assert
     `assert_eq!(mrs[0].body.as_deref(), Some("Implements feature X as described in #123."))`
     if not already (line 732 already does this — confirm).

4. **Bitbucket body coverage**
   - **If Option A in Phase 2:** add a `list_pull_requests_body_is_none` test
     that documents the limitation. The existing assertion at line 1522
     already does this; convert the magic into an explicit test name.
   - **If Option B in Phase 2:** add a `get_pull_request_populates_body` test
     that mocks `/repositories/{ws}/{slug}/pullrequests/{id}` returning a
     fixture with `"description": "Detailed body"`, then invokes
     `provider.get_pull_request(...)` and asserts
     `pr.body == Some("Detailed body".to_string())`.

5. **Unauthenticated-fallback module: `mod unauth_fallback_tests`**
   - For each of the four providers, add a test
     `list_pull_requests_falls_back_to_unauthenticated_when_token_missing`:
     1. `unsafe { std::env::remove_var(<TOKEN_VAR>) }` to simulate no creds.
        For GitHub remove both `GITHUB_TOKEN` and `GH_TOKEN`. For Gitea
        remove `GITEA_TOKEN` (and `CODEBERG_TOKEN` if applicable). For
        GitLab remove `GITLAB_TOKEN` and `GITLAB_PRIVATE_TOKEN`. For
        Bitbucket remove both `BITBUCKET_USERNAME` and
        `BITBUCKET_APP_PASSWORD`.
     2. Mount a wiremock that responds 200 with a minimal PR fixture
        regardless of the `Authorization` header (use no header matcher).
     3. Invoke `provider.list_pull_requests(...)` and assert it returns
        `Ok(prs)` non-empty. The assertion proves the provider did not
        short-circuit to `MissingCredentials` and successfully retried
        anonymously.
   - For each provider, add a paired test
     `list_pull_requests_returns_missing_credentials_when_anonymous_rejected`:
     1. Same setup (no creds in env).
     2. Mock returns 401 (or 403 with rate-limit body) for the PR endpoint.
     3. Assert the result is
        `Err(SniffError::MissingCredentials { provider, .. })` for 401 or
        `Err(SniffError::RateLimited { provider, .. })` for 403 rate
        limited. This proves the post-attempt error surfacing still works.
   - Use `#[serial_test::serial]` (`serial_test` is already a dev-dep used
     elsewhere in the workspace) to prevent env-var races between these
     tests and the existing tests that *set* env vars.
   - **Important:** the existing `setup_*_mock` helpers in this file *set*
     credentials. The new fallback tests must **not** call those helpers —
     they need a separate `setup_*_mock_no_creds` variant that just builds
     the mock + provider without touching env vars.

### Verification

```bash
cargo test -p sniff --test remote_providers
cargo test -p sniff           # full sniff suite
cargo test -p sniff-cli       # CLI tests must still pass
just -d sniff lint
just -d sniff test
```

### Phase 3 done when

- All new label/body assertions pass for GitHub, GitLab, Gitea, Bitbucket.
- All four `..._falls_back_to_unauthenticated_when_token_missing` tests
  pass.
- All four `..._returns_missing_credentials_when_anonymous_rejected` tests
  pass.
- `just -d sniff lint` is clean.

---

## Phase 4 — CLI surface, docs, and platform limitations

**Goal:** Polish the CLI experience around the new behavior:

- Update help text so `--status draft` documents the Bitbucket limitation
  (review item: "Broken or Incomplete Features → Draft Filtering on
  Bitbucket").
- Confirm verbose rendering shows labels (now populated for GitHub/Gitea/GitLab).
- Confirm error messages still match spec section 5 when the post-anonymous
  401 path triggers.
- Update relevant docs.

### Files to change

- `sniff/cli/src/args.rs`
  - Find the `--status` flag definition for the `repo pr` subcommand and
    extend its help string with a parenthetical note:
    `"... (note: 'draft' returns no results on Bitbucket — drafts are not
    a Bitbucket Cloud feature)"`.
- `sniff/cli/src/commands.rs`
  - Re-read the `handle_pr_command` body (lines 949–1043). The error mapping
    for `MissingCredentials` (line 978) now triggers only after the
    anonymous attempt has failed; confirm the error message still reads
    sensibly. Recommended update:
    `"{} requires credentials for this resource: set the {} environment
    variable"` so the user understands the unauthenticated attempt was
    already tried.
  - The `InvalidCredentials` arm (line 985) now triggers when a token *was*
    provided and rejected; the existing message is fine.
- `sniff/cli/src/output/remote.rs`
  - Verify `render_pull_requests_verbose` includes a `Labels:` row and
    handles the empty case ("Labels: (none)" or omit the row). If absent,
    add it consistent with the spec's verbose mockup
    (spec.md lines 49–60).
- `sniff/cli/README.md`
  - Add a short section documenting:
    - Auth strategy: anonymous attempt first, credentials prompted only on
      401/403/rate limit.
    - Bitbucket draft-PR limitation (returns empty list).
    - Bitbucket verbose body: either "see follow-up GET" (Option B) or
      "list endpoint does not return descriptions" (Option A).
- `.claude/skills/sniff/SKILL.md` (only if behavior change is user-visible
  enough to warrant — likely yes, the auth strategy is)
  - Under the existing `Remote` row in the Capabilities table, the auth
    strategy might warrant a one-line update if the SKILL docs cover it.
    Skip if no existing mention.

### Step-by-step tasks

1. Update the `--status` help string in `sniff/cli/src/args.rs`.
2. Update the `MissingCredentials` error message in `sniff/cli/src/commands.rs`
   to convey the "anonymous attempt failed" context.
3. Inspect `render_pull_requests_verbose` in `sniff/cli/src/output/remote.rs`.
   Add a `Labels:` line if missing; ensure empty labels render as either
   "(none)" or are omitted entirely (pick one and stay consistent with how
   the function handles other Option-ish fields like body).
4. Add a snapshot/golden test or augment the existing CLI tests
   (`sniff/cli/tests/`) to cover:
   - Verbose rendering shows the labels line (use a `PullRequestInfo`
     fixture with `labels: vec!["bug".into(), "enhancement".into()]`).
   - `--help` for `sniff repo pr` shows the new `--status draft (Bitbucket
     unsupported)` text.
5. Update `sniff/cli/README.md` with the auth-strategy and platform-
   limitation section.
6. Run `cargo doc -p sniff-cli` (or `just -d sniff/cli docs`, if a recipe
   exists) to confirm rustdoc renders without warnings.

### Tests to add or update

- `sniff/cli/tests/*` — add or extend a CLI integration test that runs
  `sniff repo pr --help` and asserts the output mentions `Bitbucket`
  somewhere in the `--status` description.
- `sniff/cli/tests/*` — add a render test using a
  `PullRequestInfo` fixture with non-empty labels and a non-empty body to
  assert the verbose output includes both. If the existing rendering tests
  already assert this, just extend the fixture.

### Verification

```bash
cargo build -p sniff-cli
cargo test -p sniff-cli
cargo doc -p sniff-cli --no-deps      # rustdoc warning sweep
cargo clippy -p sniff -p sniff-cli -- -D warnings
just -d sniff test
just -d sniff lint
```

### Phase 4 done when

- `sniff repo pr --help` mentions the Bitbucket draft limitation.
- Verbose output includes labels.
- `MissingCredentials` error message reflects the new attempt-then-prompt
  flow.
- README and any user-facing docs reference the auth strategy.
- Full `sniff` package area builds, tests, lints clean.

---

## Risks & open questions

1. **Schema regeneration risk.** Phase 1 modifies `schematic/definitions` and
   then regenerates `schematic/schema`. The generator is invoked via
   `just -d schematic generate-{one|all}`. If the generator emits unrelated
   churn (formatting, comment ordering), the diff for Phase 1 could be much
   larger than the conceptual change. Mitigation: review the regenerated
   diff carefully and split unrelated churn into its own commit.

2. **Gitea `Label` type may not exist.** If `schematic/definitions/src/gitea`
   has no `Label` type, Phase 1 needs to add one. The exact shape (name only?
   color too?) should match Gitea's actual API response. Worth a quick
   `curl https://codeberg.org/api/v1/repos/forgejo/forgejo/pulls?limit=1` to
   confirm the field shape before defining it. **Open question.**

3. **Bitbucket body — Option A vs Option B.** Option A (document limitation)
   is faster and lower risk. Option B (per-PR follow-up when `-v`) is more
   useful for users but requires a trait change, an N×latency cost in verbose
   mode, and orchestration in the CLI. The plan assumes Option A by default;
   confirm with the user before doing Option B.

4. **Bitbucket `priority`/`kind` mapping.** The review suggested mapping these
   to `labels`, but inspection shows they exist only on `Issue`, not on
   `PullRequest`. The plan rejects this suggestion and instead documents
   the limitation. **If the user disagrees**, an alternative is to fetch
   each PR's *associated issues* (Bitbucket has weak issue↔PR linkage) and
   pull labels from there, but this is high-cost low-value.

5. **`unsafe { std::env::set_var }` in tests.** The existing tests use this
   pattern, and the new fallback tests need it too (to *remove* env vars).
   Rust 1.80+ deprecates direct env mutation, but this codebase has accepted
   the wart. The new tests should follow the same `#[serial_test::serial]`
   discipline as existing ones.

6. **Worktree-local debug edits.** `schematic/define/src/headers.rs` and
   `schematic/schema/src/gitea.rs` already have unrelated local changes
   (one debug `eprintln!`, one functional Headers tweak). These are *not*
   part of this review and must be committed/reverted separately. The
   `schematic/schema/src/gitea.rs` `eprintln!` will be **overwritten** when
   Phase 1 regenerates that file — verify before regenerating that the
   developer doesn't lose unrelated work.

7. **The `unauthenticated_client()` helper name.** Each provider needs its
   own `Self -> Self` helper. Consider whether to factor this into a small
   trait/method on the schematic clients themselves rather than duplicating
   per provider — the schematic builder pattern supports
   `.variant().env_auth(vec![]).build()` directly, so each provider's
   helper is essentially one line. Probably not worth a shared abstraction.

8. **Phase ordering trade-off.** Phase 2 makes provider behavior changes
   without adding new tests, then Phase 3 adds the tests. An alternative
   ordering interleaves test + code per provider. The current ordering
   makes each phase's diff smaller and easier to review, at the cost of
   one fail-fix-pass cycle on the GitHub-labels assertion (handled in the
   plan). Decide based on reviewer preference.
