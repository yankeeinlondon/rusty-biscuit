---
$schema:
    doc: file
    context: string
$schema-descriptor: 
    - "provide 'context' for what the documentation is meant to be about"
    - "provide a _file path_ for 'doc' to indicate where this documentation will live"

operation: 
    test: "file_exists({{doc.doc}}) && !file_empty({{doc.doc}}) ? 'update' : '"

---

::block when="file_exists({{doc.doc}})"
::block when="file_empty({{doc.doc}})"
- save the documentation to {{doc.doc}}
    - NOTE: the file already exists but is _empty_ so you will be creating this document
::end-block
