# More Context Variables

A really strong feature in Darkmatter is it's ability to provide "context variables" and make them accessible to
interpolation through the `ctx` frontmatter property.

- read [context variables](@darkmatter/docs/topics/context-variables.md) for context on what we provide today

In this feature we will add to this set of available context variables _and_ add some "context functions" to compliment them. In both cases it's critical that we keep an eye on performance. The current set of context variables uses a performant and lazily loaded approach to sourcing the context information that works well but as we add more information we should always take the time to consider the most performant way of achieving the functional goal and then make sure our benchmark testing is able to detect regressions.

## New Context Variables

### Git / Repo

The following context variables are only available in a monorepo:

- `current_packages` - _lists all packages in a monorepo which reside under the current directory_
    - format of the output should mimic:

        ```sh
        💻❯ sniff repo packages --package-area claudine --md -v
        - claudine-cli ({relative-filepath})
        - claudine ({relative-filepath})
        ```

        aka, a Markdown unordered list which shows the relative path (from repo root) to the package area

- `depends_on` - _provides a list of packages in the monorepo that the current {scope} depends on_
    - where `scope` is 
        - the current **package area** when the package area is NOT "root", otherwise
        - the current **package** when user started session inside a package defined in the package
        - lists all packages in the monorepo and each top-level item has a nested unordered list of dependencies listed
        - style:
            - UnorderedList -> `'{package}' depends on:`
                - UnorderedList -> `{dep-package}`
            - if no dependencies then: `'{package}' has no dependencies on other packages in this monorepo`
- `used_by` 
    - provides the same scoping rules of `depends_on` but instead nesting dependencies we list "dependent" packages which rely on the scoped area.
    - top level list are the packages in the scoped area
        - `'{package}' is used by:`
        - or `'{package}' is not used by other packages in this monorepo`
    - nested level is the dependent packages


## Context Functions

In this feature we're going to take context variables one step further to **context functions**.

- a context function provides _context_ to the prompt about the surrounding environment just like **context variables** do but it's super power is that it can take _parameters_
- a context function consists of lowercase alpha and `_` characters and the leads to the parameter section: `(p1, p2, p3, ...)`

### File System

- `absolute(filepath)` - take a string file path and using the `FileReference` rules resolves it to a fully qualified file path
- `relative(filepath)` - takes a string file reference and using the `FileReference` rules, and returns a relative path:
    - if in a repo:
        - relative to the repo's root when filepath is contained within the repo
    - if not a repo:
        - relative the CWD when the filepath is contained within the CWD base directory
    - if the filepath is NOT in path we will try to use an alias:
        - `~` if it's off of the user's home directory
        - ENV variables which are a valid filepath can also be used as an alias
    - NOTE: this relative path logic should already exist in Darkmatter as part of the composition pipeline, see if that can be reused.
- `file_exists(filepath)`
    - validates that the passed in filepath exists in the file system
- `frontmatter(filepath, prop)` 
    - returns the value of the specified Frontmatter property
    - if the filepath is invalid then an error is raised 
    - to protect against this type of error you can: `{{ ctx.file_exists(filepath) ? ctx.frontmatter(filepath, prop) : "oops" }}`
    - if the `prop` is not defined then we return null
- `markdown_body_empty(filepath)`
    - checks if the Markdown file referenced is "empty" where "empty" means that the body has no characters other than potentially whitespace
    - the Markdown document _can_ have frontmatter and still be considered "empty" as the only criteria is based exclusively on the body content of the document
- `markdown_title(filepath)`
    - results in an error if invalid filepath
    - if valid filepath then 
        - it returns the `title` Frontmatter of the document if it exists
        - if `title` is not present it will look for H1 Heading(s)
            - if single H1 Heading is found then the text for that heading is returned as the title
            - if more than one H1 Heading, the first is returned as a title and a warning message is sent to STDERR 
- `validate_schema(filepath)`
    - can validate the schema of a Markdown or YAML document
    - if the `$schema` property is not set then this will always return **true**
    - otherwise the schema will be used to validate the frontmatter and returns the boolean outcome
    - review the [schema documentation]() for more on Darkmatter schemas





> NOTE: for all filepath parameters, there should be a small "normalization" step which ensure that filepaths like `foo//bar` are converted to `foo/bar` and references like `file://foo/bar` have the `file://` prefix removed.


### Date/Time Functions

- `date(iso, format)` - allows you to reformat a valid ISO Date or ISO DateTime into a formatted date:
    - `MMM DD` / `short`:
        - `July 12<dim>th</dim>`
    - `MMM DD yyyy` / `short-optional`:
        - `July 12<dim>th</dim>` (for dates in current year)
        - `July 12th 1999` (for dates in a different year)
    - `MMM DD YYYY`
        - `July 12th 2026`
    - `DD MMM yyyy`:
        - `12 July` (for dates in current year)
        - `12 July 1999` (for dates in a different year)
    - `DD MMM YYYY`:
        - `12 July 2021` 
    - `Do MMM DD YYYY` / `long`:
        - `Mon, July 12<dim>th</dim>, 2021`


## Side Effects

The following expressions will allow mutations to external state
