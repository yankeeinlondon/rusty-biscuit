#!/usr/bin/env bash
# Level 1 contract for the pre-push dispatcher installed in the shared Git
# hooks directory: each linked worktree must execute its own checked-out hook.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DISPATCHER="$REPO_ROOT/.githooks/pre-push-dispatcher"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

main="$tmpdir/main"
linked="$tmpdir/linked"
git init -q -b main "$main"
git -C "$main" config user.name hook-test
git -C "$main" config user.email hook@example.com
git -C "$main" config commit.gpgsign false
printf 'base\n' >"$main/README.md"
git -C "$main" add README.md
git -C "$main" commit -q -m base
git -C "$main" worktree add -q -b linked "$linked"

mkdir -p "$main/.githooks" "$linked/.githooks"
for fixture in "$main:main" "$linked:linked"; do
    root="${fixture%%:*}"
    label="${fixture##*:}"
    sed "s/__LABEL__/$label/" >"$root/.githooks/pre-push" <<'EOF'
#!/bin/sh
input="$(cat)"
printf '%s|%s|%s|%s\n' '__LABEL__' "$1" "$2" "$input" >>"$OUTPUT"
EOF
    chmod +x "$root/.githooks/pre-push"
done

hooks_dir="$(git -C "$main" rev-parse --path-format=absolute --git-path hooks)"
linked_hooks_dir="$(git -C "$linked" rev-parse --path-format=absolute --git-path hooks)"
if [ "$hooks_dir" != "$linked_hooks_dir" ]; then
    echo "fixture error: linked worktrees did not resolve one shared hooks directory" >&2
    exit 1
fi
mkdir -p "$hooks_dir"
cp "$DISPATCHER" "$hooks_dir/pre-push"
chmod +x "$hooks_dir/pre-push"

output="$tmpdir/output"
printf 'main-stdin' | (cd "$main" && OUTPUT="$output" "$hooks_dir/pre-push" origin main-url)
printf 'linked-stdin' | (cd "$linked" && OUTPUT="$output" "$hooks_dir/pre-push" upstream linked-url)

expected="$tmpdir/expected"
printf '%s\n' \
    'main|origin|main-url|main-stdin' \
    'linked|upstream|linked-url|linked-stdin' >"$expected"
if ! cmp -s "$expected" "$output"; then
    echo "dispatcher did not preserve worktree identity, arguments, and stdin" >&2
    diff -u "$expected" "$output" >&2 || true
    exit 1
fi

echo "PASS pre-push dispatcher selects each linked worktree's hook"
