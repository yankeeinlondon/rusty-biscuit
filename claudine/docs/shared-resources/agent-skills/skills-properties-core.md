## Core Skill Properties

The most important Frontmatter properties that an Agent Skill defines are:

1. `name` (required)
2. `description` (required)
3. `model`

Not all agents require `name` but all of them require `description` and for Claudine's purposes we view both properties as being required. This name and description pairing is what is used by the LLM agent to determine (description) and then lookup (name) the Agent Skills it feels are necessary.

We have added `model` as a **core** property because most agent platforms support this property as a way to explicitly ask for a particular model to be used. The complexity that it brings in, however, is that the models which are available to each agentic platform vary and therefore are not portable.
