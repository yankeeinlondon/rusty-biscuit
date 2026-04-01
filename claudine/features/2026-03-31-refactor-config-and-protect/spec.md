# Refactoring Claudine Config and Protect

This feature combines two significant things:

1. Refactoring Configuration (e.g., `~/.claudine/config.json`)

    The configuration we use today is a complete mess and causes cognitive overload when in fact what we need is quite simple.

2. Refactoring Protect Feature

    The Protect functionality was recently refactored to take advantage of the underlying `PolicyEngine`. Having the `PolicyEngine` should serve as a much better foundation but currently Protect is way too protective and we need to align it's protection features to the functionality it is meant to serve.

## Refactoring Configuration

My current configuration on this host is 557 lines! By the time this feature is complete it should be less than 20 and probably less than 10.

### Key Insights

- A large part of our configuration is a combination of hook configs _per provider_ and log configuration
- When Claudine is first initialized we immediately add a hook into EVERY event for EVERY provider that the host has installed
    - this allows us to gain insights into as much information as possible, 
    - and keeps the Agent's configuration simple (one entry for Claudine, nothing else)
- Any event the user is not interested in is NOT a configuration entry
    - only when they want something outside the norm to happen is there any need for configuration
    - when a user expresses interest in an event it is always a **Claudine Event** and cross-provider, not per-provider!
- Claudine offers two services:
    - Logging
    - Protect
- These services will interact with events but that is abstracted away from a Claudine user
- For Logging, a user must only express:
    - Do I want to use it (yes/no)?
- For Protect:
    - we will offer a set of protections which can trigger one of three actions: `accept`, `reject`, and `ask`
    - we will define a low-impact default configuration 
    - when a user runs `init` they will automatically be signed up for this default set of protections
    - in configuration, the basic setting is a boolean value: `{ protect: true }`
    - if a user wants to use protect but modify the settings then instead of `true` we will have 

### Config Schema

```ts
type TtsConfig = {
    /** 
     * This must be specified but we can provide a good default
     * as `biscuit-speaks` library should be able prioritize the 
     * available options on the host by quality and choose the
     * best option.
     */
    provider: TtsProvider;
    /** 
     * set the default voice (optionally setting it by gender) 
     */
    voice?: Voice<TtsProvider> | {male: Voice<TtsProvider>, female: Voice<TtsProvider>};
    /**
     * Optionally allow a user to specify a default gender
     * 
     * @default female
     */
    gender?: "male" | "female"
}

type ClaudineConfig = {
    /** How to handle TTS functionality on the host  */
    tts: boolean | TtsConfig;
    /** What messaging platform to send to */
    messenger?: MessengerConfig;
    /** whether or not to use the logging service */
    logging: boolean;
    /** 
     * whether or not to use the protect service,
     * while also allowing user to override default
     * protect settings
     */
    protect: boolean | ProtectConfig;
    /**
     * Any actions which the user wants to take action
     * on. These are **Claudine** events not provider
     * events.
     */
    actions: Record<ClaudineEvent, ClaudineAction>;

    /** the preferred agent to use when a lazy  */
    preferred_agent: Provider;
}
```

### Lifecycle Processes

When the user runs `claudine` or any claudine subcommand (e.g., anything other than `claudine --help`) and there is no configuration file located at `~/.claudine/config.json` we will run the **Initialization Process** (see next section).

After that point a user wanting to _change_ their configuration has two options:

1. modify the config file themselves
2. run the `config` subcommand (new)

#### Initialization Process

- This process is ONLY executed when there is no config file found in `~/.claudine/config.json`
- When engaged we will quickly cover the following topics:
    - TTS settings
        - we should explain how TTS is used in Claudine
            - to help people know when a Agent needs a user's attention
            - to indicate that an error has occurred
            - etc.
        - if the host has a decent TTS provider:
            - we'll simply tell them what settings we've set and that they can configure that later if the want to
        - if the host has NO TTS providers or only very low quality ones:
            - then we will offer to install a better one for them
            - if they choose not to install a TTS provider then we set the `tts` setting to `false` to start
    - Messenger
        - FUTURE: Claudine support this yet but will
        - For now we can instruct the user on the purpose of Messenger and tell them that by default this will be turned off
        - Purpose of Messenger:
            - leverages the `messenger` library in this monorepo to send messages to chat applications such as Discord, Slack, WhatsApp, etc.
            - by configuring this a user can be notified of events when they are away from their computer
            - eventually we may even allow them to _respond_ to events remotely
    - Preferred Agent
        - When lazy agent operations such as `compose` or `inline-compose` are used _without_ a flag specifying which agent to use, a list of all agent providers will be presented but we want the user's "favorite" agent to be the default choice.
    - Logging and Protect services
        - During the interview we just explain what these two services are and that by default they are turned on but if the user for any reasons wants them turned off or configured different that can be done later.
        - To make sure they've had a chance to read this we will wait at a confirmation prompt 
    - Actions
        - We will describe how Claudine provides a _canonical_ set of events who's scope is cross-provider instead of the user having to deal with the specifics of every platform
            - describe the limitations that not every provider has great support for events
            - and then mention that many of these canonical events maps directly to fairly established events like `PreToolCall`, however
            - we also provide events that address common user needs like `human-in-the-loop`
        - We will then tell the user that by default we play a sound effect when a `human-in-the-loop` event is encountered but they are free to configure that as they like with `claudine config`
        - Present with a confirmation prompt `Press enter to complete the Claudine initialization process`
    - When the process concludes we will report:
        - `- the configuration file for <b>Claudine</b> can be found at <a href="~/.claudine/config.json"><blue>~/.claudine/config.json</blue></a>`
        - `- you can edit this file directly with an editor but we recommend using <green>claudine edit</green> instead`


