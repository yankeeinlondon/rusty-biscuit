#!/usr/bin/env bash
# Spike S2 (throwaway): reproduces the planned boundary walk (G2-G8) with plain
# Git in scratch repositories and prints each answer and the Git call count.
set -euo pipefail
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
WINDOW=5
# Counted in a file, since most calls run inside `$(...)` subshells.
CALLS_FILE=$(mktemp)
g() { echo "$1" >>"$CALLS_FILE"; git "$@"; }
# `g` whose exit 1 is "no", like `git_command_allow_no_match`; any other
# failure is a gap and ends the walk.
g_maybe() {
  echo "$1" >>"$CALLS_FILE"; local status=0; git "$@" 2>/dev/null || status=$?
  ((status <= 1)) || { echo "  GatherGap: git $* exited $status" >&2; return 2; }
  return $status
}
# ancestor <a> <b>: yes | no | gap (a shallow "no" is a gap, as in `History`).
ancestor() {
  local status=0; g_maybe merge-base --is-ancestor "$1" "$2" || status=$?
  case $status in
    0) echo yes ;;
    1) [[ $(git rev-parse --is-shallow-repository) == true ]] && echo gap || echo no ;;
    *) echo gap ;;
  esac
}
calls() { wc -l <"$CALLS_FILE" | tr -d ' '; }

SEQ=1700000000
mk() { # mk <tree> <message> <parent>... -> sha
  local tree=$1 msg=$2; shift 2; local args=()
  for p in "$@"; do args+=(-p "$p"); done
  SEQ=$((SEQ + 1))
  GIT_AUTHOR_DATE="$SEQ +0000" GIT_COMMITTER_DATE="$SEQ +0000" git commit-tree -m "$msg" "${args[@]}" "$tree"
}
chain() { # chain <tree> <from> <prefix> <count> -> shas, oldest first, one per line
  local tree=$1 from=$2 prefix=$3 count=$4 parent=$2 n
  for ((n = 1; n <= count; n++)); do parent=$(mk "$tree" "$prefix$n" "$parent"); echo "$parent"; done
}
new_repo() {
  local dir; dir=$(mktemp -d); cd "$dir"
  git init -q -b main; git config user.email t@e.com; git config user.name T; git config commit.gpgsign false
  git commit -q --allow-empty -m r; ROOT=$(git rev-parse HEAD); TREE=$(git rev-parse HEAD^{tree})
}
name() { # abbreviates a sha to its fixture name
  local sha=$1; for key in "${!NAMES[@]}"; do [[ ${NAMES[$key]} == "$sha" ]] && { echo "$key"; return; }; done; echo "${sha:0:7}"
}

# classify <tip> <candidate>... : prints "unmerged" | "noseparate <i>" |
# "merged <i> <C> <C^1>" | "indirect <i> <C^1>", with G3's fast path.
classify() {
  local tip=$1; shift; local i=0 indirect=""
  for cand in "$@"; do
    local contains; contains=$(ancestor "$tip" "$cand")
    [[ $contains == gap ]] && { echo "gap (is-ancestor $(name "$tip") $(name "$cand"))"; return; }
    if [[ $contains == yes ]]; then
      local chain; chain=$(g rev-list --first-parent --parents "$tip..$cand" --)
      if [[ -z $chain ]]; then echo "noseparate $i"; return; fi
      local oldest; oldest=$(tail -n1 <<<"$chain")
      if [[ $(cut -d' ' -f2 <<<"$oldest") == "$tip" ]]; then echo "noseparate $i (G3 fast path)"; return; fi
      local desc; desc=$(g rev-list --ancestry-path "$tip..$cand" --)
      local c="" line
      while read -r line; do grep -qx "${line%% *}" <<<"$desc" || break; c=$line; done <<<"$chain"
      local c_sha c_p1 rest; read -r c_sha c_p1 rest <<<"$c"
      if [[ " $rest " == *" $tip "* ]]; then echo "merged $i $c_sha $c_p1"; return; fi
      [[ -z $indirect ]] && indirect="indirect $i $c_p1"
    fi
    i=$((i + 1))
  done
  [[ -n $indirect ]] && { echo "$indirect"; return; }
  echo unmerged
}

