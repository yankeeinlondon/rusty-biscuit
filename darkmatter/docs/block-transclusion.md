# Block Transclusion

## Local Markdown Files

The most basic and most often used form of transclusion in Darkmatter is where you inject another local Markdown file into your Markdown document.

```md
## My Section

::file ./some-reusable-content.md
```

This basic example will bring in another file's content but more specifically it will:

- load the foreign content into a `Markdown` struct of it's own and:
    - run through a full transform pipeline (including cleaning markdown, normalizing content, interpolation, and any downstream transclusion this file needs)
        - the fact that a foreign file may itself perform transclusion brings up an important point ... transclusion is an inherently recursive process
        - to implement a safe transclusion system we must ensure that we're able to detect "loops" in transclusion dependencies. A valid dependency tree for transclusion must be
- once the foreign document has been processed through it's own markdown pipeline, the foreign document is run through a "re-leveling" process to make sure that the headings structure of the foreign document _fits into_ the section it's being injected into
    - In our example above, the root document is at a H2 level when it calls for transclusion
    - this means that the top level heading in the foreign document must be made to be H3 (while maintaining it's structure)
    - this functionality exists already in the `Markdown` struct's `relevel()` function

### Frontmatter and Recursion

When we kickoff the [markdown pipeline](./darkmatter-pipeline.md) we're allowed to pass in some initial "state". This "state" is a key/value structure and if it's provided then it will be provided to the base document as _default values_ for the base document's frontmatter. That means before any transforms are done the base document's frontmatter will be a merged dictionary of:

- a base of the key/value passed in as the initial state
- any frontmatter that was hardcoded on the page will be maintained (and override any conflicting value passed in)
- when both the initial state and the frontmatter have a shared property which is a key/value type:
    - we merge each property of the key/value rather than treating the object as a whole
    - this means that the resulting object will have a union of the keys specified between the two and the values of these keys will give precedence to the value in the frontmatter

This process is a form of state transfer is not confined to JUST the very start of a pipeline being started but rather happens once for every document in the document tree which is engaged with. As we've already pointed out Transclusion is a recursive process so that means that every parent document which transcludes a child document will go through the same merging of states (parent to child).

### Filepaths

The example showed a relative path used to the foreign file and this is the recommended way to reference local files in most cases but it is not the only approach. Here's a summary of the allowed file referencing strategies:

- relative paths

    - the most popular and easily to reason about
    - they are identified by a leading `./` in the file path
    - these paths look for documents from the directory of the base document

- absolute paths

    - absolute paths are allowed but they are brittle and should be avoided in 99% of cases.
    - absolute paths are identified by the leading `/` character in the filepath
        - for Windows users we're expecting use of the WSL/WSL2 shells

- ${HOME} based paths

    - we often like to refer to files _relative_ to the user's home directory
    - we support this with a leading character of `~` to indicate the home directory
    - transclusion operations which are based on either a single machine or at least the same user (with some assumed standards in how the home folder is organized)

- Repo Root based paths

    - When the root document is a part of a git repo it is often natural to refer to documents from the root of this directory
    - This approach is the safest approach to use when wanting to ensure that content references are always a part of the repo
    - we identify Repo Root file references by a leading `@` character

- ENV based paths

    - in the [Markdown Pipeline](./darkmatter-pipeline.md) processing during the "prep" stages (aka, before we render transclusion) we have a Frontmatter interpolation stage which is run. This stage has access to environment variables and so it's possible to use these ENV variables to dereference your file paths.
    - for example, you could specify:

      ```md
      ::file {{ env.SOME_ENV_VAR }}
      ```

    - and by the time the _transclusion_ stage was reached this reference to an ENV variable would have been replaced with the value of that ENV variable.
    - this strategy of file referencing will typically result in an absolute path but with an abstraction layer that makes it potentially more portable then a static absolute path.

### Options and Conditionals

The syntax we've covered so far for block file transclusion is just `::file <filename>` and that is how a block file transclusion MUST start but beyond that we offer a way to assign key/value pairs to modify the behavior of the transclusion. The full syntax looks something like: `::file <filename> <key>=<value> <key>=<value>` and the `keys` represent the various aspects you're allowed to modify. These include:

1. `replace`

    - In the [text replacement](./text-replacement.md) stage of the pipelining process we look for a `replace` property in the frontmatter and if it exists we use it to do a global search-and-replace of keys -> values.
    - The typical process of passing the frontmatter's `replace` dictionary from parent to child document follows the rules described above in [frontmatter and recursion](#frontmatter-and-recursion) but this replace property used as an "option" to the transclusion allows variation:
      - if this option is not used then the behavior of passing `replace` is unchanged
      - if this option is set to the value of `true` then there is an inversion of precedence:
          - instead of the child document's values being given precedence, the parent/calling document's values are given precedence
      - if this option is given a `JSON` or `JSON5` value then:
          - the key/values defined by the JSON/JSON5 will be serialized into a key/value and included in the "replace" property merge behavior
          - this modified merge process now becomes:
              - base merge: the base merging of the parent and child key/values is unchanged
              - one-off: the serialized JSON/JSON5 key/value is processed independently and _before_ the normal merge process
              - inheritance: if the child receiving this one-off change uses transclusion itself -- and thereby passes along it's own `replace` key/value to it's child then the key/value will consist of ONLY the base merge and not the one off key/value.

2. `quotation`

    - if you want the child document to be added into the document as a block quote then this property can be set to `true`.
    - if you want to add an attribution to this block quote then assign the `quotation` to the attribution text

3. `disclosure`

    If you want to add the child document as part of a progressive disclosure section of the document then should assign `disclosure` to the summary text you want to show and the transcluded document will then be hidden by default but clicking on the summary text will make the transcluded content visible.

    > Note: this capability leverages the `<details><summary>summary text<summary>...</details>` HTML functionality. This means it works perfectly when you're output format is HTML and for the Markdown content it's also using inline HTML to represent this feature so it's support will depend on the _reader_ software you're using.
    >
    > Note: when outputting to the terminal (e.g., with escape codes and formatting), the progressive disclosure feature is stripped out and the full text is displayed as there's no way to get this to work in the terminal.

And then finally the maybe most powerful option/key is `when` which provides _conditional_ transclusion.

#### Conditional Transclusion

The `when` open allows you to express a condition which must equal `true` for the transclusion to be included. When a condition reaches a `false` outcome then nothing is rendered (and the transclusion reference is removed).

The conditional logic provided for is based on the values of frontmatter and in the same way that the [interpolation](./interpolation.md) stage got some helpful utility properties injected into the frontmatter for interpolation, we get that SAME `ctx` and `env` based frontmatter dictionaries made available here.

Logic operations include:

- `{property} == {property}` equality
- `{property} != {property}` not equal
- `{property} > {property}` - greater than (_properties which are unable to be converted to a number are converted to 0_)
- `{property} >= {property}` - greater than or equal (_properties which are unable to be converted to a number are converted to 0_)
- `{property} < {property}` - less than (_properties which are unable to be converted to a number are converted to 0_)
- Unary ops
    - `{property}` truthy evaluation
    - `!{property}` falsy evaluation
- Functions
    - `HasKey({property}, key)` - _tests whether property is a dictionary and "has" the specified "key"_
    - `Contains({property}, value)`  - _tests whether a property is an array or an object and whether one of it's elements/values is the value specified_
    - `Length({property})`
        - if the property is an array then the numeric value represents the length of the array
        - if the property is a dictionary then the numeric value represents the number _keys_ in the dictionary
        - in both numeric and string values it returns the character length
        - in boolean values it return 0
- Combinators
    - `And(a,b,c)` - _a tuple of properties or operations which are each evaluated to a true/false value and if ALL are `true` then the resultant value is `true`_
    - `Or(a,b,c)` - _a tuple of properties or operations which are each evaluated to a true/false value and if ANY are `true` then the resultant value is `true`_

Example:

```md
::file ./possibly-interesting.md when="Or(env.OPENAI_API_KEY, env.ANTHROPIC_API_KEY)"
```

in this example:

- the two frontmatter variables are evaluated to see if they are _truthy_
- if either one is then the condition result in a `true` outcome and the transclusion is executed

## Non Markdown Local Files

Up to known we've focused on Markdown documents _transcluding_ other Markdown documents and that is overwhelming the most common use case, however, we do support a few other file types.

- `PDF` - we leverage the `biscuit-file` library in this monorepo to take advantage of it's ability to convert PDF's into Markdown.
- `TEXT` - any text document with a `.txt` or `.text` file extension is also allowed and just brought into the document "as is" with no expected formatting or structure.
- `CSV`, `TSV` - files with `.csv` or `.tsv` extensions can be brought in and will be converted to a Markdown table

We may add other formats over time -- Word and Excel docs and HTML are likely at some point -- but for now any other file type should result in an error.

## Remote Content

Beyond local content we support the transclusion of remote content too using the syntax:

```md
## My Section

::url https://site.com/content.md
```

The same options we used for local content can be used for remote content but remote content does introduce some additional challenges:

- Content type identification
- Network Outages
- Slow connections

For these reasons we will start by implementing only local documents.
