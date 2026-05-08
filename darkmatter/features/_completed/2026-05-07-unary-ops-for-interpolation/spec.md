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

In addition to a single logical evaluation we can also find situations where it makes sense to have another logical evaluation in either branch of the `{cond} ? {when_truthy} : {when_falsy}` expression. This means that this pattern is actually a recursive pattern. All of the following expressions are valid:

- `{cond} ? "truthy" : "falsy"`
- `{cond} ? {cond2} ? "truthy-truthy" : "truthy-falsy" : "falsy"`
- `{cond} ? {cond2} ? "truthy-truthy" : "truthy-falsy" : {cond-3} ? "falsy-truthy" : "falsy-falsy"`

To help the user focus on the groupings we will allow them to use parenthesis to group a tree of nodes. These parenthesis don't change the outcomes at all but they help the human viewer understand better the groupings. As an example, the last example above could be also be written as:

- `{cond} ? ( {cond2} ? "truthy-truthy" : "truthy-falsy" ) : {cond-3} ? ("falsy-truthy" : "falsy-falsy")`

In this second articulation we see:

- the parenthesis -- with or without interior whitespace -- act as a visual indicator of groups
- in this example we have a the top level condition and then the true and false paths are grouped together with parenthesis
- to re-emphasize, this produces **exactly** the same result as the same expression without the parenthesis

Beyond the human being better able to visually see the groups, the parenthesis will also reject invalid grouping. For instance the following are invalid expressions:

- `{cond} ? ( {cond2} ? "truthy-truthy" : "truthy-falsy" : {cond-3} ? ("falsy-truthy" : "falsy-falsy")`
    - a group can not encapsulate both the `true` and `false` path of the top level comparison!
- `{cond} ? ( {cond2} ? "truthy-truthy" : "truthy-falsy" : {cond-3} ? ("falsy-truthy" : "falsy-falsy"`
    - a grouping must be balanced, here there is no terminating closing parenthesis
- `{cond} ?  {cond2} ? "truthy-truthy" : "truthy-falsy" : {cond-3} ? ("falsy-truthy" : "falsy-falsy")`
    - this represents another unbalanced set of parenthesis but in this case it's the leading/openning parenthesis which is missing
- `{cond} ? {cond2} ? ( "truthy-truthy" : "truthy-falsy" ) : {cond-3} ? ( "falsy-truthy" : "falsy-falsy" )`
    - the parenthesis must wrap a complete "top level" pattern consisting of **condition**, **true path**, **false path** and in this example we the parenthesis surrounding the true value, the `:` operand, and the false value.
- `{cond} ? {cond2} ( ? "truthy-truthy" : "truthy-falsy" ) : {cond-3} ( ? "falsy-truthy" : "falsy-falsy" )`
    - another example where the parenthesis encapsulates a part of the top level expression but not the full expression
