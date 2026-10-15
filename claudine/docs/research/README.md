---
content-policy:
    - SubDirectories
---
# Research Folders

This README's responsibility is to describe the focus/responsibility of the various folders under _research_.

```mermaid
mindmap
    root((Areas))
        Provider(Provider\nPlatforms)
        Local(Local AI\nRunners)
        Industry(Industry\nTopics)
```

## Provider Based Fleet Research

Provider based fleet research is what a majority of the research in Claudine is focused on. This research digs into the details of all the [supported Agentic CLI's](^claudine/docs/provider.yaml) that Claudine is responsible for supporting.

We provide both **primary** research topics as well as buckets of **summary** research where:

- the **primary** research gathers information for a specific _topic_ on a specific _provider_
- the **summary** research _aggregates_ research by either:
    - the provider
    - the topic

### Primary Research Topics

```mermaid
mindmap 
    root((Research))
        Shared Assets((Shared\nAssets))
            Prompts
            Agent Skills
            Hooks
            Sub-Agent Defn
        Interface((Interface))
            CLI
            API
        Handling((Handling))
            Logging
            Errors
            Non-Interactive
        Vendor((Vendor))
            Desktop App
            Plans
            Caching
        Config((Config))
            Models
            Reasoning Lvl
            Permissions
            System Prompt
        Capabilities((Capabilities))
            Resumption
            Steering
            Sandboxing
            Decision Models
            Memory Systems
```

#### ACP


#### CLI


#### Errors


#### Logging

#### Model Configuration

Addresses the configuration files that a provider has and how the different _scopes_ (e.g., typically **user** and **repo**) interact. It also tackles **which** LLM models are provided to a user "out of the box" as well as how a user can configure local LLM's to use with the platform.

#### Permissions

Focuses on the 

#### Hooks


#### Slash Commands / Prompts


#### Agent Skills



#### Reasoning Levels


#### Prompt Resumption


#### Prompt Steering


### Cross Provider Research


## Local Runner Research

Another area of interest for Claudine is how to integrate well into locally run LLM solutions.
