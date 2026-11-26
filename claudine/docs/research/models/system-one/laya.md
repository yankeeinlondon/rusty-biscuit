---
$schema: 
    website: string -> the website for the model
    repo: string -> the repo for the model
    company: string -> the company name
    company_url: string -> the company's website
    license: enum(commercial,open-source, apache-2)
license: apache-2
repo: https://github.com/NandhaKishorM/laya
website: https://nandhakishorm.github.io/laya/
company: ConvAI Innovations
company_url: https://convai.com/
prompt: |-
    Contrastive Language Model (CLM) is a new class of System One model trained with a contrastive learning objective that connects states and actions. Your task is to research and report on this model.

    Make sure to:

    - describe what CLM is and what distinguishes this kind of model
        - how does it compare to an LLM?
        - how does it compare to [Jev]()
    - what does it do well? what does it not do well?
    - what kind of API surface is there? What SDK's exist for various languages. How does one authenticate?
    - how to run this with vllm
        - can you run this under any other platform?
    - give an example of the API surface exposed
    - what are the question types you can ask?


    Once you're done writing your research in prose, we need you to fill in the following Frontmatter properties too (note: if property is already set then just verify):

    - set `website` to the best URL for referencing this model
    - if there is a repo available, set `repo` to the URL to the repo
    - if there is a clear company behind the model then set `company` to the companies name; set `company_url` to the companies URL
    - set the `license` property to "commercial" or "open-source"
hash: 3e129e45b3d69a2a-9fb701d128e71ddf
last_updated: 2026-09-28
---
## Overview