#### Configuration with the `config` Subcommand

- Today we have an `init` command but no `config` command.
- As a part of this feature we will REMOVE the `init` command and _create_ the `config` command.
- The config command is responsible for changing the Claudine configuration in a user friendly way
- It is NOT, however, responsible for the initial configuration (which uses the aforementioned Initialization Process)
    - that means that if `claudine config` is run before there's a config file defined, we will instead pass execution over to the Initialization Process

The `config` command will bring up a TUI implemented using the [ratatui](https://github.com/ratatui/ratatui) crate in Rust. It will present a tab based UI where that tabs are:

- `TTS`
- `Messenger`
- `Services`
- `Actions`
- and `Preferences`

To use the tabs in the TUI, the user will:

- move forward through the tab's with the "tab" key
- move backward through the tabs with "shift+tab"
- `Q`, `q`, `ESC` to exit
- and `ENTER` to **select** a tabbed area.

Pressing ENTER in any of the tabs will bring up a modal which provides details which largely consist of either:

- an action the user can take which will render a new modal view (this will be used where the UI space required to perform this action needs more space)
- or a form/switch/etc to directly change the settings (more direct and the preferred method but not practical in some cases)

**Note:** take the time to make the design of the TUI beautiful and ergonomic to work with!

##### Actions by Tab

> **NOTE:** when a user has selected a particular tab by pressing ENTER they are now in the **detail mode** and pressing ESC will no longer exit the app but instead return the app to the default tab state where tab/shift+tab navigate between tabs again. 
>
> **NOTE:** when the user hits the the ESC key (anywhere) it will act as "going back" one level


1. TTS

    - Default view shows the on/off toggle, all other controls are greyed out when the toggle is in the "off" state.
    - Under the on/off toggle you have a horizontal info panel which shows:
        - `<b><inverse>{provider}</inverse></b> → {female_voice} / <dim>{male voice}</dim>`
    - the **inverted** provider (above) indicates that it's "selected", pressing tab and shift+tab now moves between the items in this row
    - Below the selection row, we provide keyboard hints to what the actions are:
        - Provider:
            - Press ENTER to be shown the list of available providers as a vertical list, pressing ENTER again selects the highlighted provider
                - Poor quality options will show in the list but should be shown in an accent color that indicates their lower desirability
            - Press `I` to install a new provider
                - a vertical list of providers we support minus those already installed or inappropriate for the host's OS are shown in a new modal
        - Female / Male Voice
            - Press Enter to be presented with a new modal with the vertical list of voices for that provider and gender
            - Press `G` to toggle the gender which is the default 
                - In the display string above we indicated `{male voice}` to be **dim**; the gender which is NOT dim would be the _default_ gender

2. Messenger

    For now at least we'll make the design decision that a user can configure **one** app to send messages to, not multiple. They can configure as many apps as they like, in fact they can even configure an app multiple times (with different settings of course). But only one is active at a time.

    - The UI should look like a _closed_ Select Box with the selected Messenger app displayed (it will display "None" if the user has chosen to disable this feature or hasn't yet configured any apps)
    - To the right of the Select Box is a "Add" button
    - using Tab / Shift-Tab moves the selection between the Select Box and the Add button
    - pressing ENTER when the Select Box is _selected_ opens the Select Box to show all configured apps
        - the user then can move up and down with the arrow keys and when they press enter it indicates the highlighted app (or None) is the new setting
    - pressing ENTER when the "Add" button is selected will bring up a Modal for user to choose the App and it's settings
    - pressing `S` will open the Select Box and allow user to use up and down arrows to select the app configuration they want
    - pressing `A` will open the add App modal

3. Services

    - **Logging** is presented as an on/off toggle switch
    - **Protect** is presented as an on/off toggle switch but with the text `<i>default config</i>` or `custom config (<dim><i>{#} enabled</i></dim>)` to the right
    - Pressing `L` will toggle Logging
    - Pressing `P` will toggle Protect
    - Pressing `C` will open another modal showing a vertical list of the features of Protect
        - each feature has a checkbox next to it to represent whether it is on or off
        - user navigates using up and down arrows, highlighting one of the features each time
        - pressing space bar toggle the state of the selected feature
        - pressing ENTER or ESC exits the feature list modal

4. Actions

    - this tab will show the events which have something 

5. Preferences

