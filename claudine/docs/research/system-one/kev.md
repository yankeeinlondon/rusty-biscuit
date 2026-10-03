---
$schema: 
    website: string -> the website for the model
    repo: string -> the repo for the model
    company: string -> the company name
    company_url: string -> the company's website
    license: enum(commercial,open-source, apache-2)
license: open-source
repo: https://github.com/jaredpalmer/kev
website: https://github.com/jaredpalmer/kev
prompt: |-
    Kev is an open source variant of [Jev](https://typesafe.ai/). Your task is to research and report on this model.

    Make sure to:

    - describe what Kev is and what distinguishes this kind of model
        - how does it compare to an LLM?
        - how does it compare to [Jev](https://typesafe.ai/)
    - what does it do well? what does it not do well?
    - what kind of API surface is there? What SDK's exist for various languages. How does one authenticate?
    - what local runners can you run this model with?
    - give an example of the API surface exposed
    - what are the question types you can ask?


    Once you're done writing your research in prose, we need you to fill in the following Frontmatter properties too (note: if property is already set then just verify):

    - set `website` to the best URL for referencing this model
    - if there is a repo available, set `repo` to the URL to the repo
    - if there is a clear company behind the model then set `company` to the companies name; set `company_url` to the companies URL
    - set the `license` property to "commercial" or "open-source"
hash: 2fa3fd1a0620a6f8-9c40e505c15d5b76
last_updated: 2026-09-28
---
# Kev: an open System One decision model

## What Kev is

[Kev](https://github.com/jaredpalmer/kev) is a family of open-weight models by Jared Palmer that turns a supplied piece of context (the **state**) and a set of typed questions into structured decisions and probabilities. The current family includes 0.8B, 4B, and 9B checkpoints fine-tuned from Qwen3.5 base models, plus a 27B checkpoint based on Qwen3.8. The smaller checkpoints use a LoRA adapter and a pointer head trained to score the supplied options. Rather than generate a sentence and parse it, Kev returns the selected value and a probability distribution. All questions in a request are evaluated independently against the same state.

This is the System One pattern: a narrow, machine-facing decision primitive for classification, routing, rubric scoring, and yes/no judgments. It complements a general-purpose LLM; it is not a replacement for open-ended conversation, explanations, code generation, or multi-step reasoning. An application can use an LLM to propose or explain, Kev to make small bounded judgments, and ordinary code to enforce policy and decide what confidence level warrants action. A typed result is structurally convenient, but does not guarantee that the underlying judgment is correct.

### Comparison with Jev and LLMs

Kev follows the same public System One request and response shape as TypeSafe AI's [Jev](https://docs.typesafe.ai/introduction): state plus named questions, with `choice`, `score`, and `noul` results. That wire compatibility lets the TypeSafe SDK target a self-hosted Kev endpoint by changing the base URL. Jev is TypeSafe's hosted model; Kev is an independently trained, downloadable alternative. Kev's repository says it did not train on Jev outputs. The implementation, checkpoints, and training recipe differ, so compatible requests do not mean equivalent behavior or identical accuracy.

The Kev maintainer's published comparison reports that Kev-4B and Kev-9B approach Jev on several classification-shaped evaluation sets, while Jev remains stronger on some knowledge questions and on the fraction of predictions that can be automated at a fixed error budget. These are maintainer-run evaluations with different training and evaluation data, not an independent, controlled proof of parity. Treat probability thresholds as estimates to validate on the target workload. In particular, confidence is not an accuracy percentage, and calibration fitted on development data can drift on new domains.

Unlike an autoregressive LLM chat endpoint, Kev scores a finite set of candidate answers and emits typed values rather than natural-language completions. Its strengths are low-latency batched judgments, predictable output shape, option-level probabilities, and the ability to run or fine-tune weights in your own environment. Its limits follow from that specialization: it cannot invent an answer outside the offered options (unless a suitable option is included), explain its reasoning, or reliably perform extended reasoning. The project reports lower performance on general-knowledge questions than Jev, sensitivity to option ordering, and reduced accuracy on long states; its training states were much shorter than the server's maximum accepted context. Fine-tuning for a domain can help, but requires representative labeled examples and a held-out evaluation.

## API, clients, and authentication

The included Python server exposes `POST /v1/systemone` and `GET /v1/models`; it also has diagnostic `/v1/systemone/permute` and `/v1/systemone/separate` routes. Requests contain `state`, a `model` identifier (default `kev-latest`), and a `questions` map. State and instructions can be strings or structured JSON. The response has keyed `answers`, token usage, and model latency. The endpoint is compatible with the TypeSafe System One protocol.

The official [TypeSafe Python SDK](https://docs.typesafe.ai/sdk/python) is the SDK documented by the Kev project and is included in its server setup; point its `base_url` at the local or deployed Kev endpoint. TypeSafe also publishes a [JavaScript SDK](https://docs.typesafe.ai/sdk/javascript), which can call the compatible HTTP surface. These are TypeSafe SDKs, not Kev-maintained language-specific SDKs. The protocol is ordinary JSON over HTTP, so other languages can use generated or handwritten HTTP clients, but the Kev repository does not document official Go, Rust, Java, or other native Kev SDKs.

By default, the local server binds to `127.0.0.1` and does not require a key. Set `KEV_API_KEY` in the server environment to require `Authorization: Bearer <key>` on `/v1/*`. For a network-facing deployment, the project documents a Modal-hosted option with the same bearer authentication; a self-hosted remote server should be placed behind an appropriately secured proxy. With the TypeSafe Python SDK, configure `api_key` and `base_url`; the key can be a placeholder for a local server without auth.

## Local execution

The project provides its own Python runner (`python -m kev.serve`). The documented acceleration paths are NVIDIA CUDA and AMD ROCm GPUs, and Apple Silicon through MLX. It downloads the adapter and base checkpoint on first use. Kev-0.8B is the lightest option; 4B and 9B have larger hardware needs, and the 27B model is documented for high-memory GPUs rather than Macs. PyTorch also provides a CPU execution path, though the project's performance guidance emphasizes GPU and Apple Silicon use. The repository does not document Ollama or llama.cpp as supported runners, so do not assume a GGUF or drop-in Ollama package exists. A browser-based Hugging Face Space is available for trying the model without installing it.

## Example request and response

This example asks three independent questions about one support ticket. It is adapted from the project's [API example](https://github.com/jaredpalmer/kev#quick-start):

```http
POST http://127.0.0.1:8009/v1/systemone
Content-Type: application/json

{
  "state": "The shoes arrived late and in the wrong size; my card was also charged twice.",
  "model": "kev-latest",
  "questions": {
    "team": {
      "type": "choice",
      "instructions": "Which team should handle this?",
      "criteria": {"returns": "Exchanges and wrong sizes", "billing": "Payment problems"}
    },
    "urgent": {"type": "noul", "instructions": "Does this need urgent human attention?"},
    "sentiment": {
      "type": "score",
      "instructions": "How upset is the customer?",
      "criteria": ["calm", "frustrated", "angry"]
    }
  }
}
```

An abbreviated response has this shape (the values vary by model and input):

```json
{
  "model": "kev-latest",
  "answers": {
    "team": {
      "type": "choice",
      "choice": "returns",
      "confidence": 0.42,
      "probabilities": {"returns": 0.71, "billing": 0.29}
    },
    "urgent": {"type": "noul", "noul": 0.36},
    "sentiment": {
      "type": "score",
      "score": 1.2,
      "confidence": 0.55,
      "legend": {"0": "calm", "1": "frustrated", "2": "angry"},
      "probabilities": {"0": 0.1, "1": 0.6, "2": 0.3}
    }
  },
  "usage": {"input_tokens": 54, "output_tokens": 113},
  "latency_ms": 24
}
```

The `choice` answer includes a winning option, per-option probabilities, and confidence. `noul` returns a probability that the yes statement is true. `score` returns the expected level index (starting at zero), per-level probabilities, a legend, and confidence. Kev does not generate a rationale or free-form completion.

## Question types

- **Choice** (`choice`): select among 1–255 named options, optionally with descriptions; returns the winning name, probabilities for each option, and confidence.
- **Score** (`score`): assess a state against 1–255 ordered rubric levels; returns an expected numeric level, probabilities by level, and confidence.
- **Yes/no** (`noul`): evaluate whether a proposition is true; returns a 0–1 probability for yes, with optional descriptions for true and false.

Questions work best when each asks one focused judgment. For a broad decision with multiple dimensions, ask separate questions and combine their outputs in application code. For API details and current model limitations, see the [Kev repository](https://github.com/jaredpalmer/kev) and [TypeSafe's primitive reference](https://docs.typesafe.ai/primitives).