# boundary <tip> <stop>... : G2. Prints "<B> <n>" (n = B's distance from tip) or "root".
boundary() {
  local tip=$1; shift; local shown
  shown=$(g log --first-parent --format='%H %ct %P' --max-count "$WINDOW" "$tip" --not "$@" --)
  local count; count=$(grep -c . <<<"$shown" || true)
  if ((count < WINDOW)); then
    local b; b=$(tail -n1 <<<"$shown" | cut -d' ' -f3)
    [[ -z $b ]] && { echo root; return; }
    echo "$b $count"; return
  fi
  local n; n=$(g rev-list --first-parent --count "$tip" --not "$@" --)
  local b; b=$(g log --no-walk=unsorted --ignore-missing --format='%H %P' "$tip~$((n - 1))" -- | cut -d' ' -f2)
  [[ -z $b ]] && { echo root; return; }
  echo "$b $n"
}

# walk <branch> <record base_sha or -> <candidate>... -- <stop>...
walk() {
  local branch=$1 record=$2; shift 2; local cands=() stops=()
  while [[ $1 != -- ]]; do cands+=("$1"); shift; done; shift; stops=("$@")
  local tip; tip=$(git rev-parse "$branch")
  local d=""
  if [[ $record != - ]]; then
    d=$(g rev-list --first-parent --count "$record..$tip" --)
    local check; check=$(g log --no-walk=unsorted --ignore-missing --format=%H "$tip~$d" --)
    [[ $check == "$record" ]] && echo "  record $(name "$record") at d=$d" || { echo "  record $(name "$record") is stale"; d=""; }
  fi
  local visited=() prev=-1 fork=""
  while :; do
    read -r b n <<<"$(boundary "$tip" "${stops[@]}")"
    [[ $b == root ]] && { echo "  boundary: root, stop"; break; }
    echo "  boundary B=$(name "$b") at n=$n (stops: $(for s in "${stops[@]}"; do name "$s"; done | tr '\n' ' '))"
    if ((n <= prev)); then echo "  not strictly older: incomplete"; break; fi
    if [[ -n $d ]] && ((d <= n)); then echo "  cutoff: record d=$d <= n=$n, stop"; break; fi
    local answer; answer=$(classify "$b" "${cands[@]}")
    read -r kind i c c1 _ <<<"$answer"
    if [[ $kind != merged ]]; then echo "  classify(B): $answer, stop"; break; fi
    echo "  classify(B): merged into C=$(name "$c") (C^1=$(name "$c1")) on candidate $i"
    # SKIP_C1_CHECK=1: rely on classification, whose C is the oldest chain
    # commit descending from B, so C^1 does not contain B.
    [[ ${SKIP_C1_CHECK:-0} == 1 ]] || case $(ancestor "$b" "$c1") in
      yes) echo "  B is an ancestor of C^1: reject"; break ;;
      gap) echo "  is-ancestor(B, C^1) is a gap: keep verified edges, incomplete"; break ;;
    esac
    local kept=("$c1")
    for s in "${stops[@]}"; do [[ $(ancestor "$b" "$s") == yes ]] || kept+=("$s"); done
    stops=($(printf '%s\n' "${kept[@]}" | awk '!seen[$0]++'))
    fork=$(g merge-base "$c1" "$b")
    echo "  edge ($(name "$b") -> $(name "$c")); fork = merge-base(C^1, B) = $(name "$fork")"
    prev=$n
  done
}

report() { local label=$1; shift; : >"$CALLS_FILE"; echo "$label"; walk "$@" || echo "  walk ended by a gap"; echo "  git calls: $(calls) ($(sort "$CALLS_FILE" | uniq -c | tr -s ' ' | tr '\n' ','))"; }

