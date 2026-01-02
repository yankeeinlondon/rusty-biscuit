# Mat CLI Refactor

We recently built the `mat` CLI to leverage the newly created `Markdown` struct in the shared library. Unfortunately the CLI's setup is not correct so this feature will act as a major refactor.

While the majority of changes involved in implementing this feature will involve refactoring the CLI command. Some of these features may result in some refactoring of the `shared` library functionality.

## CLI Syntax

This CLI does **NOT** use subcommands. The correct syntax is:

> **mat \<filename\>** [_options_]

- It's primary use-case is to be used with no CLI flags/switches and in this mode it will simply render the markdown file to the terminal:
    - The prose should **not** use the theme's background color
    - The prose, however, should be formatted by the theme

> **NOTE:** the CLI should also take it's content input from STDIN if no filename was provided; allowing for chained expressions like `cat "foo.md" | mat`

### CLI Switches

#### Theming

- `--theme <theme pair name>`
    - the user can explicitly state the theme pair they would like to use with this flag
    - this will then be fed into the light/dark detection

    > **NOTE:** 
    > 
    > We should implement the **From** trait for `ThemePair` so that either a kebab-case (the external representation of the theme name) or maybe even PascalCase `String`/`&str` representations of a ThemePair can be mapped into the appropriate variant. 
    > 
    > - If the `String` or `&str` passed in doesn't map to `ThemePair` variant then we should just default to the `OneHalf` variant

- `--code-theme <theme pair name>`
    - typically the theme used for code blocks is _derived_ from the Markdown theme but it can be set explicitly

- `--list-themes`
    - this will list the themes which a user can choose from
    - **Note:** these are theme-pairs, not a discrete theme (aka, choosing a theme chooses both a light and dark theme simultaneously)
    - valid themes include `github`, `base-16-ocean`, `gruvbox`, `one-half`, `solarized`, `nord`, `dracula`, `monokai`, `visual-studio-dark`
    - when themes are listed the output format should come from `ThemePair::description(theme)` however the output format looks incorrect. 

    > **NOTE:**
    >
    > Looking at the `description` function for `ThemePair` it appears to not 
    > be implement well. The signature shows a misunderstanding on how it 
    > should be used.
    > 
    > The actual output of this function is described clearly on lines 
    > 152-162 of the @shared/docs/md/themes.md file!

#### Mutation Switches

- `--clean`
    - This indicates the document should be _cleaned up_ 
    - The document, once cleaned up, is then rendered to STDOUT
    - The cleaned document in the filesystem is not updated

- `--clean-save`
    - This performs the same function as the `--clean` switch except that it also **saves** the cleaned document back to the file.

- `--fm-merge-with <JSON Object>`
    - this flag merges the document's _frontmatter_ with the JSON object provided; giving the JSON object precedence to update existing properties where there is overlap
    - the resultant frontmatter is reported to STDOUT in YAML format
    - the file is updated and saved to filesystem

- `--fm-defaults <JSON Object>`
    - this flag merges the JSON object with the document's _frontmatter_; giving the document's frontmatter precedence.
    - this means the JSON object plays the role of setting _default values_ which may have been unset before but it can NOT overwrite any existing frontmatter key values that existed in the document.
    - the resultant frontmatter is reported to STDOUT in YAML format
        - new lines should be shown in green
        - existing lines in white
    - the file is updated and saved to filesystem

#### Output Switches

- `--html`
    - when this flag is used it indicates that the output should be HTML instead of terminal based output. The HTML is sent to STDOUT.
- `--show-html`
    - when this flag is used it indicates that HTML output should be generated and saved to a `tmp` filepath and then this file should be _opened_ by the host's default application for HTML files (presumably a browser).
- `--ast`
    - when this flag is used it indicates that an MDAST representation should be output instead of terminal output

## Theme Strategy

Unfortunately when the `Markdown` struct was created, the @shared/docs/md/theme-strategy.md document had not been completed. That document HAS now been completed and should be reviewed to understand how we deal with:

- light/dark themes in terminal versus HTML/browser
- how the `theme` (a "theme pair") is selected/detected
- how the theme for the Markdown influences the theme used for code blocks within the theme.

Make sure to read: [Theme Strategy](@shared/docs/md/theme-strategy.md).


## Concerns

I think there are some problems in the underlying library code in the shared library. Things which raise concern for me include:

1. The code implementation of `ThemePair` seems quite different from the design document's description:

    ```rust
    pub enum ThemePair {
       // natural light/dark pairing
       Base16Ocean(Theme::Light_Base16Ocean, Theme::Dark_Base16Ocean),
       GruvBox(Theme::Light_GruvboxLight, Theme::Dark_GruvboxDark),
       OneHalf(Theme::Light_OneHalfLight, Theme::Dark_OneHalfDark),
       // Base16Mocha is only a dark theme, map -- by default -- all missing 
       // light pairings to Theme::OneHalfLight
       Base16Mocha(Theme::Light_OneHalfLight, Theme::Dark_Base16Mocha),
       // the only two "light only" themes are `Github` and `GithubInspired`
       // in both cases we'll map to Theme::Dark_VisualStudioDarkPlus as 
       // apparently the map to each other well
       Github(Theme::Light_Github, Theme::Dark_VisualStudioDarkPlus),
       GithubInspired(Theme::Light_GithubInspired, Theme::Dark_VisualStudioDarkPlus)
       // ...
   }
   ```

   > this is defined in [Themes](@shared/docs/md/theme.md)

   I can't explain the difference and it feels like it's a move in the wrong direction. 

2. Because the [Theme Strategy](@shared/docs/md/theme-strategy.md) document was not written up before the `Markdown` struct was implemented I feel this has led to a number of incorrect assumptions being made and these assumptions have crept into many parts of the implementation.

3. The `mat` CLI was implemented in a way that was entirely unintended but that is in part because there wasn't enough documentation on what was expected. This feature is meant to address this lack of documentation.

    > **Note:** in my very limited testing of the incorrect `mat` CLI, i saw absolutely NO highlighting!

