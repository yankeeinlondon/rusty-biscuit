#!/usr/bin/env bash
# Phase 1 spike S3: the Git discovery commands behind R4 (classification),
# R6 (anchor distances), and R9 (error shapes), run against scripted
# topologies. Throwaway; prints outputs, exit codes, stderr, and timings.
#
# Usage: git-discovery.sh [work dir]   (default: a fresh mktemp dir)
set -uo pipefail

work="${1:-$(mktemp -d "${TMPDIR:-/tmp}/wt-s3.XXXXXX")}"
mkdir -p "$work"
cd "$work" || exit 1
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0 LC_ALL=C
export GIT_AUTHOR_NAME=spike GIT_AUTHOR_EMAIL=spike@example.invalid
export GIT_COMMITTER_NAME=spike GIT_COMMITTER_EMAIL=spike@example.invalid
tick=1700000000
now_ms() { perl -MTime::HiRes=time -e 'printf "%d\n", time*1000'; }

echo "host: $(uname -s) $(uname -m); $(git --version)"

commit() { # commit <message>
  tick=$((tick + 60))
  GIT_AUTHOR_DATE="@$tick +0000" GIT_COMMITTER_DATE="@$tick +0000" \
    git commit -q --allow-empty -m "$1"
}
merge() { # merge <branch> <message>
  tick=$((tick + 60))
  GIT_AUTHOR_DATE="@$tick +0000" GIT_COMMITTER_DATE="@$tick +0000" \
    git merge -q --no-ff -m "$2" "$1"
}
short() { git rev-parse --short "$1"; }
fresh() { rm -rf "$1"; git init -q -b main "$1"; cd "$1" || exit 1; commit root; }

# run <label> <git args...>: prints exit code, stdout (SHAs shortened), stderr.
run() {
  local label=$1; shift
  local out err code
  out=$(git "$@" 2>"$work/.err"); code=$?; err=$(cat "$work/.err")
  local pretty
  pretty=$(printf '%s\n' "$out" | while read -r line; do
    for word in $line; do
      if [[ $word =~ ^[0-9a-f]{40}$ ]]; then printf '%s ' "$(short "$word")"; else printf '%s ' "$word"; fi
    done; echo; done | sed 's/ $//')
  printf '  %-34s exit=%s out=[%s]%s\n' "$label" "$code" "$(echo "$pretty" | paste -sd '|' -)" "${err:+ stderr=[$(echo "$err" | paste -sd '|' -)]}"
}

# classify <tip ref> <candidate ref>: the R4 procedure for one candidate.
# C is the oldest commit on X's first-parent chain that descends from T: the
# intersection of `rev-list --first-parent T..X` (the chain, newest first) and
# `rev-list --ancestry-path T..X` (descendants of T). The plan's original
# single command, `--first-parent --ancestry-path`, is printed beside it.
classify() {
  local tip=$1 x=$2 t
  t=$(git rev-parse "$tip")
  if ! git merge-base --is-ancestor "$t" "$x" 2>/dev/null; then
    echo "  => $tip vs $x: not contained (Unmerged candidate)"; return
  fi
  local original chain descendants oldest=""
  original=$(git rev-list --first-parent --ancestry-path --parents "$t..$x" | tail -n 1 | cut -c1-7)
  chain=$(git rev-list --first-parent --parents "$t..$x")
  descendants=$(git rev-list --ancestry-path "$t..$x")
  if [[ -z $chain ]]; then
    echo "  => $tip vs $x: empty chain with is-ancestor true: T is X (No separate history) [plan cmd oldest: ${original:-empty}]"; return
  fi
  while read -r line; do
    [[ -z $line ]] && continue
    if grep -qx "${line%% *}" <<<"$descendants"; then oldest=$line; else break; fi
  done <<<"$chain"
  if [[ -z $oldest ]]; then
    echo "  => $tip vs $x: GAP (contained, but no chain commit descends from T)"; return
  fi
  read -r c first rest <<<"$oldest"
  local verdict
  if [[ $first == "$t" ]]; then verdict="first parent is T (No separate history)"
  elif [[ " $rest " == *" $t "* ]]; then verdict="second parent is T (Merged directly), C^1=$(short "$first")"
  else verdict="parents=$(short "$first") ${rest:+$(for p in $rest; do short "$p"; done | paste -sd, -)} (Integrated otherwise)"; fi
  echo "  => $tip vs $x: C=$(short "$c") $verdict [plan cmd oldest: ${original:-empty}]"
}

