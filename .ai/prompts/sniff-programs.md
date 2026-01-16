# Sniff Library / CLI

We are going to add a top level feature to the Sniff library:

- programs

We will primarily be looking for ways to quickly determine which programs are available on the host system by leveraging the `which` crate. There is a skill `which` which should be used when designing and developing these features.

- we have added a `find_program` and a `find_programs_parallel` utility commands in `find_program.rs`
- We have added a number of structs which will test for a set of related programs:
    - `InstalledEditors`
    - `InstalledUtilities`
    - `InstalledLanguagePackageManagers`
    - `InstalledOsPackageManagers`

    Each of these structs provides a `new()` method which will concurrently check the status for all the packages it is responsible for.

    We need to ADD to this:

    - `refresh()` function to refresh the state of the struct
    - `path(Program enum)` returns the path to the binary on the host for given program
    - `version(Program enum)` returns the version which is installed on the host system
    - `website(Program enum)`
    - `description(Program enum)`


    This means that for every struct which has been defined, we will need to:
        - create an `enum` for each program in the struct
        - create a lookup table that allows us to provide the `website()`, `description()`, and metadata to determine the version of the program.

## Sniff CLI

- we will add the `programs` key to the top level object of the unfiltered `--json` output
- the `programs` key will have a key for each struct we've defined (e.g., tts_programs, language_package_managers, etc.)
- we will need to add a top level filter `--programs`, and sub-level filters:
    - `--tts-programs`, `--package-managers`, `--editors`, `--utilities`