Laya is an open-weight, non-autoregressive **System One** decision model: give it a state (such as a support ticket, email, or structured record) and one or more typed questions, and it returns the decisions and probabilities directly. The current project offers English, multilingual, and typed-decision checkpoints plus a router that selects a checkpoint using the input language. The model does not write a natural-language answer token by token. Its encoder represents the state and question options, then decision heads produce the requested result in a single inference pass. This makes it suitable for fast classification, routing, scoring, and guardrail decisions, but not for general conversation or content generation. [Project documentation](https://nandhakishorm.github.io/laya/) · [Model hub](https://huggingface.co/convaiinnovations/laya)

The idea overlaps with an LLM in that both interpret language, but their interfaces and jobs differ. An LLM generates a sequence of tokens and can explain, summarize, or compose text; Laya evaluates a fixed decision schema and returns typed values. Its output is easier to consume without parsing generated prose and typically much faster, but it cannot invent an answer format or handle an open-ended request outside the questions supplied. Laya's authors describe the architecture as RLCD (reinforcement learning for calibrated decisions) over an encoder model. This is a project-specific training description, not a generally established model category.

### Comparison with Jev

Jev is TypeSafe's hosted, closed-weight System One service. Laya is an open-weight alternative with a local Python runtime, optional self-hosted HTTP service, and a wire-compatible `POST /v1/systemone` endpoint. Both expose `choice`, `score`, and `noul` questions, making many Jev clients reusable by pointing their base URL at Laya. Jev removes model hosting and hardware management; Laya offers local control, model fine-tuning, and no per-request hosted-model fee, at the cost of operating the model and validating its behavior on your own data. Laya's published benchmark reports show fast inference and competitive results on selected tasks, but also report weak zero-shot results for some tasks, degradation for large choice sets, and sensitivity to checkpoint and calibration. These are the project's own measurements, not a controlled universal ranking; its benchmark page documents the datasets and caveats. [Laya benchmarks](https://nandhakishorm.github.io/laya/benchmarks/) · [Jev API reference](https://typesafe-jev.com/en/guides/api/)

## Strengths and limitations

Laya's key strengths are low-latency structured decisions, parallel evaluation of multiple questions about one state, confidence/probability outputs, local deployment, and multilingual routing. It supports fine-tuning for a domain and exposes prediction hooks for auditing, redaction, caching, metrics, and confidence-based escalation. The repository reports a 32.8 ms single-question median on a Tesla T4 for its multilingual checkpoint; actual latency depends on hardware, batch size, checkpoint, and startup/loading costs. [Benchmarks and known limits](https://nandhakishorm.github.io/laya/benchmarks/)

It is not an all-purpose LLM. The stock context windows are limited (512 tokens for the English checkpoint and 1,024 for multilingual, with longer multilingual contexts configurable), and the model does not generate explanations or arbitrary text. The project documents weaker performance on ordinal scoring, a known `noul` label-sensitivity issue in the English checkpoint, and a sharp drop when a choice has many options because the decision head has a limited token budget per option. Use the router for mixed-language data, evaluate on representative labeled examples, calibrate where appropriate, and fall back to another system or a human for uncertain cases. Do not interpret a model confidence value as a guarantee of correctness.

## API, clients, and authentication

The Python package is the primary SDK (`pip install laya`, import `laya`); it includes `Agent`/`load`, `Router`, and batch prediction. The project also publishes `laya-ts/` for TypeScript/JavaScript (Node and browser), provides an optional MCP server (`pip install "laya[mcp]"`), and offers LangChain/LangGraph integrations. For other languages, use the JSON HTTP API or any client compatible with the Jev/TypeSafe wire contract. [Repository and SDK details](https://github.com/NandhaKishorM/laya)

Self-host the API with `pip install "laya[serve]"` and `laya-serve`; it serves `POST /v1/systemone`, `GET /v1/models`, and `GET /health`. By default the local server does not require a key. Set `LAYA_API_KEY` to require `Authorization: Bearer <key>`. Checkpoint downloads from Hugging Face are public and need no token; a Hugging Face token can be supplied if using gated/private assets. Do not expose an unauthenticated server beyond a trusted local network. The compatible hosted Jev service instead requires a TypeSafe API key.

### Example request

```sh
curl http://localhost:8000/v1/systemone \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer your-local-key' \
  -d '{
    "state": {"subject": "Duplicate invoice charge", "body": "Please refund us."},
    "questions": {
      "department": {"type": "choice", "instructions": "Which team should handle this?", "criteria": {"billing": "Invoices, charges, refunds", "technical": "Bugs and outages"}},
      "urgent": {"type": "noul", "instructions": "Does this need urgent attention?"},
      "priority": {"type": "score", "instructions": "How urgent is it?", "criteria": ["Routine", "Soon", "Immediate"]}
    }
  }'
```

The response's `answers.department.choice` is the selected option and includes probabilities/confidence; `answers.urgent.noul` is the probability of true; and `answers.priority.score` is the expected rubric index. The `usage` object reports token counts. Omit the authorization header when the self-hosted server has no `LAYA_API_KEY` configured.

### Question types

- **`choice`** selects one item from named alternatives, each optionally described in `criteria`.
- **`score`** places the state on an ordered scale described by at least two `criteria` levels; the numeric answer is an expected level index.
- **`noul`** (no/yes, or boolean) estimates the probability that a statement is true.

Each question supplies natural-language `instructions`; a single request can ask several questions about one state. The API is for structured judgments, not conversational follow-up or free-form answer generation.

## Running Laya and vLLM

The documented Laya deployment does **not** use vLLM to serve the model. Install Laya and run its PyTorch inference runtime directly, or use `laya-serve` for HTTP. For a local CPU server, install the serving extra and launch `laya-serve`; configure `LAYA_DEVICE=cpu` as needed. For an NVIDIA GPU, install the matching CUDA PyTorch build and use `LAYA_DEVICE=cuda`. The project also documents Docker/Compose and Nix/NixOS deployment. It supports Apple Silicon through PyTorch's available MPS path, Intel XPU where a compatible PyTorch build is installed, and ONNX Runtime CPU inference through `ONNXAgent`. The project documents native macOS, Linux, and Windows installation. [Self-hosting](https://github.com/NandhaKishorM/laya#self-hosting-http-server-jev-compatible) · [Docker guide](https://nandhakishorm.github.io/laya/docker/)

Although Laya and CLM are both System One decision models, do not apply CLM's vLLM pooling-server instructions to Laya: those instructions start a Qwen embedding service for CLM's contrastive heads. Laya's published runtime uses its own encoder and decision-head implementation. A custom vLLM integration may be possible, but the Laya project does not document or support that as its serving recipe; use the Laya runtime/API unless you build and validate an adapter yourself.

## Source and licensing

The source repository is [NandhaKishorM/laya](https://github.com/NandhaKishorM/laya), and the released checkpoint family is published by ConvAI Innovations on Hugging Face. The repository and model weights are Apache-2.0 licensed. The project identifies Nandakishor M as founder and CEO of ConvAI Innovations. [License and project](https://github.com/NandhaKishorM/laya) · [ConvAI Innovations](https://convai.com/)
