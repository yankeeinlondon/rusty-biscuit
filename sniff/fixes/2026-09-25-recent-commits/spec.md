# Recent Commits

We quite recently rolled out a new version of recent commits -- see 2026-09-15-recent-commits -- but in actual usage it is VERY broken. I'm confused how testing passed at all!


## Inconsistency of Commits

I run `sniff repo git-status` and it reports the most recent commits as:

```sh
- [d935853] planning(worktree) at 11:26am Today: schedule 2026-09-25-worktree-file fix
- [6c682a7] chore(prompts) at 11:26am Today: clarify review item format, add only-bullets, and note non-focused test
  windows
- [d0f7607] test(worktree) at 11:27am Today: add Kitty graph, PowerShell remove, and styled-capture Level 2 tests
- [24f3725] planning(worktree) at 11:27am Today: close cycles 1-3 and record review-4 for ux-improvements fix
- [fc30476] docs(worktree) at 11:28am Today: note push URL behavior and update README/git-graph for Phase 6
- [75a72d0] docs(skills) at 11:28am Today: record KittyInstance, ConPTY, handoff endpoint, and fingerprint in agent skills
- [6c7ed9a] fix(worktree) at 2:15pm Today: refine wt remove safety across review cycles 4-6
- [2ce35ff] planning(worktree) at 2:16pm Today: close cycles 4-6 of ux-improvements fix
- [40b5d6f] planning(worktree) at 2:16pm Today: schedule 2026-09-25-list-remove-performance fix
- [1634e6e] planning(worktree) at 2:16pm Today (HEAD -> fix/wt-ux, fix/sniff): record execution plan for 2026-09-25-
  worktree-file
```

I then run `sniff repo recent-commits 5 --compact` expecting to see the same commits duplicated:

```sh
ken in fix-wt-ux/worktree on  fix/wt-ux [$!?]
💻❯ sniff repo recent-commits 5 --compact
- [1634e6e] planning(worktree) at 2:16pm Today: record execution plan for 2026-09-25-worktree-file
- [40b5d6f] planning(worktree) at 2:16pm Today: schedule 2026-09-25-list-remove-performance fix
- [2ce35ff] planning(worktree) at 2:16pm Today: close cycles 4-6 of ux-improvements fix
- [6c7ed9a] fix(worktree) at 2:15pm Today: refine wt remove safety across review cycles 4-6
- [75a72d0] docs(skills) at 11:28am Today: record KittyInstance, ConPTY, handoff endpoint, and fingerprint in
agent skills
```

these are two completely different lists! What is going on?


## Package Filter Does What?

I run the following query: `sniff repo recent-queries 5`: 

```sh
ken in fix-wt-ux/worktree on  fix/wt-ux [$!?]
💻❯ sniff repo recent-commits 5
- [1634e6e] planning(worktree) at 2:16pm Today: record execution plan for 2026-09-25-worktree-file
  Files Impacted:
  - added: worktree/fixes/2026-09-25-worktree-file/plan.md
  - modified: worktree/fixes/2026-09-25-worktree-file/spec.md

- [40b5d6f] planning(worktree) at 2:16pm Today: schedule 2026-09-25-list-remove-performance fix
  Files Impacted:
  - added: worktree/fixes/2026-09-25-list-remove-performance/spec.md

- [2ce35ff] planning(worktree) at 2:16pm Today: close cycles 4-6 of ux-improvements fix
  Files Impacted:
  - modified: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
  - modified: worktree/fixes/2026-09-24-ux-improvements/review-4.md
  - added: worktree/fixes/2026-09-24-ux-improvements/review-5.md
  - added: worktree/fixes/2026-09-24-ux-improvements/review-6.md
  - modified: worktree/fixes/2026-09-24-ux-improvements/spec.md

- [6c7ed9a] fix(worktree) at 2:15pm Today: refine wt remove safety across review cycles 4-6
  Files Impacted:
  - modified: .claude/skills/worktree/SKILL.md
  - modified: worktree/README.md
  - modified: worktree/cli/src/commands/remove/mod.rs
  - modified: worktree/cli/src/commands/remove/report.rs
  - modified: worktree/cli/tests/remove.rs
  - modified: worktree/lib/src/remove/inventory.rs
  - modified: worktree/lib/src/remove/remote.rs
  - modified: worktree/lib/src/remove/safety.rs

- [75a72d0] docs(skills) at 11:28am Today: record KittyInstance, ConPTY, handoff endpoint, and fingerprint in agent skills
  Files Impacted:
  - modified: .claude/skills/biscuit-test-harness/SKILL.md
  - modified: .claude/skills/os/SKILL.md
  - modified: .claude/skills/os/macos.md
  - modified: .claude/skills/os/windows.md
  - modified: .claude/skills/worktree/SKILL.md
```

