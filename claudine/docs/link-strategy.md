# Link Strategy

The primary means for ensuring that things like "agent skills", "slash commands", "agent/subagent" definitions are portable across different CLI platforms is through some kind of symbolic linking strategy but there is nuance in this and we must be careful to get this right.

## Rules

The first big consideration is whether we're trying to resolve **User** scoped resources or **Repo** based resources.

- **Repo** resources always _override_ a similarly named resource from **User** scope but this is handled by the Agentic CLI itself and is less of a concern for us except for in some cases of reporting
- we will NEVER link a User scoped resource to a Repo scoped resource or vice-versa!
- symbolic links used to link resources at the **User** scoped level will always use fully qualified file paths
- symbolic links used to link resources at the **Repo** scoped level will always use relative file paths
- we will prefer linking individual resources rather than the entire directory structure for a given resource type as this provides greater flexibility for both Claudine and the user
- a "category-level symbolic link" means the entire resource root is linked (for example `.codex/skills -> .claude/skills`) instead of linking each resource asset individually
- if the user has created this category-level symbolic link already then we should look for ways in the CLI to encourage the user to remove this in favor of a more granular linking strategy
- under no circumstances are we to remove a symbolic link which we did not create without asking the user for permission

After we consider the **scope** we must also look at each of the **resource** types individually. These types are:

- Agent Skills
- Slash Commands
- Agent/Subagent Definitions
- Shared Scripts Directory

The key considerations here are,

- which platforms support the given resource type?
- do all platforms which support the given resource type do it in a way that is compatible?
    - first off, we need to have easy access in the Claudine library to what properties for each resource type are considered required and which are considered optional; this metadata will live in capabilities and be sourced from `claudine/docs/cross-referencing`
    - when we determine compatibility we need to make sure if there are variant "required" properties we can _map_ or in some manner ensure that all required properties will be satisfied for all platforms
- CORE ASSUMPTION and STRATEGY:
    - The Frontmatter key/value pairs may vary slightly between providers but no provider will reject a resource simply because it has _additional_ properties that it doesn't use;
    - For this reason, when we've identified the canonical source for a given resource we will always put it through an "upgrade" process to ensure that it contains as many key/value pairs as we can reasonably create to satisfy all provider's needs
        - There will likely be simple cases where one provider uses a different "key" name so all we need to do is duplicate the keys. Another simple example is that the `name` field was not provided but the name of a resource is always equal to the filename (no filepath or extension).
        - Where we can confidently fill in the missing properties we will do so but if there is any doubt we will NOT do this which will lead the canonical source to be categorized as a `IncompleteSource` instead of just a `Source`
    - canonical frontmatter upgrades are applied in place to the canonical source file(s)

## Base / Canonical Providers

To have an effective linking strategy we must determine which provider should be considered the "canonical" or "base" provider of resources (which other providers will link or derive off of). The User scope will have one canonical provider per resource type but each repo will also have a canonical provider assigned per resource type.

The process we'll use to determine this is somewhat automated but aided by requiring the user to specify which Agentic CLI's they prefer.

### Preference Assignment

When the `claudine init` command is executed, if the user's configuration does not yet specify the user's favorite Agentic CLI's then we will ask them to specify.

> **Note:** we will add a `preference` property to the user's configuration file which will be a vector of Provider's

- if the user only has ONE Agentic CLI installed then we can skip asking and we'll just assign the one they have installed as their favorite.
- If the user has TWO Agentic CLI's installed then we'll ask for their favorite and add that to the front of the list with the other installed app added afterward
- If the user has THREE agentic CLI's installed then we'll ask for their favorite and second favorite
- If the user has FOUR or more Agentic CLI's installed then we'll ask for their first, second, and third favorite and then add the other installed ones in alphabetical order.

> **Note:** if a user has already specified their preferences but then over time has installed more Agentic CLI's; they will be again asked for preferences if they run `claudine init` again. This is detectable because the number of preferences configured directly maps to the number of installed Agentic CLI's at the time the user last ran `claudine init`.

