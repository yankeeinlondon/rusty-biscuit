; Zsh lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect echo statements (potential debug prints)
(command
  name: (command_name) @_cmd
  (#eq? @_cmd "echo")) @diagnostic.debug-print
