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

# ---- Declaration boundaries (keyword and signature text) ----

# A comment naming declaration keywords: function local declare
BOUNDARY_TABLE=(
  first
  second
)

boundary_multi_line_header ()
{
  echo "header"
}

# ---- Scope and qualified names ----

outer_function() {
  nested_function() { echo "nested"; }
}

# ---- Flags and metrics ----

# deprecated: a comment above a function is not documentation here
test_helper() {
  :
}