### Choosing the Canonical Provider

To choose a canonical provider we will iterate over the providers until we find a valid candidate.

- We will sort the providers first by the `preference` property in the user's configuration.
- All providers, for the specified scope (user,repo) which do NOT use the Markdown file format will be filtered out for canonical selection.
    - these providers are still eligible for synchronization through derived representations (`DerivedLink`, `DerivedStale`, `DerivedMissing`) when a converter exists.
- All providers, for the specified scope (user,repo) which have a symbolic link as their _entry point_ for the given resource type, will be filtered out

The Canonical Provider then is:

- the first provider which has one or more valid assets for the specified scope and resource type.
- if no providers meet the first criteria then we just choose the first provider in the list
- if the filtering process has left us with no provider choices then we CAN NOT currently provide a meaningful linking service.

### Saving the Canonical Provider

While in the future we may consider allowing the user to switch the canonical provider, to start we want to determine the canonical providers once and then have the provider saved to the user's configuration file.

- this approach both simplifies things while increasing performance (aka, recalculation of canonical providers on every operation can be avoided)
- currently, we only have a User based configuration file but we need to change this to have both a User and Repo based configuration file
    - Both User and Repo configuration files provide the same configuration structure but the Repo configuration file makes all properties optional _except_ the canonical provider `canonical_provider: Provider`
    - The Repo configuration overrides properties defined in the User scope but we do want the repo scope to explicitly set the `canonical_provider` property.
    - We also need to change the name of the configuration file to make it a more obvious association to Claudine:
        - instead of `.hooker` or `.hook-config` files
        - we will instead use `.claudine` configuration files
        - this will be an immediate switch with no backward compatibility because there are no current users of Claudine

## Repo Root Resolution

Repo scoped detection and linking operations must resolve the repository root from the current working directory by using the Sniff library.


## Resource Detection

The idea of **Resource Detection** is to evaluate both **user** and **repo** scopes (separately) for each of the resource types (skills, slash commands, etc.).

- Each resource type will provide their own struct and all structs will implement the `LinkDetector` trait
    - `Skills`
    - `SlashCommands`
    - `AgentDefinitions`
    - `SharedScripts`
- The internal state for these structs should be something like this (modify as needed to meet requirements):

    ```rust
    struct ExampleLinkDetector {
        /// the providers which support this resource type
        providers: Vec<Providers>,
        /// provides a lookup of all canonical resources of **user** scope and of the given resource type
        user_canonical_resources: HashMap<String, ResourceDefinition>,
        /// the list of ALL resource references of **user** scope for the given resource type
        user_resources: Vec<ResourceReference>,
        /// provides a lookup of all canonical resources of **user** scope and of the given resource type
        repo_canonical_resources: Option<HashMap<String, ResourceDefinition>>,
        /// the list of ALL resource references of **user** scope for the given resource type
        repo_resources: Option<Vec<ResourceReference>>,

        repo_base_path: Option<String>
    }
    ```


### Types

