---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-24-ux-improvements/spec.md
plan: worktree/fixes/2026-09-24-ux-improvements/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
docs_created_during_phase_1:
    - worktree/fixes/2026-09-24-ux-improvements/spike-s1.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s2.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s3.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s4.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s5.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
    - .claude/skills/os/build-hosts.md
packages:
    - worktree
---

# Implementation Log for 2026-09-24-ux-improvements (6 phases)

## Phase 1

Phase 1 is rulings and spikes only; no production code changes.

### Rulings (R1–R11)

- Recorded as **proposed** Decisions 20–30 in `spec.md`, amended where a spike changed the answer. Three further proposals came from the spikes: 31 (ignored-entry display), 32 (graph natural-width API), and 33 (PowerShell output encoding).
- The spec is set to `human_review: true`. The plan's validation checkpoint stays unchecked until the author confirms Decisions 20–33.
- R1: `getrandom` 0.4 is already in the workspace graph (Darkmatter), so `worktree` adds no new crate.
- R9: `cache::cache_path` returns a file, not a directory, so the new records are sibling files keyed by the same repository hash. Callers already prefer the main worktree's path (`cache.rs:53`).

### Spikes

- **S1** (subagent, `spike-s1.md`): every provider exposes the source repository, head SHA, and state. Only Bitbucket Cloud abbreviates SHAs (12 characters).
  - R8 is confirmed with three amendments: compare against the local object-ID length, require at least 7 hex characters, and check the prefix is unique locally.
  - Blocking defect for Phase 2: sniff's focused client turns a list-endpoint 404 into an empty list.
  - Gitea's single-PR endpoint reports the live branch tip, even after merge.
- **S2** (`spike-s2.md`, run on macOS, `$BUILD_LINUX`, and `$BUILD_WIN`):
  - No prompt appeared in any case with `GIT_TERMINAL_PROMPT=0`, `BatchMode=yes`, and `credential.interactive=never`. The cases covered HTTPS without credentials, an unknown host key, and a passphrase key without an agent.
  - A plain kill leaves the transport grandchild holding stderr on every OS. On Windows the real `git.exe` is also orphaned and holds its working directory.
  - Killing the process group (Unix) or `taskkill /T /F` (Windows) released everything at the 3 s deadline.
- **S3** (`spike-s3.md`): an `inquire` probe ran under zsh, bash, and fish wrappers in detached tmux sessions.
  - Prompts render on stderr, the variable is scoped to one invocation, and non-ASCII paths with spaces arrive exact.
  - The token reaches `wt` as one literal argument; a `$(…)` payload inside it never ran.
  - A failed `cd` stops the wrapper before the handoff.
  - PowerShell 5.1 on `$BUILD_WIN` corrupts non-ASCII captured output unless `[Console]::OutputEncoding` is UTF-8.
  - POSIX wrappers must act after the read loop, so the handoff call keeps the terminal's stdin.
  - fish 4.9.3 was installed on this macOS host with Homebrew for the spike.
- **S4** (`spike-s4.md`):
  - `--ignored=matching` collapses only directories matched by a directory pattern. This repository's `**/target/*` lists the children of `target/`, which drives proposed Decision 31.
  - Cost: 0.07 s against 0.03 s for a plain status on this checkout. `traditional` takes 0.20 s.
  - The Windows rename probe costs about 1.5 ms per pair, and a held directory fails on the first rename in 2 ms.
- **S5** (subagent, `spike-s5.md`):
  - The sizing API is `measure_svg_dimensions` (`viewbox_width`), not `render_svg_with_dimensions`.
  - `Terminal::cell_size()` exists; on native Windows it is always `None`.
  - Rows fall through stdout, stderr, and stdin, which confirms R10.
  - 0.3.1 rejects Mermaid input with no header, which is a Phase 4 fixture risk.

### Skill updates

- `.claude/skills/os/windows.md`: Windows `git` deadline kills need a tree kill; PowerShell 5.1 capture encoding; lock-probe cost.
- `.claude/skills/os/build-hosts.md`: `tar` on `$BUILD_WIN` is Cygwin's; use System32 `tar.exe`.
- The `worktree` skill is unchanged, because no worktree behavior changed in this phase.

### Tests and gates

- No test was added or changed: Phase 1 has no behavior change. Its evidence is the spike notes, and each probe was a throwaway crate outside the repository, now deleted.
- `just test` (worktree): 149 passed, 11 skipped. `just lint` (worktree): clean.
- Scratch probes were removed from `$BUILD_LINUX` and `$BUILD_WIN` (`B:\scratch-s2probe`, `B:\scratch-s4probe`).
