foo() { echo hi; }
function bar { echo bar; }
export VAR=1
export PATH
export -f foo
declare -x Z=3
declare -fx bar
readonly R=1
alias ll='ls -l'
