# AI Pipeliner
> A library (and service) to provide

This library intends to provide strong AI/LLM pipelining capabilities to callers by providing a set of high-level and composable primitives for building pipelines.

## Packages

1. AI Pipeliner Library [`./lib`]

     The core of **AI Pipeliner** is the library. It is meant for other Rust projects to call into directly to setup and execute AI pipelines.

     > **Note:** this library leverages the `rig` crate and other crates in the **rig** ecosystem to help it provided consistent utility across underlying model providers. For more information on the integration see: [Rig Integration](./docs/rig-integration.md) document.

2. Model Generator [`./gen`]

    A code generator module which will generate enumerations for all of the providers we provide support for in the `ai-pipeline` package.

    When run, the enumerations will be placed in the `./lib` package to be used. The _primary_ user of these enumerations is the `ProviderModel` enumeration (`lib/src/rigging/models/mod.rs`) which wraps all of the individual provider's models into a single enumeration which provides access to all models across all

3. AI Pipeliner TUI Components [`./tui`] - FUTURE

    A set of TUI components which can be used in CLI programs to provide richer Human in the Loop experiences.

4. AI Pipeliner CLI [`./cli`] - FUTURE

     A really simple CLI interfaces which leverages the pipeliner library as well as the pipeliner TUI components for all the heavy lifting.

5. AI Pipeliner Service [`./server`] - FUTURE

     A server which provides:

    - a REST based interface to process pipelines
    - a gRPC based interface to process pipelines

## Primitives

The primitives this library provides will be broken out in the following sections with links in these sections for still further details on specific symbols which play a significant role in how this library works.

We will now discuss the key primitives which this library exposes:

- First we'll cover "Atomic Operations" which have act as a single transaction. Examples include:
    - describing a prompt which will be passed to an LLM (with `Prompt`)
    - wrapping a future call to an Agentic CLI tool like **Claude Code** (with `AgentDelegation`)
    - or even the most basic, loading in user content (with UserContent)
- Then we'll look at how atomic operations can be _chained_ and fanned out for _concurrency_.

### Atomic Operations

#### Prompt

- generating prompts is done with the [`Prompt` struct](./docs/prompt.md)
- the key areas a prompt can have a point of view include:
    - Prompt content (text, image, audio)
    - Which model or model(s) should be used (one model per modality)
    - What [tools](./docs/tools.md) the model should have when it's called
- to have any utility, a prompt must have some form of "prompt content", all other expressions for a Prompt are optional.
- for more on **Tools** refer to the [Tools Documentation](./docs/tools.md)

##### Example

```rust
let all_about_apples = Prompt::new(
  "Tell me all about apples. What kinds of apples are there? What are the most popular kinds of apples? Who is a famous person who was known to like apples?"
)
  .with_model(Model::Fast)
  .with_tools([Tool::WebSearch, Tool::WebScrape]);
```

In this example we've created a text prompt, described the _kind_ of model we want to handle the Text modality (aka, a fast one), and we've said that when this prompt is used it should be given access to the `WebSearch` tool.

#### UserContent

Sometimes we just want to insert some content into a pipeline. This content doesn't need to directly interact with an LLM or

### Interactions (Human in the Loop)

I know we all keep hearing about the plight of humans as AI becomes more and more capable but for now at least those humans can be darn useful. In order for them to be brought in at the right time we provide the [`Ask` struct](./docs/ask.md).

### Example

```rust
let question = Ask::new(
  "username", // name for State
  "What would like the **username** to be?"
).from_list(["bob", "bobs-your-uncle", "oh-bob"]);
```



### Agent Delegation


### Outputs

the expected output from your pipeline is shaped by the [`OutputStructure`](./docs/OutputStructure.md) enumeration and contains variants such as:

- String
- Document
- Image
- Audio
- JsonStructure


### Pipeline Operators

- Chaining
    - chain(a,b,c)
- **Parallelism / Concurrency**
    - `Parallel` struct

        - the `Parallel` struct struct is the base primitive for executing prompts or embeddings in parallel
        -

    - compose()
    -

### Models, Embeddings, and Providers

#### Models

- the [`Model`](./docs/Model.ts) enumeration is an _abstract_ representation of how capable and performant you want the model to be.
- variants include:
    - `Model::Fast`,
    - `Model::FastThinking`,
    - `Model::Normal`,
    - `Model::NormalThinking`,
    - `Model::Smart`
    - `Model::SmartThinking`,
    - and many more
- the [`Model`](./docs/Model.ts) **IS NOT** a reference to a specific model being hosted by a particular provider; That is a [`ProviderModel`](./docs/ProviderModel.md).
- In almost all cases, callers of this library should avoid troubling themselves with these details of `ProviderModel` and just use the `Model` struct to hint at what you want.

> **Note:** for more on how we move from abstract models like those defined by `Model` to concrete models found in `ProviderModel` you should read the [From Abstract to Concrete Models](./docs/abstract-to-concrete-models.md) document.

#### Embeddings

Embedding models are a bit different then generative models in that you MUST ensure you have the same model to encode and decode. Therefore while some abstractions regarding the providers of these models exist, the abstraction is less valuable then how we're able to approach generative models.


- the [`Embeddings`](./docs/embeddings.md) enumeration helps you define the specific embedding algorithm you want to have used (but not the provider)

#### Providers

This library supports the following providers:

- Direct Providers:
    - OpenAI
    - Anthropic
    - Deepseek
    - Cohere
    - Azure OpenAI
    - Perplexity
    - Google Gemini
    - DeepSeek
    - xAI
    - Z.ai (custom provider)
- Aggregators:
    - OpenRouter
    - ZenMux (custom provider)

For now the number of provider this library supports is static but we may add in the ability for the caller to extend this.



## Example Usage

### Via the Library

```rust
use ai_pipeliner::{Prompt,Pipeline,Concurrent,State,Document,Model};
use color_eyre::{Result};

[tokio::async]
fn main() -> Result<String> {

   let apple = Prompt::new(
      "What is an apple? What are the major types of apples? What makes apple's so delicious?",
      Model::Fast
    );

    let pair = Prompt::new(
      "What is a pair? What are the major types of pairs? What distinguishes the types of pairs? What makes pair's so delicious?"
    )

    // Creates a single Markdown document by:
    //
    // 1. Cleaning both response documents to be well formed
    //    markdown documents
    // 2. SmartPush the headings of both documents to have a TOC which
    //    starts with a single H2 heading
    // 3. Uses the Model type provided to create the correct H1 name
    //    for this combined document
    // 4.
    let fruits = Compose([apple, pair], Model::Fast);

   let state = State::new(HashMap {
     fruits: "string[]",

})


}
```


## Interaction with Rig
