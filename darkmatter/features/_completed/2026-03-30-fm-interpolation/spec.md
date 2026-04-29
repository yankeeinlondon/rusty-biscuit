# Frontmatter Interpolation

We have offered [interpolation](@darkmatter/docs/inline/interpolation.md) as part of the compose pipeline for some time now but in this feature we'll be adding something we are calling "frontmatter interpolation", this new type of interpolation is:

- after the initial state is established by merging in any parent frontmatter into the current page
- we will gather all of the frontmatter properties which DO NOT have the `{{string}}` pattern in their values into a key/value dictionary
- we will then use interpolation 

## Example

We will compose the following document with the following values SET: 

```json
{
    "base": "/path/to/something"
}
```

The target Markdown document is defined as:

~~~md
---
spec: "{{base}}/spec.md"
plan: "{{base}}/plan.md"
---
# My Document

The spec is located at: {{spec}}
The plan is located at: {{plan}}
~~~

In this example 

- the Frontmatter Interpolation is kicked off _after_ we've merged in the parent state of:

  ```yaml
  spec: "{{base}}/spec.md"
  plan: "{{base}}/plan.md"
  base: "/path/to/something"
  ```

- The FM Interpolation identifies a dictionary of `{ base: "/path/to/something" }` to interpolate with
- It then interpolates the remaining properties: spec, plan

  ```yaml
  spec: "/path/to/something/spec.md"
  plan: "/path/to/something/plan.md"
  base: "/path/to/something"
  ```

- At this point the Frontmatter Interpolation operation completes
- Later the normal Interpolation process is executed and the document becomes:


  ~~~md
  ---
  spec: "/path/to/something/spec.md"
  plan: "/path/to/something/plan.md"
  base: "/path/to/something"
  ---
  # My Document

  The spec is located at: /path/to/something/spec.md
  The plan is located at: /path/to/something/plan.md
  ~~~
