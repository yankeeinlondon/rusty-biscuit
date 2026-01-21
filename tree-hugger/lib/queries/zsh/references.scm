; Zsh identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Variable references via expansion
(simple_expansion
  (variable_name) @reference)

; Variable references in expansions
(expansion
  (variable_name) @reference)

; Command names (potential function references)
(command_name) @reference

; Word tokens that may be identifiers
(word) @reference