```rust

pub enum ResourceScope {
    /// a resource which is defined in the **Repo** scope (e.g., within
    /// the working git repo)
    Repo,
    /// a resource which is defined in both the **User** and **Repo** scopes
    /// but because **Repo** scope overrides the **User** scope, it's always
    /// the **Repo** resource which will be used
    RepoMasked,
    /// a resource which is defined in the **User** scope and available everywhere
    /// on the given host so long as the same user is logged in.
    User
}


/// Every `ResourceReference` will provide a `status()` function which reports
/// on the status which
pub enum ReferenceStatus {
    /// variants like `Source`, `Isolated`, `Link` and `DerivedLink` will
    /// be returned as **Ok** because they are able to fully meet their
    /// obligations to the reuse system.
    Ok,
    /// variants like `DerivedStale`, `DerivedMissing`, and `MissingLink`
    /// are all returned as **IsFixable** because while they are not
    /// in their ideal state, the Claudine CLI can fix them without
    /// the need for human intervention
    IsFixable,

    NeedsUserAttention
}

pub enum ResourceReference {
    /// this variant indicates that the resource in question is
    /// the "base" or canonical definition which will be used not
    /// only for the provider type but as the basis for all linked
    /// resources.
    Source(ResourceDefinition),
    /// this variant indicates that the resource in question is
    /// the "base" or canonical definition which will be used not
    /// only for the provider type but as the basis for all linked
    /// resources.
    ///
    /// Unlike the `Source` variant, however, this source is missing
    /// some required properties that some providers require for this
    /// resource to be used. The `PartialSource` will specify which
    /// of the required properties it is unable to fulfill and then
    /// other provider's who looks at this definition will need to
    /// determine if they can (`Link`) or cannot (`IncompleteLink`)
    /// provide a valid link to
    PartialSource(ResourceDefinition, Vec<String>),

    /// when we have a resource definition that is NOT from the base
    /// provider then we treat this as a one-off asset for this
    /// provider only. This also applies when multiple non-symlink
    /// candidates exist with different content and no valid canonical
    /// provider assets are available.
    Isolated(ResourceDefinition),

    /// This indicates that the provider has a symbolic link pointing
    /// to the canonical source. This can only be used when a provider's
    /// frontmatter properties are met for the resource type and the
    /// file format is the same format as the canonical source (e.g., TOML,
    /// Markdown, YAML)
    Link(Provider, ResourceScope),
    /// The provider is _able_ to link to the canonical source without
    /// the need for a conversion of formats but there is no symbolic
    /// link currently.
    LinkMissing(Provider, ResourceScope),
    /// The provider would ideally create a symbolic link to the
    /// canonical source because the provider shares the same file
    /// formatting as the canonical source, however, the canonical
    /// source is missing required properties that this provider needs
    /// so this link will NOT be added until the required properties
    /// are added.
    IncompleteLink(Provider, ResourceScope),

    /// This indicates that the provider has _derived_ what the asset
    /// should be for this provider from the canonical source but needed
    /// to convert the file format so instead of a symbolic link to the
    /// canonical source it is a _derived_ representation who's frontmatter
    /// and body hashes match the canonical provider's hashes.
    DerivedLink(Provider, ResourceScope),

    /// This indicates that the provider _can_ derive a valid representation
    /// of the canonical source by converting it's source but the current
    /// version of the canonical source does not match the current hashes
    /// so is considered out of date.
    DerivedStale(Provider, ResourceScope),

    /// This indicates that the provider is able to derive a valid representation
    /// of the canonical source by converting it's file format but currently
    /// this resource is NOT available for the given provider.
    DerivedMissing(Provider, ResourceScope),

}

impl ResourceReference {
    fn provider() -> Provider {
        todo!()
    },
    fn status() -> ReferenceStatus {
        todo!()
    }
}

/// The `ResourceDefinition` type is a part of the core
/// state of all of the
pub struct ResourceDefinition {
    name: Provider,
    provider: Provider,
    scope: ResourceScope,
    filepath: Path,
    frontmatter: Frontmatter,
    body: String,
    fm_hash: u32,
    body_hash: u32
}

/// A `ResourceType` enumerates all of the resources which
/// may be shared across Agentic CLI providers.
pub enum ResourceType {
    /// an Agentic Skill definition
    Skill,
    /// a repeatable prompt exposed as a Slash Command
    SlashCommand,
    /// a definition of an Agent or Subagent which an orchestrator might delegate to
    AgentDefinition,
    /// a grouping of executable scripts that are available to prompts without needing
    /// to specify a distinct filepath to the script.
    SharedScripts,
}

/// A `Resource` is a canonical representation of a sharable
/// asset that can be used cross Agentic CLI providers.
pub struct Resource {
    /// The **name** of the resource.
    name: String,
    /// The **kind** of resource this is (skill, slash command, etc.).
    kind: ResourceType,
    /// Whether this refers to the **user** or **repo** scope
    scope: ResourceScope,
    /// The provider this resource is for
    provider: Provider,

    /// Leverages the `ResourceDefinition` enumeration to define
    /// both the _metadata_ properties for the resource as well
    /// as the prose content of the body.
    definition: ResourceReference
}


/// All resources, regardless of the "base" provider
pub enum ResourceFormatConversion {
    /// serialize and deserialize to a YAML representation of the state
    /// for the given provider.
    Yaml,
    /// serialize and deserialize to a YAML representation of the state
    /// for the given provider.
    Toml,
    Bespoke((
      (in: String) -> String,
      (out: String) -> String
    ))
}

/// Allows the user to specify an informal preference
/// to the Agentic CLI's that they use. This will be configured
/// during the initialization stage (e.g., `claudine init`)
/// and this will be useful in identifying the canonical/base
/// providers for various resource types.
///
/// Note: the ordering ideally wouldn't matter if the user's behavior
/// of leveraging these reusable resources aligned to their preferred
/// Agentic CLI but this might not always be the case.
pub struct ProviderOrdering (Vec<Provider>);

impl ProviderOrdering {
    /// provides the user specified ordering preference
    /// for their Agentic CLI's which they use.
    ///
    /// Will return an error if the user hasn't yet
    /// initialized their configuration with preferences.
    pub fn new() -> Result<Self, ConfigurationError> {
        todo!()
    }
}


/// The `LinkDetector` trait is a contract that ensures that structs
/// given responsibility for managing a certain resource type will
/// expose their functionality in a consistent fashion.
pub trait LinkDetector {
    /// initiates the resource type and caches the state
    /// as internal properties to service other operations
    /// as well as serializing to disk.
    fn new() -> Self;


    /// Lists the providers which support this resource type
    fn supported_providers(self) -> Vec<Provider>;

    /// Provides the canonical provider for **User** scoped resources of the given resource type
    fn user_base(self) -> Provider;
    /// Provides the canonical provider for **Repo** scoped resources of the given resource type
    fn repo_base(self) -> Provider;

    fn list(self, filepath: Option<Path>) -> Vec<Resource>;
}

```



