When we report errors like this:

```sh
󰀨 Claude rate limit status: allowed_warning; next session window opens at 2026-04-19 00:00:00 UTC
```

The message is not super clear to the user and the time being presented in UTC is fairly unworkable.

- we need to change the message to: `Claude rate limit warning: your current session window is almost fully utilized and you will be capped soon. The next session window opens at {datetime}`
- the `{datetime}` should be the user's local time:
    - `YYYY-MM-DD <i>at</i> HH:MM` in local time
