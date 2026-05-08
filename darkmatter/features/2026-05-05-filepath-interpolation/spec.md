We use the `FileReference` struct provided by **biscuit-file** to resolve file references in many places in Darkmatter and the ability to use magic paths as well as context-aware implicit relative paths (see @biscuit-file/docs/topics/file-references.md for more info).

However, hyperlinks or image references which use these context-aware paths are currently never transformed into a path which would be valid in the final Markdown document. That being said, it is very likely that many AI agents will be able to follow these paths (in particular the magic paths) and a human looking it likely could probably make sense of it quite easily. Still this is not an ideal situation as Markdown expects only file paths which are:

- explicitly relative paths
- explicitly absolute paths

The absolute paths are notoriously brittle and should be avoided in most cases but when we look at the composition of a full tree of documents that may include interpolation we actually **do** want to use absolute paths as an "intermediate" form of the document.

Composition of all local assets should follow a process where:

- **Link Resolve:** all links on a page are converted to absolute references during the Inline-Pre stage of the [Darkmatter Compose Pipeline](@darkmatter/docs/darkmatter-compose-pipeline.md)
    - "links" include standard Markdown syntax (hyperlinks, images) and a specific set of HTML tags/attributes:
        - `<a>` (href)
        - `<img>` (src)
        - `<video>` and `<audio>` (src and `<source src>`)
        - `<iframe>` (src)
        - `<link>` (href, for local assets)
- all transclusion is completed (where all child documents have been run through Inline Pre, Transclusion, and Inline Post stages of the pipeline) and then we finally move the **Finalization** stage of the pipeline.
- this Finalization stage is new and is **only** run on the root node for the overall compose operation
- as a part of this feature we'll add the one and only operation (**Link Normalization**) in this stage (more operations will be added later):
    - "links" which were converted to "absolute paths" during the **Link Resolve** stage will now be converted into more portable links:
        - whenever a link's absolute path is found inside the **same repo** as the base document then we will replace the absolute path with a relative path between the two documents.
            - The **Repo Boundary** is defined as the root of the current Git repository. This ensures that relative links work correctly across different workspace members within a monorepo.
        - in all other cases we will use the following logic:
            - if the referenced file is _relative_ to the user's home directory then we will use the `~` alias to reference the file. This may not be perfectly portable but it increases the portability from a link that only works on that one host to any which have the same file at the same offset to the home directory.
        - when the file is neither a part of the same repo nor relative to the current user's home directory we will then look at the ENV variables which are set:
            - We use a **Strict Whitelist** approach for environment variable selection. Only variables explicitly whitelisted in the Darkmatter configuration, or a known default set (e.g., `PROJECT_ROOT`, `DOCS_BASE`), will be considered for path abstraction to prevent leaking sensitive system paths.
            - we will filter down to just the whitelisted ENV variables which are a base directory of the file reference
            - we will choose the most specific ENV variable from the matched ENV's (the one which has the longest path AND matches the start of the file reference)
            - we will then use the ENV variable as an abstraction to make it more portable
        - because this is less portable, we will send a warning log to STDERR using the `Status` struct in biscuit-terminal: "the path <blue>{{absolute-filepath}}</blue> was found to be an offset of the <b>{ENV}</b> environment variable and will use this abstraction."
