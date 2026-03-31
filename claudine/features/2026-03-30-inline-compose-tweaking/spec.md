# Claudine's `inline-compose` Improvements

- I'm a bit shocked that when i run `claudine inline-compose <file>` that I do not get the prompt message. This works just fine with the `compose` subcommand! I'm worried we are not using largely the same execution pipeline!!!
    - `inline-compose` get's the prompt from the `prompt` property (versus the body of the document)
    - `inline-compose` adds to the frontmatter `prompt` by adding text which STRONGLY encourages the Agent to write to the body of the same file and NOT to edit Frontmatter.
    - Other than that, `inline-compose` and `compose` are very much the same
        - both do pre_validation and post_validation checks (including handlers when needed)
        - both use JSON streaming
        - both **should** present the prompt
        - both should then report the session ID once received
        - both should then report all of the thinking and info that the Agent has for us to display
            - rather than displaying it directly though, we run it through the Darkmatter rendered so that an Agent's Markdown syntax is immediately converted to something more terminal friendly
        - both then conclude with the same metadata info
        - both offer the `--perf` flag to evaluate performance metrics in the pipeline
- Also because we _used_ to have a `compose-inline` command and many people have dyslexia we should add a `compose-inline` command which immediately errors and suggests the use of `inline-compose` instead.

- When I run 'claudine claude inline-compose @claudine/docs/research/permissions/claude.md' I get:

    ```sh
    Claudine ▸ Claude inline-compose

    Environment Variables:
    • ANTHROPIC_API_KEY
    • BRAVE_API_KEY
    • DEEPSEEK_API_KEY
    • ELEVEN_LABS_API_KEY
    • GEMINI_API_KEY
    • GITHUB_TOKEN
    • GROQ_API_KEY
    • HUGGINGFACE_AUTH_TOKEN
    • MISTRAL_API_KEY
    • MOONSHOT_API_KEY
    • OPENAI_API_KEY
    • OPEN_ROUTER_API_KEY
    • PVE_API_TOKEN
    • TOKENIZERS_PARALLELISM
    • X_AI_API_KEY
    • ZAI_API_KEY
    • ZENMUX_API_KEY
    • AGENT=claude
    • AGENT_PARAMS=["inline-compose","@claudine/docs/research/permissions/claude.md"]
    • CLAUDINE_SESSION_ID=8f8394a2-df55-4c38-975b-7b24415b8919
    • INTERACTIVE=false
    • PACKAGE_AREA=claudine
    • YOLO=false

    - Info: potentially dangerous ENV variables were removed; if you need one of these to be included
       use the --include <ENV> CLI switch

    - Claude session ID 8eaa986b-863

    Let me understand what you're referring to. Let me look at the current changes and the
    compose-related feature spec. Now I understand the context. This is about implementing the
    claudine inline-compose <file-ref> command as part of the compose refactor. Let me explore the
    current codebase to understand what exists. The inline-compose command already has substantial
    implementation — CLI args, the full composition pipeline (execute_composition_request),
    prepare/resolve/select/closure stages, and integration tests.

    What specifically do you need me to do with inline-compose? For example:

    - Fix a bug or issue you're seeing?
    - Implement a missing piece from the spec?
    - Review the current implementation against the spec?
    - Run the tests?
    - Something else?



    ✓ 208s · 9 input tokens · 2K output tokens · 170K cached tokens · $0.49 cost basis · no tool calls
    ```

    this kind of response happens fairly consistently and begs a LOT of questions!
