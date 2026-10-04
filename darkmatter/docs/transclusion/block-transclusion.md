# Block Transclusion

## Local Markdown Files

The most basic and most often used form of transclusion in Darkmatter is where you inject another local Markdown file into your Markdown document. For listing document files as a linked tree instead of inlining their content, see the [`::file-links`](../inline/file-links.md) directive.

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

When we kickoff the [markdown pipeline](../darkmatter-compose-pipeline.md) we're allowed to pass in some initial "state". This "state" is a key/value structure and if it's provided then it will be provided to the base document as _default values_ for the base document's frontmatter. That means before any transforms are done the base document's frontmatter will be a merged dictionary of:

- a base of the key/value passed in as the initial state
- any frontmatter that was hardcoded on the page will be maintained (and override any conflicting value passed in)
- when both the initial state and the frontmatter have a shared property which is a key/value type:
    - we merge each property of the key/value rather than treating the object as a whole
    - this means that the resulting object will have a union of the keys specified between the two and the values of these keys will give precedence to the value in the frontmatter

This process is a form of state transfer is not confined to JUST the very start of a pipeline being started but rather happens once for every document in the document tree which is engaged with. As we've already pointed out Transclusion is a recursive process so that means that every parent document which transcludes a child document will go through the same merging of states (parent to child).

### Filepaths

We will _resolve_ file referencing by leveraging the `biscuit-file` library's [`FileReference` struct](^biscuit-file/lib/src/file_reference/mod.rs).

The example showed a relative path used to the foreign file and this is the recommended way to reference local files in most cases but it is not the only approach. Here's a summary of the allowed file referencing strategies:

- relative paths

    - the most popular and easily to reason about
    - an **explicit** relative path carries a leading `./` (or `../`) and looks for documents from the directory of the base document **only**
    - a **bare/implicit** relative path (no leading `./`, e.g. `some-content.md` or `sub/some-content.md`) is resolved **from the base document's directory first, then the repository root**

- absolute paths

    - absolute paths are allowed but they are brittle and should be avoided in 99% of cases.
    - absolute paths are identified by the leading `/` character in the filepath
        - for Windows users we're expecting use of the WSL/WSL2 shells

- ${HOME} based paths

    - we often like to refer to files _relative_ to the user's home directory
    - we support this with a leading character of `~` to indicate the home directory
    - transclusion operations which are based on either a single machine or at least the same user (with some assumed standards in how the home folder is organized)

- Repo based paths

    - When the root document is a part of a git repo it is often natural to refer to documents from the root of this directory
    - This approach is the safest approach to use when wanting to ensure that content references are always a part of the repo
    - a leading `&` character pins the reference to the repository root exactly
    - a leading `^` character is the monorepo-aware variant: it searches the enclosing package root, then the package-area root, then the repository root, and takes the most specific match
    - both forms require the document to be inside a repository and reject any target which escapes it

- Magic paths

    - a leading `@` character searches a prioritized list of roots: any roots the embedding application registered as prepends, then the package root, the package-area root, the repository root, and the user's home directory, then any registered appends
    - unlike `&` and `^`, a magic path *can* reach the user's home directory, so it is the right choice for "find it in the usual places" and the wrong choice when repository containment is the point

- ENV based paths

    - in the [Markdown Pipeline](../darkmatter-compose-pipeline.md) processing during the "prep" stages (aka, before we render transclusion) we have a Frontmatter interpolation stage which is run. This stage has access to environment variables and so it's possible to use these ENV variables to dereference your file paths.
    - for example, you could specify:

      ```md
      ::file {{ env.SOME_ENV_VAR }}
      ```

    - and by the time the _transclusion_ stage was reached this reference to an ENV variable would have been replaced with the value of that ENV variable.
    - this strategy of file referencing will typically result in an absolute path but with an abstraction layer that makes it potentially more portable then a static absolute path.

### File Trees

Every document has a **tree root** (`base_dir`): the directory its relative references (`./x.md`, `../x.md`, `x.md`) may move around in but not leave. A relative reference whose target lands outside the tree root is an error, not a missing file:

