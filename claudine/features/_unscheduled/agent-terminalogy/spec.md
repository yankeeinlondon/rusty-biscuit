
There are certain words each Agent platform describes differently than the others. To account for that, Claudine will look for the following file in the root directory of the repo:

- `agent-terminology.yaml`

> Note: if `agent-terminology.yml` is found then present a warning each time claudine is called specifying that they should rename to a `.yaml` extension.

The YAML file will define an array of "terms", each term will have any of the following keys set:

- claude - _how this term is expressed in Claude Code_
- codex - _how this term is expressed in Codex_
- gemini - _how this term is expressed in Gemini_
- qwen - _how this term is expressed in Qwen Code_
- kimi - _how this term is expressed in Kimi Code_
- opencode - _how this term is expressed in OpenCode_
- **default** - _the default way of expressing this term across agent platforms_

To be a valid term, the term must have at least **default** defined or _all providers_ defined.

When we find this vocabulary then this will be used in all markdown composition operations with Darkmatter. Fortunately Darkmatter already provides us the `replace` frontmatter key which will provide swaps from one string (key) to another (value). To enable this we only have to determine how to express each term based on the agent that is being used and and then build a key/value dictionary which we will merge with the existing state of the `replace` property.

