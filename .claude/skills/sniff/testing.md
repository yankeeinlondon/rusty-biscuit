# Sniff Testing Details

Use this reference when writing or reviewing Sniff tests, fixtures, or suite
audits. The package-area recipes and the clippy caveat are in
[SKILL.md](SKILL.md#testing-and-verification).

The CLI's `test-fixtures` feature is L2-only. Ordinary local L1 leaves it off;
`just test-l2` and CI's reusable all-tier build enable it.

Test-suite audits and performance evidence for this area run through the
shared `tools/test-audit` tool with
`sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json`, which keeps the
`remote`, `network`, and `test-fixtures` selections apart and declares the
2026-07-22 production-caching boundary no counter comparison may span; see the
`rust-testing` skill's `test-audit-tooling.md`.

Use exact original regression inputs and assert dependent projections and work
counters, not only the immediate return value. For repository/config/parser
artifacts, include passive corpus coverage plus an end-to-end CLI test through
the real shipped artifact. Do not run workspace-wide Cargo gates for a
Sniff-only change; include only dependency-derived downstream packages.

Real-tool lockfile fixtures live in `sniff/lib/tests/fixtures/lockfiles/<tool>-<version>/<case>/`,
each with a `PROVENANCE.md`; the accepted-version matrix is
`sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`. The
nested-marker walk has no fixture-directory exclusion (unlike the manifest
index's `is_fixture_manifest`), so a `.ignore` there keeps Sniff from reporting
each fixture as a layer of this monorepo. That `.ignore` also hides the files
from `rg`/Grep (use `--no-ignore`). Copy a fixture into a temporary directory
before detecting it: in place, the enclosing monorepo's `.git` and workspaces
take over.

Level 1 `sniff-cli` integration tests obtain the binary through
`cli/tests/common::SniffCliFixture`. Its default command pins disposable
cwd/home/config/cache/install roots and a bounded PATH; use the named
`host_path` and `fake_only_path` PATH escapes with a call-site `//` comment
naming the tool observed or the absence proved — `cli/tests/l1/spawn_site_guard.rs`
scans for one and fails the suite without it. The `ambient_context` CWD escape
needs no comment: it accepts only a directory the test built, inside the fixture
workspace (builder form) or the system temporary root (fluent forms), and
panics otherwise. Live-child tests use `command_std`, and parent-side Git setup
uses the fixture Git command so inherited plumbing and host Git configuration
cannot contaminate the repository.

Level 2 Sniff terminal tests run only through `just test-l2`, which serializes
the shared broker pane. After sending a command, poll the complete final
`CapturedFrame` predicate with `cli/tests/common::capture_until`; do not add a
fixed readiness sleep or treat prompt appearance alone as proof that styling
and layout finished rendering.
