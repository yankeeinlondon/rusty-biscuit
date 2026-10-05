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

# ---- Declaration boundaries (keyword and signature text) ----

# A comment naming declaration keywords: function local declare
BOUNDARY_TABLE=(
  first
  second
)

# tree-sitter-zsh rejects a body brace on the line after the header, so
# this function keeps a one-line header.
boundary_header() {
  print -- "header"
}

# ---- Scope and qualified names ----

outer_function() {
  nested_function() { print -- "nested"; }
}

# ---- Flags and metrics ----

# deprecated: a comment above a function is not documentation here
test_helper() {
  :
}
