#!/usr/bin/env bash
# S3 - git worktree move / final unregister semantics (throwaway spike).
# Every question uses its own fresh worktree, so one result cannot leak
# into the next.
set -u
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=s3 GIT_AUTHOR_EMAIL=s3@example.invalid GIT_COMMITTER_NAME=s3 GIT_COMMITTER_EMAIL=s3@example.invalid
ROOT=$(mktemp -d "${TMPDIR:-/tmp}/gfc-s3.XXXXXX")
ROOT=$(cd "$ROOT" && pwd -P)
cd "$ROOT"
echo "S3 git=$(git --version)"
say() { echo "S3 $*"; }
run() { local out; out=$("$@" 2>&1); local rc=$?; out=${out//$ROOT/\$R}; echo "    \$ ${*//$ROOT/\$R}  -> rc=$rc ${out//$'\n'/ | }"; return $rc; }
snapshot() {
  {
    find main/.git/worktrees -type f 2>/dev/null | sort | while read -r f; do echo "$f $(shasum "$f" | cut -c1-12)"; done
    find . -maxdepth 3 -name .git -type f | sort | while read -r g; do echo "$g -> $(cat "$g")"; done
    echo "config: $(grep -A3 '^\[extensions\]' main/.git/config | tr '\n' ' ')"
  } 2>/dev/null | sed "s#$ROOT#\$R#g"
}
newwt() { git -C main worktree add -q -b "$1" "$ROOT/$1" "${@:2}"; }

git init -q -b main main && (cd main && echo a > a.txt && mkdir sub && echo s > sub/s.txt && git add . && git commit -qm init)
newwt other; newwt gone; rm -rf "$ROOT/gone"   # unrelated records: healthy + prunable

say "== Q1 git worktree move onto existing destinations"
newwt q1a; mkdir -p "$ROOT/holder/q1a"; ino_before=$(ls -di "$ROOT/holder/q1a" | awk '{print $1}')
run git -C main worktree move "$ROOT/q1a" "$ROOT/holder"
say "  existing EMPTY holder/q1a (ino $ino_before) now: ino $(ls -di "$ROOT/holder/q1a" | awk '{print $1}'), has .git=$([ -f "$ROOT/holder/q1a/.git" ] && echo y || echo n)"
newwt q1b; mkdir "$ROOT/q1b-empty"
run git -C main worktree move "$ROOT/q1b" "$ROOT/q1b-empty"
say "  explicit existing empty dir destination: moved inside? $([ -d "$ROOT/q1b-empty/q1b" ] && echo y || echo n)"
newwt q1c; echo f > "$ROOT/q1c-file"
run git -C main worktree move "$ROOT/q1c" "$ROOT/q1c-file"
newwt q1d; mkdir -p "$ROOT/q1d-holder/q1d"; echo keep > "$ROOT/q1d-holder/q1d/keep"
run git -C main worktree move "$ROOT/q1d" "$ROOT/q1d-holder"

say "== Q2 files a successful git move rewrites (absolute links)"
newwt q2; before=$(snapshot)
run git -C main worktree move "$ROOT/q2" "$ROOT/.q2.wt-removal-x"
diff <(echo "$before") <(snapshot) | grep '^[<>]' | sed 's/^/    diff /'

say "== Q3 relative links (worktree.useRelativePaths; Git >= 2.48)"
if run git -C main -c worktree.useRelativePaths=true worktree add -q -b q3 "$ROOT/q3"; then
  say "  config after relative add: $(grep -A3 '^\[extensions\]' main/.git/config | tr '\n' ' ')"
  before=$(snapshot)
  run git -C main worktree move "$ROOT/q3" "$ROOT/.q3.wt-removal-x"
  diff <(echo "$before") <(snapshot) | grep '^[<>]' | sed 's/^/    diff /'
  git -C main -c worktree.useRelativePaths=true worktree add -q -b q3b "$ROOT/q3b"
  say "  manual native move to a same-depth sibling: .git=$(cat "$ROOT/q3b/.git") admin gitdir=$(cat main/.git/worktrees/q3b/gitdir)"
  mv "$ROOT/q3b" "$ROOT/.q3b.manual"
  run git -C "$ROOT/.q3b.manual" rev-parse --absolute-git-dir
  run git -C main worktree list --porcelain
  printf '%s\n' "../../../.q3b.manual/.git" > main/.git/worktrees/q3b/gitdir
  say "  after rewriting only the admin gitdir (relative form):"
  run git -C "$ROOT/.q3b.manual" status --porcelain
  run sh -c "git -C main worktree list --porcelain | grep -A3 q3b"
  run git -C main worktree prune --dry-run -v
  git -C main -c worktree.useRelativePaths=true worktree add -q -b q3c "$ROOT/q3c"
  mkdir "$ROOT/deeper"; mv "$ROOT/q3c" "$ROOT/deeper/q3c"
  say "  move one level deeper (relative .git now wrong):"
  run git -C "$ROOT/deeper/q3c" status --porcelain
else
  say "  relative worktrees unsupported by this Git"
fi

say "== Q4 manual native move + rewrite of exactly the admin gitdir (absolute links)"
newwt q4; before=$(snapshot)
mv "$ROOT/q4" "$ROOT/.q4.manual"
run sh -c "git -C main worktree list --porcelain | grep -A3 q4"
printf '%s\n' "$ROOT/.q4.manual/.git" > main/.git/worktrees/q4/gitdir
run sh -c "git -C main worktree list --porcelain | grep -A3 q4"
run git -C "$ROOT/.q4.manual" status --porcelain
run git -C main worktree prune --dry-run -v
diff <(echo "$before") <(snapshot) | grep '^[<>]' | sed 's/^/    diff /'

say "== Q5 locked worktree"
newwt q5; run git -C main worktree lock --reason kept "$ROOT/q5"
run git -C main worktree move "$ROOT/q5" "$ROOT/.q5.x"
run git -C main worktree move -f "$ROOT/q5" "$ROOT/.q5.x"
run git -C main worktree remove --force "$ROOT/q5"
mv "$ROOT/q5" "$ROOT/.q5.manual"; printf '%s\n' "$ROOT/.q5.manual/.git" > main/.git/worktrees/q5/gitdir
say "  lock survives a manual move: $(git -C main worktree list --porcelain | grep -A4 q5.manual | grep locked)"

say "== Q6 submodule"
git init -q -b main "$ROOT/subrepo" && (cd "$ROOT/subrepo" && echo x > x && git add x && git commit -qm sub)
newwt q6
run git -C "$ROOT/q6" -c protocol.file.allow=always submodule add -q "$ROOT/subrepo" mod
say "  submodule .git: $(cat "$ROOT/q6/mod/.git" | sed "s#$ROOT#\$R#")"
run git -C main worktree move "$ROOT/q6" "$ROOT/.q6.x"
run git -C main worktree remove "$ROOT/q6"

say "== Q7 index files of a linked worktree (split index, entry flags)"
newwt q7
run git -C "$ROOT/q7" config core.splitIndex true
run git -C "$ROOT/q7" update-index --split-index
echo n > "$ROOT/q7/new.txt"; run git -C "$ROOT/q7" add -N new.txt
run git -C "$ROOT/q7" update-index --skip-worktree sub/s.txt
say "  admin dir: $(ls main/.git/worktrees/q7 | tr '\n' ' ')"
say "  common dir sharedindex: $(ls main/.git | grep sharedindex | tr '\n' ' ')"
say "  ls-files -v: $(git -C "$ROOT/q7" ls-files -v | tr '\n' ' ')"
say "  ls-files --stage --debug (new.txt): $(git -C "$ROOT/q7" ls-files --stage --debug new.txt | tr '\n' ' ' | tr -s ' ')"

say "== Q8 final unregister of a quarantine emptied by Worktree"
newwt q8; q="$ROOT/.q8.wt-removal-final"
mv "$ROOT/q8" "$q"; printf '%s\n' "$q/.git" > main/.git/worktrees/q8/gitdir
rm -rf "$q/a.txt" "$q/sub"
say "  only .git left: $(ls -A "$q" | tr '\n' ' ')"
run git -C "$q" status --porcelain
run git -C main worktree remove "$q"
say "  (a) plain remove, only .git left: dir=$([ -e "$q" ] && echo kept || echo gone) record=$(git -C main worktree list --porcelain | grep -c q8)"
rm "$q/.git"; rmdir "$q"
run git -C main worktree remove "$q"
say "  (b) plain remove of the now-absent directory: record=$(git -C main worktree list --porcelain | grep -c q8) admin=$([ -d main/.git/worktrees/q8 ] && echo kept || echo gone)"
newwt q8c; q="$ROOT/.q8c.wt-removal-final"
mv "$ROOT/q8c" "$q"; printf '%s\n' "$q/.git" > main/.git/worktrees/q8c/gitdir; rm -rf "$q/a.txt" "$q/sub"
run git -C main worktree remove --force "$q"
say "  (c) single --force, only .git left: dir=$([ -e "$q" ] && echo kept || echo gone) record=$(git -C main worktree list --porcelain | grep -c q8c)"
newwt q8d; q="$ROOT/.q8d.wt-removal-final"
mv "$ROOT/q8d" "$q"; printf '%s\n' "$q/.git" > main/.git/worktrees/q8d/gitdir; rm -rf "$q/a.txt" "$q/sub" "$q/.git"; rmdir "$q"
mkdir "$q"; echo new > "$q/recreated"
run git -C main worktree remove "$q"
say "  (d) dir recreated without .git before plain remove: dir=$([ -e "$q/recreated" ] && echo kept || echo DELETED) record=$(git -C main worktree list --porcelain | grep -c q8d)"
say "  unrelated records intact: other=$(git -C main worktree list --porcelain | grep -c '/other$') gone(prunable)=$(git -C main worktree list --porcelain | grep -c '/gone$')"

say "== Q9 locked + absent directory"
newwt q9; git -C main worktree lock "$ROOT/q9"; rm -rf "$ROOT/q9"
run git -C main worktree remove "$ROOT/q9"
cd /; rm -rf "$ROOT"
