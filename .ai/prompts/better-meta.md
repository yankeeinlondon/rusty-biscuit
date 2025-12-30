# Better Meta

This feature request involves improving the metadata capture in the Research Area.

## Custom Providers

- We have defined a custom provider for **Z.ai** but it is currently defined in the Research Area's Library module
    - This needs to be moved to the shared library area so that everyone can benefit from the custom provider
- We have started to define properties about the [**ZenMux**](https://zenmux.ai) aggregation provider but we currently do not have any way of using it through our **rig** implementation because we do not have a custom provider defined for it
    - This needs to be added and should take advantage of the provider information metadata we've provided in @shared/src/providers/ 
    - consider using `PROVIDER_AUTH`, `PROVIDER_BASE_URLS`, `PROVIDER_ENV_VARIABLES` while implementing this custom provider

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
        Cli(SoftwareDetails),

        /// Software bundled together as an executable application 
        /// (but not a CLI).
        App(UtilityDetails),

        /// Research into the services and software that a cloud provider
        /// is providing to it's users.
        CloudProvider(InfraDetails),

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

    /// Allows for a set of event driven scenarios
    /// which will determine whether content should
    /// be considered expired.
    pub enum ExpiryEvent {
        // TODO
        // should include an implementation for a
        // `check(datetime)` function which will
        // perform the expiry check.
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

    pub struct ResearchSourcing {
        /// the research was driven by an LLM prompt;
        /// 
        /// - the prompt and the kind of module used
        ///   is also captured
        PromptDriven(String, Model),

        AlgorithmDriven(ExpirePolicy),

        ApiDriven(),
        /// the document was manually provided by the
        /// user.
        StaticallyProvided,

    }

    /// Research documents produced with knowledge which
    /// will eventually be fed into final deliverables 
    /// but in isolation are not a "final product"
    pub struct UnderlyingResearch {
        /// the filename for the research (usually without 
        /// any directory structure but if the file is in 
        /// a subdirectory of the root folder for that research
        /// then you can provide a relative path to it)
        filename: String;
        /// the frontmatter properties which MUST be present
        /// in this document to be considered "valid"
        required_frontmatter: Option<>
        /// How was this content created (prompt, user provided,
        /// etc.)
        sourcing: ResearchSourcing,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>
    }

    pub struct ResearchMetadata {
        pub schema_version: u32,
        /// The kind of research
        pub kind: ResearchKind,

        /// the underlying documents produced while doing
        /// research
        pub underlying_research: Vec<UnderlyingResearch>,

    }

    ```



2. the `SkillFrontmatter` structure defines a `tool` property but this is incorrect and should be called `allowed_tools`. Rename and make sure all references in code and docs reflects this change.
