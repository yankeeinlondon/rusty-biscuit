---
kind: sequence
success:
  - stdout: "{{ seq.outputs }}"
sequence:
  - name: outer
    shell: echo outer
  - name: inner
    sequence:
      - name: child
        shell: echo inner
  - name: checks
    group:
      name: checks
      tasks:
        - name: lint
          shell: echo clean
---
This executable example returns `["outer", ["inner"], {"lint": "clean"}]`.
Run it with `claudine sequence <this-file> --goose --yolo`, replacing the
provider selection with your installed provider. Its tasks run shell commands;
no provider session is launched. The body is documentation because every step
has an explicit executable.
