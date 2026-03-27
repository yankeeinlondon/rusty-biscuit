# Validations, Timeouts and Handlers

When we are running non-interactive prompts we will add some conventions to allow Claudine to act more as a harness of Agentic processes.


## Validation Operations

- `file_exists(file_ref, msg = "{{status}} the file {{file}} exists")`
    - The file exists in the filesystem
- `dir_exists(dir_ref, msg = "{{status}} the file {{file}} exists")`
    - The directory exists in the filesystem
    
- `json_file_exists(file_ref, shape?)`
    - The file exists in the filesystem
    - AND the file is a valid JSON file
    - the validation may optionally specify a broad shape ('scalar', 'array', 'object') that the root of the data structure must have
- `yaml_file_exists(file_ref, shape?)`
    - The file exists in the filesystem
    - AND the file is a valid YAML file
    - the validation may optionally specify a broad shape ('scalar', 'array', 'object') that the root of the data structure must have
- `has_write_permission(file_ref)`
    - Checks the agent config to see if the user has **write** access to the given file
- `toml_file_exists(file_ref)`
    - The file exists in the filesystem
    - AND the file is a valid TOML file
- `shell_command(cmd_and_params, show_stdout = true, show_stderr = true)`
    - The shell check returns a valid exit code
- `no_dirty_source_code(file_offset = ".")`
    - Checks if there is are any source code files (from the specified file path and deeper)
    - fails when dirty source code files found
- `has_dirty_source_code(file_offset = ".")`
    - Checks if there is are any source code files (from the specified file path and deeper)
    - fails when no dirty source code files found

### Only Available for Post Validation

- `file_unchanged(file_ref)`
    - when this validation is added to a page claudine will hash the file reference before and after the agent prompt is run
    - fails when the hashes change
- `file_changed(file_ref)`
    - when this validation is added to a page claudine will hash the file reference before and after the agent prompt is run
    - fails when the hashes remain unchanged
- `frontmatter_prop_changed(prop)`
    - tests whether property specified changed it's value during the execution of the Agent
- `frontmatter_prop_unchanged(prop)`
    - tests whether the property changed has remained at the same value
- `frontmatter_prop_equals({prop: value})`
    - tests that the key's defined equal the expected values
- `response_length_at_least(length)`
    - test the length (characters) of the response in the Agent's final (non-thinking) response to STDOUT
    - fails if the response is less than `length`
- `response_length_at_most(length)`
    - test the length (characters) of the response in the Agent's final (non-thinking) response to STDOUT
    - fails if the response is greater than `length`
- `response_includes(find)`
    - test whether the Agent's final (non-thinking) response to STDOUT contains the given substring
    - fails if the substring is not found
- `response_missing(find)`
    - test whether the Agent's final (non-thinking) response to STDOUT does NOT contain the given substring
    - fails if the substring is found


> **Note:** all validations will have an optional `msg` property which a caller can override but has a sensible message structure as a default. This `msg` is used to report to the non-interactive prompt:
> 
> - on success: `<b><green-500>✓</green-500></b> ...`
> - on failure: `<b><red-500>⤫</red-500></b> ...`

## Handlers

- **retry**`(prompt?, set?, msg?, say? retries?)`
    - this reruns the same prompt used before as a new session
    - Claudine will track the number of retries and not go above 3 retries (or what you set in `retries`)
    - if the user specifies a `prompt` in the handler then that will be appended to the end of the composed prompt
    - the user doesn't have to specify `prompt` and if they don't then a generic message will be appended to the 
    - if the user wants to override the page's frontmatter they can use the **set** object property
- **resume**`(prompt, set?, msg?, say? retries?)`
    - The `prompt` must be specified when using the **resume** handler
    - this leverage's the Agent's "resume" features along with Claudine's ability to capture the session_id of non-interactive sessions
    - we start a new interactive session but resume to the end point of the failed session and then post the new `prompt`. 
- **redirect**`(file, set?, msg?, say? resume?)`
    - Handles the error by redirecting to a different Markdown document
    - this new Markdown document must be composed
    - there is an optional `resume` boolean flag which determines whether the prior session should be resumed and then redirected to the new file or the new file should start with a fresh context window.
