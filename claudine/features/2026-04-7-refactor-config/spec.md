# Refactoring Claudine Config and Actions

This feature simplifies Claudine's configuration model and updates the action system. It does **not** include the Protect refactor, which was completed separately (see [Protect Service](../../docs/topics/protect-service.md)).

The configuration we use today is a complete mess and causes cognitive overload when in fact what we need is quite simple.

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
    - Protect uses a binary decision model (`Allow` / `Block`) with 12 built-in rule groups (see [Protect Service](../../docs/topics/protect-service.md))
    - we will define a low-impact default configuration 
    - when a user runs `init` they will automatically be signed up for this default set of protections
    - in configuration, the basic setting is a boolean value: `{ protect: true }`
    - if a user wants to use protect but modify the settings then instead of `true` they provide a `ProtectConfig` object that toggles individual rule groups and adds custom patterns

### Simplifying Assumptions

- Logging
    - A user can either turn on or off the `logging` service that is all that's provided via configuration
    - The "actions" configuration does NOT need to add any actions to **log** because we either log all events or none based on the state of the one configuration
- Protect
    - When Claudine is initialized we'll assign them the "default" set of protections (all 12 built-in rule groups enabled)
    - Users can turn on or off the service or toggle individual rule groups from the 12 built-in groups to create a custom config
- User vs Repo Scoping
    - Most configuration is **User** scoped but where both User scoped and Repo scoped are needed it is described explicitly in this spec
    - Repo scoped configuration is NOT stored in the user's config (e.g., ~/.claudine/config.json`) only the repo's configuration (e.g., `{repo-root}/.claudine/config.json`)
    - If a user runs `claudine config` in a non git directory then repo scoped UI will not be presented

### Canonical Events

A key change in this refactor is that actions are bound to **Claudine Events** (canonical, cross-provider) rather than per-provider event bindings. The current config stores actions nested under each provider's event map. The new config uses a flat `actions` map keyed by canonical event names.

