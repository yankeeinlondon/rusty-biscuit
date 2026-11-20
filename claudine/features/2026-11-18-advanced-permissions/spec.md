## Default Stance

When a filesystem, network, environment, fn-execution or shell-execution configuration is empty _then_ **nothing** is allowed.

This default stance, however, is modified by configuration and the _default configuration_ (e.g., what is set as the repo's configuration when configuration is missing) opens up a set of permissions that are meant to provide useful ergonomic access as a starting point.

## Configuration Variance

When working in a repo, the repo based configuration -- _which always is the authority over the user-scoped configuration_ -- is treated differently under the following conditions:

- **Local Repo Config different from Remote**
    - If the repo defines a git "remote" _and_ the Claudine/Darkmatter configuration file is **dirty**
    - Then the last known version of remote's configuration is considered the valid configuration
    - If a deterministic operation is _not_ allowed with the remote's configuration but _is_ allowed with the dirty configuration then:
        - a user can use the `--force` flag to assume the permissions dictated by the repo's dirty configuration
        - if a user calls Claudine/Darkmatter from an interactive terminal but does not use the `--force` flag then the error should clearly indicate that this is an option in this circumstance
- **Local Repo Config same as Remote**
    - in cases where the repo's configuration file is the same as the remote's then what is allowed and not allowed has no variance
    - as a result the use of `--force` in these circumstances will have no effect (other than a warning message sent to the terminal to indicate that `--force` was ignored because there was no variance)
- **No Repo Config** (local)
    - running either Claudine or Darkmatter will immediately add a configuration file locally b
        - with the _default_ permissions if the remote has no configuration
        - with the _remote's_ permissions if the remote does have configuration
    - at this point the remote permissions are the accepted permissions but if there is variance with the local config then the `--force` flag will use the local definition
- **Repo Config in `.gitignore`**
    - if the repo's configuration is in `.gitignore` (and remote does not have it either) then whatever configuration is present will be treated as the authority

## Permission Config File

All permissions settings are under the `permissions` root property. The child properties for permissions are: `filesystem`, `network`, `environment`, `fn-execution` (future) , and `shell-execution`. The values associated to these child properties are an array of permission definitions where the permission definitions take the form of:

- `{allow}`
- `!{deny}`
- `[{attributes}]: {allow}`
- `[{attributes}]: !{deny}`

The _attributes_ provide a way to "scope" the permission grant. If no attributes are provided then "the scope" includes everything.
The attributes available are dependent on the child property:

- **filesystem**:
    - `read` - scoped to file reads
    - `write` - scoped to file writes
    - `metadata` - scoped to metadata about a file (see [file metadata](#metadata))
    - `deterministic` / `non-deterministic`
- **network**:
    - `http`
    - `https`
    - `deterministic` / `non-deterministic`
- **environment:**
    - `metadata`
    - `deterministic` / `non-deterministic`
- **shell-execution:**
    - `deterministic` / `non-deterministic`
- **fn-execution:**
    - `deterministic` / `non-deterministic`

### Examples

- **filesystem** (RWM,D/ND)
    - `[read, write, metadata]: **/*`
    - `[read, write]: !**/.env` - exclusion (exclude .env in root as well as sub directories)
    - `[read]: ~/.claudine`
    - `[write, deterministic]: ~/.claudine`

- **network**
    - `[get, post]: {git-remote}`
        - a dynamic variable which matches the URL of the git remote (always uses the git remote defined in 'main' and ignores any local changes)
        - by expressing the `get` and `post` attributes we are
    - `[deterministic]: {git-remote}`
        - this allows Claudine's git operations to be run but NOT the agent only via deterministic expressions in Claudine
    - `google.com` -- allows with HTTP and HTTPS
    - `https://google.com` - allows but only with HTTPS

- **environment**
    -
- **shell-execution**
    - Allows for shell commands to be enabled with either "exact" parameters or glob based patterns
    - `rm -rf ^tmp` -- allow the `tmp` directory off of the repo's root to be removed; but no other use of `rm` is allowed
    - `!rm -rf` -- never allow `rm -rf` to be used
        - Note: if both of the two permissions were present the `rm -rf ^tmp` rule would still persist but ALL other attempts of `rm -rf` would be denied

- **fn-execution**
    - FUTURE
    - Not currently being used but will soon be a compliment to the **shell-execution** functionality
    - instead of being able to run shell commands, this will allow blessed files to expose

## Determinism

- deterministic / non-deterministic - by default a permission is for both deterministic and non-deterministic
    - deterministic file operations: `edit`, `set_frontmatter`, etc.
    - deterministic network operations: `::url <URL>`, etc.
    - non-deterministic file operations: _the agent making reading or writing files_
    - non-deterministic network operations: _the agent making network calls_

## Metadata

- **filesystem:**
    - file_exists(file) - allows for checking if a file exists (even if a user doesn't have read or write permissions)
    - is_empty(file)
    - has_fm_prop(file) -
- **env:**
    - env_var_exists(env) - boolean response indicating whether the caller's environment has the given ENV variable
