# Seedling

A CLI that allows for creating a new repo quickly based on an existing template.

```sh
seed website stack=vite-vue
```

```mermaid
flowchart LR
    caller(CLI Host)
    template()
```

## Features

- uses git repos as the source for templates
- allows caller to build up a set of named templates that they like to use
- provides a DSL that can be located on the calling host _or_ included as part of the template repo
- the DSL provides affordances for:
    - stating a set of variables that must be set
        - the caller may pass these variables in as part of their call but if they don't then an interactive dialog will
          walk the user through the process
    - composing documents with Darkmatter including interpolating variables provided by the caller
    - removing file from the repo (this can be useful when you don't control the template website)
    - installing dependencies
        - many/most template repos will come with there own `package.json`, `cargo.toml`, etc. file which states what
          dependencies there are from a "starting point" standpoint
        - being able to _extend_ this list of dependencies is the most common use of the DSL's dependency feature
        - but we can also _override_ a dependency version, _replace_ a dependency for another

        > Note: this functionality _can_ be expressed in the template repo but is most typically expressed in the caller
        > config. In both cases the syntax is the same.

    - mutating or replacing the template's README.md
    -
