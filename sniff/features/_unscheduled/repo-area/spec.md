# Repo Area feature

## Summary

Add a `sniff repo area` command to the CLI and parallel "area" functionality to the sniff library. The "area" combines the existing notions of "package" and "package-area" into a single name that is most useful inside a monorepo: when you are inside a package, it's the package name; otherwise it falls back to the package-area string.

## CLI behavior

`sniff repo area` mirrors the shape of `sniff repo package` and `sniff repo package-area`.

### Success output

On success, print the bare area name to **stdout** followed by `\n`. No styling, no decoration.

### Flags

Inherited global flags (declared on the root `Cli`, available to every subcommand):

- `--base <path>` — if provided, replaces CWD as the directory the area is computed against. Otherwise CWD is used. There is no positional dir argument.
- `--verbose` / `-v` — styled output, and styled error messages on stderr (see below).
- `--json` — emit the standard `name_outcome` JSON shape (an object with a `name` field), mirroring `sniff repo package`.

Command-local flags:

- `--no-error` — suppress the non-zero exit code on failure.
- `--on-error <value>` — on detection failure, print `<value>` on stdout in place of the area name.

### Error behavior

When the user is not in a monorepo (either they are in a non-monorepo repo, or not in a repo at all), the command fails:

- Default (non-verbose) path: zero stdout output and a non-zero exit code.
- `--verbose` / `-v`: the corresponding error message is written to **stderr** (not stdout):
    - "'area' is a term that holds meaning only in a monorepo; you are in a repo but not a monorepo!"
    - "'area' is a term that holds meaning only in a monorepo; you are not in a repo!"
- `--no-error` suppresses the non-zero exit code but does not change what is written to stdout/stderr.
- `--on-error <value>` causes `<value>` to be printed on stdout in the failure case.

## Library API

Two items live in the same module of `sniff-lib`.

### `RepoInfo::area_for_dir`

A method on `RepoInfo`, parallel to the existing `package_for_dir` and `package_area_for_dir`:

```rust
impl RepoInfo {
    pub fn area_for_dir(&self, dir: &Path) -> &str;
}
```

This is the primitive. It applies the area determination rule (below) against an already-detected `RepoInfo`. It does not itself decide whether the surrounding repo is a monorepo — that distinction is the caller's concern.

### `detect_area` free function and `AreaError`

A free convenience function for CLI ergonomics, in the same module:

```rust
pub fn detect_area(dir: &Path) -> Result<String, AreaError>;
```

`AreaError` is a `thiserror` enum (per project convention), marked `#[non_exhaustive]` to allow future variants without breaking callers. It has three variants:

- `AreaError::NotInRepo`
- `AreaError::NotMonorepo`
- `AreaError::Io(std::io::Error)` — wraps I/O failures encountered during repo detection (via `#[from]`).

The CLI matches on `NotInRepo` / `NotMonorepo` to select the correct verbose error message; `Io` is reported with the underlying error.

## Area determination rule

The area is the **package name** if the CWD is inside a package; otherwise the area is the CWD's **package-area** string.

At the repo root — or anywhere else that is not inside a package and whose `package-area` is `"root"` — this rule naturally returns `"root"`, because `RepoInfo::package_area_for_dir` already returns the literal `"root"` for top-level locations.

In pseudocode:

```
area = package_for_dir(cwd)
        .map(|p| p.name)
        .unwrap_or_else(|| package_area_for_dir(cwd))
```

The two existing helpers `RepoInfo::package_for_dir()` and `RepoInfo::package_area_for_dir()` already do the right thing; `area_for_dir` is a thin composition over them.
