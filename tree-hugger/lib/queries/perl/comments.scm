; Perl comment captures for ignore directive parsing
; tree-sitter-perl names hash-style comments `comments` (plural)
; POD documentation is a separate node and is treated as comment text so it
; is excluded from effective SLOC and scanned for debt markers.

(comments) @comment
(pod_statement) @comment
