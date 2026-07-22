---
$schema: 
    research: file
description: Provides research on the top API standards which cloud model providers use for interacting with an LLM model
doc_structure: |-
    # Model Serving API Standards

    ## OpenAI API

    ## Anthropic API

    ## Ollama API
research: "@claudine/docs/research/model-servering-api-standards/model-serving-api-standards.md"
    
sequence: 
    - name: openai
      section: OpenAI API
    - name: anthropic
      section: Anthropic API
    - name: ollama
      section: Ollama API
    - name: comparison
---
## Context

There are three primary API's which LLM model providers and local LLM runners will use to expose model's to AI agents and other software:

1. OpenAI API - the OG and still the most popular option, almost everyone will support this API first
2. Anthropic API - the main variant to OpenAI's API
3. Ollama API - for local runners, the Ollama API serves as a useful way to expose a model

## Task

::block when="state.name != 'comparison'"

Your task is to research the {{ state.section }} and use that research to update the `## {{ state.section }}` section
in the document: {{ research }}

Your research should be able to answer at least the following questions:

- a summary paragraph that explains the utility of this API along with it's strengths and weaknesses
- key URL's: API documentation, changelog, schema file
- when did this API first get published?
- what is the latest version of the API as of today?
    - what are they key features and variations to the API that have taken place over time?
    - how stable is the API surface today?
- which model providers (both _originators_ and _aggregators_) provide this API surface? Are there any important variations that any of these providers do with the informal API standard (e.g., an unusual auth method, additional metadata returned on certain endpoints, etc.)
    - you should always include -- at a minimum -- the following model providers:
    - openai, anthropic, google, X.AI, Groq
    - Moonshot AI, Z.ai, Qwen/Alibaba, Minimax, DeepSeek
    - OpenCode Zen
    - OpenRouter, Zenmux
- which local runners support the {{ state.section }} as a way to interact with their locally served models? 
    - be sure to include -- at a minimum -- the following local runners:
    - ollama, llama.cpp, vllm, oMLX
    - but try to include as many as you can find reference to
- list out the various endpoints which are provided as part of the API and give a brief description of each

Your research should be written in clear prose which targets a technical audience who understands all of the basics of API terminology but don't assume they know any of the specifics of the {{ state.section }} when writing.

- the document will already exist along with a document structure (some of the sections may already be filled out)
- you must be careful not to overwrite any of the existing content but as long as you put your research into the H2 heading "## {{state.section}}" you can include any document structure (H3 downward) that you like

Once the prose content of your research is complete we must update the Frontmatter of the document:

- set `{{state.name}}_api_docs` to the URL for API documentation
- set `{{state.name}}_api_schema` to a URL defining the schema of the API
- set `{{state.name}}_latest_version` to the latest published version of the API
- set `{{state.name}}_model_providers` to the latest 

::end-block

::block when="state.name == 'comparison'"

::end-block
