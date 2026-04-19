# Permissions Engine

Understanding what permissions an Agent is giving to the filesystem and/or other tools is really important but unfortunately each Agent platform varies considerably in how and where they store this information, what CLI switches can modify permissions, and even which tools are available.

While this large variation makes making interacting with all these Agents more difficult it is also an important feature for Claudine. By Claudine understanding and interacting with all Agents the users of Claudine don't need to and can instead just use Claudine.

## The Engine

We need to create a Rust struct `PolicyEngine` which will centralize all of the policy based queries as well as mutations.

```rust
pub struct PolicyEngine {
    //...
}
```


## Queries

This struct will need to offer the user the ability to query a particular provider's policies through two lenses:

- based on configuration which exists in the filesystem
- based on _both_ the configuration files _and_ the CLI arguments passed in

The API would looks something like:

```rust
// provides a query surface which is cross-provider but not
// privy to any changes that may have happened due to the
// parameters 
let engine = PropertyEngine::new();

// the Claude Agent allows for read permissions to the given file
let read = engine(Provider::Claude).can_read("/path/to/file/foobar.md");
// the Codex Agent allows for read permissions to the given file
let read = engine(Provider::Codex).can_read("/path/to/file/foobar.md");

// once we provide the CLI parameters we can be more definitive on 
// permissions but at the same time our query surface becomes Agent 
// specific (in our example ... Claude specific)
let engine = engine.with_cli_params(Provider::Claude, Vec<String>)

// definitive 
engine.can_read("/path/to/file/foobar.md")
```

### Query Scope

We should be able to query the following things:

- **read** permissions to files or directories
- **write** permissions to files or directories
- **bash(cmd)** whether certain Bash commands can be run
- _what other permissions should we be querying for?_


## A Universal Policy

We should also be able to have `PolicyEngine` provide any caller a structured description of the security policy for a provider (or across providers). Rather than have it expressed in a provider-by-provider manner, `PolicyEngine` will have a canonical representation that it uses for all providers.


## Policy Mutation

In addition to checking what the policy is we want to be able to change the policy. This will involve a struct called `PolicyChange` where a `PolicyChange` represents a _proposed_ policy change.

```rust
// nothing has changed but we're proposing a permission be granted
let proposal = PolicyChange::new().grant_read_access("/some/file/path");

// this will execute the change request by changing Claude's configuration files
proposal.change_configuration(Provider::Claude);

// this would NOT change any configured permissions but instead indicate the
// CLI arguments to use to provide one-time access
let args = proposal.one_time(Provider::Claude);
```

