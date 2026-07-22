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
in the 

::end-block

::block when="state.name == 'comparison'"

::end-block
