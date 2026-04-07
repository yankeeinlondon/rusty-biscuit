---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo has-merge-conflict` Subcommand

Checks whether the repository has merge conflicts in its index. Communicates the result purely via exit code — no output is produced by default.

 With `-v`/ `--verbose`, the conflicted file paths are printed to STDERR.

 The exit codes remain unchanged.

## Default Behavior

Produces no output. Use the exit code in shell conditionals.

 With `--verbose`, conflicted file paths are written to STDERR, one per line.

 ## Arguments and Flags

 | Argument | Description |
|----------|-------------|
| `-v/--verbose` | Print conflicted file paths to STDERR |
 | `-b/--base <DIR>` | Analyze a specific directory instead of current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Merge conflicts detected |
 | `1` | No merge conflicts found |

 A merge conflict exists when the git index contains unmerged entries from a merge, rebase, cherry-pick, or revert.

 ## Examples

 ```bash
 # Shell conditional - fail if conflicts exist
 if sniff repo has-merge-conflict; then
 echo "Resolve conflicts before proceeding"
 fi

 # Shell conditional - fail if clean
 if ! sniff repo has-merge-conflict; then  echo "No conflicts"
 fi

 # With verbose - list conflicted files
 sniff repo has-merge-conflict -v
 # Output (to stderr):
 # src/main.rs
 # Cargo.toml

 # In a justfile recipe
 check-merge-conflicts:
  @sniff repo has-merge Conflict || { echo "Conflicts detected!"; exit 1; }
 ```

 ## Usage in CI/CD

 ```yaml
 # GitHub Actions example
 - name: Check for merge conflicts
  run: |
   if sniff repo has-merge-conflict; then
 echo "::error::Merge conflicts detected in $conflicted file"
 exit 1
 fi
 ```

 ## Related Subcommands

 | Subcommand | Output |
 |------------|--------|
 | `has-merge-conflict` | Exit code only (optionally verbose stderr) |
 | [`is current-package-area dirty`](./repo_is_current_package_area_dirty.md) | Exit code for uncommitted changes in current area |
 | [`git-status`](./repo_git_status.md) | Full git status with commit history |
