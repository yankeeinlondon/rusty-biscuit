---
$schema:
    website: string -> the website for the model
    repo: string -> the repo for the model
    company: string -> the company name
    company_url: string -> the company's website
    license: enum(commercial,open-source, apache-2)
license: apache-2
repo: https://github.com/Contrastive-LM/CLM
website: https://huggingface.co/Contrastive-LM/CLM-v0.1-8B
company: Contrastive-LM
company_url: https://github.com/Contrastive-LM
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


    Once you're done writing your research in prose, we need you to fill in the following Frontmatter properties too:

    - set `website` to the best URL for referencing this model
    - if there is a repo available, set `repo` to the URL to the repo
    - if there is a clear company behind the model then set `company` to the companies name; set `company_url` to the companies URL
hash: 1ac1c38ffaa47d65-6f57779d3edb6a82
last_updated: 2026-09-28
---
# CLM-v0.1-8B

## What CLM is

Contrastive Language Model (CLM) is an open-weight **System One** decision model: give it a textual state and candidate actions, and it scores the candidates instead of generating a response. CLM-8B uses a frozen Qwen3-8B encoder and two small learned projection heads, one for states and one for actions. Bidirectional InfoNCE training pulls observed state/action pairs together and pushes mismatched pairs apart. The published training recipe has three stages: roughly 60 million Nemotron Q&A pairs, 30 million synthetic hard negatives, and 1 million agent trajectories.

This is still built on a language-model backbone, but its deployed job differs from a generative LLM. An LLM produces a token sequence that an application must interpret; CLM embeds the state and candidate answers, computes similarity scores, and returns typed values and relative probabilities. It does not write explanations, invent candidates, or reason through arbitrary open-ended requests. Use an LLM for broad language generation and deliberation, and CLM for the small, repeated choices inside an agent or workflow.

## Comparison with Jev

Jev is TypeSafe AI's hosted System One decision model; CLM is an open-weight implementation of the same general typed-decision pattern. Both take a state plus Choice, Score, or yes/no (Noul) questions and return structured decisions. CLM's distinctive mechanism is explicit state/action contrastive scoring over supplied candidates; Jev is a hosted service whose underlying implementation is not published in the CLM materials. CLM can be self-hosted, fine-tuned, and cached locally, while Jev is accessed as a service. CLM's authors report parity with Jev on selected computer-use, gaming, and tool-calling evaluations, with up to 9x lower latency; this is a project-reported benchmark claim, not a general guarantee. They also report much faster candidate verification after task-specific head fine-tuning, which should not be confused with the zero-shot reference checkpoint.

## Strengths and limits

CLM is suited to classification, routing, rubric scoring, yes/no verification, selecting a tool or next action, and ranking a supplied set of candidate answers or trajectories. Many questions about one state can be submitted together. Candidate/action embeddings can be cached and reused across changing states, reducing repeat work, especially for large candidate sets. Only the relatively small projection heads need task fine-tuning.

The candidate set is the boundary of the decision: CLM cannot choose an answer that was not provided. Probabilities are normalized over that set, so they are relative scores, not automatically calibrated real-world confidence. It does not generate text. The reference head is tied to Qwen3-8B embeddings with last-token pooling, and the server truncates input at its configured token limit. The headline DeepSWE and Terminal-Bench results use fine-tuned verifier heads, not the zero-shot release. The initial release is text-only; vision and multimodal support are listed as future work.

## API, SDKs, and authentication

The repository provides the `contrastive-lm` Python package, including `CLMClient`, typed `Choice`, `Score`, and `Noul` question helpers, plus an in-process `Engine`. The server is a small HTTP API, so applications in other languages can call it with ordinary HTTP/JSON. The project does not document official language-specific SDKs beyond Python.

The main endpoint is `POST /v1/systemone`; `POST /v1/rank` exposes direct candidate ranking. `GET /v1/models` lists loaded heads, and `GET /health` reports server health/cache information. The web playground is served at `/` unless started with `--no-ui`. By default, the service is local and does not require a key. To protect a remotely reachable service, set `CLM_API_KEY`; clients then send `Authorization: Bearer <key>`. The Python client reads `CLM_BASE_URL` and `CLM_API_KEY`. CORS is disabled by default and should only be enabled when browser access is needed and the deployment is appropriately protected.

Example request:

```http
POST /v1/systemone
Content-Type: application/json
Authorization: Bearer YOUR_KEY

{
  "state": "A customer was charged twice and cannot reach support.",
  "questions": {
    "urgent": {"type": "noul", "instructions": "Is this urgent?"},
    "team": {
      "type": "choice",
      "instructions": "Which team should handle this?",
      "criteria": {"billing": "Charges and refunds", "technical": "Bugs and outages"}
    },
    "sentiment": {
      "type": "score",
      "instructions": "How upset is the customer?",
      "criteria": ["calm", "frustrated", "very angry"]
    }
  }
}
```

A response contains an answer per question: `noul` returns the probability that the statement is true; `choice` returns the selected key, confidence, and probability distribution; `score` returns an expected rubric-level index, confidence, legend, and probabilities. The API also accepts object or array states, rendered as prose rather than JSON for embedding.

## Running with vLLM and other platforms

The reference setup runs the Qwen3-8B encoder as a vLLM pooling server and the CLM API separately:

```sh
pip install contrastive-lm
vllm serve Qwen/Qwen3-8B \
  --served-model-name qwen3-8b \
  --runner pooling \
  --max-model-len 2048 \
  --port 8090
```

In another process, start `clm-serve` (default API port 8700). It downloads the roughly 75 MB reference head on first use. The encoder endpoint defaults to `http://127.0.0.1:8090/v1/embeddings`. Increase both vLLM's `--max-model-len` and CLM's `--max-tokens` together for longer states; this raises memory needs. The projection heads run on CUDA when available or CPU otherwise; `--device`/`CLM_DEVICE` can select the head device.

The documented turnkey encoder backend is vLLM. The Python engine accepts an embedding URL, so another server may be usable if it implements the expected OpenAI-style embeddings endpoint and produces embeddings compatible with the exact Qwen3-8B model and last-token pooling used to train the head. That is a compatibility route, not a separately documented or validated CLM platform integration. The small head/API can run on CPU, but the 8B encoder remains the main compute and memory requirement; the repository does not claim a native Windows, Apple MLX, or Ollama serving path.

## Question types

- **Noul**: a yes/no proposition, returned as the probability that it is true (optionally supply descriptions for true and false).
- **Choice**: select among named options, each optionally described; returns the winner and distribution.
- **Score**: place the state on an ordered rubric of at least two levels; returns the expected level and distribution.
- **Rank**: rank arbitrary free-form candidates against a context/question using `/v1/rank` or `Engine.rank`.

These are all decisions over text candidates. Prompts should define a clear state, ask for a discriminative judgment, and supply meaningful alternatives or rubric levels where the type requires them.

## Sources

- [CLM repository and API reference](https://github.com/Contrastive-LM/CLM)
- [CLM-v0.1-8B model card](https://huggingface.co/Contrastive-LM/CLM-v0.1-8B)
- [Jev / TypeSafe AI overview](https://www.jevtypesafe.org/)
