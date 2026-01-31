# Unchained AI
> A library (and service) to provide AI/LLM pipelining capabilities

This library intends to provide strong AI/LLM pipelining capabilities to callers by providing a set of high-level and composable primitives for building pipelines.

## Packages

1. Unchained AI Library [`./lib`]

     The core of **Unchained AI** is the library. It is meant for other Rust projects to call into directly to setup and execute AI pipelines.

     > **Note:** this library leverages the `rig` crate and other crates in the **rig** ecosystem to help it provided consistent utility across underlying model providers. For more information on the integration see: [Rig Integration](./docs/rig-integration.md) document.

2. Model Generator [`./gen`]

    A code generator module which will generate enumerations for all of the providers we provide support for in the `unchained-ai` package.

    When run, the enumerations will be placed in the `./lib` package to be used. The _primary_ user of these enumerations is the `ProviderModel` enumeration (`lib/src/rigging/models/mod.rs`) which wraps all of the individual provider's models into a single enumeration which provides access to all models across all

3. Unchained AI TUI Components [`./tui`] - FUTURE

    A set of TUI components which can be used in CLI programs to provide richer Human in the Loop experiences.

4. Unchained AI CLI [`./cli`] - FUTURE

     A really simple CLI interfaces which leverages the unchained-ai library as well as the TUI components for all the heavy lifting.

5. Unchained AI Service [`./server`] - FUTURE

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

> **Note:** all primitives are lazy by nature and must implement the `Runnable` trait with `execute(state) -> Result<Output, StepError>`. Agent delegation primitives should also implement `AgentDelegation` to expose `is_interactive` for interactive vs headless execution.

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

Agentic delegation operations allow the pipeline's **state** and a **prompt** to be passed into agentic software. The interaction with this software can be either via an interactive CLI session (human-in-loop) or via a non-interactive print/headless run. In both cases the pipeline should:

1. Serialize State into a JSON object.
2. Provide JSON Schema for the state (and expected output) plus a short instruction on how the agent should read/write state.

#### `AgentDelegation` trait

Agent delegation primitives should extend `Runnable` with an explicit interactivity signal:

```rust
trait AgentDelegation: Runnable {
    fn is_interactive(&self) -> bool;
}
```

The `is_interactive` flag guides whether the pipeline launches an interactive REPL (human-in-loop) or a print/headless run for the agent.

#### `ClaudeCode` CLI

Claude Code supports two useful modes for delegation.

##### Interactive mode (human-in-loop)

- `claude "initial prompt"` starts a REPL seeded with the initial prompt (use `claude` alone for an empty REPL).
- The user can answer follow-up questions directly in the terminal.
- Built-in commands and skills are only available in interactive mode.
- Exit with `/exit` or `Ctrl+D`.

To capture a machine-readable result after the user exits, run a finalization call that continues the same session and requests structured output:

```bash
claude -p "Return the final output as JSON matching the schema." \
  --continue \
  --output-format json \
  --json-schema '<schema>'
```

The JSON response includes `structured_output` (when `--json-schema` is used) or `result` (plain text), plus a `session_id` for audit/logging.

##### Print/headless mode (automation)

- `claude -p "<prompt>"` runs once and exits.
- `--output-format json` returns `result` plus `session_id`; `--output-format stream-json` emits newline-delimited JSON for streaming.
- `--include-partial-messages` adds partial streaming events in `stream-json` mode.
- `--json-schema` validates and returns structured output in `structured_output`.
- `--continue` / `--resume <session_id>` continue a conversation in a later call.
- `--session-id <uuid>` lets the pipeline pre-assign a session ID.
- `--allowedTools` / `--tools` restrict tool access; `--permission-mode` or `--permission-prompt-tool` manage approvals in non-interactive runs.
- `--append-system-prompt` is the safest way to inject pipeline state and schema while keeping default Claude Code behavior.

##### State update strategy

- Default to schema-validated output (`--json-schema`) and read from `structured_output`.
- Treat `result` as a fallback only when schema validation is explicitly disabled.
- Update pipeline state using the same path as the `Prompt` result (string, document, or JSON structure depending on `OutputStructure`).

##### Default CLI invocation (recommended)

```bash
claude -p "${PROMPT_WITH_STATE}" \
  --append-system-prompt "Use the provided state JSON and schema." \
  --output-format json \
  --json-schema "${OUTPUT_SCHEMA}"
```


#### `OpenCode` CLI

OpenCode should mirror the same contract as Claude Code: initial prompt + state payload in, structured final output out. Use interactive mode for human-in-loop and a follow-up print mode call to capture structured output for state updates.

For OpenCode-specific details, see [OpenCode Agent Delegation](./docs/opencode-agent-delegation.md).



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
use unchained_ai::{Prompt,Pipeline,Concurrent,State,Document,Model};
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
