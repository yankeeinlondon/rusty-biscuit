---
dir: {{ctx.package_area_root}}
area: {{ctx.current_package_area}}
log: "{{dir}}/blast-radius.jsonl"
operation: "blast-radius"
---
# Blast Radius

The following documents in the '{{area}}' package area have a `blast_radius` frontmatter property:

::shell sniff docs --blast-radius | rg "{{area}}"

## What a "blast_radius" Means

The `blast_radius` property will be a list of source code files. These source code files are the files which the document is _sensitive_ to (aka, if these source code files change then this document likely should change).

## The Log History of Checks

To keep track of when the '{{area}}' package area has run updates on 
