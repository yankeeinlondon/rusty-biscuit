---
prompt: |-
    ## Context

    - Use the 'claudine' skill for this task

    The **rendezvous** daemon will include a process monitor that looks across a host and detects running instances of Agentic CLI's:

    - these Agentic CLI's may have been run directly by (e.g., someone ran "claude", "codex", etc.), or
    - they may have been run _by_ Claudine in a "wrapped execution" (e.g., someone ran claudine which in turn ran the underlying CLI agent to do most of the work)
        - in this mode the CLI command might have been any of the following:
            - Direct wrapped Execution
                - `claudine claude ...`
                - `claudine codex ...`
                - `claudine {agent} ...`
            - Document Composition
                - `claudine compose ...`
                - `claudine inline-compose ...`
                - `claudine sequence ...`

    Its also important to recognize it that these processes may be running as an **interactive** or **non-interactive** session. This distinction shapes how much of activity lifecycle that Claudine directly controls (for non-interactive sessions) versus acting more as a passive wrapper (for interactive sessions).

    It is highly valueable for the **rendezvous** daemon to not only observe the start/stop lifecycle of Agentic processes but also to extract as much metadata and understanding as possible from these processes.

    ## Task

    
    
    
---