echo "### E1 behind: main=P, origin/main=C"
new_repo
declare -A NAMES=([r]=$ROOT)
mapfile -t D < <(chain "$TREE" "$ROOT" d 4)
mapfile -t W < <(chain "$TREE" "${D[1]}" w 3)
P=$(mk "$TREE" P "${D[3]}")
S=$(mk "$TREE" "sync main" "${W[2]}" "$P")
mapfile -t X < <(chain "$TREE" "$S" x 7)
B=${X[6]}
C=$(mk "$TREE" "Merge PR" "$P" "$B")
N=$(mk "$TREE" N "$B")
git update-ref refs/heads/main "$P"; git update-ref refs/remotes/origin/main "$C"
git update-ref refs/heads/fix/wt-ux "$N"; git update-ref refs/heads/fix/sniff-pr "$B"
NAMES+=([d1]=${D[0]} [d2]=${D[1]} [P]=$P [S]=$S [B]=$B [C]=$C [N]=$N)
report "fix/wt-ux, no record (candidates: main, origin/main)" fix/wt-ux - "$P" "$C" -- "$P" "$C"
report "fix/sniff-pr tip classification" fix/sniff-pr - "$N" "$P" "$C" -- "$P" "$C"
echo "  classify(sniff-pr tip, [fix/wt-ux, main, origin/main]) = $(classify "$B" "$N" "$P" "$C")"
report "fix/wt-ux with fix/sniff-pr's record shape (base_sha=B): cutoff" fix/wt-ux "$B" "$P" "$C" -- "$P" "$C"

echo; echo "### E1 diverged: main=P' (a commit on P), origin/main=C"
git update-ref refs/heads/main "$(mk "$TREE" "P'" "$P")"; PP=$(git rev-parse main); NAMES+=([P\']=$PP)
report "fix/wt-ux (candidates: main, origin/main)" fix/wt-ux - "$PP" "$C" -- "$PP" "$C"

echo; echo "### Sparse lanes (observed_sparse_lanes shape)"
new_repo
declare -A NAMES=([r]=$ROOT)
mapfile -t D < <(chain "$TREE" "$ROOT" d 12)
mapfile -t WB < <(chain "$TREE" "${D[4]}" w 8); W1=${WB[7]}
M103=$(mk "$TREE" M103 "${D[11]}" "$W1")
mapfile -t SN < <(chain "$TREE" "$W1" n 94)
mapfile -t WA < <(chain "$TREE" "$W1" w 64)
D13=$(mk "$TREE" d13 "$M103")
M104=$(mk "$TREE" M104 "$D13" "${SN[93]}")
B1=$(mk "$TREE" B1 "${WA[63]}" "$M104")
mapfile -t XS < <(chain "$TREE" "$B1" x 3)
git update-ref refs/heads/main "$M104"; git update-ref refs/remotes/origin/main "$M104"
git update-ref refs/heads/fix/wt-ux "${XS[2]}"; git update-ref refs/heads/fix/sniff "${SN[93]}"
NAMES+=([d5]=${D[4]} [d12]=${D[11]} [W1]=$W1 [M103]=$M103 [d13]=$D13 [M104]=$M104 [B1]=$B1)
report "fix/wt-ux (unmerged; stop [main])" fix/wt-ux - "$M104" -- "$M104"
report "fix/wt-ux with realistic record base_sha=d5 (E5)" fix/wt-ux "${D[4]}" "$M104" -- "$M104"
report "fix/wt-ux with today's L2 record base_sha=<parent's final tip> (stale)" fix/wt-ux "$M104" "$M104" -- "$M104"
report "fix/sniff (merged directly into M104; stop [d13]; candidates wt-ux, main)" fix/sniff - "${XS[2]}" "$M104" -- "$D13"

echo; echo "### Merged twice and continued"
new_repo
declare -A NAMES=([r]=$ROOT)
D1=$(mk "$TREE" d1 "$ROOT"); B1=$(mk "$TREE" b1 "$D1"); P1=$(mk "$TREE" p1 "$D1"); C1=$(mk "$TREE" C1 "$P1" "$B1")
B2=$(mk "$TREE" b2 "$B1"); P2=$(mk "$TREE" p2 "$C1"); C2=$(mk "$TREE" C2 "$P2" "$B2"); N=$(mk "$TREE" n "$B2")
git update-ref refs/heads/main "$C2"; git update-ref refs/heads/b "$N"
NAMES+=([d1]=$D1 [b1]=$B1 [p1]=$P1 [C1]=$C1 [b2]=$B2 [p2]=$P2 [C2]=$C2 [n]=$N)
report "b" b - "$C2" -- "$C2"

