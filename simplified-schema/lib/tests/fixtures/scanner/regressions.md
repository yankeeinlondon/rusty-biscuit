# Scanner regression fixture

Frozen inputs for the balanced, parse-gated `{{ … }}` scanner. The changed
forms come from the 2026-11-05 repository corpus comparison; the prose
counterexamples are the cases a quote-aware scan without the parse gate
regressed. Edit only to add cases.

## Changed forms

Yet another option is to use `{{ "{{" }}`:

```just
braces:
  echo 'I {{ "{{" }}LOVE}} curly braces!'
```

```markdown
{{ load_data(f, "{ a: { b: string }}") }}
```

- `{{ load_data(f, "{a: {b: string}}") }}` composes as one call.
- `{{ {a: {b: 1}} }}` yields the object `{a: {b: 1}}`.
- `{{ "}}" }}` yields the string `}}`; `{{ '{{' }}` yields `{{`; an escaped
  quote inside a string (`{{ "a\"}}" }}`) does not end the string.
- `{{ 'it\'s }}' }}` keeps its escaped apostrophe.
- On its own line, `{{ "a\"}}" }}` keeps its escaped quote.
- A string spanning lines, `{{ f("multi
  line }}") }}`, yields the whole call.

## Prose counterexamples

{{ don't }} it's fine {{ x }} and more }}

{{ "open }} then {{ y }} and " more }}

{{ don't }} and later 'quoted' text {{ x }}

Ken's notes: {{ name }} isn't here, and {{ x }}'s value.

## Unchanged forms

- `{{ "a\\" }} b` ends at the escaped backslash.
- `{{ "open }} rest` keeps the older span.
- `{{ a { }} b` keeps the stray brace.
- `{{ a {{ b }} c }}` nests as before.
- `{{{VAR}}}`, `{{{ "}}}" }}}`, and `{{{a: 1}}}` are literals.
- `\{{ escaped }}` is text.

## Open to the end

These stay last: no `}}` may follow them, or the older scan would have
swallowed the expressions below it as well.

The check uses `contains("{{")` to decide whether text may hold an
expression.

Later sections still render {{ title }}, then {{ b * 2 }} and {{ c + 1 }},
and a literal token "x {{!data:v1:aGk}}" stays a token.

{{ "open and no close
