---
initialize:
    - when: "fm.ready"
      then:
          info: { text: "ready" }
      else:
          ensure_dir: { dir: logs }
success:
    - when: "fm.ready"
      then:
          say: { text: "done" }
          no_error: true
      else:
          ensure_file: { file: notes.md }
    - when: "fm.ready"
      append_line: { file: log.txt, text: "finished" }
cancelled:
    - when: "fm.ready"
      then:
          shell: { command: "git stash" }
      else:
          stop: { no_error: true }
---
# single-action-branch-dictionaries
