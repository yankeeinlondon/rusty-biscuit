- Sound Dialog Box
    - the `P: Play preview` in the bottom row does not stand out nearly enough:
        - include a blank row between the last visible sound effect and the Hot Key row
        - the Hot Key row needs to have a different background color just like we do on the main tab's based Hot Key row
    - we are missing important Hot Keys:
        - `<b><yellow>P</yellow></b>: Play Sound, <b><yellow>ENTER</yellow></b>: Select, <b><yellow>ESC</yellow></b>: Exit, <b><yellow>D</yellow></b> <i>default for</i> Success`
        - The last option `D` needs to be dynamic to whether the user is choosing a Success, Error, or Attention based sound effect.
    - BUG: When a user brings up this dialog box the default choice needs to be the _currently selected choice_!
    - BUG: There are a lot of sound effects, and in most cases it will be more than we have to render vertically without a scrolling window. 
        - Currently when a user goes below the last visible option the "selected row" just disappears the the user is "driving blind"!
        - In addition to having a scrolling window we should show a vertical scrolling UI to the right of the choices as is customary for scrolling windows like this


- Exit message:
    - If there were changes:
        - `\n<b>Claudine</b> configuration was updated:\n`
        - User config should say: `- The <b>User</b> configuration was saved to <blue>~/.claudine/config.json</blue>`
        - Repo config should say: `- The <yellow>{repo}</yellow>(<dim>{branch}</dim>) <i>repo configuration</i> was saved to <blue>./.claudine/config.json</blue>`
        - `\n`
    - If no changes:
        - `\nNo changes were made to the <b>Claudine</b> configuration.\n`
        - `If you want to view the configuration, the are located at:`
        - `    - <b>User</b> configuration is found in <blue>~/.claudine/config.json</blue>\n`
        - if in a repo:
            - `    - <b>Repo</b> config is found at <blue>./.claudine/config.json</blue> off the repo's root directory`
        - if not in a repo:
            - `<dim><i>    - <b>Repo</b> config is found at <blue>./.claudine/config.json</blue> off the repo's root directory</i></dim>`
            - `<dim><i>    - because you are not in a repo currently no repo based configuration options were presented</i></dim>\n`

- BE CONSISTENT!
    - We have the `Logging` toggle looking one way on the `Services` configuration but then have it look a completely different way on the next line when we show Protect!
    - Also the `Logging:` option has a colon after it but `Protect` does not?
    - This kind of attention to details and ensuring that similar UI elements are treated similarly is CRITICAL
    - You need to fix the example given here but then look across the entire UI and look for other inconsistencies that need fixing!

- In the **Actions** configuration:
    - Hot Keys
        - You have a set of vertical choices which the user navigates by using the arrow keys
        - In all situations like this the ENTER key should "select" that item, in this menu that is the "Edit" menu
        - In the Hot Keys you mention that `E` is to edit but it just be `ENTER`
        - Note: secondary actions like delete and add can have a letter hot key (as they currently do)
    - Delete Confirmation Dialog
        - Ugly!
        - You did NOT make the hot keys yellow ... be consistent!
        - You did NOT make the letter hot keys capitalized ... be consistent!
        - The Choices available (Y, ESC) should be centered ... be consistent!
            - Note: I would have expected `Y` and `N`; I would ESC to act like `N` but it doesn't need to be mentioned
        - You have two blank lines after the choices but no blank lines before the first message? This makes the dialog feel unbalanced vertically!
    - When we enter the Edit Dialog I'm able to add a Message but it doesn't ask for the text message!!!!
    - You also DID NOT follow my instructions about how to present the actions in the main Actions window! You must follow instructions!!!!
        - The action name is NOT italicized
        - The configuration of the action is italicized and dimmed
        - You clearly need a `:` character separating the EVENT and the ACTIONS
        - Also you will clearly need a "truncation strategy" for text configurations like Message and Speak or we'll start having unwanted line wrapping ... this need careful consideration!

- TTS

    - You are now showing the TTS Provider but the voices are wrong!
        - A user has no idea what the "kokoro default" is!!!!!
        - You must list the actual voice name which is the default!
    - Stylistically I would NOT indent the Male and Female voices under Provider
        - Instead just put a blank row between provider and Male/Female voices
    - The dimmed text is not working as a good visual aid for whether the male vs female voice is the default and you've provided no "key" to help the user
        - Add a `<b><yellow>*</yellow></b>` marker next to the default (aka, male or female)
        - Then below the voices you'll add a blank line and then:
            - `<dim><i><b><yellow>*</yellow></b> indicates the default gender to be used`
    - Pressing `P` does bring up the provider dialog and `T` does toggle the on/off setting
    - but `F` and `M` do NOTHING!

## Task

Use the 'tui' and 'cli' skills and create a high confidence plan to fix these issues. Save the plan to `claudine/features/2026-04-7-refactor-config/fix-plan-1.md`

Make sure the plan provides time to look for other inconsistencies in the UI beyond what I've already explicitly demonstrated.
