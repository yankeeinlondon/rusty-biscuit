# Better Errors

> **IMPORTANT:** use the 'darkmatter', 'cli', and 'biscuit-terminal' skills

It's extremely important that we provide high quality error messages to users of the Darkmatter CLI as well as try to encourage library users to do the same by providing useful and ergonomic ways of reporting Darkmatter's errors.

## Packages Areas

Some of the work we'll do in this feature will be done on the `biscuit-terminal` package area to allow for greater reuse but a majority of the implementation will be in the `darkmatter` package area.

## Resources

- this feature will make extensive use of the `Status` and `StatusBlock` struct provided by `biscuit-terminal` to provide a nicely stylized error message.
- a full list of errors which Darkmatter emits is included in [Darkmatter Errors](./errors.md)

## Block Style Error

- Title/Heading Line:
    - Uses the `Status` struct to print the error's name (followed by a colon) in BOLD RED and then the rest of the title line in BOLD (default color)
- Block
    - Uses `StatusBlock` to paint a red vertical line to the left which "appears" to come out of the "error" icon of the title line
    - The prose section of the error block can contain descriptive text, a code block example, or hints on how to avoid the error

It is important to understand that the text in the "title" row is NOT to be repeated in the block section. The title section is more "summary level" whereas the block area provides details and context.

## BlockError Trait

- this feature will implement a new trait called **BlockError** which will be used with errors which can be implemented in the "Block Style" described below.
- this trait should be implemented in the `biscuit-terminal` library to promote better reuse
- at a minimum we expect this trait to ensure that a `report_block_error(term: &Terminal)` function is able to receive a `Terminal` struct and then provide back the string rendering that should be applied to that terminal.
