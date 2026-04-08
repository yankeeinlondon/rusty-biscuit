# Shell Expansion in Frontmatter

During Darkmatter's [Compose Pipeline](@claudine/docs/darkmetter-compose-pipeline.md) we have an operation we are calling [shell expansion](@claudine/docs/inline/shell-expansion.md). In this feature we will add a new operation in the Compose Pipeline called "Frontmatter Shell Expansion" and while it's related and shares some of today's "Shell Expansion" features it is also a separate operation with somewhat more limited configuration options.

## What is Frontmatter Shell Expansion

**Frontmatter Shell Expansion** allows a user to run shell commands who's STDOUT content will be placed into this Frontmatter property.

An example would be:

```md
---
files: "$(sniff repo dirty-files)"
---

The following files have changes in them:

{{files}}
```

In this example we've revealed the core syntax of **Frontmatter Shell Expansion** which is:

- a frontmatter property with a string value that
- starts with `$(` 
- and ends with `)`

The content in between the `$(` and `)` is taken as being the shell command as well as optional parameters.

## Security

The preflight checks we do for the existing [shell expansion](@claudine/docs/inline/shell-expansion.md) will be extended to detect **Frontmatter Shell Expansions** as well. Then the process of authorizing the composition of the Markdown document remains unchanged except that these new Frontmatter based shell commands will be included.

## Order Matters

The **Inline Pre** stage of the compose pipeline is run serially and the **Frontmatter Shell Expansion** is the second operations run, directly _after_ the [Frontmatter Interpolation](@claudine/docs/inline/fm-interpolation.md). Because **interpolation** _precedes_ the **shell expansion** that means that it is possible that a shell command has a variable parameter rather than a fully realized variable.

```md
---
dir: "$(dirname {{file}})"
file: ""
---
```

- in the example above we see how this could be used effectively
- but allowing an interpolated variable be part of a shell command does add some risk
- to lower this risk we will do the following things:
    - No binary can be represented by a interpolated value:
        - `$({{cmd}} p1 p2)` is NOT allowed
        - neither is `$(cat foobar.md | {{cmd}})`
        - nor is `$(cat foobar.md || {{cmd}})`
        - nor is `$(cat foobar.md && {{cmd}})`

## Timeouts

As an oversight, the original [shell expansion](@claudine/docs/inline/shell-expansion.md) did not express how long a command should be run before it _times out_. Having a timeout period is important however to avoid composition hanging or taking an unexpectedly long time.

As part of this feature we will introduce **timeouts** for both the original shell expansion as well as the new **Frontmatter Shell Expansion**.

- any command running longer than the timeout window will:
    - by default result in an error and an immediate exit from the composition process
    - a caller can indicate that they'd instead like the timeout to result in an empty string
        - in the library this should be added as an option
        - in the CLI we should add a `--allow-shell-timeout` flag to enable this feature
    - the default timeout window is 10 seconds
    - if a caller wants to change this timeout window then they can:
        - globally change the timeout to some other value (always measured in seconds)
            - the library should expose this as an option
            - `--timeout #` in CLI
        - if we want to modify the timeout of the specific shell call then:
            - in the Frontmatter we will allow the postfix of `::timeout:{#}` to be appended AFTER the closing `)` character. For example: `$(ls -la)::timeout:1` would change the timeout to 1 second.
            - in the body we would allow the same sort of syntax:

                ```md
                ::shell ls -la ::timeout:1
                ```
                