```text
repo/                    ← tree root of every document inside the repository
├── shared.md
└── docs/
    └── guide.md         ::file ../shared.md       → repo/shared.md        (inside: fine)
                         ::file ../../outside.md   → error: leaves repo/   (outside)
```

The tree root is chosen by the first rule that applies:

1. the **repository** the document is in;
2. a root the host supplied explicitly;
3. the deepest configured **vault** that contains the document;
4. the **anchor of the reference that opened the document**, when it is `~` (or a leading `{{VAR}}` file-reference anchor) and the anchor contains the document;
5. otherwise the document's own directory. This last one is only a fallback: nothing said where the tree is, so it rejects nothing.

Rule 4 is what gives a document outside any repository a boundary:

```md
<!-- work/parent.md (not in a repository) -->
::file ~/Downloads/a.md
```

```md
<!-- ~/Downloads/a.md: its tree root is ~ -->
::file ../b.md            → ~/b.md (inside the home tree)
::file ../../outside.md   → error: leaves ~
```

Opened as `::file /Users/me/Downloads/a.md` instead, the same `a.md` has no anchor, falls back to its own directory, and `../../outside.md` resolves. An anchor never replaces a tree that already contains the document: `~/repo/docs/a.md` inside the repository is still bounded by the repository.

For an environment anchor, remember that the interpolation stage evaluates `{{ … }}` before transclusion runs (see "ENV based paths" above): in `::file {{NOTES}}/inbox/a.md`, `{{NOTES}}` is evaluated as a Darkmatter expression (an unknown name, so the target becomes `/inbox/a.md`) and no anchor is left. Write the anchor with triple braces so the composed directive keeps a literal `{{NOTES}}` for the file reference to resolve:

```md
::file "{{{NOTES}}}/inbox/a.md"   → a.md's tree root is $NOTES
```

