# Adding unary conditionals to Interpolation

Today in Darkmatter we have several **interpolation** based operations including:

- [Frontmatter Interpolation](@darkmatter/docs/inline/fm-interpolation.md)
- [Content Interpolation](@darkmatter/docs/inline/interpolation.md)

All interpolation operations in Darkmatter use the "handlebars" type syntax where template tags live inside of curly braces: `{{ tag }}`. Other current aspects of Darkmatter interpolation include:

- you can reference any Frontmatter property using the handlebars tags
- interpolation will also be able to leverage `env` and `ctx` dictionaries for relevant information:
    - `env` is a dictionary of all ENV variables
    - `ctx` is an enumerated set of "context variables" that is defined in: [Context Variables](@darkmatter/docs/topics/context-variables.md)
- we also provide fallback values using the `||` logical operand so that: 
    - `{{ foobar || "no foobar present" }}` will be the value of `foobar` in Frontmatter if it's defined otherwise it falls back to "no foobar present"
    - you can find more about this in [boolean conditional logic](@darkmatter/docs/topics/boolean-conditional-logic.md)

## New Functionality

The new functionality we will be providing in this feature is to allow for the commonly seen unary `if-then-else` logic such as:

```yaml
in_pkg_dir: "{{ctx.current_package}} ? 'in a package directory: {{ctx.current_package}}' : 'not in a package directory'"
```

This provides more flexibility on both sides of boolean evaluation of `ctx.current_package` (note: these evaluatuations do not require the variable to be a boolean value but are evaluated as _truthy_ or _falsy_ values).
