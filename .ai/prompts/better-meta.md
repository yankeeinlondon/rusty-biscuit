# Better Meta

This feature request involves improving the metadata capture in the Research Area.

## Custom Providers

### Move Z.ai Provider to Shared Library

- The Z.ai custom provider is currently defined in the Research Area's Library module
- **Action**: Move the provider implementation to `shared/src/providers/`
- **Cleanup**: Delete the original implementation from research area after moving (no re-export needed)

### Add ZenMux Custom Provider

- [**ZenMux**](https://zenmux.ai) is an aggregation provider with metadata defined in `@shared/src/providers/` but no rig implementation
- **Action**: Create a custom rig provider for ZenMux
- **Authentication**: Bearer token (standard `Authorization: Bearer <key>` header)
- **Implementation**: Use `PROVIDER_AUTH`, `PROVIDER_BASE_URLS`, `PROVIDER_ENV_VARIABLES` from the providers module

## Library Area

1. **Per Type Metadata**
    - today we structure our metadata capture with only the 'library' type of research in mind
    - as we grown into different types of research we need to have a type structure that can grown with us
    - The `ResearchMetadata` struct consists of both:
        - properties which make sense for ALL research types, and
        - properties which are 'library' specific
    - We will reduce the `ResearchMetadata` struct to just core properties which apply equally to _any_ type of research
    - We will then add a `details` property in `ResearchMetadata` which will point to an enumeration `ResearchDetails` which will have variants for each `ResearchKind`.

    > Treat the code below as a rough draft. You are free to mutate this as you see fit but hopefully it can act as a starting point to the design work that must be done.

    **Implementation Notes:**
    - All detail structs should be defined as empty unit structs initially (e.g., `pub struct PersonDetails;`)
    - Fields will be added to each struct as that research type is implemented
    - This allows the enum to be complete while deferring detailed design

    ```rust
    pub enum ResearchDetails {
        /// `library` research involves information on _library software_
        /// which was written to be _included_ or _composed_ into an
        /// application (or another library) by a programmer.
        Library(LibraryDetails),

        /// Research which describes a kind of problem that software
        /// libraries or applications might try and solve for. Things
        /// like "Concurrency in Rust" might be a topic you'd find here.
        SolutionSpace(SolutionSpaceDetails),

        /// `cli` is software that is packaged up as a CLI for use.
        /// The information researched for CLI software involves the
        /// usage of the CLI, what commands/subcommands exist, what
        /// flags or switches exist, what structured and unstructured
        /// outputs are output and whether the structured outputs
        /// are "typed".
        Cli(CliDetails),

        /// Software bundled together as an executable application
        /// (but not a CLI).
        App(AppDetails),

        /// Research into the services and software that a cloud provider
        /// is providing to it's users.
        CloudProvider(CloudProviderDetails),

        /// Research around a recognized standard. The "kind" of
        /// standard could be anything (network standard, software
        /// standard, education standard, regulatory standard, etc.)
        Standard(StandardDetails),

        /// Research into a particular API. How is it defined?
        /// Does it use one or more standards for schema definition?
        /// Does it fit into the REST definition? Is it more of a RPC
        /// based API? What is the API used for?
        Api(ApiDetails),

        /// This involves research into a particular person,
        /// regardless of whether the person is a known friend,
        /// family member, someone famous, etc.
        Person(PersonDetails),

        /// Research involving a "group" of people. Could have topics
        /// like `US Senators`, `Musical Artists`, etc.
        People(PeopleDetails),

        /// Research into a particular place.
        Place(PlaceDetails),

        /// Research into a product for sale.
        Product(ProductDetails),

        Company(CompanyDetails),

        CompanyCategory(CompanyCategoryDetails),

        News(NewsDetails),

        SkillSet(SkillSetDetails),
    }

    // Initially all detail structs are empty unit structs:
    pub struct LibraryDetails;      // Will get fields when refactoring existing library metadata
    pub struct SolutionSpaceDetails;
    pub struct CliDetails;
    pub struct AppDetails;
    pub struct CloudProviderDetails;
    pub struct StandardDetails;
    pub struct ApiDetails;          // Will get fields when implementing `research api` command
    pub struct PersonDetails;
    pub struct PeopleDetails;
    pub struct PlaceDetails;
    pub struct ProductDetails;
    pub struct CompanyDetails;
    pub struct CompanyCategoryDetails;
    pub struct NewsDetails;
    pub struct SkillSetDetails;

    /// Allows for a set of event driven scenarios
    /// which will determine whether content should
    /// be considered expired.
    pub enum ExpiryEvent {
        /// A new major version of the software was released
        NewMajorVersion,
        /// The software/API has been officially deprecated
        DeprecationAnnounced,
        /// A security advisory has been issued
        SecurityAdvisory,
    }

    impl ExpiryEvent {
        /// Check if the expiry condition has been triggered
        pub fn check(&self, datetime: DateTime<Utc>) -> bool {
            // Implementation will query relevant sources based on event type
            todo!()
        }
    }

    /// Provides a means to describe how long content is
    /// considered valid before it needs to refreshed.
    pub enum ExpiryPolicy {
        /// expires after a number of days
        Days(u32),
        /// expires after a number of months
        Months(u32),
        /// expires after a number of years
        Years(u32),
        /// content should be long lasting; never expires
        Evergreen,
        /// when content renewal is easy to refresh and
        /// the underlying data is not static this policy
        /// will ensure that whenever content is refreshed
        /// that this document will be included in the 
        /// refresh process
        Always,
        EventDriven(ExpiryEvent)
    }

    /// How the research content was generated/sourced.
    /// Simplified to three variants with common metadata.
    pub enum ResearchSourcing {
        /// Content was generated by an LLM prompt
        LlmGenerated {
            /// The prompt used to generate the content
            prompt: String,
            /// The model that generated the response
            model: Model,
            /// When the content was generated
            generated_at: DateTime<Utc>,
        },

        /// Content was derived from an external API
        ApiDerived {
            /// Name/identifier of the API source
            api_name: String,
            /// The endpoint or query used
            source_url: Option<String>,
            /// When the data was fetched
            fetched_at: DateTime<Utc>,
        },

        /// Content was manually provided by the user
        UserProvided {
            /// When the content was added
            added_at: DateTime<Utc>,
        },
    }

    /// Research documents produced with knowledge which
    /// will eventually be fed into final deliverables
    /// but in isolation are not a "final product"
    pub struct UnderlyingResearch {
        /// The filename for the research (usually without
        /// any directory structure but if the file is in
        /// a subdirectory of the root folder for that research
        /// then you can provide a relative path to it)
        pub filename: String,
        /// The frontmatter properties which MUST be present
        /// in this document to be considered "valid"
        pub required_frontmatter: Option<Vec<String>>,
        /// How was this content created (prompt, user provided, etc.)
        pub sourcing: ResearchSourcing,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub expires_at: Option<DateTime<Utc>>,
    }

    pub struct ResearchMetadata {
        pub schema_version: u32,
        /// The kind of research
        pub kind: ResearchKind,

        /// the underlying research documents produced while 
        /// doing research
        pub underlying_research: Vec<UnderlyingResearch>,

        /// Details which are specific to the _kind_ of research
        /// which is being done.
        pub details: ResearchDetails,

    }

    ```

    - since there is a `schema_version` property we will be moving from 0 to 1

2. **Schema Migration (v0 → v1)**
    - **Strategy**: Auto-upgrade on read
    - When reading a `metadata.json` with `schema_version: 0` (or missing version):
        - Automatically convert to v1 format in memory
        - Save the upgraded version back to disk
    - This ensures seamless transition without requiring user intervention
    - Add `fn migrate_v0_to_v1(old: MetadataV0) -> ResearchMetadata` function

3. Ensure the `library` command works correctly after the refactoring from #1 and #2.

4. **Add `api` Command**
    - Add a new command to the Research CLI: `research api <api-name> [...prompt]`
    - This adds the `Api` research type
    - Structurally similar to the `library` command
    - Will populate `ApiDetails` struct (initially empty, fields added as needed)

5. **Rename `tool` to `allowed_tools` (Breaking Change)**
    - The `SkillFrontmatter` structure incorrectly defines a `tool` property
    - **Action**: Rename to `allowed_tools` (no deprecation period)
    - Update all references in:
        - Rust code (struct definitions, field accesses)
        - Documentation files
        - Any existing skill files that use this property
