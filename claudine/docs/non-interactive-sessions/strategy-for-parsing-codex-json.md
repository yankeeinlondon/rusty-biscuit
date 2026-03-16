---
prompt: |-
	The Codex CLI can output a stream of JSONL output when the `exec` command is paired with the `--json` flag. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.

    Here's an example of the JSONL data you might get in a request:

    ```json
    {"type":"thread.started","thread_id":"019cf582-ae5f-71f1-af52-8a6e62c1bc22"}
    {"type":"turn.started"}
    {"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hi Ken. What do you want to work on?"}}
    {"type":"turn.completed","usage":{"input_tokens":28903,"cached_input_tokens":4480,"output_tokens":61}}
    ```

    - This metadata can be used to present metadata to the user on STDERR when they are executing a non-interactive command.
    - This metadata can be used to enhance the data we're providing to our logging platform

    Your task is to:
    
    - research other examples online and fill in any other missing details not self-evident from the example data
    - determine how best to feed the metadata to logging and non-interactive sessions.

last_update: 2026-03-16
---
