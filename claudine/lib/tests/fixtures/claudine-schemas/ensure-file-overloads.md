---
start:
    - ensure_file: notes.md
    - ensure_file: [notes.md]
    - ensure_file: [notes.md, "# Notes"]
    - ensure_file: { file: notes.md }
    - ensure_file: { file: notes.md, content: "# Notes" }
---
# ensure-file-overloads
