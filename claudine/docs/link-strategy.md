# Link Strategy

The primary means for ensuring that things like "agent skills", "slash commands", "agent/subagent" definitions are portable across different CLI platforms is through some kind of symbolic linking strategy but there is nuance in this and we must be careful to get this right.

## Rules

The first big consideration is whether we're trying resolve **User** scoped resources or **Repo** based resources.

- **Repo** resources always _override_ a similarly named resource from **User** scope but this is handled by the Agentic CLI itself and is less of a concern for us except for in some cases of reporting
- we will NEVER link a User scoped resource to a Repo scoped resource or visa-versa!
- symbolic links used to link resources at the **User** scoped level will always use fully qualified file paths
- symbolic links used to link resources at the **Repo** scoped level will always use relative file paths
- we will prefer creating a single link to link ALL resources of a resource type but ...
- we will fall back to a strategy of linking resource by resource if a provider being linked to the **base** provider already has some definitions themselves

After we consider the **scope** we must also look at each of the **resource** types individually. These types are:

- Agent Skills
- Slash Commands
- Agent/Subagent Definitions
- Shared Scripts Directory

The key considerations here are,

- which platforms support the given resource type?
- do all platforms which support the given resource type do it in a way that is compatible?
    - first off, we need to have easy access in the Claudine library to what properties for each resource type are considered required and which are considered optional
    - when we determine compatibility we need to make sure if there are variant "required" properties we can _map_ or in some manner ensure that all required properties will be satisfied for all platforms

## Link Detection

The idea of **Link Detection** is to evaluate both **user** and **repo** scopes (separately) for the current state of links. This would identify:

1. Where links are missing
2. Where links are in place
3. What resources are available -- of a given resource type -- by platform

### Process

- Each provider will have a entry point for each resource type they support.
    - This will be true for both **user** and **repo** scope.
- We will evaluate each resource type separately but the process we'll use for each resource will be the same:
    - Our first goal is to establish the **base** provider for this resource type and a scope (user, repo)
    - We will first check if Claude Code meets this criteria by:
        - validating that the entry point for the resource type is NOT a symbolic link
        - validating that at least one resource items exists and that this resource item is NOT a symbolic link
        - we prefer Claude Code mainly because it has held such a dominant position in this space for that year+
        - however, if Claude Code doesn't pass the test for being a **base** provider then we will iterate through the providers in this order:
            - Opencode CLI
            - Codex CLI
            - ... all the rest in any order
        - needless to say any provider who doesn't support the resource type will be excluded from the base search
    - Now that we've established a **base** for both **user** and **repo** scope we will now try to link all other providers who support this resource type to the resource items found on the **base**.
    - We will start by determining the **user** scope and iterate over all providers outside of the **base** provider
        - We will

### Structs supporting this Detection

- we will create `SkillDetection`, `SlashDetection`, `AgentDetection`, and `ScriptsDetection` structs
- these structs will all implement a trait called `LinkDetector` which will enforce the following:
    - `fn new() -> Self`
        - calling the new function will immediately evaluate the the current link state for the given resource and store this into the structs internal properties
    - `fn repo_scoped()`
        - provides all the resource items which are repo scoped
        - all items will be returned with a ResourceScope variant:
            - this is either a `Provider`  which means that this is a locally defined resource that is not shared
            -
    - `fn user_scoped()`
        - provides all the links items of this resource type which are found in the user scope
    - `fn list(Option<FilePath>) -> Vec<Resource>`
        - will list all resources of the given resource type
    - `fn user_base() -> Provider`
    - `fn repo_base(Option<FilePath>) -> Option<Provider>`
    - `fn fix_broken() -> Vec<FixAction>`


### Types

```rust
// This is just the code bases `LinkScope` renamed to `ResourceScope`
pub enum ResourceScope {
    /// a resource which is defined in the **Repo** scope (e.g., within
    /// the working git repo)
    Repo,

    /// a resource which is defined in both the **User** and **Repo** scopes
    /// but because
    RepoMasked,
    /// a resource which is defined in the **User** scope and available everywhere
    /// on the given host so long as the same user is logged in.
    User
}

/// Unlike `ResourceScope` which addresses the **user** and **repo** based scopes,
/// the `LinkScope` distinguishes
pub enum LinkScope {
    /// a resource definition for a single provider (NOT shared/linked to other providers)
    Isolated(Provider),
    /// a resource that is shared across all providers and where the specified `Provider`
    /// is the "base"
    Linked(Provider),

    /// the given `Provider` has a base definition of the resource which _should be_
    /// linked to other providers who support this resource type but currently no
    /// links exist
    BrokenLink(Provider),
    /// the given `Provider` has a base definition of the resource which _should be_
    /// linked to all providers who support this resource type but currently some
    /// of the providers are missing that link.
    PartiallyBrokenLink(Provider, Vec<Provider>)
}

pub enum ResourceType {
    Skill,
    SlashCommand,
    AgentDefinition,
    SharedScripts,
}

pub struct Resource {
    name: String,
    kind: ResourceType,

}

```
