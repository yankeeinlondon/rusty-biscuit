# Leveraging Logs

When using **OpenCode CLI** is used with the flags `--log-level ERROR --print-logs` we get a stream sent to STDERR which is quite useful.

Review some example information I found when running `opencode run "hi" --model zai-coding-plan/glm-5.1 --format json --dangerously-skip-permissions --log-level ERROR --print-logs 2> example-of-usage-limit.txt`: 

- STDERR results are in [log](./example-of-usage-limit.txt)
- This stream of text includes a clear message about the fact that I'm being limited and what time the limit cap will be reset.
- By comparison, if we had not asked for logs we would have gotten absolutely nothing streamed to us, just a hanging connection.


