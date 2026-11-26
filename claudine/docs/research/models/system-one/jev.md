---
$schema: 
    website: string -> the website for the model
    repo: string -> the repo for the model
    company: string -> the company name
    company_url: string -> the company's website
    license: enum(commercial,open-source)
license: commercial
company: TypeSafe AI
company_url: https://typesafe.ai/
prompt: |-
    Typesafe AI splashed onto the scene in Sept of 2026 with it's **Jev** model. Your task is to research and document this model.

    Make sure to:

    - describe what Jev is and how it differs from an LLM
    - what does Jev do well? what does Jev not do well?
    - what kind of API surface is there? What SDK's exist for various languages. How does one authenticate?
    - give an example of using the Jev API
    - what are the question types you can ask?
    - what is guarenteed? what is not? What does the 'no hallucinations' claim really mean

    Once you're done writing your research in prose, we need you to fill in the following Frontmatter properties too (note: if property is already set then just verify):

    - set `website` to the best URL for referencing this model
    - if there is a repo available, set `repo` to the URL to the repo
    - if there is a clear company behind the model then set `company` to the companies name; set `company_url` to the companies URL
    - set the `license` property to "commercial" or "open-source"
website: https://docs.typesafe.ai/introduction
hash: 0c3497166b85c38b-169a2b33c9c7e0e6
last_updated: 2026-09-28
---
# Jev by TypeSafe AI

## What Jev is