- I have intentionally NOT used the compact reporting here because I want you to see the files which were in the commit. 
- I then run `sniff repo recent-commits 5 --package worktree` and what I get is unrelatable
- The important thing is that if the first command has commits in it from the 'worktree' library then we'll see that same commit in the second command but we don't!

```sh
ken in fix-wt-ux/worktree on  fix/wt-ux [$!?]
💻❯ sniff repo recent-commits 5 --package worktree
- [6c7ed9a] fix(worktree) at 2:15pm Today: refine wt remove safety across review cycles 4-6
  Files Impacted:
  - modified: .claude/skills/worktree/SKILL.md
  - modified: worktree/README.md
  - modified: worktree/cli/src/commands/remove/mod.rs
  - modified: worktree/cli/src/commands/remove/report.rs
  - modified: worktree/cli/tests/remove.rs
  - modified: worktree/lib/src/remove/inventory.rs
  - modified: worktree/lib/src/remove/remote.rs
  - modified: worktree/lib/src/remove/safety.rs

- [7f96c09] fix(worktree) at 11:25am Today: track resolved push endpoint in handoff and fingerprint git index
  Files Impacted:
  - modified: worktree/cli/src/commands/remove/mod.rs
  - modified: worktree/cli/src/commands/remove/report.rs
  - modified: worktree/cli/tests/level2_remove.rs
  - modified: worktree/cli/tests/remove.rs
  - modified: worktree/lib/src/remove/handoff.rs
  - modified: worktree/lib/src/remove/inventory.rs
  - modified: worktree/lib/src/remove/live_remote.rs
  - modified: worktree/lib/src/remove/remote.rs
  - modified: worktree/lib/src/remove/safety.rs

- [dba1ba1] docs(worktree) at 1:48am Today: drop color/glyph/format-string narration across list, remove, and lib (Phase 6)
  Files Impacted:
  - modified: worktree/cli/src/commands/dirty_tree.rs
  - modified: worktree/cli/src/commands/list_table.rs
  - modified: worktree/cli/src/commands/remove/report.rs
  - modified: worktree/lib/src/listing.rs
  - modified: worktree/lib/src/pull_requests.rs
  - modified: worktree/lib/src/remove/safety.rs

- [cc401da] feat(worktree) at 1:23am Today: ship generalized comparison cache, list data model, and PR store
  Files Impacted:
  - modified: worktree/lib/src/cache.rs
  - modified: worktree/lib/src/default_target.rs
  - modified: worktree/lib/src/lib.rs
  - added: worktree/lib/src/listing.rs
  - added: worktree/lib/src/pull_requests.rs
  - modified: worktree/lib/src/worktree.rs

- [ecb6c05] feat(worktree) at 11:41pm Yesterday: rewrite wt remove with safety tiers, report-first flow, and move-first removal
  Files Impacted:
  - modified: Cargo.lock
  - modified: worktree/cli/Cargo.toml
  - modified: worktree/cli/src/args.rs
  - modified: worktree/cli/src/commands/dirty_tree.rs
  - modified: worktree/cli/src/commands/mod.rs
  - added: worktree/cli/src/commands/remove/mod.rs
  - added: worktree/cli/src/commands/remove/policy.rs
  - added: worktree/cli/src/commands/remove/report.rs
  - deleted: worktree/cli/src/commands/remove.rs
  - modified: worktree/cli/src/exit.rs
  - modified: worktree/cli/src/main.rs
  - modified: worktree/cli/tests/level2_dirty_tree.rs
  - added: worktree/cli/tests/level2_remove.rs
  - added: worktree/cli/tests/powershell_wrapper_exec.rs
  - modified: worktree/cli/tests/remove.rs
  - modified: worktree/lib/Cargo.toml
  - added: worktree/lib/src/default_target.rs
  - modified: worktree/lib/src/error.rs
  - modified: worktree/lib/src/git.rs
  - modified: worktree/lib/src/lib.rs
  - added: worktree/lib/src/remove/handoff.rs
  - added: worktree/lib/src/remove/inventory.rs
  - added: worktree/lib/src/remove/live_remote.rs
  - added: worktree/lib/src/remove/mod.rs
  - added: worktree/lib/src/remove/remote.rs
  - added: worktree/lib/src/remove/safety.rs
  - added: worktree/lib/src/remove/test_support.rs
  - modified: worktree/lib/src/worktree.rs
```
