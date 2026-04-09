# Validation Types

## Validation Structure

The _pre_ and _post_ validation stage of a Claudine transaction consist of an enumerated set of operations which can be used to provide validation. Every validation operation takes a `name` property as it's first parameter but can also exposes some optional parameters to help in expressing how to handle failures.

It's important to understand that the following two definitions of the **file_exists** validation are identical in scope:

- **Shorthand**

    ```yaml
    pre_validations:
        - file_exists: "@path/to/file"
    ```

- **Full Syntax**

    ```yaml
    pre_validations:
        - file_exists:
            - name: "@path/to/file"
    ```

Assuming we're all lazy (_a reasonable assumption_), why not just use the shorthand form? Well you'll be happy to hear that we try to reward this form of laziness by providing good defaults for the shorthand so you can do just that. However, if you want more fine grained control over handling or messaging then you'll need to leverage the longer form.

### Shorthand

The shorthand will assume the following:

- when the condition is met, there is no "handling" of this outcome and a failure/error state should result
- **Claudine** will provide a well structured error message which not only describes that the document has _failed_ to be run but _why_ it failed.
- failures

### Full Syntax

- in it's basic form (_as demonstrated above_), there is no difference from the shorthand and all assumptions remain the same
- however, the full syntax provides the additional properties and capabilities:
    - `handle`
    - `outcome` - `fail` | `skip`

#### Handlers

For every validation which returns **false** we are now 


## Validations

### Filesystem And Data Shape Checks

These checks are about whether the expected input or output files exist and are structurally sane:

- `file_exists(name)`
- `dir_exists(name)`
- `json_file_exists(name)`
- `yaml_file_exists(name)`
- `toml_file_exists(name)`
- `has_write_permission(name)`

The typed file checks matter because "the file exists" is often too weak. A JSON file that exists but is malformed is not a usable prerequisite. Claudine treats those as distinct validation concerns so the error is closer to the real problem.


### Repository State Checks

These checks look at dirty source state:

- `no_dirty_source_code`
- `has_dirty_source_code`
- `no_merge_conflicts`

These are useful when a prompt is intended to work from a clean baseline, or when a prompt is only meaningful if the user has already made local edits that the agent is supposed to inspect.



## Logical Operands

In order to give you more flexibility in how and what you check, we provide the following logical operands:

- Combinatorial
    - `every`
    - `any`
- Atomic
    - `not`


