# CI area policy — `areas.json`

`areas.json` is the single policy surface for the curated CI package areas. Each
record describes one area; `scripts/ci/affected_scope.py` validates every record
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
| `check_args` | string | yes | — | Args for the macOS compile-check `cargo check --all-targets` (e.g. `-p foo -p foo-cli`). |
| `full_os` | string[] | no | `["ubuntu-latest","windows-latest"]` | Runner OSes that run the full L1 suite. |
| `check_os` | string[] | no | `["macos-latest"]` | Runner OSes that only compile-check. |
| `soft_os` | string[] | no | `["windows-latest"]` | Test-leg OSes whose failures are advisory (`continue-on-error`). Must not list an OS the area's cross-platform contract requires to gate. |
| `shards` | string[] | no | `["1/1"]` | nextest `--partition count:i/N` specs; `["1/1"]` = no sharding. Sized from measured cold-run duration. |
| `l2` | bool | no | `false` | Run the real-terminal (L2) tier on Linux. |
| `browser` | bool | no | `false` | Run the headless-browser tier on Linux. |
| `kache` | bool | no | `true` | Enable the kache `RUSTC_WRAPPER` (Linux/macOS only — `kache-action@v1` rejects `win32-x64`). |
| `ai_provider_stubs` | bool | no | `false` | Install inert AI-provider CLI stubs for tests needing provider discovery. |
| `backends` | string[] | no | `[]` | L2 terminal backends this area's tests require. One of: `tmux`, `wezterm`, `kitty`, `apple-terminal`. |
| `native` | object | no | `{}` | Map of runner OS → system packages needed to build/test (e.g. `{"ubuntu-latest": ["libasound2-dev"]}`). |
| `canary` | bool | no | `false` | Whether this area is a global-change canary (Phase 4). |

Supported runner OS values: `ubuntu-latest`, `windows-latest`, `macos-latest`.

`native` has a second consumer outside CI: the root `justfile`'s
`_ensure-native-libs` (a `just init` prerequisite) provisions developer hosts
from the same declaration, so a new requirement is declared once. Non-Debian
Linux hosts need the apt name mapped to `dnf` / `pacman` / `apk` in that
recipe's table.

## Ownership completeness (`exemptions.json`)

Every Cargo workspace member (per `cargo metadata`) must be owned by exactly one
of: a curated area (it lives under an `area` directory), or an explicit exemption
in `.github/ci/exemptions.json`. `validate_ownership` fails the scope calculation
— naming the offending package — for an unmapped member, a package that is both
owned and exempt, or an exemption for a package that no longer exists.

`exemptions.json` is a list of `{ "package": "<name>", "reason": "<why>" }`.
Reasons are required and non-empty. Exemptions cover shared test-infra crates
(exercised transitively), experimental/unstable packages, and real areas whose
justfiles do not yet define the full canonical recipe set (promote them by
completing the recipes, adding them to the `areas` list + this file, and removing
the exemption).

## Adding or changing an area

1. Add/adjust the record here **and** keep the `area` order aligned with the root
   `justfile` `areas :=` list.
2. Shard-count, backend, native, and OS-policy changes require evidence
   (measured durations, real backend/native requirements) — not guesses.
3. `python3 scripts/ci/test_affected_scope.py` and
   `cargo nextest run -p test-toolkit --test ci_workflow_contracts` must pass.