- **deviate**`(cmd, set?, msg?, say?)`
    - the **deviate** handling replaces the prompt's actions with a call to an executable command (and params)
    - the shell command needs validation just like the shell-expansion option in Darkmatter works
        - ideally it would be linked into the same approval file

> **Note:** the optional `msg` property on all handlers allows you the handler to provide a message to STDOUT and what ever text they use will be loaded into `Prose` struct so that a user use terminal formatting.

## Special Frontmatter Properties

### Validations

Validations are used to test certain things about the **state** _before_ and _after_ the Agent is called with the _composed_ prompt.

- `pre_checks`
    - this property is used to describe the **state** which must exist for this prompt to start it's execution
    - if this property is set and it's a list of dictionaries then we will use it to validate that we are in a valid starting state
    - if the property is set but is not a valid structure then we will immediately exit with an error:
        - `<b><red-500>ERROR:</red-500></b> the <a href={absolute-filename}><blue>{relative-filename}</blue></a> prompt/composition file has an invalid <b>pre_checks</b> frontmatter property!`
- `post_checks`
    - this property is used to describe the **state** which must exist _after_ the Agent successfully completes for this job to be considered "complete"
        - Any failure will result in an error status being returned (unless handled)
    - if the Agent fails during it's execution we will provide as much context as possible 

#### Example: Validation Config

```md
---
post_checks:
    file_changed: "@this-file-must-change.md"
    response_missing: "failed"
    response_length_at_least: 150
---
# My Markdown Page
```


### Timeout

To stop processes from running forever, we allow a prompt to set their default "timeout" period:

- `timeout`
    - accepts `{#}{unit}` where unit is 
        - `s`, `sec`, `second`, or `seconds` for seconds
        - `m`, `min`, `minute`, or `minutes` for minutes
        - `h`, `hr`, `hour`, `hours` for hours

### Handlers

An Agent can fail all by itself and a validation or timeout can also put us into an error state. Handlers let us try to recover from that error.

- `handle`
    - Note: this is the lone _programmatic_ handler; the remaining are configuration based
    - passes all error conditions to an executable script to process and return either
        - `false`/`void` if the error can not be handled
        - `true` results in a **retry** action with default params
        - Dictionary of the type gives full control:

            ```ts
            type HandlerType = "retry" | "resume" | "redirect" | "deviate";
            type HandlerResponse = {
                action: HandlerType;
                say?: string;
                msg?: string;
                retries?: number;
                set?: Record<string, unknown>;
                
                /** a file reference for pairing with the "redirect" handler */
                file?: string;
            }
            ```

            > **Note:** programmatic handlers aren't allowed to use the `deviate` handler type. YAML-configured `deviate` commands are statically known at parse time and can be pre-screened against the shell command whitelist before execution begins. Programmatic handlers return commands at runtime, so Claudine cannot guarantee they've been screened before invocation.

- `handle_{EVENT}`
    - you can _handle_ any validation operation using just YAML configuration
    - event names are the same as validation names, plus the following:
        - agent_failure (e.g., `handle_agent_failure`)
        - timeout

#### Example: Handlers Config


```md
---
handle_file_exists:
    "this-file-should-exist.md":
        retry: 
            set: 
                try_again: true
handle_timeout: 
    resume:
        msg: "<red-500>⤫</red-cross> we timed out!"
        say: "The agent timed out, we will try again"
        retries: 2
---
```


## Design Considerations

- all errors that are possible should be returned in plain english and with enough context so that the user knows immediately how to correct the problem
- any useful feedback on progress is always welcome on STDOUT
- our testing must have strong unit and integration test coverage
- during design we need to make sure we have precise information for every supported CLI on how to resume a session from the command line
    - note: it would make a lot of sense for Claudine to have a consistent `--resume <session>` flag that maps to each provider's methods of resuming. 
- any shell commands detectable at runtime should be validated against our whitelist and if any of them -- even if they might not be executed -- are not approved you must ask the user for approval immediately.