## Recommendations
Proceed incrementally, preserving the current safety characteristics while adopting the redesign in phases.

1. **Unify source of truth first**
   - Make linker path/discovery generation consume `linking::capabilities` instead of hardcoded provider lists.
   - Remove mismatches (for example command support/path differences between `capabilities.rs` and `paths.rs`).

2. **Ship practical linker upgrades before full redesign**
   - Add explicit apply mode (`--apply`) and scope selection (`--scope user|repo`) to CLI.
   - Keep dry-run as default for safety.
   - Expand active linking to all providers/resources that are already same-format and low risk.

3. **Implement canonical provider model directly**
   - Persist canonical provider decisions and user preferences in config.
   - Enforce canonical provider selection per `(scope, resource type)` in linker behavior.
   - Keep non-Markdown providers out of canonical selection, but include them in derived sync when converter support exists.

4. **Introduce frontmatter contract checks next**
   - Add required/optional contract metadata to capabilities (backed by cross-referencing research).
   - Parse frontmatter and classify `Source` vs `PartialSource`.
   - Apply canonical upgrade in place where mappings are deterministic (`name` from filename, key alias duplication, etc.).

5. **Defer conversion engine until contracts stabilize**
   - Implement conversion as separate module with explicit adapter tests.
   - Start with one high-value path (for example markdown command to TOML/YAML) before generalizing.

6. **Config migration plan**
   - Switch immediately to `.claudine` for both user and repo configs.
   - Do not implement backward compatibility reads for `.hooker`/`.hook-config`.

7. **Testing strategy**
   - Add integration fixtures for all providers and both scopes.
   - Add regression tests for canonical selection by `(scope, resource type)`, partially compatible frontmatter, and derived stale detection.
   - Add tests for repo-root detection using Sniff from nested working directories.
