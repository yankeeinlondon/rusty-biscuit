---
prompt: |-
    # Postman API Application

    The **Postman** application is one of the more popular tools to test API's on a desktop OS. One of the convenient features which often get's overlooked is it's ability to organize a collection of endpoints together to represent an API. These project's can then be exported and shared with others which super helpful.

    Your task is to do a deep dive into the Postman Collections feature:

    - what is the file format for a collection?
        - there different variants of collection including:
            - HTTP (the most common)
            - GraphQL
            - AI
            - MCP
            - gRPC
            - WebSocket
            - Socket.IO
            - MQTT
        - for each of these 
            - describe the focus and capabilities of these different collection types
            - provide a documentation URL for each
            - describe how these collections interact with entities like:
                - Environment
                - Flow
                - Workspace
                - and Insights
            - describe how security is managed with these collections
    - once we've documented the details of a collection:
        - discuss how one might convert an OpenAPI schema to the collection file format
        - discuss how the inverse might be done (collection -> OpenAPI spec)

    All code examples throughout this documentation should be done in Rust. The final output must be a well formed, idiomatic Markdown document. If using Mermaid diagrams to illustrate ideas is of use then please include this too.

---