echo; echo "### New branch at a merged tip, record at B"
new_repo
declare -A NAMES=([r]=$ROOT)
D1=$(mk "$TREE" d1 "$ROOT"); B=$(mk "$TREE" b1 "$D1"); P=$(mk "$TREE" p "$D1"); C=$(mk "$TREE" C "$P" "$B"); N=$(mk "$TREE" n1 "$B")
git update-ref refs/heads/main "$C"; git update-ref refs/heads/new "$N"
NAMES+=([d1]=$D1 [B]=$B [P]=$P [C]=$C [n1]=$N)
report "new, record base_sha=B" new "$B" "$C" -- "$C"
report "new, no record (control)" new - "$C" -- "$C"

echo; echo "### B integrated indirectly"
new_repo
declare -A NAMES=([r]=$ROOT)
D1=$(mk "$TREE" d1 "$ROOT"); B=$(mk "$TREE" t1 "$D1"); O1=$(mk "$TREE" o1 "$D1"); O=$(mk "$TREE" O "$O1" "$B")
C=$(mk "$TREE" C "$D1" "$O"); N=$(mk "$TREE" t2 "$B")
git update-ref refs/heads/main "$C"; git update-ref refs/heads/t "$N"
NAMES+=([d1]=$D1 [t1]=$B [o1]=$O1 [O]=$O [C]=$C [t2]=$N)
report "t" t - "$C" -- "$C"

echo; echo "### Ordinary unmerged lanes (P1 budget)"
new_repo
declare -A NAMES=([r]=$ROOT)
D1=$(mk "$TREE" d1 "$ROOT"); D2=$(mk "$TREE" d2 "$D1")
mapfile -t SHORT < <(chain "$TREE" "$D1" s 3)
mapfile -t LONG < <(chain "$TREE" "$D1" l 9)
mapfile -t PAR < <(chain "$TREE" "$D2" p 2)
mapfile -t KID < <(chain "$TREE" "${PAR[0]}" k 2)
git update-ref refs/heads/main "$D2"; git update-ref refs/heads/short "${SHORT[2]}"; git update-ref refs/heads/long "${LONG[8]}"
git update-ref refs/heads/parent "${PAR[1]}"; git update-ref refs/heads/kid "${KID[1]}"
NAMES+=([d1]=$D1 [d2]=$D2 [p1]=${PAR[0]})
report "short (3 commits < window)" short - "$D2" -- "$D2"
report "long (9 commits >= window)" long - "$D2" -- "$D2"
report "kid (recorded parent, forked on it; candidates parent, main)" kid - "${PAR[1]}" "$D2" -- "$D2" "${PAR[1]}"
report "short with a recorded parent that does not hold B (candidates parent, main)" short - "${PAR[1]}" "$D2" -- "$D2" "${PAR[1]}"

echo; echo "### Shallow: second boundary past the cutoff"
new_repo
SRC=$PWD
declare -A NAMES=([r]=$ROOT)
mapfile -t D < <(chain "$TREE" "$ROOT" d 4)
B1=$(mk "$TREE" b1 "${D[3]}"); P1=$(mk "$TREE" p1 "${D[3]}"); C1=$(mk "$TREE" C1 "$P1" "$B1")
B2=$(mk "$TREE" b2 "$B1"); P2=$(mk "$TREE" p2 "$C1"); C2=$(mk "$TREE" C2 "$P2" "$B2"); N=$(mk "$TREE" n "$B2")
git update-ref refs/heads/main "$C2"; git update-ref refs/heads/b "$N"
NAMES+=([d4]=${D[3]} [b1]=$B1 [p1]=$P1 [C1]=$C1 [b2]=$B2 [p2]=$P2 [C2]=$C2 [n]=$N)
DST=$(mktemp -d)/clone
git clone -q --depth 2 --no-single-branch "file://$SRC" "$DST" 2>/dev/null; cd "$DST"; git branch -q b origin/b
echo "  shallow: $(git rev-parse --is-shallow-repository); has b1: $(git cat-file -e "$B1" 2>/dev/null && echo yes || echo no); has C1: $(git cat-file -e "$C1" 2>/dev/null && echo yes || echo no)"
report "b (depth 2)" b - "$C2" -- "$C2"
SKIP_C1_CHECK=1 report "b (depth 2), without the is_ancestor(B, C^1) check" b - "$C2" -- "$C2"
echo "  b2's parents as the clone sees them: '$(git log --no-walk --format=%P "$B2")'"
