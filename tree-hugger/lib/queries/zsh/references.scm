; Zsh identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Variable references via expansion ($foo)
(variable_ref
  (simple_variable_name) @reference)

; Variable references in braced expansions (${foo})
(expansion
  name: (simple_variable_name) @reference)

; Command names (potential function references)
(command_name) @reference

; Word tokens that may be identifiers
(word) @reference
