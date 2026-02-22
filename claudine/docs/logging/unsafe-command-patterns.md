# Unsafe Command Patterns

- Recursive deletes / wipes: rm -rf, git clean -fdx, find … -delete, del /s, etc.
- Disk/partition operations: dd, mkfs, wipefs, fdisk, diskutil eraseDisk
- Dangerous mass edits: sed -i across globs, perl -pi, or editor tools targeting huge directory sets
- “Blow away history” git ops: git reset --hard, git rebase --onto (depending), git push --force, deleting branches/tags


