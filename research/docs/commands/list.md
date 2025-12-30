# `list` command

Lists out all **research** topics.

- refer to the [research filesystem](./research-filesystem.md) to understand where to find and how to navigate the existing research items.

## Syntax

> **research list** \<filter\> [FLAGS]

### Parameters

- with no parameters this command lists ALL known research topics
- one or more parameters can be included to add glob pattern filters
    - using `research list foo` will only list research topics which include "foo" in the name
    - using `research list foo bar` will only list research topics which include "foo" or "bar" in the name of the topic
    - using `research list 'foo*'` will only list research topics which _start_ with "foo" in their name

### Switches

- `--type`, `-t` this switch provides a filter on a particular type of research topic:
    - `research list -t library` will list all topics which are of the type "library"
    - more than one type can be achieved by using this switch more than once. For instance, `research list -t library -t software` will list all topics which are either of the type "library" or "software".
- `--verbose` will output detailed metadata with sub-bullets showing issues and additional prompts (note: `-v` is reserved for global logging verbosity)

#### Output

The default output for this command is terminal friendly but you have the optional use of the `--json` switch to get structured data on STDOUT.

The command iterates over each topic that meets the filter criteria and produces a `TopicInfo` result.

```rust
enum ResearchOutput {
    deep_dive,
    skill,
    brief
}

struct TopicInfo {
    /// the name of the topic
    name: string;
    /// the type of the topic
    type: string;
    additional_files: string[];
    missing_underlying: string[];
    missing_output: ResearchOutput[];
    missing_metadata: boolean;
    /// the filepath to this topic's directory
    location: string;
}
```

- When the `--json` flag is used then the array of `TopicInfo` objects is returned.
- By default, however, we produce a "terminal friendly" output which prints a list of topics, with each line consisting of:
    - each line starts with the `-` character followed by a space (no formatting)
    - in BOLD text we then print the topic's name (followed by a space)
        - in BOLD and RED when there is a missing output file (deep_dive.md, brief.md, skill/SKILL.md) or the `metadata.json` is missing
        - is BOLD and ORANGE when there is a missing _underlying_ document (but all output files are present as is `metadata.json`)
    - then -- unless we're filtering on only one "type" -- we show the type:
        - The type will have a background color and be padded with a space and after it's name (which will receive the background color too)
    - now add some contextual icons if they meet the criteria:
        - 🦀 if the `metadata.json`'s has a `library_info.language` property and it's equal to "Rust"
        - 🐍 if the `metadata.json`'s has a `library_info.language` property and it's equal to "Python"
        - 🐘 if the `metadata.json`'s has a `library_info.language` property and it's equal to "PHP"
        - `ʦ` (BLUE background, BLACK text) if the `metadata.json`'s has a `library_info.language` property and it's equal to "Javascript" or "Typescript"
    - Add `: {{ITALIC}}{{Description}}{{RESET}}` where the "description" is the one sentence description in the `brief` property of the `metadata.json` file.
    - In verbose mode: 
        - Now we will add sub-bullets for the following optional/conditional items (indented 4 spaces from primary list):
            - if there are missing properties in the `metadata.json` then we will report: `- 🐞 {{BOLD}}metadata.json{{RESET}} missing required props: PROP, PROP, ...`
            - if there are missing _underlying_ documents then report `- 🐞 missing {{ITALIC}}underlying{{RESEARCH}} research docs: FILE, FILE, ...`
            - if there are missing final outputs then report: `- 🐞 missing {{ITALIC}} final{{RESET}} output deliverables: (Deep Dive Document | Skill | Brief)[]`
            - if there are additional custom prompts beyond the core research prompts then report: `- 💡 {{#}} additional prompts used in research: FileNoExt, FileNoExt, ...`        
    - In non-verbose mode:
        - Add a `💡` icon after a research topic which has additional prompts
        - Add a `🐞` icon to a topic which is missing data
        - at the very end of output add a blank line and then `- use {{BG_GREY}} --verbose {{RESET}} for greater metadata on the topics`
