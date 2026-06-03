; Perl identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Scalar variable references
(scalar_variable) @reference

; Array variable references
(array_variable) @reference

; Hash variable references
(hash_variable) @reference

; Bareword function names in calls
(call_expression_with_bareword
  function_name: (identifier) @reference)

; Package-qualified identifiers
(package_name
  (identifier) @reference)
