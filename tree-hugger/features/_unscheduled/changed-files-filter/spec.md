# Spec: `--changed` File Selection for Symbol Reports

## Problem

There is no direct way to ask "what declarations are in the files I have
changed?" A user must list changed files by hand and pass them to `hug`.

## What was proposed

Proposed as item C8 in the pre-split draft of `2026-10-03-symbol-improvements`,
and moved here by the author on 2026-10-03.

`--changed[=<ref>]` restricts scanning to the union of tracked files whose
current contents differ from the supplied commit, staged/unstaged changes, and
nonignored untracked files.

- The default reference is `HEAD`: "what declarations are in my uncommitted
  changed files?" `--changed=main` compares against the local `main` tip,
  including committed branch changes. It compares that tip directly, not the
  branch's common ancestor, and does not filter individual changed lines.
- Use the equals form for the optional reference so a positional file/name
  token is not consumed as a Git reference.
- Inspect Git through sniff's focused APIs, not full host discovery, remote
  refresh or shell-parsed status. sniff's `GitRepo` status API
  (`sniff/lib/src/filesystem/git/types.rs`) provides working-tree state.
  No fetch is permitted.
- Invalid refs, missing repositories, merge conflicts or Git failures are
  actionable errors, not an empty successful selection. An unborn `HEAD` uses
  an empty base only for the default reference.
- Intersect changed files with normal language/path/exclusion rules. Parse
  current files only: skip deleted paths, use a rename's destination,
  deduplicate paths, and retain the normal explicit-path override for fixture
  exclusions. Ignored untracked files stay excluded. No changed files is a
  successful empty report, even where the scanner ordinarily reports
  `NoSourceFiles`.

## Why deferred

It is independent of the performance and filtering work and may need a sniff
API extension, which widens the change beyond the tree-hugger area.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (the explicit file selectors and
  one filter path it would combine with).
- Possibly a focused sniff API for comparing an arbitrary reference's paths
  with the working tree; that extension and its tests would be in scope, along
  with sniff's Git docs and skill (local-only cost and comparison semantics).

## Notes

- **Testing (from the pre-split draft):** temporary repositories covering
  staged, unstaged, untracked, ignored, renamed/deleted, invalid-reference and
  unborn-`HEAD` cases, plus a branch containing committed changes against a
  supplied tip.
- Git behavior is OS-sensitive; tests run on the environments CI schedules for
  the area and follow the `os` skill's path-comparison guidance.
