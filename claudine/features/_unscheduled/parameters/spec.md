# Parameters Feature

We have introduced `compose`, `inline-compose`, and `sequence` commands to claudine which all require a "file-reference" be passed in. In this feature we're going to allow for Markdown document to **declare** the parameters they are expecting (beyond the initial file reference which _points_ to the entry prompt/markdown document). In the process of doing that we're going to introduce a basic "schema" solution for Claudine too.

## Syntax

Any markdown document which has expectations about being passed parameters -- regardless of whether they are required or optional -- must set the `parameter` frontmatter property. This parameter can be either an inline definition or a file reference to an external definition. In both cases, however, the variables defined are **named** parameters not **positional** parameters.

### Calling Example

Let's say that we have a prompt "@testing.md" which defines a property `color` that is required and must be a **string** type.

- A successful call might look like this:

    ```sh
    claudine compose "@testing.md" color="red"
    ```

- If we leave out the required `color` variable:

    ```sh
    claudine compose "@testing.md"
    ```

    We will get a `MissingParameter` error which looks something like:

    - `Status::with_prose("<b><red>MissingParameter:</red></b> the <blue>@testing.md</blue> prompt requires that the parameter <green>color</green> be passed in with a <i>string</i> value!").state(Error)`

- If we provide the `color` variable as a number:

    ```sh
    claudine compose "@testing.md" color=42
    ```

    We will get a `InvalidParameterType` error which looks something like:

    - `Status::with_prose("<b><red>InvalidParameterType:</red></b> the <blue>@testing.md</blue> prompt requires that the parameter <green>color</green> be passed in with a <i>string</i> value!").state(Error)`

## Defining Parameters

### Inline

Let's start with an example:

```md

```

### External Reference
