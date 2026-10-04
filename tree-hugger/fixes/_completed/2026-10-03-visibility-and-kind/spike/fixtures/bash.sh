#!/usr/bin/env bash
# Symbol-kind fixture for Bash. Shell functions have no access levels.

keyword_function() {
  local local_value="$1"
  echo "$local_value"
}

function function_keyword_function {
  echo "keyword"
}

function function_keyword_with_parens() {
  echo "both"
}

_conventionally_private_function() {
  :
}

GLOBAL_VARIABLE="value"
