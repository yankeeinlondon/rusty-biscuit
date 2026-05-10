# Prompt Reporting

## Context

The **System Prompt** and **User Prompt** are both critical instructions which an Agent platform uses when executing a session and Claudine reports on them (by default) like this currently:

```sh
System Prompt(appended):
  ▌ ██ Context
  ▌
  ▌ - you are working in the rusty-biscuit monorepo
  ▌ - this session was started with a focus on the claudine package area
  ▌     - you must use the 'claudine' agent skill
  ▌
  ▌
  ▌ ██ Best Practices
  ▌
  ▌ - when rendering to the terminal use the biscuit-terminal and darkmatter skills!
  ▌     - leverage the Prose struct from biscuit-terminal for rich text (color, style), hyperlinks (OS8), and more
  ▌
  ▌ - when attempting to do host discovery (hardware, software, os, file-system, repo/git) you should use the sniff skill
  ▌ - when doing file conversions between JSON, YAML, TOML always use the biscuit-file skill
  ▌ - whenever you are attempt to convert a string based file reference to a real file path in the filesystem you should use
  ▌   FileReference struct from biscuit-file and use the biscuit-file skill
  ▌
  ▌ ██ Non-Interactive Prompt
  ▌
  ▌ You are running in a non-interactive session — the user cannot answer prompts for feedback or permissions.
  ▌
  ▌ - If blocked (tool call denied, file unreadable, etc.): tell the user, then find an alternative way to get the information.
  ▌ - If truly unable to complete the task: report what happened, note how to avoid it going forward, and exit with an error code.
  ▌ - Never run commands that need an interactive terminal or stdin follow-up (REPLs, editors, pagers, prompts, anything waiting
  ▌   on input).
  ▌ - Prefer one-shot commands with explicit non-interactive flags; if an approach would require sending more input to a running
  ▌   command, pick a different approach.
  ▌
  ▌ ████ Credential and signing blockers (this is very likely why you're hanging)
  ▌
  ▌ - Git commits with GPG/SSH signing enabled will block waiting for a passphrase via gpg-agent / ssh-agent / pinentry. Before
  ▌   running git commit in a non-interactive session, assume the passphrase is NOT cached. If a commit appears to hang, abort the
  ▌   command, report it, and do not retry.
  ▌ - Never run gpg, ssh, ssh-add, sudo, op signin, aws sso login, gh auth login, docker login, npm login, or any credential
  ▌   helper that may prompt — even if stdin is closed, many of these open /dev/tty directly.
  ▌ - If a shell command does not complete within ~60s, treat it as stuck and abandon that approach; do not wait longer and do not
  ▌   assume progress.
  ▌
  ▌ ████ Sub-agent propagation
  ▌
  ▌ - When spawning sub-agents (Task tool or equivalent), always repeat in their brief that the entire session tree is
  ▌   non-interactive. Sub-agents inherit nothing from this system prompt unless you include it.
  ▌ - Give each sub-agent a narrow, bounded task. Never dispatch a sub-agent that depends on waiting for another sub-agent's
  ▌   output unless you are acting as an explicit orchestrator and can collect results yourself.
  ▌
  ▌ ████ Subprocess hygiene
  ▌
  ▌ - Export GIT_TERMINAL_PROMPT=0 before running any git command that could prompt for credentials.
  ▌ - Prefer command forms that declare non-interactivity explicitly: apt-get -y, brew --quiet, npm install --no-audit --no-fund,
  ▌   cargo --color=never.
  ▌ - Never run a command in a background shell with &; if you need parallelism, use the framework's concurrent tool-call feature
  ▌   instead.Progress reporting (fights the silence-looks-like-hang problem)
  ▌ - Emit a short status message to the user at least once every ~30 seconds of work. If you are about to start a long-running
  ▌   tool call, say so first.

Agent Prompt:
  ▌ You are a planning agent. Convert the following documents into a high confidence execution plan:
  ▌
  ▌ - Functional Specification: claudine/features/_unscheduled/1-remove-agent-capabilities/spec.md
  ▌
  ▌ ██ Requirements
  ▌
  ▌ - Break work into phases and tasks
  ▌ - Order tasks by dependency
  ▌ - Flag parallelizable work
  ▌ - Include validation checkpoints

- remaining prompt truncated for brevity, use --verbose to show entire prompt
```

## Changes

In this release our focus will be on how we're going to _change_ how we report these two prompts:


- **Agent Prompts**
    - line 1: 
        - `Prose::new(println!("{icon} <b>System Prompt(<dim><i>{action}</i></dim>)</b>", icon, action))`
        - where `icon` is
        - where `action` is 'appended' | 'replaced' | 'unchanged'
        - this is shown when:
            - the system prompt has been changed, or 
            - caller has used `verbose` flag
            - except when `--silent` flag is in invocation in which case it is NEVER shown
    - body:
        - the body is NEVER shown if "line 1" is not being shown
        - what is shown in the "body" section is based CLI flags and other conditions used but these variants exist:
            - **Summary**
                - The system prompt can often be quite long and the user might like _seeing_ it at first but it can become quite repetitive so we will often choose instead prefer the "summary" mode of reporting
                - The summary takes advantage of the 
            - **Partial Prompt**
            - **Full Prompt**
- 🧠