Claudine's dispatch pipeline already normalizes provider-native events into canonical events. This change simply moves the user-facing configuration to match that model — users no longer need to think about which provider they're configuring.

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
     * protect settings.
     * See: claudine/docs/topics/protect-service.md
     */
    protect: boolean | ProtectConfig;
    /**
     * Actions bound to canonical Claudine events (cross-provider).
     * Each event can have zero or more actions that execute sequentially.
     */
    actions: Record<ClaudineEvent, ClaudineAction[]>;

    /** the preferred agent to use for lazy composition operations */
    preferred_agent: Provider;

    /**
     * The canonical provider for this scope.
     * 
     * In user-scoped config (~/.claudine/config.json): sets the 
     * default provider used across all repos on this host.
     * 
     * In repo-scoped config ({repo}/.claudine/config.json): overrides
     * the user-scoped canonical provider for this repo only.
     * The repo-scoped list includes ALL supported providers (not just
     * those installed on the host).
     */
    canonical_provider?: Provider;
}
```

### Migration

This is a clean break from the old configuration format. Existing configs will not be auto-migrated. When Claudine detects an old-format config it will back up the file (e.g., `config.json.bak`) and treat it as if no config exists, triggering the Initialization Process. The new config is simple enough that re-initialization is fast.

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
        - We will explain the purpose of Messenger and offer to configure it during initialization
        - Purpose of Messenger:
            - leverages the `messenger` library in this monorepo to send messages to chat applications such as Discord, Slack, WhatsApp, etc.
            - by configuring this a user can be notified of events when they are away from their computer
            - eventually we may even allow them to _respond_ to events remotely
        - If the user chooses to skip Messenger during init, it can be configured later via `claudine config` (Messenger tab)
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
        - `- you can edit this file directly with an editor but we recommend using <green>claudine config</green> instead`


#### Configuration with the `config` Subcommand

- Today we have an `init` command but no `config` command.
- As a part of this feature we will REMOVE the `init` command and _create_ the `config` command.
- The config command is responsible for changing the Claudine configuration in a user friendly way
- It is NOT, however, responsible for the initial configuration (which uses the aforementioned Initialization Process)
    - that means that if `claudine config` is run before there's a config file defined, we will instead pass execution over to the Initialization Process

The `config` command will bring up a TUI implemented using the [ratatui](https://github.com/ratatui/ratatui) crate in Rust. It will present a row of tabs horizontally; the tab titles are:

- `Preferences`
- `Services`
- `Actions`
- `TTS`
- `Messenger`

The User enters the app in **overview mode** and can:

- move forward and backward on which tab is "focused" with the "tab" and "shift+tab" keys
    - user can also alternatively use left and right arrow keys
- pressing `ENTER` will switch the **focused** tab to become the **selected** tab and move the App from **overview mode** into **detail mode**
    - the tabs UI must show clearly which tab is _focused_ and visually distinguish that from _selected_ when the user presses enter
- While in the **overview mode**, pressing `Q`, `q`, `ESC` will exit the app

> **Note:** take the time to make the design of the TUI beautiful and ergonomic and use the `tui` skill!

##### Actions by Tab (in detail mode)

> **NOTE:** 
> 
> - when a user has selected a particular tab by pressing ENTER they are now in the **detail mode** 
> - in this mode pressing ESC will no longer exit the app, instead:
>     - when the tab has a modal box open, pressing ESC will dismiss the modal but keep the current tab as **selected**
>     - if the tab does not have a modal box open then pressing ESC returns the App to the **overview mode** and the _selected_ tab will go back to just being _focused_.

1. Preferences

    This tab is responsible for managing the following things:
     - Preferred Agent
     - Canonical Provider (for User scope and Repo Scope)
     - Default Sounds (success, attention, error)

    Here are the details:

    - Preferred Agent
        - When this tab _is not selected_ this just displays the name of the Agent
        - When this tab _is selected_ then the Agent's name remains the same but instead of plain text it is rendered as a "select/dropdown" component
        - Pressing `A` when this tab is selected will open the "select/dropdown" component to reveal the installed Agents on the given host
            - in this mode the user navigates the selected Agent with the up and down arrow keys
            - pressing ENTER will select the focused Agent as the new preferred agent and close the "select/dropdown"
            - pressing ESC will close the "select/dropdown" without changing the preferred agent
    - Canonical Provider (User scoped)
        - The "user scoped provider" behaves from a UI standpoint just like the preferred agent
        - When the Preferences tabs becomes focused it shows as a closed "select/dropdown" component
        - Pressing `U` will open the "select/dropdown" component
            - the list of agents a user can choose from are those which the host computer has installed
            - in this mode the user navigates the selected canonical provider with the up and down arrow keys
            - pressing ENTER will change the canonical provider to being the Agent the user had _selected_ when they pressed ENTER; the "select/dropdown" box will be closed
            - pressing ESC will close the "select/dropdown" without changing the canonical provider
    - Canonical Provider (Repo scoped)
        - if the user is in a repo when they ran `claudine config` then we will manage this repo's canonical provider
        - if the user is NOT in a repo then the UI will show a message indicating this (e.g., "not in repo", etc.)
        - we will not allow a user to configure all of their repo-configs from within this UI
        - Assuming that the user WAS in a repo when running `claudine config`:
            - The UI is exactly the same as choosing the user scoped provider, except:
                - The list of Agents is all of our supported agents (not just those installed on the host)
                - The keybinding is `R` to open the "select/dropdown" 
    - Default Sounds
        - these configuration settings allow us to specify a sound effect from the available sound effects which should be the _default_ for "successful", "attention", and "error" outcomes
        - The UI should present all three defaults on one line
        - Keybindings are:
            - `S` will pop-up a Sound Effects modal for the "successful" default
                - all sound effects provided by Playa will be listed along with "None", current selection for "successful" is highlighted
                - the user can change the _selected_ effect with up and down arrow keys
                - they can press `P` to hear that sound effect play
                - they can press ENTER to change to the selected effect and exit the modal
                - they can press ESC to exit the modal and rejecting any changes
            - `A` will pop-up a Sound Effects modal for the "attention" default; behavior identical to "successful" UI described above
            - `E` will pop-up a Sound Effects modal for the "error" default; behavior identical to "successful" UI described above

1. Services

    - **Logging** is presented as an on/off toggle switch
        - we may offer some configurability later but for now the only thing a user gets to choose is whether they want to use it or not
    - **Protect** is presented as an on/off toggle switch but with the text `<i>default config</i>` or `custom config (<dim><i>{#} enabled</i></dim>)` to the right
    - Pressing `L` will toggle Logging
    - Pressing `P` will toggle Protect
    - Pressing `C` will open another modal showing a vertical list of the features of Protect
        - each feature has a checkbox next to it to represent whether it is on or off
        - user navigates using up and down arrows, highlighting one of the features each time
        - pressing space bar toggle the state of the selected feature
        - pressing ENTER accepts the user's changes and exits the feature list modal
        - pressing ESC rejects the user's changes and exits the feature list modal

1. TTS

    - Default view shows the on/off toggle at the top
        - all other controls are greyed out when the toggle is in the "off" state
        - `T` toggles the TTS setting between on and off
        - all other keybindings are in the disabled when TTS is off
    - Under the on/off toggle you have a horizontal info panel which shows:
        - `<b>{provider}</b> → {female_voice} / <dim>{male voice}</dim>`
        - note: one of the two genders is _dimmed_ where the non-dimmed gender is the "default gender" 
    - Keybindings are:
        - `P` brings up a TTS Provider Modal:
            - the modal presents each TTS Provider the host has installed in a vertical list
            - the user navigates this list with the up and down arrow keys
            - pressing ENTER accepts the user's choice and exits the modal
                - if the user did change the provider then we must update the male and female voices
                - this is required because voices are determined by the provider, so _changing_ the provider means that whichever voices had been selected are no longer valid
                - we will make this change automatically by switching the voices to the "default" voices for that provider
            - pressing ESC rejects the user's choice and exits the modal
        - `SHIFT+M` makes the preferred gender "male"
            - the change will immediately be reflected by the horizontal info panel
        - `SHIFT+F` makes the preferred gender "female"
            - the change will immediately be reflected by the horizontal info panel
        - `M` brings up the Voice choice modal for male voices
            - the available voices are a product of which TTS provider has been chosen
            - list is presented vertically (one voice per line), the current choice starts out being the selected item
            - user navigates with up and down arrow keys
            - pressing ENTER accepts the user's changes and exits the modal
            - pressing ESC rejects the user's changes and exits the modal
        - `F` brings up the Voice choice modal for female voices


1. Messenger

    > **NOTE:** A user should be allowed to configure as many "configurations" as they like (including multiple configs for the same provider) but the user will configure just one for "user scope" that is active. 
    > 
    > - if a user has run `claudine config` from within a git repo then the option to set an "override" for this repo is allowed too.


    - The UI should look like a _closed_ Select Box with the selected Messenger app displayed (it will display "None" if the user has chosen to disable this feature or hasn't yet configured any apps)
    - To the right of the Select Box is a "Add" button
    - using Tab / Shift-Tab moves the selection between the Select Box and the Add button
    - pressing ENTER when the Select Box is _selected_ opens the Select Box to show all configured apps
        - the user then can move up and down with the arrow keys and when they press enter it indicates the highlighted app (or None) is the new setting
    - pressing ENTER when the "Add" button is selected will bring up a Modal for user to choose the App and it's settings
    - pressing `S` will open the Select Box and allow user to use up and down arrows to select the app configuration they want
    - pressing `A` will open the add App modal

1. Actions

    - this tab will show the events which have something configured to them already
        - logging service (which attached to all events when turned on) does not count
        - protect service (same thing)
    - the events will be organized vertically with the top one "selected" (with visual cueing)
        - Here's an example of what an event might look like in this list: `<b>{event-name}</b> <dim><i>SoundEffect, Messenger, and 2 Bash</i></dim>`
    - NOTE: remember that `Log` actions are no longer recorded as actions because the logging service either logs ALL or NONE of the Hook events
    - user can move up and down with arrow keys to change which is selected
        - we need some sort of visual indicator for which is selected
    - pressing `D` will:
        - will bring up a confirmation dialog to confirm
        - if accepted, will delete all actions for that event
    - pressing `A` will:
        - bring up a list of events that do not have any actions assigned to them
        - the user navigates with up and down arrow keys to highlight an event
        - pressing ENTER selects that event and opens its Event Modal so the user can add actions to it
        - pressing ESC dismisses the list without adding an event
    - pressing ENTER or `E` will open the Event Modal for the selected event
        - each event will have 0:M actions configured to be triggered by this event being fired
        - these actions will be listed vertically
        - user will use up and down arrows to navigate selection of the existing actions
        - pressing `A` will bring up the "new action" dialog
            - How actions are configured today is described in [Configuring Actions](@claudine/docs/topics/configuring-actions.md)
            - However, in this feature we will modify this a bit, this is described below in the [Better Action Configuration](#better-action-config)


## Better Action Config

Today we have a functioning schema for configuring Actions (to Events) which is defined in [Configuring Actions](@claudine/docs/topics/configuring-actions.md). This solution isn't terrible but we do want to make a few adjustments.

With this refactor, the `log` action type is removed (the logging _service_ handles all-or-nothing logging). The `fire_and_forget` action type is replaced by `bash`. The `call` and `report` action types are retained as-is from the current implementation.

The built-in actions we provide in Claudine are:

1. **sound_effect** (existing but updated)

    - play a sound effect from the `playa` library
    - format: `{ type: "sound_effect", effect: "{effect name}", volume?: {#}, speed?: {#}  }`

      | Field   | Type | Default | Description |
      |-------  |------|---------|-------------|
      | `effect`  | `string` | (required) | Effect name from playa's 88 embedded effects |
      | `volume`| `f32` | `1.0` | Playback volume (0.0 to 1.0) |
      | `speed` | `f32` | `1.0` | Playback speed multiplier |

    - we changed the property `name` to `effect`
    - we should support deserialization from JSON5 instead of just JSON (this is supported by the `biscuit-file` library)
    - otherwise the same as current implementation

2. **speak** (existing but updated)

    - speak words using the TTS functionality provided by `biscuit-speaks` library
    - format: `{ type: "speak", message: "{message}", voice?: "{voice}", gender?: "{male|female}"}`

      | Field     | Type     | Default    | Description                                       |
      |-------    |------    |---------   |-------------                                      |
      | `message` | `string` | (required) | Template message with `{{variable}}` placeholders |
      | `voice`   | `undefined` \| `string` | _undefined_ | A specific voice to use; if there is not a voice match then default voice used |
      | `gender`  | `enum(undefined,male,female)` | _undefined_ | Can override user's preference for gender |

    - this property _does_ exist in the current schema but we've added the `voice` and `gender` properties
    - same rule about allowing JSON5 deserialization applies here too

3. **message** (existing)

    - sends a message to Claudine's configured chat/message app
    - leverages the `messenger` library in this monorepo
    - format: `{ type: "message", message: "{message}", image?: "{image_ref}" }`

      | Field     | Type     | Default    | Description                                       |
      |-------    |------    |---------   |-------------                                      |
      | `message` | `string` | (required) | Template message with `{{variable}}` placeholders |
      | `image`   | `undefined` \| `string` | `undefined` | filepath to a raster image; if message platform supports it then it will be rendered, otherwise it will be ignored |

    - same rule about allowing JSON5 deserialization applies here too


4. **bash** (new)

    - this was supposed to be implemented before but must have been missed
    - this allows a user to specify a command to shell out to and run
    - format: `{ type: "bash", command: "{command}", params?: "{parameters}" }`

      | Field     | Type     | Default    | Description                                       |
      |-------    |------    |---------   |-------------                                      |
      | `command` | `string` | (required) | Template message with `{{variable}}` placeholders |
      | `params`  | `string` | ""         | parameters to pass to the command, `{{variables}}` are permitted and will be interpolated before being used |


    - the `command` must either be fully qualified path to the executable or be in the executable path, Claudine will check this before executing and it will marked as invalid
    - there will be a small set of rules for commands not allowed which will mark the configuration as invalid; these commands will never be executed even if present in the configuration
    - Executing JS/TS
        - if the file has a "shebang" on line 1 of the file, then the executor in the shebang will be used (if present on the host); if it is not present then this will create an error
        - if there is no "shebang":
            - if the path points directly to a Javascript or Typescript file, Claudine will execute it with `bun` if the host has it installed
            - if `bun` is not found but `node` is, then Javascript files will be executed with `node` (but not Typescript)
    - same rule about allowing JSON5 deserialization applies here too

5. **report** (existing, unchanged)

    - Print event information to stdout, making it visible in the agent's output stream
    - Retained as-is from current implementation (see [Configuring Actions](@claudine/docs/topics/configuring-actions.md) for full schema)
    - same rule about allowing JSON5 deserialization applies here too

6. **call** (existing, unchanged)

    - Execute an external command synchronously and map its output to a `HookResponse`
    - This is the only action type that can influence agent behavior on blocking events (e.g., `before_tool`, `before_prompt`, `permission_request`)
    - Retained as-is from current implementation (see [Configuring Actions](@claudine/docs/topics/configuring-actions.md) for full schema)
    - same rule about allowing JSON5 deserialization applies here too

### Action Type Summary

| Action | Behavior | Status |
|--------|----------|--------|
| `sound_effect` | Fire-and-forget audio playback | Updated (`name` → `effect`) |
| `speak` | Fire-and-forget TTS | Updated (added `voice`, `gender`) |
| `message` | Fire-and-forget messenger | Unchanged |
| `bash` | Fire-and-forget shell execution | **New** (replaces `fire_and_forget`) |
| `report` | Stdout event visibility | Unchanged |
| `call` | Synchronous with HookResponse | Unchanged |

**Removed**: `log` (replaced by logging service toggle), `fire_and_forget` (replaced by `bash`)
