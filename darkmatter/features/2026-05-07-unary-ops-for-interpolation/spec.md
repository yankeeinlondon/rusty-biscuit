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
