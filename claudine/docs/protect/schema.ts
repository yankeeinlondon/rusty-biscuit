export type PreToolAction = "stop" | "exit" | "ask-stop" | "ask-exit";

/**
 * This type indicates the properties that will WILL be available
 * as Frontmatter in each of the documents found in @claudine/docs/protect
 * once both prompt have been run on the document at least once.
 */
export type Schema = {
    /** prompt for doing research and building out the docs BODY */
    prompt: string;
    /** prompt for using the research to complete the frontmatter properties */
    closure: string;

    /** the latest version of the agent software at the time of the research */
    agent_version: string;
    has_blocking_pre_tool_event: boolean;
    pre_tool_event: "n/a" | "influence" | "guarantee";
    pre_tool_actions: PreToolAction[];
    pre_tool_subagent: boolean;

    user_prompt_event: boolean;
    user_prompt_blocking_event?: boolean;
    user_prompt_mutation_event?: boolean;
    user_prompt_subagent?: boolean;

    other_events?: Record<string, string>;

    /** the URL to the Agent's docs on MCP */
    mcp_docs: string;
    /** the filepath to the Agent's MCP configuration (user scoped) */
    mcp_config_user: string;
    /** the filepath to the Agent's MCP configuration (repo scoped) */
    mcp_config_repo: string;

    /**
     * boolean flag indicating whether there is an event which gives us access
     * to the MCP response..
     */
    mcp_event: boolean;
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



    /** in the format of YYYY-MM-DD */
    last_updated: `${number}-${number}-${number}`
    /** an xxHash of the Markdown's content **/
    body_hash: number
}
