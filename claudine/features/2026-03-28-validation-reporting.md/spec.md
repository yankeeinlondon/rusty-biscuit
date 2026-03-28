# Validation and Handler Reporting

When we run Claudine in a non-interactive compose/inline-compose sessions, we are allowed to add both **validations** and **handlers**. In this feature we will introduce some key enhancements to reporting on this as well as a few improvements to ensure a non-interactive session is able to "fail fast" if we detect that it WILL fail.


## Reporting Validations

**IMPORTANT:** all validation states will be reported on using `biscuit-terminal`'s `Status` component and it's state model with the **circular** theme. The message component of `Status` will use the `Prose` crate.

When a non-interactive session starts up we will:

1. validate that the referenced file exists
    - success: `the file reference <blue-500>{ref}</blue-500> to the <blue-500><a href={absolute_path}>{filepath}</a></blue-500> file on this host`
    - failure: `the file reference <blue-500>{ref}</blue-500> found no match on host computer!`
2. look for all validation properties (_in `pre_checks` and `post_checks`_)

