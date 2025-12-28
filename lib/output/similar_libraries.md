`rig-core` is a relatively new and highly ergonomic Rust library designed for building LLM-powered agents with a focus on structured outputs and type safety. The Rust ecosystem for LLMs is growing rapidly, with several libraries offering different trade-offs between "all-in-one" frameworks and "minimalist" abstractions.

Here are the primary comparable libraries to `rig-core`.

---

### 1. LangChain-Rust

**Description:** A comprehensive Rust implementation of the LangChain ecosystem. Like its Python/JS counterparts, it provides a wide array of tools for document loading, vector store integration, memory management, and complex chain construction.

* **Pros:**
  * Extremely feature-rich with dozens of pre-built integrations (Postgres, Qdrant, OpenAI, etc.).
  * Follows the familiar LangChain "Logic," making it easier for developers coming from Python.
  * Includes robust support for "Agents" and complex RAG (Retrieval Augmented Generation) workflows.
* **Cons:**
  * Can feel "un-idiomatic" to Rust purists due to heavy use of abstractions.
  * Steeper learning curve because of the sheer volume of traits and types.
* **Links:**
  * **Repository:** [github.com/mreadables/langchain-rust](https://github.com/mreadables/langchain-rust)
  * **Docs.rs:** [docs.rs/langchain-rust](https://docs.rs/langchain-rust/)
  * **Documentation Site:** [langchain-rust.vercel.app](https://langchain-rust.vercel.app/)

---

### 2. GenAI

**Description:** A high-level, multi-provider LLM client library. While `rig-core` focuses on agents and tools, `genai` focuses on providing a unified, clean interface for interacting with different model providers (OpenAI, Anthropic, Gemini, Groq, etc.) with minimal boilerplate.

* **Pros:**
  * Extremely lightweight and easy to integrate into existing projects.
  * Excellent "Common Layer" abstraction that makes switching between providers seamless.
  * Focuses on the "Request/Response" ergonomics rather than forcing a specific agent architecture.
* **Cons:**
  * Does not include high-level "Agent" or "Memory" abstractions out of the box.
  * Smaller feature set compared to Rig or LangChain for RAG pipelines.
* **Links:**
  * **Repository:** [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
  * **Docs.rs:** [docs.rs/genai](https://docs.rs/genai/)

---

### 3. Kalosm

**Description:** Part of the Floneum project, Kalosm is a framework for local-first LLMs, audio, and vision models. It is designed to simplify the process of running models locally (like Llama 3 or Mistral) and building agents around them.

* **Pros:**
  * Strong focus on **Local Inference** (no API keys required).
  * Includes utilities for not just text, but also voice (Whisper) and image generation.
  * Provides high-level "Context" and "Search" tools specifically optimized for local performance.
* **Cons:**
  * Heavier dependencies (requires compiling local inference engines like Candle).
  * Less focused on cloud-based API orchestration compared to Rig.
* **Links:**
  * **Repository:** [github.com/floneum/floneum](https://github.com/floneum/floneum)
  * **Docs.rs:** [docs.rs/kalosm](https://docs.rs/kalosm/latest/kalosm/)
  * **Documentation Site:** [kalosm.floneum.com](https://kalosm.floneum.com/)

---

### 4. LLM-Chain

**Description:** One of the earliest frameworks in the Rust LLM space. It allows for the creation of chains of LLM calls, providing a structured way to pass data from one prompt to the next.

* **Pros:**
  * Mature architecture with a focus on modularity.
  * Strong support for structured output via "Output Parsers."
  * Supports both cloud APIs and local models (via `llm` crate/llama.cpp).
* **Cons:**
  * Development has slowed down significantly compared to newer libraries like Rig.
  * The API can be verbose and complex for simple use cases.
* **Links:**
  * **Repository:** [github.com/sobelio/llm-chain](https://github.com/sobelio/llm-chain)
  * **Docs.rs:** [docs.rs/llm-chain](https://docs.rs/llm-chain/)
  * **Documentation Site:** [llm-chain.xyz](https://llm-chain.xyz/)

---

### 5. Ollama-Workflow (Part of the Ollama-rs ecosystem)

**Description:** While many use `async-openai` or `ollama-rs` for raw API calls, several wrapper libraries have emerged to handle "Workflow" or "Agent" logic. These are often used as more lightweight alternatives to Rig when targeting local Ollama instances.

* **Pros:**
  * Deeply integrated with the Ollama ecosystem.
  * Simplifies the process of local tool-calling.
* **Cons:**
  * Often lacks the multi-provider flexibility that Rig offers.
  * Feature set is usually narrower (focused on simple completion/chat).
* **Links:**
  * **Repository:** [github.com/otavio/ollama-rs](https://github.com/otavio/ollama-rs) (Primary client used to build these workflows)
  * **Docs.rs:** [docs.rs/ollama-rs](https://docs.rs/ollama-rs/)

### Summary Table: Which one to choose?

|Library|Best For|Philosophy|
|:------|:-------|:---------|
|**Rig-Core**|Modern Agents & Tool-calling|Minimalist, Type-safe, Ergonomic|
|**LangChain-Rust**|Enterprise/Complex RAG|Feature-heavy, Comprehensive|
|**GenAI**|Multi-model switching|Lightweight, Developer-centric|
|**Kalosm**|Local LLMs & Multimodal|Privacy-focused, Integrated runtime|
|**LLM-Chain**|Structured pipeline logic|Robust, Established modularity|