autoload -Uz +X foo
autoload -w mod.zwc
zmodload zsh/datetime
typeset -g -x GVAR=1
typeset -fu lazy
functions -c old new
export -a ARR
