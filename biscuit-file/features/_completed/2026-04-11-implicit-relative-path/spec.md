# The "implicit relative path" for FileReference

When a file reference does not:

- explicitly express itself as an absolute path by leading with a `/` character
- nor explicitly express itself as a relative path by a leading "." (e.g., `./foobar.md`, `../foobar.md`)
- nor use the `@` magic operator (already implemented and documented)
- nor use the `vault:` or `!` prefixes
- We then have either an **invalid** path or a path that _implicitly_ is expressing a relative path and this feature is about making sure it that _implicit_ relative paths are handled correctly.

## Handling

The missing variable for an _implicit_ relative path is the directory the path is _relative to_. 

- when someone makes a reference to "foobar.md" they are VERY likely referring to the `foobar.md` in the current working directory, however, that is NOT the only location that might be considered valid
- another common convention is that an implicit relative path -- when working inside a repo of some sort -- is a relative path from the repo's root.

When someone uses the `FileReference` struct with an implicit relative path we will check (in this order):

- the current working directory
- the repo's root directory (if in a git repo)

If we match in the current working directory then that is used, the repo's root is used as the fallback.

> **Note:** in the examples so far we've used examples where the implicit relative path had no path segments but there is nothing wrong with that and that will behave the same. For instance, the `foo/bar/doc.md` reference will be checked relative to the CWD and the repo's root. If neither have a `./foo/bar/doc.md` hanging off of them then this is not a valid reference.


