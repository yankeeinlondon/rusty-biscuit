# Preserve whitespace within schema list values

The following Markdown frontmatter produced a schema diagnostic at the `v`
in `GPL v3`. The same list also contains another unquoted multiword value,
`OSI Approved`.

```yaml
$schema:
  license: string(suggest(MIT, BSD, "Apache 2.0", "GPL v2", GPL v3, MPL, OSI Approved, AGPL))
```

## Grammar

- Read each bare `suggest(...)` candidate and `enum(...)` member through its
  next delimiter. Trim surrounding grammar whitespace (ASCII spaces, tabs,
  and line breaks) and preserve interior whitespace exactly.
- Use either quote style to preserve whitespace at the edges of a value or
  include punctuation such as a comma, semicolon, or parenthesis in the
  value. A semicolon after the last enum member starts its constraint list.
- Keep the existing interpretation, source spans, and duplicate handling for
  candidates. A numeric suggestion containing spaces remains invalid
  suggestion metadata when it does not form a simple decimal.
- Keep `{{ … }}` available as a complete element. Keep the existing grammar
  for other argument positions, including patterns and file references.

## Verification

- SimplifiedSchema preserves the decoded values and their source spans, and
  continues to handle quoted delimiters, enum constraints, and list slots.
- Darkmatter validates Markdown with the widened grammar. Its language
  server offers `GPL v3` as one frontmatter completion.
- Claudine can prepare a prompt with an inline schema using a multiword
  suggestion and a multiword enum member.
