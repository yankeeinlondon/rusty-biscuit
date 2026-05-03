All messages to the user about cap timing should always be converted to the user's local timezone and the timezone listed to clear up any ambiguity:

- when using Opencode with 'kimi-for-coding' model we get:

    ```sh
    ⚠ Rate Limit — Usage limit reached for glm-5.1 (zai-coding-plan); resets at 2026-04-29 21:48:11
    ```

    This is ambiguous which timezone is being referred to but I suspect it is UTC.
