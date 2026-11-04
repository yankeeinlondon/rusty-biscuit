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

Level 1 `sniff-detection-cli` integration tests obtain the binary through
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

`filesystem query` CLI tests (`cli/tests/l1/filesystem_query.rs`) replay a
report built from library types through the hidden
`SNIFF_FILESYSTEM_QUERY_REPLAY` variable (a JSON file path). That is how
they compare text with JSON for one observation and produce `unavailable`
and `unsupported` outcomes on any host. The fixture scrubs inherited
`SNIFF_*` variables, so set it per command.

Keep each L1 test well under the 5 s budget on native Windows, where a fresh
file costs milliseconds (`rust-testing`'s `test-speed.md`). Patterns already in
use here: the shared walk's inventory cap is injected through
`build_filesystem_system_view_with_cap`, so saturation tests use a cap of 48
rather than writing 10,000 files, and a separate test pins that the production
entry point does not use the test cap; repository-level docs tests build a
small temporary Git workspace instead of hashing every Markdown file in the
ambient monorepo (and `test_detect_completes_in_reasonable_time` runs
`detect()`'s defaults with a fixture repository as `base_dir`); the `no_membership_field_shape_corroborates_…` matrix runs
one test per row, with a guard test that every row in
`MEMBERSHIP_SHAPE_ROWS` has one; and `build_large_monorepo` writes its Git
objects through an in-memory pack and its churn commits without restaging
the tree (identical commit IDs, about 2 s instead of 6.5 s on Windows).

On macOS the dominant L1 costs are per process, so a `LazyLock` cannot share
them: the first CoreAudio call sets up the audio HAL, and that setup is
serialized across processes (about 0.75 s each on Ken's Mac, so four tests
started together each took about 4 s and, in the old suite, 6–9 s), and
`NetworkRequest::full()` (the `detect_network()` and `DetectionPlan`/`SniffConfig`
default) issues a live WAN IP request bounded at 2 s per endpoint. A test that
does not assert audio devices or the WAN address requests
`HardwareRequest::summary()` (or `.include_audio(false)`) and
`NetworkRequest::interfaces_only()`. `network::tests` reads interfaces through
`local_network()`/`local_network_filtered()`, and the `SniffConfig` key-shape
tests lower kept domains through a `cheap_tiers(config)` helper that keeps the
config's skip flags. The full default path keeps exactly these live callers:
`tests::test_detect_returns_result`, `integration::test_detect_completes_in_reasonable_time`,
`network::tests::test_detect_network_returns_info`, the two
`hardware::audio` detection tests, and
`hardware::tests::test_detect_hardware_full_includes_gpu_on_macos`; add another
only when it asserts what those calls return.
