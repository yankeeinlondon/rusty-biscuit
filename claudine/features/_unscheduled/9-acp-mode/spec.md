# ACP Mode

Currently we leverage the streaming JSON response formats provided by Agents to keep the caller abreast of the status of non-interactive queries. We also provide hook/actions functionality which can be quite limited with some providers who don't expose many events.

Using ACP to interact with the various providers would give us a lot more control but it would also require that Claudine:

- take over responsibility for tool calling
- ...