# anchor <anchor ref> <tip ref>: R6 distance + verification.
anchor() {
  local a t d at
  a=$(git rev-parse "$1"); t=$(git rev-parse "$2")
  d=$(git rev-list --first-parent --count "$a..$t")
  at=$(git rev-parse "$t~$d" 2>/dev/null || echo "?")
  if [[ $at == "$a" ]]; then echo "  anchor $1 on $2: distance=$d verified"
  else echo "  anchor $1 on $2: distance=$d NOT on chain (tip~$d=$(short "$at" 2>/dev/null || echo "$at"))"; fi
}

echo; echo "== observation 1: current branch merged via merge commit =="
fresh "$work/obs1"
commit d1; git switch -q -c fix/wt-ux; commit w1; commit w2
git switch -q main; merge fix/wt-ux "Merge PR #103"; git update-ref refs/remotes/origin/main HEAD
git reset -q --hard HEAD^   # local main stays one merge behind, like the observation
classify fix/wt-ux origin/main
run "rev-list fp ancestry parents" rev-list --first-parent --ancestry-path --parents fix/wt-ux..origin/main
run "merge-base(C^1, T)" merge-base origin/main^1 fix/wt-ux
run "merge-base(default tip, T)" merge-base origin/main fix/wt-ux
run "lane: fp T --not C^1" rev-list --first-parent fix/wt-ux --not origin/main^1
run "plain log origin/main (today)" log --format=%H -5 origin/main
run "fp log origin/main (R5)" log --first-parent --format=%H -5 origin/main

echo; echo "== default moved on before the merge (C is not the chain's oldest) =="
fresh "$work/moved"
commit d1; git switch -q -c B; commit b1; git switch -q main; commit e1; commit e2; merge B "merge B"; commit after
classify B main

echo; echo "== observation 2: nested parent, both merged, local main one merge behind =="
fresh "$work/obs2"
commit d1; git switch -q -c fix/wt-ux; commit w1; commit w2
git switch -q -c fix/sniff; for i in $(seq 1 14); do commit "s$i"; done
git switch -q main; merge fix/wt-ux "Merge PR #103"
merge fix/sniff "Merge PR #104"; git update-ref refs/remotes/origin/main HEAD
git reset -q --hard HEAD^
obs2_since=$(git log -1 --format=%ct fix/sniff~3)
classify fix/sniff fix/wt-ux
classify fix/sniff origin/main
classify fix/wt-ux origin/main
run "fork: merge-base(parent tip, T)" merge-base fix/wt-ux fix/sniff
run "sniff lane count (fp T --not C^1)" rev-list --first-parent --count fix/sniff --not origin/main^1
run "plain log -10 origin/main (today)" log --format=%s -10 origin/main
run "fp log -10 origin/main (R5)" log --first-parent --format=%s -10 origin/main
anchor main origin/main
anchor fix/wt-ux origin/main

echo; echo "== merged into a non-default parent =="
fresh "$work/into-parent"
commit d1; git switch -q -c parent; commit p1; git switch -q -c child; commit c1; commit c2
git switch -q parent; merge child "merge child into parent"; commit p2
classify child parent
classify child main

echo; echo "== fast-forward =="
fresh "$work/ff"
commit d1; git switch -q -c ff; commit f1; commit f2; git switch -q main; git merge -q --ff-only ff; commit d2
classify ff main
git switch -q -c at-tip; git switch -q main
classify main main

echo; echo "== equal tips: C merged at M, D created at C's tip =="
fresh "$work/equal"
commit d1; git switch -q -c C; commit c1; git switch -q main; merge C "merge C"; commit d2
git branch D C
classify D C
classify C main
run "rev-parse C D" rev-parse C D

echo; echo "== continued after merge =="
fresh "$work/continued"
commit d1; git switch -q -c B; commit b1; commit b2; git switch -q main; merge B "merge B"
git switch -q B; commit b3; git switch -q main
classify B main
run "fork: merge-base(main, B)" merge-base main B
run "lane: fp B --not fork" rev-list --first-parent B --not "$(git merge-base main B)"

