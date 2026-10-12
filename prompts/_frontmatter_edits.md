<rule name="Frontmatter Edits">
**IMPORTANT:** a YAML key may appear only once in a frontmatter block. A duplicate key makes the whole file unparseable, and every later prompt that reads it fails.

When you set a frontmatter property on any file (spec, plan, review, log):

- first read the file's current frontmatter
- if the key already exists, **replace its value in place**; never add a second copy of the key
    - this includes multi-line values (`|-` blocks such as `message_to_agent`): replace the whole old block, down to the next top-level key
- if the key does not exist, add it as a new top-level key just before the closing `---`
    - never anchor an edit on a line inside `$schema:` (for example its `pipeline_step: number -> …` line); those lines describe the properties and are not the properties themselves
- edit only the keys you were asked to set; never re-paste the frontmatter, or a run of its keys, to change one of them
- do not use `md set` to write frontmatter: it reorders and re-indents the whole block

After editing, confirm the file still parses by reading back each key you set:

```sh
md get <file> <key>...
```

If `md get` reports a frontmatter parse error (for example `duplicate entry with key "…"`), fix the frontmatter before you continue.
</rule>
