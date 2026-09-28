---
area: sniff
status: unscheduled
created: 2026-09-26
owner: Ken Snyder <ken@ken.net>
origin: ruling R4 of 2026-09-26-lockfile-corroboration
related:
    - 2026-09-26-lockfile-corroboration
packages:
    - sniff
---

# Package-manager detection labels uv, Poetry, and PDM projects as pip

## Problem

The Sniff library's `detect_package_managers`
(`sniff/lib/src/filesystem/repo/detection.rs`) fills
`PackageInfo::package_managers`, a list of labels such as `cargo`, `pnpm`, and
`pip`. For Python, it looks only for `requirements.txt` or `pyproject.toml` and
then always reports `pip`.

A project managed by uv (with a `uv.lock`), Poetry (`poetry.lock`), or PDM
(`pdm.lock`) is therefore reported as `pip`. That label is wrong, and it is
visible in `sniff repo` output.

JavaScript detection has a related gap. It recognizes `pnpm-lock.yaml` and
`yarn.lock`, and treats everything else with a `package.json` as `npm`. Bun
projects (`bun.lock` or `bun.lockb`) are reported as `npm`.

## Why this is separate

`2026-09-26-lockfile-corroboration` answers a different question: does a
workspace's lockfile agree with the manifest about which packages are members?
Its ruling R4 keeps package-manager labels out of that feature, because tool
identity and membership evidence use different selection rules. For example, a
`pyproject.toml` without any lockfile still has a tool label but has no
lockfile observation.

## Scope

- Detect uv, Poetry, and PDM from their lockfiles, and optionally from
  `[tool.uv]`, `[tool.poetry]`, and `[tool.pdm]` tables that are already parsed.
  Keep `pip` for projects with no stronger signal.
- Detect Bun from `bun.lock` or `bun.lockb`.
- Where the lockfile-corroboration feature's request-local presence cache
  already holds a probe result for the same path, reuse it rather than probing
  again.
- Decide whether a package inside a workspace inherits the workspace root's
  lockfile-derived label, or keeps only its own directory's evidence (today:
  its own directory only).

## Out of scope

- Workspace membership corroboration. That belongs to
  `2026-09-26-lockfile-corroboration`.
- Executing any package manager.

## Closes when

- A uv, Poetry, PDM, and Bun fixture each report the right label, and a plain
  `requirements.txt` project still reports `pip`.
- Work counters show no additional metadata probes beyond the new candidate
  names checked at each package root.