echo; echo "== indirect integration (T merged into Y, Y merged into main) =="
fresh "$work/indirect"
commit d1; git switch -q -c T; commit t1; git switch -q main; git switch -q -c Y; commit y1
merge T "merge T into Y"; git switch -q main; merge Y "merge Y"
classify T main
run "lane: fp T --not C^1" rev-list --first-parent T --not main^1

echo; echo "== anchor not on the first-parent chain =="
cd "$work/obs2" || exit 1
anchor fix/sniff origin/main

echo; echo "== shallow clones (missing history) =="
cd "$work" || exit 1
rm -rf shallow1 shallow2
git clone -q --depth 1 --no-single-branch "file://$work/obs2" shallow1 2>&1 | sed 's/^/  clone: /'
cd shallow1 || exit 1
run "is-shallow" rev-parse --is-shallow-repository
run "is-ancestor sniff origin/main" merge-base --is-ancestor origin/fix/sniff origin/main
run "merge-base sniff main" merge-base origin/fix/sniff origin/main
run "rev-list fp ancestry parents" rev-list --first-parent --ancestry-path --parents origin/fix/sniff..origin/main
run "rev-list fp count sniff..main" rev-list --first-parent --count origin/fix/sniff..origin/main
run "rev-parse main~3" rev-parse origin/main~3
run "rev-parse main^2" rev-parse origin/main^2
run "cat-file -e main^1 (missing?)" cat-file -e "$(git rev-parse origin/main 2>/dev/null)^1"
cd "$work" || exit 1
git clone -q --shallow-since="@$obs2_since" --no-single-branch "file://$work/obs2" shallow2 2>&1 | sed 's/^/  clone: /'
cd shallow2 || exit 1
run "shallow-since: commits kept" rev-list --count --all
run "is-ancestor wt-ux origin/main" merge-base --is-ancestor origin/fix/wt-ux origin/main
run "merge-base wt-ux sniff" merge-base origin/fix/wt-ux origin/fix/sniff
run "fp ancestry wt-ux..main" rev-list --first-parent --ancestry-path --parents origin/fix/wt-ux..origin/main

echo; echo "== timing: 9,000-commit first-parent chain, fork 5,000 back, merge 3,000 back =="
cd "$work" || exit 1
rm -rf big; git init -q -b main big; cd big || exit 1
{
  mark=0; t=1700000000
  emit() { # emit <branch> <parent mark|""> [merge mark]
    mark=$((mark + 1)); t=$((t + 1))
    printf 'commit refs/heads/%s\nmark :%d\ncommitter s <s@e> %d +0000\ndata 2\nc\n' "$1" "$mark" "$t"
    [[ -n $2 ]] && printf 'from :%d\n' "$2"
    [[ -n ${3:-} ]] && printf 'merge :%d\n' "$3"
    echo
  }
  prev=""
  for i in $(seq 1 4000); do emit main "$prev"; prev=$mark; done
  fork=$prev
  side=$fork; for i in $(seq 1 3); do emit side "$side"; side=$mark; done
  for i in $(seq 1 1999); do emit main "$prev"; prev=$mark; done
  emit main "$prev" "$side"; prev=$mark
  for i in $(seq 1 3000); do emit main "$prev"; prev=$mark; done
} | git fast-import --quiet
git reset -q --hard main
echo "  first-parent length: $(git rev-list --first-parent --count main)"
time_it() { local label=$1 start end; shift; start=$(now_ms); "$@" >/dev/null 2>&1; end=$(now_ms); printf '  %-40s %4d ms\n' "$label" $((end - start)); }
for round in 1 2 3; do
  echo "  -- round $round"
  time_it "is-ancestor side main" git merge-base --is-ancestor side main
  time_it "rev-list fp ancestry parents side..main" git rev-list --first-parent --ancestry-path --parents side..main
  time_it "merge-base side main^{merge}^1" git merge-base side "$(git rev-list --merges -1 main)^1"
  time_it "rev-list fp count fork..main" git rev-list --first-parent --count "$(git merge-base side main)..main"
  time_it "rev-parse main~5003 main~3000 batch" git rev-parse main~5003 main~3000 main~0
  time_it "log fp -10 main" git log --first-parent --format=%H -10 main
done
classify side main
anchor "$(git rev-list --merges -1 main)" main
anchor "$(git merge-base side main)" main   # merge-base is side's tip once merged: must NOT verify
anchor "$(git rev-parse side~3)" main       # the real fork: must verify
echo; echo "work dir: $work"
