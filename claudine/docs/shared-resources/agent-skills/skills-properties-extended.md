## Extended Skill Properties

Beyond the basic properties that you see frequently in Agent Skill definitions you find a fairly wide variance of properties that each agentic provider recognizes in some fashion. Claudine's main concern remains _portability_ and in this widely divergent set of properties we need recognize some reusability principles to achieve our goals:

1. **Additive Principle**

    While different agentic platforms might recognize different other properties in a `SKILL.md` file they will also ignore any properties which they don't recognize. That means that if an Agent Skill defines properties that don't have meaning on other platforms

1. **Semantically Empty**

    A lot of the _extra_ properties that agent skills add do not have any real semantic meaning enforced by the agent nor do they create a deterministic behavior change that would be worth understanding. It is possible that the agent seeing a key/value pair might adjust it's thinking based on it's inclusion but this is very non-deterministic not to mention mainly LLM biased not Agent biased.


### Additional Skill Properties

We do research on every supported CLI agent and you'll find the detailed results on their support for Agent Skills in @claudine/docs/research/skills (one file per agent) but here are some properties you may have run into in the past to help you understand what's out there in the wild:

- `when_to_use` is supported in Claude (maybe others) but has a huge amount of sematic overlap with the `description`. Claudine will not make any changes to this property but you might want to move some of the text in `when_to_use` into `description` so that all agents are better able to distinguish when it's best to use this skill
- `effort` is also defined in Claude as a way to override the default thinking level, not many others include it but can't be harmed by it's inclusion. As an instruction it's a bit odd anyway as it seems like you'd want a thinking level assigned to an overall prompt but not a skill.
    - including this instead of in what Claude Code calls a slash command is probably largely because these primitives have overlapped in Claude Code over time. You can treat any skill as a slash command (e.g., typing `/my-skill` at an interactive terminal will kick off a prompt that includes the skill file as part of the prompt)
- `metadata` when used is structured as a key/value object that allows tooling to place key/value pairs
