At times an Agent's servers/capacity can become overloaded and they start returning a 529/overloaded_error. I recently encountered that and our current behavior is to immediately fail and exit:

```sh
┃ API Error
┃ API Error: Overloaded

✗ 1238s · 42 input tokens · 15K output tokens · 4.1M cached tokens · $3.43 cost basis · 41 tool calls
 Feature review 1 in the claudine package area failed to complete!
ERROR: There was a problem trying to create the review for Flattened Bridge
error: Recipe `review` failed with exit code 1
```

That is a very bad response because in non-interactive sessions that means we've built up a bunch of useful context and now we're suddenly throwing it all away! We need to handle this more gracefully by:

- having a graduated retry logic kick in
- wait 1 minute and try again
- wait 2 minutes and try again
- wait 4 minutes and try again
- wait 8 minutes and try again
- wait 16 minutes and try again
- FAIL

My example was with Claude Code but the idea is the same for all providers ... if we find that they are "overloaded" it doesn't mean that we did anything wrong it means that we need to move into a retry loop so that we can continue when they recover.

- the timing logic above should be used as a default for all providers
- a global override for this timing can be set with OVERLOAD_RETRY env variable:

    ```sh
    export OVERLOAD_RETRY="15,30,60"
    ```

    The variable takes a comma separated list if timings (based in seconds) so in the above example we will wait 15 seconds, then 30 seconds, then 60 seconds, and then fail.

- if you want to only reset the timing for a particular provider then the env variables `OVERLOAD_RETRY_{provider}` will have precedence
    - if `OVERLOAD_RETRY` were set to "15,30,60" but `OVERLOAD_RETRY_CLAUDE` were set to "5,10,15" then we would use the "5,10,15" timing for Claude Code but 15,30,60 for all other providers.