Jev is TypeSafe AI’s first “System One” model, announced in September 2026. It is a hosted decision model: an application supplies a state (text, JSON object, or array) and one or more narrowly scoped typed questions, and receives structured answers with probabilities. It is designed to supply judgments to software workflows, not to hold a conversation. Unlike an LLM that generates a sequence of text tokens, Jev returns values from answer shapes declared in advance; ordinary code then decides what actions to take. TypeSafe describes its training approach as Reinforcement Learning for Calibrated Decisions (RLCD) and its serving approach as evaluating the questions in parallel. Performance and calibration claims are the company’s published results, not independent guarantees. ([launch announcement](https://typesafe.ai/blog/introducing-system-one-models-and-jev), [introduction](https://docs.typesafe.ai/introduction))

This makes Jev useful alongside an LLM rather than a replacement for one. It can route a request, classify content, score a candidate, or screen a prompt before or after an LLM call. Use an LLM when the system must draft, explain, translate, summarize, or perform open-ended multi-step reasoning; use code for exact arithmetic, permissions, and other deterministic rules. Jev does not return free-form text.

## What it does well, and where it struggles

Jev is aimed at fast, repeatable, small-scope judgments such as ticket routing, moderation signals, relevance scoring, and deciding whether a message meets a defined condition. A single request can ask several independent questions about the same state; the model returns typed values and distributions that code can threshold, rank, or combine. Its bounded output is valuable when downstream software needs an enum-like choice or score without parsing generated prose. The provider reports that it is optimized for low latency and cost, but these figures are workload- and service-dependent.

The model is not a source of truth. TypeSafe’s notes for `jev-1.13` say it can read questions literally, struggle with indirection and large state containing irrelevant detail, and make mistakes on adversarial content or contradictory instructions and criteria. It is unreliable at counting and numeric precision, including exact date/time comparisons. Keep calculations and comparisons in code; trim state to relevant evidence, make criteria consistent and explicit, and evaluate thresholds against representative examples before automating consequential actions. For generation, use a generative model. These limitations are version-specific and may change in later releases. ([Jev 1.13 jaggedness](https://docs.typesafe.ai/model-jaggedness/jev-1.13))

## API, SDKs, and authentication

The hosted HTTP API exposes `POST https://api.typesafe.ai/v1/systemone` for evaluations and `GET https://api.typesafe.ai/v1/models` for model discovery. Requests use JSON with `state`, `questions`, and an optional `model` such as `jev-latest`; responses contain a typed answer for each question and usage information. Authenticate with an API key from the TypeSafe dashboard in `Authorization: Bearer <API_KEY>`. Keep the key server-side, for example in `TYPESAFE_API_KEY`, rather than embedding it in a browser application. See the [API reference](https://api.typesafe.ai/docs) and [quick start](https://docs.typesafe.ai/introduction/quickstart).

TypeSafe publishes official SDKs for Python (`pip install typesafe-sdk`, Python 3.10+) and JavaScript/TypeScript (`npm install @typesafe-ai/sdk`, Node.js 20+). Both read `TYPESAFE_API_KEY` by default, expose typed question builders, and support configuring the model and API client. Other language packages and integrations exist, but are community-maintained or third-party gateway adapters rather than official TypeSafe SDKs; check their maintainers and status independently. The model itself has no public source-code or weight repository identified in the official model/API materials, so `repo` is omitted. The provider’s hosted service is commercial; the official SDK client code is separately MIT-licensed and that does not make the model weights or hosted inference open source. ([Python SDK](https://github.com/typesafe-ai/typesafe-sdk-python), [JavaScript SDK](https://github.com/typesafe-ai/typesafe-sdk-js), [service agreement](https://typesafe.ai/legal/mca))

## Question types

Each question is named by an ID chosen by the caller. The three types can be mixed in one request and evaluated independently against the same state:

- **Choice** selects one item from a caller-defined set. The response gives the selected option, probabilities across the options, and confidence. Include an `other` or `none` option when the set may not cover every case.
- **Score** places the state on caller-defined ordered levels, such as low/medium/high severity. The response gives a score, its level legend, probabilities, and confidence.
- **Noul** evaluates whether a statement is true and returns a value from 0 to 1 representing the probability of “yes.” It does not provide a separate confidence field.

Questions should each express one focused judgment. If an outcome depends on multiple factors, ask separate questions and combine them in application code. The provider says every question in a request sees the same state and is evaluated independently; a question’s answer is not hidden context for another question. ([question primitives](https://docs.typesafe.ai/primitives), [confidence](https://docs.typesafe.ai/confidence))

## Example: direct HTTP request

This cURL example asks a yes/no question about a support ticket. It uses the documented wire format and keeps the API key in the environment:

```sh
curl -X POST https://api.typesafe.ai/v1/systemone \
  -H "Authorization: Bearer $TYPESAFE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "jev-latest",
    "state": {"message": "I was charged twice. Please refund the duplicate."},
    "questions": {
      "refund_requested": {
        "type": "noul",
        "instructions": "Does `message` request a refund for a duplicate charge?"
      },
      "team": {
        "type": "choice",
        "instructions": "Which team should handle this message?",
        "criteria": {
          "billing": "Payments, charges, or refunds",
          "technical": "Product bugs or integrations",
          "other": "Anything else"
        }
      }
    }
  }'
```

The response includes `answers.refund_requested.noul` between 0 and 1, and for `team` a selected `choice`, its option probabilities, and confidence. Treat the returned decision as evidence for application logic, not as an instruction to take an action automatically.

## Guarantees and the “no hallucinations” claim

The provider’s narrow, defensible claim is about output shape: the API schema constrains a Choice to the supplied options, a Score to the supplied levels, and a Noul to a probability in the defined range. There is no generated paragraph in which Jev can invent a new label or malformed prose value. The launch post explicitly says its plotted “0%” type-error figure is not an empirical accuracy measurement; it is based on the schema-matching guarantee. ([launch announcement](https://typesafe.ai/blog/introducing-system-one-models-and-jev))

That does **not** guarantee that the selected label is factually correct, that the model understood the state, or that a probability/confidence estimate is calibrated for every task, prompt, or individual answer. A well-typed result can still be a wrong judgment, including a confidently wrong one. “No hallucinations” therefore means no out-of-contract generated answer in the type-safety sense, not no false beliefs, no mistakes, or universal factual reliability. Validate Jev on the application’s own data, set conservative thresholds, and preserve a review or fallback path when an incorrect decision would matter. The provider describes calibration as a property to use across decisions, not a per-answer truth guarantee. ([confidence guide](https://docs.typesafe.ai/confidence), [version limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13))
