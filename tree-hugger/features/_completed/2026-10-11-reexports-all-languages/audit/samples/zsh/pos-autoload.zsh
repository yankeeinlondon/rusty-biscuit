autoload -Uz compinit
autoload -U zmv add-zsh-hook
fpath=(~/.zfunc $fpath)
foo() { echo hi; }
function bar { echo bar; }
export VAR=1
export -f foo
typeset -x Z=3
typeset -fx foo