A tree-root error follows the rules in [When an Included File Fails](#when-an-included-file-fails): inside a nested document it becomes a `_Could not transclude …_` notice and a warning in lenient mode, and stops the composition with `fail_fast`. `&` and `^` keep their own repository-only rule.

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

    If you want to add the child document as part of a progressive disclosure section of the document then assign `disclosure` to the summary text you want to show and the transcluded document will then be hidden by default but clicking on the summary text will make the transcluded content visible.

    ```md
    ::file ./long-section.md disclosure="License Agreement"
    ::file ./short-section.md disclosure=true
    ```

    - `disclosure="Summary text"` wraps the transcluded content in a `::disclosure` block with the given summary.
    - `disclosure=true` (or an empty summary) uses the default summary `"Details"`.
    - The transclusion stage emits the render-time DSL (`::disclosure`, `::details`, `::end-disclosure`), not inline HTML. The disclosure is expanded to the appropriate target during rendering. See [Disclosure Blocks](../rendering/disclosure.md) for target behavior and styling.

4. `exclude`

    Sometimes we want to transclude a document into a part of the parent document but we want to expressly **exclude** certain heading sections. This is what the **exclude** option is for:

    ```md
    ::file ./some_content.md exclude="## Bad Content for Bad People*"
    ```

    This will include the Markdown content from `./some_content.md` but before adding it it will look for a H2 section who's title _starts with_ (because of the `*` wildcard usage) and remove those sections.

    - The **exclude** command can be used more than once for a single transclusion but each exclusion needs to express the full `exclude={match-pattern}` key value pair.
    - The exclusion requires a valid _exclusion_ string; valid strings:
        - start with `## `, `### `, `#### `, `#### `, and `##### ` for the H2 to H6 headings
            - a match is achieved when
        - or equal `!prelude` - any content _before_ the first heading tag (of any level)

5. `set`

    The `set` option allows the parent document to override frontmatter properties on the transcluded child document. This is useful for parameterizing transcluded content — the child's interpolation, page blocks, replace rules, and shell directives all observe the overridden values.

    There are two forms:

    **Object form** — pass a JSON5 dictionary that deep-merges onto the child's frontmatter:

    ```md
    ::file child.md set='{name: "Bob", age: 42}'
    ```

    **Property form** — set individual properties using dot-free identifiers:

    ```md
    ::file child.md set.name="Bob" set.age=42
    ```

    Both forms can coexist on the same directive; property-form values take precedence over the object form when keys overlap.

    The merge uses a **three-layer precedence** model:

    1. **Base** — the child's own frontmatter
    2. **Middle** — the object-form `set=<dict>` payload (deep-merged onto the base)
    3. **Top** — each `set.NAME=<value>` property (overrides both layers above)

    Dict values deep-merge (union of keys, higher layer wins on leaf conflicts). Arrays and scalars (including `null`) hard-override. For example, `set.x=null` sets `x` to the literal null value — it does **not** delete the key.

    The overlay is applied **before** any of the child's pre-op pipeline stages run, so child interpolation (`{{ name }}`), page blocks (`::block when="role == 'admin'"`), replace rules, and shell directives all see the overridden values. The overlay does **not** propagate to grandchildren — only the direct child sees it.

    **Error handling:**

    - `set=42` (non-object JSON5) raises `InvalidFrontmatterAssignment`. Use `--allow-invalid-frontmatter-assignment` to downgrade to a warning; sibling valid set clauses still apply.
    - `set.name="Bob" set.name="Mary"` raises `InvalidReassignedFrontmatterProperty`. Use `--allow-reassigned-frontmatter-property` to downgrade to a warning; the rightmost assignment wins.

And then finally the maybe most powerful option/key is `when` which provides _conditional_ transclusion.

#### Conditional Transclusion

The `when` option allows you to express a condition which must evaluate to `true` for the transclusion to be included. When the condition is false, nothing is rendered and the directive is removed.

Transclusion uses Darkmatter's shared [Darkmatter Expressions](../topics/darkmatter-expressions.md) evaluator. That same evaluator is also used by page blocks, so `when=` behaves consistently across both features.

Conditions can read from:

- frontmatter and inherited compose state
- `ctx.*` runtime context variables
- `env.*` environment variables

Common patterns include:

- equality checks like `stage == 'draft'`
- env gates like `env.AGENT == 'claude'`
- truthy checks like `draft`
- negation like `!env.AGENT`
- compound conditions like `and(release.enabled, env.CI)`

Example:

```md
::file ./possibly-interesting.md when="or(env.OPENAI_API_KEY, env.ANTHROPIC_API_KEY)"
```

in this example:

- the two environment variables are evaluated to see if they are _truthy_
- if either one is then the condition result in a `true` outcome and the transclusion is executed

For the full grammar, truthiness rules, supported operators, functions, and edge cases, see [Darkmatter Expressions](../topics/darkmatter-expressions.md).

### When an Included File Fails

An included file is composed as part of the same composition, so what happens when it fails depends on why it failed.

- **A shell span fails the composition, exactly as it would inline.** A `::shell`, `::shell-block`, or frontmatter `$( … )` that fails without a handler (a non-zero exit with no `when_error`, a missing program, a timeout without `--allow-shell-timeout`) stops the composition, and the error names the included file. A command the author gave a fallback renders that fallback, as it does inline.
- **Structural and context failures also stop it:** a cycle, the depth limit, a remote fetch that failed, a `ctx` value the composition never captured, and a command missing from the approved set.
- **Any other failure is tolerated in lenient mode (`fail_fast` off, the default).** The included file is replaced by a visible `_Could not transclude `name`_` notice, a warning is reported, and the parent composes on. With `fail_fast` on, it stops the composition.

```md
<!-- facts.md -->
::shell-block when_error="(unknown)"
git rev-list --count origin/main..HEAD
::end-block
::shell-block
just ci-local --plan
::end-block
```

Transcluding `facts.md` with `git` working but `just` missing stops the composition with an error that names `facts.md`. With `just` present and the `git` command exiting non-zero, the first block renders `(unknown)` and the composition continues.

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
