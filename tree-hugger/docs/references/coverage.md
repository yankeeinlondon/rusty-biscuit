# Coverage and Limitation Reasons

Part of [Scope-Aware References](../references.md).

Coverage tells you how far to trust the absence of a reference. A region
whose coverage is `Complete` was analyzed with rules that can see every
call it makes; a `Limited` region holds something those rules cannot see
through, and says what. For the
[example on the entry page](../references.md#a-first-look), a file index's
`reference_coverage` reads (owner names and spans shortened):

```json
[{"span":"1:1–22:1","state":"Complete"},
 {"owner":"parse","span":"3:1–5:2","state":"Limited","reasons":["indirect_dispatch"]},
 {"owner":"run","span":"7:1–12:2","state":"Limited","reasons":["indirect_dispatch"]},
 {"span":"10:17–10:34","state":"Limited","reasons":["indirect_dispatch"]},
 {"owner":"countdown","span":"14:1–16:2","state":"Complete"},
 {"owner":"Parser::parse","span":"20:5–20:35","state":"Complete"}]
```

The first region is the whole file. `parse` calls the method `text.len()`
and `run` calls `other.parse()`, both on receivers whose type the file does
not prove. The ownerless region at line 10 is the closure, which calls
`s.len()`. `countdown` makes only a call it can prove, so its
`is_recursive` can be decided.

`reference_coverage` lists each owning region of the file (the whole file
first, then outer regions before inner ones) with an optional owner, its
span, and a state of `Complete` or `Limited` with machine-readable reasons,
in ascending order. A named callable has a region exactly when it has a body,
even one that holds no occurrence (`function e() end` in Lua, or a Lua body
holding only a comment); a signature without a body (a Rust trait method
`fn r();`) has none. A reason belongs to the innermost region where it occurs, so a
macro inside a closure limits the closure, not the function around it:

| Reason | When |
|---|---|
| `unsupported_language` | The language has no resolver rules (every region of such a file). |
| `unsupported_binding_form` | The region uses a binding form the language table does not support. |
| `missing_call_capture` | The region has invocation syntax the reference query does not capture. |
| `indirect_dispatch` | A call goes through a receiver, a variable or dynamic access, calls a name or path that is, or may be, a value (a constant, static, variable, parameter, field or property, such as Rust `Self::C()` with `const C: fn()`), or calls a value that is not a name or path once [transparent wrappers](invocation-forms.md) are removed (a sequence, conditional or dereference). A name of a function, method, constructor, enum variant, tuple struct or other type is never a value. |
| `unexpanded_macro` | A macro invocation whose expansion could hide calls. |
| `ownership_unknown` | Code whose evaluating owner cannot be established (for example field initializers). |
| `parse_error` | The region contains a syntax error. |
| `empty_reference_query` | The language has no reference query at all. |

New reasons may be added; existing ones keep their meaning.
`Complete` is never inferred from "the pass ran" or "nothing was found".

## Related pages

- [Invocation forms](invocation-forms.md): which callees limit a region and
  which do not.
- [Name lookup](name-lookup.md): the binding forms that make a name
  uncertain.
- [Language support](language-support.md): the state of each rule, per
  language.
