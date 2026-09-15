# File Contents Directive

## Directive Overview

The file contents directive allows you to assign _text_ content from a file into a Frontmatter variable. An example of what this would look like is:

```yaml
contents: "<< ./some-content.md"
```

The `./some-content.md` is what we will call the content's **source** and a source can be:

- a file reference (supporting all local [file references](../topics/file-references.md) we support elsewhere in Darkmatter)
- the only _valid_ types of files to be brought into a Frontmatter variable will be text files (more on that in the type validation section)
- in the future we may consider allowing HTTP URL's to gather remote textual content but today that remains out of scope


## Return Types

The return _type_ that this directive provides depends on whether the optional `as` and `take frontmatter` operators are used:

```yaml
contents: "<< ./some-content.md as string"
```

Let's discuss some basic rules that the `<< {source}` directive uses for typing:

1. the `as` operator consumes a "type" as defined by [SimplifiedSchema](../topics/simplified-schemas.md)
2. the `as` operator is purely optional and without it Darkmatter will specify the return type purely on the file's file extension:
    - `.md` -> string
    - `.txt` / `.text` -> string
    - `.yaml` -> unknown
    - `.toml` -> unknown
    - `.json` / `.json5` -> unknown
    - no other file extension is allowed currently and will result in an error
3. if what we want from a Markdown file -- and this feature is ONLY allowed for files with `.md` file extensions -- is not the file's content but instead the Frontmatter contained in that file then we can use the `take frontmatter` directive:

    ```yaml
    fm: "<< ./some-content.md take frontmatter"
    ```

    - by default when we express just `take frontmatter` the return type will become "object"
    - however, we can combine the `take frontmatter` and `as` operations to provide more type information:

        ```yaml
        fm: "<< ./some-content.md take frontmatter as {  }"
        ```

4. 



## Handing File Validation
