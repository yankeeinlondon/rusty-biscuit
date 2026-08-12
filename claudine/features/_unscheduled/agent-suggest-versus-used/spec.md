# Feature Agent Suggestions versus Agent Used

## Today's Semantic Conflict

Right now we are using the `agent` frontmatter to allow Markdown authors to suggestion 1 or more Agent's to use for
the given prompt. However, in addition, we have established a convention in our prompt templates of writing the "actual"
agent that was used to the `agent` frontmatter.

- the "suggested/forecasted" versus "actual" Agent are indeed related but independent things
- it is helpful to "suggest" but:
    - if a user uses the CLI to explicitly choose an agent then the CLI always dictates the actual model used
    - furthermore, the Markdown author may very well suggest not a singular model but rather a set of models they deem
      to be a good choice for the task; at this point there is a more obvious divide between what is "suggested" versus
      what is actually used

It might be thought that the best solution would be just to change the "convention" that we use for the actuals and that
might be a good answer but we also need to recognize that when we use the `inline-compose` operation that the "prompt file"
and the "output file" are the same file and it's actually in these circumstances where this semantic conflict is most
common and most damaging. In the inline-compose operation, Claudine has a more direct responsibility for the file that
would be the case of a `compose` operation where the _side-effect_ of the compose operation was that Markdown documents
were written to the filesystem which indicated the agent property in frontmatter.

## Representations of Agent and Model

While today we allow _independent_ suggestions of Agent and Model -- and these are indeed valid things to want to allow for -- we actually miss out on the more common and more useful mechanism of combining the agent and model string into one entity:

- Today we might state that the agent is `opencode` and then provide a list of models like `kimi-for-coding/k2p7`, `zai-coding-plan/glm-5.2`, etc.
    - this works well when we have just one Agent provider selected but far less well when there are more than one
- Instead what if we could express the agent/model pairings like so:
    - `opencode/kimi-for-coding/k2p7`, `opencode/zai-coding-plan/glm-5.2`
- This would mean exactly the same thing but more importantly it would allow us to create more useful choices like:
    - `opencode/kimi-for-coding/k2p7`, `opencode/zai-coding-plan/glm-5.2`, `claude`, `codex`
    - this list combines "resolutions" to allow the aggregator agent "opencode" to explicitly allow for two models while allowing the two vendor-focused agents to just use their "default model" as the suggestion
    - this kind of list is often exactly what a Markdown author really wants to express
- So then the question becomes, does the separate `agent` and `model` frontmatter properties provide a real utility if we allow both types of agent specificity?
    - the truth is that while it DOES provide a purpose, that purpose is actually better served as a "config" level feature
      than it is a Markdown document feature
    - specifically, what can definitely be useful is for a repo or a user (mapped to repo and user scoped config) to express
      which models they "favor" in general terms or possibly as a map of "capability" to "model"
    - this would allow all interactive dialogs to either filter to or favor these choices
    - it would also allow

## Adding Agent Preferences

We are going to add "agent preferences" to the configuration of Claudine. This configuration will be allowed at both the
user and the repo level. As is always the case when we have two scopes for the same configuration we must be explicit about
how the two scopes will interact. As a general rule -- which is always the case by default -- the "repo scope" overrides the
user scope.

Considerations for this config design:

1. General Model Preferences versus Capability Model Preferences
    - in many development environments we will refer to agents and models (where needed) by their idiomatic names
    - this offers a precise identification mechanism and is likely what most companies and individuals will gravitate towards (at least to start but with maturity more abstraction will be a welcome power user tool)
    - Claudine is happy to accept idiomatic and explicit naming but also provides a powerful abstraction: naming a "capability" of the model desired instead of an explicit model
    - by abstracting the model choice to a _capability_ we immediately benefit two significantly sized audiences:
        - open source projects who are genuinely looking for contributions can reasonably state their preference for a
          category/capability of model to be used in various parts of their repo instead of requiring a specific agent/model pairing that aligns with the maintainer's subscriptions but not necessarily the world at large
        - developers who have more than one subscription and are constantly moving between them based on plan caps or other
          considerations are able to express a more fluid definition of what is capability is needed and Claudine will
          then:
            - (day 1) let the user choose the appropriate model based on the developers awareness of their plan's current
              status
            - (future) defer to Claudine's awareness of the developer's current plans and how much of a token budget each
              plan has left before being capped.

2. Suggestion versus Constraint
    - for people and organizations, to maintain model control we need to allow a repo level constraint to be clearly
      marked as either a "suggestions" or an explicit "constraint"
    - when a repo defines it's agent/model preferences as a constraint then Claudine will refuse to let a user to use
      a model that doesn't comply with the required constraint system.
        - this includes not allowing the configuration file to be dirty (aka, do not allow developer to just change the constraint temporarily so that the constraint can be broken)
        - this also includes that the latest version exists on the upstream remote in the current form (aka, a configuration
          only becomes valid once it has been pushed and accepted into the default branch of the remote); the intention
          is to force the remote to be the authority on the actual configuration that Claudine will use
        - admittedly in this arrangement a user can easily use Agents directly outside Claudine and circumvent the constraints
          defined.
        - in it's current form, it is not designed to stop bad intent by other developers only to reinforce what the
          intent is so that developers who want to respect the wishes of the repo owner can easily do so.
            > Note: if there are any low-cost ways of making this somewhat more enforcible that can be included into the scope
            > of this feature but we do not expect this to be a "hardened" solution. In fact, at some future point we may
            > develop this more hardened approach as a "corporate" version which is offered commercially (instead of open source
            > like the rest of this monorepo)

