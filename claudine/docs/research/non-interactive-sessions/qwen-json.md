---
prompt: |-
	The Qwen CLI can output a stream of structured JSON/JSONL data instead of just text when we run in a non-interactive mode. This structured data is much more valuable to Claudine than just text as we not only get the Agent's response but also lots of useful metadata which we can either respond to or report back to the caller in a well formatted way.


    
    
    JSONL output when the `--output-format stream-json` flag is included. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.



    - This metadata can be used to present metadata to the user on STDERR when they are executing a non-interactive command.
    - This metadata can be used to enhance the data we're providing to our logging platform

    ## Your task is to:
    
    ### Research

    - research online to find:
        - a formal specification of the structured data that Qwen CLI provides 
        - any and all CLI switches which are involved in changing the output format of non-interactive sessions 
    - other examples online and fill in any other missing details not self-evident from the example data
    - determine how best to feed the metadata to logging and non-interactive sessions.
    
    ### Frontmatter in this Document 

    - set the `schema` frontmatter property on this document to a URL that defines the schema for Qwen CLI's streaming responses
    - set the `docs` frontmatter property on this document to a URL that defines the schema for Qwen CLI's documentation for streaming responses

last_updated: 2026-03-16
---
