# Model Citizen
> A library and CLI which helps you manage your AI models you have locally

This library attempts to:

- give you an overall inventory of the models you have downloaded for **Ollama**, **Llama.cpp**, and **LM Studio**
- help you to share models between the different runners you have installed
- inspect metadata on the default settings for models
- find and download models from HuggingFace


## CLI syntax

> `model <subcommand> <options>`

The CLI has following sub-commands:

1. `list` - lists all the models known to be on the host system
2. `info <model>` - provides useful metadata about a particular model
3. `search <filter>` - search for models on hugging face
4. `download <model-id>`
      - shows variants available of the given model and allows you to interactively select those you wish to download
5. `remove <model>`