So now that we've defined the two dimensions which we want to have configuration for let dig into how this might be structured:

> NOTE: the specifics provided below represent an initial point of view but should be considered changeable during the clarification process this design will undergo

- configuration items will be saved under:
    - `model.favorites` - the user/repo's model favorites
        - this is a flat list of model's the user/repo has "preference for" more than others
        - this list is not made to **make** any decisions but instead to provide preferred models at the top of any interactive list presented to the user
    - `model.capability_map`
        - a dictionary where the keys are "capabilities" and the values are a list of models which
          should be considered to be part of this group
        - the capabilities map will be set by default using the data we can get from unchained-ai (this kind of abstraction is already provided); this default should be saved as "default" in the configuration so that as new versions of Claudine are released the "defaults" are constantly being updated to the latest models supported in the Agent ecosystem. However, for the
          user's benefit ... when they are viewing the configuration through the `config` TUI we must make sure a user who
          has selected "default" has a visual report of what "default" is equal to so they can understand their configuration
        - this idea of a "default" which auto-updates with new versions of Claudine is a nice way to help the user not
          accidentally "set and forget" a mapping and then over time have the capability based map point to old and outdated models
        - I think it MAY make sense to only allow a user to add additional key/values to the capability map rather then
          allowing them to override what the default configuration gives. This at least reduces the surface area of the user's
          bespoke key/value map
        - Regardless if we allow or don't allow for a user to override the default capability map, we should always check any
          customer provided key/values and if the model's they are specifying are not the latest version of that "model family" then we should report a WARN status message indicating this when claudine starts a wrapped command.

## Frontmatter Suggestions

For a Markdown author to create a cognitively clear "suggestion" on which agent and/or model should be used; we will:

0.  Decommission the base `agent` and `model` properties as being a special properties in Claudine

1.  Suggestions will now all be defined in the `suggest_agent` frontmatter property and there will NOT be any property taking on
    the "model" name. However, this new `suggest_agent` property will accept: - 1:M canonical references where a canonical reference is allowed to be model neutral or model specific; the following
    would be considered a valid definition:

           ```yaml
           suggest_agent:
             - claude
             - codex
             - opencode/zai-coding-plan/glm-5.2
           ```

             The `claude` and `codex` references suggest these agents with their respective "default model" whereas the
             the `opencode` agent is also a suggestion but only when paired with the `zai-coding-plan/glm-5.2` model.

         - a "capability" model is also allowed and can be left unbounded or attached to an agent/agents; this means the following config would be considered valid:

             ```yaml
             suggest_agent: flagship
             ```

         - it's also possible to mix capabilities and explicit models

             ```yaml
             suggest_agent:
                 - flagship
                 - github-models/xai/grok-3
                 - opencode/github-models/xai/grok-3-mini
             ```

             In this example we're including the "flagship" model group but also adding in the grok-3 model (inside any of the
             support Agent providers), and grok-3-mini when working with OpenCode as the Agent provider.

             - it's important to recognize that the 2nd config item of `github-models/xai/grok-3` does NOT differentiate
               on which Agent provider is used; only the model
             - although not explicitly shown in this example, the opposite is also possible of defining just an agent
               provider and not the model

2.  The "capability" definitions are defined in configuration and we also allow in configuration for the allowed models
    to be constrained to only allow a subset of the possible set of agent/model choices.
    - We must agree on how a Markdown suggestion (or CLI instruction) on what model or agent to use does not fall inside
      the allowed set of models when a constraint has been imposed.
        - in cases where a user chooses a agent/model via a CLI switch that is outside that allowed by the configuration's constraint system we must immediately exit with a fatal error describing the set mismatch
        - in cases where the suggested agent/model set lives fully outside of the allowed constraint then we should raise
          an error not only when the prompt is executed but also when a user runs as a `--dry-run`
        - when processing any wrapped command that has both a suggested agent/model and a suggested set of agent/models in
          configuration, Claudine will use the intersection of these two sets to define the suggested agent/models
        - when processing any wrapped command that has both a suggested agent/model and a constrained agent/model configuration,
          then only the intersection is used as a favorite/suggested choice but

    - this then must also apply to the `model` property too; that means `suggest.model` is where Model suggestions should go

3.  when we run the `inline-compose` operation and we detect that the `agent` property is set we should:
    - valid use case:
        - in cases where we are running the inline-compose on a prompt which has been run before it is entirely possible
          that the value in Agent is
