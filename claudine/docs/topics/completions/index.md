# Auto Completion Functionality

In an ongoing effort to support **effective laziness** (altruism at it's finest), Claudine supports two forms of _completions_ that will help you make a lack of a precision on your part an "optimization" instead of a mistake:

1. **Shell Completions**

    Most of you will be familiar **\*nix**'s shell completions feature and any CLI worth it's salt will provide _some_ sort of shell completions but there is a range of what and won't be detected as user intent and converted into a valid executable CLI expression. 

    With Claudine we will always complete the CLI switches which the CLI exposes (e.g., `--verbose`, `--plain`, etc.). The "scope" of what CLI switches are available will vary based on which "sub command" you are attempting and Claudine will only present those which options are are valid for you're current expression. As an example:

    - if you were typing `claudine context ` and press TAB for your completions you will see a number of CLI switches of which `--expressions` is amongst them 
    - the `--expressions` CLI switch is directly associated to the **context** sub command and is presented because you're in that "scope"
    - if you were typing `claudine providers ` and press TAB you will be presented with a different set of CLI flags which are appropriate for the "providers" subcommand but notably `--expressions` is NOT included

    > **Note:** if you're shell is _bash_, _zsh_, _fish_, _powershell_, _elvish_ then your shell is on the supported list but if not then give it a whirl anyway and fingers crossed it will work there too

    Want more details? Check out: [shell completions in Claudine](./shell-completions.md)

2. **Auto Complete TUI**

    If you press ENTER instead of TAB _prematurely_ you'll get what we call the **auto complete** feature in Claudine. This functionality is associated to the **[compose](../composition.md)** functionality in the CLI as it has the greatest completion demands and we get the benefit of "schemas" to understand intent.

    What does _prematurely_ mean? Well if any of the conditions are met you'll find out:

    - you referenced a file with an incomplete file path 
    - you didn't assign a value to a Frontmatter property which was "required" by the prompt file

    In these cases, Claudine will bring up a TUI to help you complete an executable contract and then run this contract through the CLI.

    - want more details? Check out: [auto complete in Claudine](./auto-complete.md). 
    - want to know how a prompt file can declare it's Frontmatter schema? Check out: [frontmatter schema definitions]()

With these two features we expect your Claudine CLI experience to be more fulfilling and less encumbering. Enjoy and you're welcome.

> **Note:** some of the more anal retentive of you may have seen and _objected_ to this document being called **auto completions** while structurally/semantically linking to two distinct concepts -- _shell completions_ and _CLI auto-completions_ -- where one is called "auto-completions" too? Well Claudine noticed that too and doesn't care. She said that you should seek counseling for you're overzealous need to emphasize structure over flow.
