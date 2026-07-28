# CI area policy — `areas.json`

`areas.json` is the single policy surface for **every** package area in the repo.
Each area `sniff repo package-areas` discovers has exactly one record, whether or
not it gates in CI. Each record describes one area; `scripts/ci/affected_scope.py` validates every record
against the schema below (via `validate_area_schema`) and fails loudly on a
missing required field, an unknown field, an unsupported runner OS, an unknown L2
backend, or a mistyped value.

Cargo metadata — not this file — remains the source of truth for package
membership. `areas.json` only assigns policy to the curated areas, and its `area`
order must match the root `justfile` `areas :=` list.

## Fields

| Field | Type | Required | Default | Meaning |
|-------|------|----------|---------|---------|
| `area` | string | yes | — | Area directory; used for `cd`, cache keys, and ownership. |
| `ci` | bool | no | `true` | Whether the area gates in the fan-out matrix. |
| `check_args` | string | when `ci` | — | Args for the macOS compile-check `cargo check --all-targets` (e.g. `-p foo -p foo-cli`). |
| `reason` | string | when not `ci` | — | Why the area does not gate, and what would unblock it. |
| `full_os` | string[] | no | `["ubuntu-latest","windows-latest"]` | Runner OSes that run the full L1 suite. |
| `check_os` | string[] | no | `["macos-latest"]` | Runner OSes that only compile-check. |
| `soft_os` | string[] | no | `["windows-latest"]` | Test-leg OSes whose failures are advisory (`continue-on-error`). Must not list an OS the area's cross-platform contract requires to gate. |
| `shards` | string[] | no | `["1/1"]` | nextest `--partition count:i/N` specs; `["1/1"]` = no sharding. Sized from measured cold-run duration. |
| `l2` | bool | no | `false` | Run the real-terminal (L2) tier on Linux. |
| `browser` | bool | no | `false` | Run the headless-browser tier on Linux. |
| `kache` | bool | no | `true` | Enable the kache `RUSTC_WRAPPER` (Linux/macOS only — `kache-action@v1` rejects `win32-x64`). |
| `ai_provider_stubs` | bool | no | `false` | Install inert AI-provider CLI stubs for tests needing provider discovery. |
| `backends` | string[] | no | `[]` | L2 terminal backends this area's tests require. One of: `tmux`, `wezterm`, `kitty`, `apple-terminal`. Required (non-empty) when `l2` is set: the L2 job exports `BISCUIT_REQUIRED_BACKENDS` as this list intersected with `affected_scope.py`'s `HOSTABLE_L2_BACKENDS`, turning those backends' skips into failures. |
| `native` | object | no | `{}` | Map of runner OS → system packages needed to build/test (e.g. `{"ubuntu-latest": ["libasound2-dev"]}`). |
| `canary` | bool | no | `false` | Whether this area is a global-change canary (Phase 4). |

Supported runner OS values: `ubuntu-latest`, `windows-latest`, `macos-latest`.

`native` has exactly one installer: the root `justfile`'s `_ensure-native-libs`.
CI runs `just _ensure-native-libs <area>` before every build, test, and lint
command so a `-sys` crate never fails to compile for a missing system library,
and `just init` runs it with no argument to cover every area on a developer host.
A new requirement is therefore declared once and installed by one implementation.
Non-Debian Linux hosts need the apt name mapped to `dnf` / `pacman` / `apk` in
that recipe's table.

## Choosing canaries

A global change runs the `canary` areas before the rest fan out, and a canary
failure blocks that fan-out (D11). That only produces signal if the canary area
is **otherwise green**: the canary must fail because the shared change broke it,
not because the area already had failing tests. A red canary is worse than no
canary, because it hides every other area's result behind a known failure.

Current set: `biscuit-hash` (pure Rust, fast) and `playa` (native dependencies).

- `darkmatter` is the intended heavy/sharded canary and should be re-added once
  its L1 suite is green.
- Do **not** use `homelab` or `research` as canaries.

Keep the set small — it is a serial stage in front of everything else.

## Ownership completeness

Every Cargo workspace member (per `cargo metadata`) must live under an area that
has a record here. `validate_ownership` fails the scope calculation — naming the
package — when one does not, so a package in a brand-new directory cannot land
without someone deciding what its area is and whether it should gate.

Areas that do not gate set `"ci": false` and give a `reason`. They still declare
`native`, still confer ownership, and still appear to `just _ensure-native-libs`;
they simply launch no area job. Current non-gating areas fall into three groups:

- **Internal test infrastructure** — `tools` (test-toolkit),
  `biscuit-test-harness`, `biscuit-browser-harness`. Not public-facing; exercised
  transitively by every consumer that dev-depends on them. These are not expected
  to gain their own matrix entry.
- **Real areas awaiting promotion** — `messenger`, `biscuit-visualized`. The
  blocker is the canonical `just` recipe set (they define only `test`/`lint`),
  which `check-canonical` requires. Complete the recipes, flip `ci` to `true`,
  and add the area to the root justfile's `areas :=` list.
- **Not ready** — `visualizer`, `biscuit-clipboard`, `reaper` (not stabilized),
  `agent-sandbox` (experimental), `tabby` (a stub).

Only gating areas appear in the root justfile's `areas :=` list;
`validate_area_config` keeps the two in step.

## Adding or changing an area

1. Add/adjust the record here **and** keep the `area` order aligned with the root
   `justfile` `areas :=` list.
2. Shard-count, backend, native, and OS-policy changes require evidence
   (measured durations, real backend/native requirements) — not guesses.
3. `python3 scripts/ci/test_affected_scope.py` and
   `cargo nextest run -p test-toolkit --test ci_workflow_contracts` must pass.
