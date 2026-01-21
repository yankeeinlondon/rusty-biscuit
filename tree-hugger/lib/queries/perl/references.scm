; Perl identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Scalar variable references
(scalar_variable) @reference

; Array variable references
(array_variable) @reference

; Hash variable references
(hash_variable) @reference

; Bareword identifiers (function names, etc.)
(bareword) @reference

; Package-qualified identifiers
(package_variable) @reference

; Subroutine calls
(function_call
  (bareword) @reference)
