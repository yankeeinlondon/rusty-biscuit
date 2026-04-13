## A Better Interview for Installing Software

The current sniff CLI for installing applications is:

- not nicely formatted 
- not verbose enough
- and doesn't provide fallback's or URL info. 
 
When a user chooses to install an application we should:

1. Using `Prose` display something like `The <b><blue><a href={software_url}>{software}</a></blue></b> will be installed through the **npm** package manager using the command: <dim><green>{cmd}</green></dim>`
      - you will need slightly variant text for methods which are not package managers but use above as the template
2. The command is executed and STDOUT/STDERR captured

    - when successful:
        - Report STDOUT with `BlockQuote` (and a grey vertical bar) (adding one blank line afterward)
        - then with `Status` in success state using circular style: `<b><blue><a href={software_url}>{software}</a></blue></b> has been installed`successfully`
    - when error:
        - Report STDERR with `BlockQuote` (using red vertical bar), one blank line afterward
        - then with `Status` in error state using circular style: `failed to install <b><blue><a href={software_url}>{software}</a></blue></b>.`
        - then offer a choice if there are other install methods known:
            - Prose::new("Try installing using **{alternative}** instead")
                - Retry with alternative installer
            - Prose::new("Quit (_and try manually if desired_)")
                - exit

## Key Design Decisions

This interview process _should_ be done in the Sniff Library as much as possible so that not only the Sniff CLI benefits but also any library callers can hook into the same interview process.

> Note: because `biscuit-terminal` is a consumer of the Sniff library, the library should "prepare" the "Prose" strings for the caller but will not use the components directly and thereby creating a circular dependency.
> 
> Note: the current implementation is NOT using biscuit-terminal's `Renderable` property correctly; most likely passing a static Terminal instead of the ACTUAL Terminal. This means no colors, poor word wrap, etc.
