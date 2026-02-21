export type PreToolAction = "stop" | "exit" | "ask-stop" | "ask-exit";

/**
 * This type indicates the properties that will be available
 * as Frontmatter in each of the documents found in @claudine/docs/protect
 * once both prompts have been run on the document at least once.
 */
export type Schema = {
    /** prompt for doing research and building out the docs BODY */
    prompt: string;
    /** prompt for using the research to complete the frontmatter properties */
    closure: string;

    /** the latest version of the agent software at the time of the research */
    agent_version: string;

    // ── Event Hooks ──────────────────────────────────────────────

    /** whether the Agent has a blocking event that fires before a planned tool call */
    has_blocking_pre_tool_event: boolean;
    /** the degree of control the pre-tool event's return value provides over execution */
    pre_tool_influence: "n/a" | "influence" | "guarantee";
    /** which blocking actions the pre-tool event listener can perform */
    pre_tool_actions: PreToolAction[];
    /** whether pre-tool hooks fire inside subagents */
    pre_tool_subagent: boolean;

    /** whether the Agent provides an event where user prompts can be received before processing */
    user_prompt_event: boolean;
    /** whether the user prompt event can block execution or force user confirmation */
    user_prompt_blocking_event?: boolean;
    /** whether the user prompt event allows mutating the prompt before the Agent processes it */
    user_prompt_mutation_event?: boolean;
    /** whether user prompt events fire inside subagents */
    user_prompt_subagent?: boolean;

    /** other events useful for safety; keys are event names, values describe trigger and blocking behavior */
    other_events?: Record<string, string>;

    // ── Intercepting MCP Calls ───────────────────────────────────

    /** whether the Agent supports MCP servers at all */
    mcp_supported: boolean;
    /** the URL to the Agent's docs on MCP */
    mcp_docs: string;
    /** the filepath to the Agent's MCP configuration (user scoped) */
    mcp_config_user: string;
    /** the filepath to the Agent's MCP configuration (repo scoped) */
    mcp_config_repo: string;

    /**
     * boolean flag indicating whether there is an event which gives us access
     * to the MCP response.
     */
    mcp_event: boolean;
    /** name of the event providing access to MCP responses */
    mcp_event_name?: string;
    /**
     * boolean flag indicating whether there is an event which gives us access
     * to the MCP response and which allows us to modify the response before it's used.
     */
    mcp_event_modifiable: boolean;
    /**
     * boolean flag indicating whether there is an event which gives us access
     * to the MCP response and which allows us to STOP the flow if needed.
     */
    mcp_event_stop: boolean;

    // ── Completion Gates ─────────────────────────────────────────

    /** whether the Agent fires an event when it considers work complete */
    has_completion_event: boolean;
    /** whether the completion event can prevent the Agent from stopping */
    completion_event_blocking: boolean;
    /** event names that relate to task/turn completion */
    completion_event_names: string[];
    /** whether the Agent has built-in protection against infinite loops from blocking completion */
    completion_loop_protection: boolean;

    // ── Subagents ────────────────────────────────────────────────

    /** whether subagent creation/completion fires events */
    has_subagent_events: boolean;
    /** whether pre-tool and post-tool hooks fire inside subagents the same as the main agent */
    hooks_fire_in_subagents: boolean | null;
    /** whether subagent permissions can be restricted independently */
    subagent_permissions_configurable: boolean;

    // ── Escalated Privileges ─────────────────────────────────────

    /** whether the Agent provides sandbox or container-based isolation */
    has_sandbox: boolean;
    /** whether the Agent detects and warns about running as root or elevated */
    detects_elevated_privileges: boolean;
    /** whether the Agent has a mode that bypasses permission checks entirely */
    has_bypass_mode: boolean;

    // ── Metadata ─────────────────────────────────────────────────

    /** in the format of YYYY-MM-DD */
    last_updated: `${number}-${number}-${number}`;
    /** an xxHash of the Markdown's content */
    body_hash: number;
};
