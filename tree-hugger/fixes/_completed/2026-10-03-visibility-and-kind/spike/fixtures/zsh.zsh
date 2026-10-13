#!/usr/bin/env zsh
# Symbol-kind fixture for Zsh. Shell functions have no access levels.

paren_function() {
  local local_value="$1"
  print -- "$local_value"
}

function keyword_function {
  print -- "keyword"
}

function keyword_with_parens() {
  print -- "both"
}

_conventionally_private_function() {
  :
}

GLOBAL_VARIABLE="value"
