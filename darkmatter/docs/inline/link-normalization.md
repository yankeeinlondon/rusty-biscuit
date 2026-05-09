# Link Normalization

**Link Normalization** is a finalization operation that converts absolute paths back into portable forms. It runs only on the **root document** at the very end of the composition pipeline.

## Normalization Rules

Link Normalization applies the following rules in order of precedence:

### 1. Same-Repo Rule (Relative Paths)
If an absolute path points to a file within the same Git repository as the root document, it is converted to a relative path. This ensures that documentation remains portable across different checkouts of the same repository.

- **Example:** `/home/user/repo/docs/img.png` becomes `../docs/img.png` (relative to the output file).

### 2. Home-Dir Rule (`~/` Alias)
If the path is outside the repository but within the user's home directory, the home prefix is replaced with the `~/` alias.

- **Example:** `/Users/ken/notes/todo.md` becomes `~/notes/todo.md`.

### 3. ENV-Var Rule (`${VAR}` Substitution)
If the path matches a prefix defined in the whitelisted environment variables, it is replaced with the variable reference. The rule selects the variable with the longest matching path prefix.

- **Whitelisted Defaults:** `PROJECT_ROOT`, `DOCS_BASE`.
- **Custom Whitelist:** Can be extended via `ComposeOptions::with_env_path_whitelist`.
- **Example:** If `ASSETS_DIR=/shared/assets`, then `/shared/assets/icons/save.svg` becomes `${ASSETS_DIR}/icons/save.svg`.

> **Warning:** When an environment variable substitution occurs, a warning is emitted to STDERR to inform the user that the output now depends on an external abstraction.

## Phase

- **Phase:** `Finalization`
- **Root Only:** This operation *never* runs on transcluded child documents; it only runs once on the final, fully-composed root document.

## Configuration

The environment variable whitelist can be configured in `ComposeOptions`:

```rust
let options = ComposeOptions::new()
    .with_env_path_whitelist(vec!["MY_VAR".to_string()]);
```

## Source Files

- `darkmatter/lib/src/markdown/compose/link_normalization.rs`
