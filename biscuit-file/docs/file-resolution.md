# File Resolution

The `biscuit-file` library provides a handy way to allow for _file references_ to be captured and then resolved at some later period in time.

## File References

A _file reference_ is any string based descriptor of a file. All of the following would be considered valid file references:

- **Relative Paths**
    - `./foobar.md` - a filename we would expect to be in the _future_ current working directory
    - `./foo/bar/foobar.md` - a filename we expect to be nested under the _future_ current working directory
    - `../foobar.md` - a filename to be found in the parent directory of the _future_ current working directory
- **Absolute Paths**
    - `/Users/bob/foobar.com`

> **Note:** the reason we say _future_ current working directory versus just "current working directory" is that when we first define the file reference the current working directory at that point in time has NO influence on the file's filepath resolution later when we call `.resolve()` or `.resolve_relative()`. The CWD which _does_ matter for relative paths is the CWD at the time we call the resolve method.

All of the above examples are the basic table stakes for a file reference but we go further. In the next set of subsections we'll explore more advanced solutions which are possible.

### Magic References

The `@` symbol is imbued special meaning when added as the first character of a file reference. This symbol will serve as a replacement for (in this order):

- the root of the current git repo (if in a repo)
- the user's HOME directory

That means that:

- `@foo/bar/foobar.md` 
    - is looked for at the root of the current repo
        - e.g., a relative path of `./foo/bar/foobar.md` from the root of the repo
        - the "repo" is determined by the current working directory at the time of _resolution_ (not at the time of reference definition)
    - if not found there then it will try to resolve to the path `${HOME}/foo/bar/foobar.md`

#### More Magic on Demand

The resolution process described above is the default but we can add to it with builder functions:

```rust
let ref: FileReference = FileReference::new("@foobar.md")
    .add_magic_path(PathPosition::End, "/User/bob/.claude/dir")
    .add_magic_path(PathPosition::Start, "/Library/Applications/foobar");
```

With the example above, the `ref` file reference when asked to resolve the file path will look for `foobar.md` in:

- `/Location/Applications/foobar/foobar.md`
- if CWD is in a git repo, `{RepoRoot}/foobar.md`
- and finally in `/User/bob/.claude/dir/foobar.md`




### Package Root References

When the file reference's first character is a `!` we treat this as a relative reference to the current repo's package.

This means that, at the time of resolution, we look at the CWD and:

- if the CWD is not in a git repo then the file will never resolve
- if the CWD **is** in a git repo but it is _not_ a monorepo then it will resolve from the root of the repo
- if the CWD is in git repo and it is a monorepo then we will resolve relative to the current "package root".

### Vault References

When a file reference is prefixed with `vault:` this make the file reference an Obsidian _vault reference_.

- Obsidian allows 1 or more "vaults" to be defined to a host computer and each vault is given a filepath representing it's root directory.
- The content of Obsidian notes is Markdown but with some small variances from normal Markdown
    - the most important variance is the use of two-way links like `[[something]]` which is a link to a page named, or aliased "something" somewhere in the vault
    - This form of linking pages adds a level abstraction and empowers the "two way" linking feature that means when you link to a page, that page then can determine who links to it (aka, a back-link).
    - use the `obsidian` skill for more details on this
- To be able to have vault references resolve you must either use the builder methods provided to add vaults or have the VAULT environment variable set.

Example:

```rust
let ref = FileReference::new("vault::foobar.md")
    .add_vault("/Users/bob/my-vault");
```

### Recursive References

A "recursive" reference is indicated by a leading `%` character in the file reference. It will look for the file in the target directory and all sub-directories:

- `%foobar.md` will look for the file `foobar.md` in the future CWD and all subdirectories
    - `%./foobar.md` works the same
- `%@foobar.md` will look at all the magic paths defined and and their subdirectories for a file named `foobar.md`
- `%!foobar.md` will resolve the package root directory and look for `foobar.md` in the package root and all its subdirectories

The resolution of a recursive reference will always resolve to `Option<Path>` which means that if multiple files are found only the first one is returned.

### Interpolation

While we've seen various strategies or scopes for where to look with things like magic references, package root references, etc. so far all our file references have been static.

You can, however, use references to ENV variables:

- a reference to the ENV variable LOCATION could be incorporated into a FileReference with `FileReference::new("{{LOCATION}}/foo/bar/foobar.md")`
- to resolve a file, the variables contained in it must all be defined when the file is being resolved (not when defined).

## CLI Integration

The idea of a file-reference in the CLI makes sense even if the idea of lazily defining and then later resolving will be compressed into one transaction.

We will add a new subcommand called `reference` with alias of `ref` which can be used to look for a file reference:

```sh
bf reference @foobar.md
```

This will look for the file foobar.md in the repo's root and then the user's home directory. It will return the first match it finds as a fully qualified absolute path to the file.

We will also add the following CLI flags to modify behavior:

- `--relative` will return a _relative path_ to the file instead of an absolute path. The relativeness would be to it's base path:
    - in our example there are multiple base paths (e.g., repo root, user's home directory) so the relative path leaves some ambiguity
- `--relative-cwd` will a _relative path_ to the file where the relative base directory is always the current working directory
    - that means that if foobar.md were found in the `~/foobar.md` and the CWD is `~/some/path` then the relative path returned would be `../../foobar.md`
- `--add-vault` / `-v` allows you to specify a path for an Obsidian Vault
