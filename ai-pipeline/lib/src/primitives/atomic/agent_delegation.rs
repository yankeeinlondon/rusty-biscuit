//! Agent delegation primitives for the OpenCode CLI.
//!
//! This module provides a concrete `OpenCodeDelegation` step that executes the
//! `opencode` CLI in either interactive or headless mode. The step embeds a
//! serialized state payload and JSON schema in the prompt, then parses the
//! assistant output from OpenCode's JSON event stream.

use std::collections::{HashMap, HashSet};
use std::io;
use std::process::{Command, Stdio};

use serde_json::{Map, Value};
use thiserror::Error;

use crate::primitives::runnable::{AgentDelegation, Runnable};
use crate::primitives::state::{PipelineState, StateKey, StepError};

const DEFAULT_STATE_INSTRUCTIONS: &str =
    "Use the provided state JSON and schema. Return the final output as JSON matching the output schema. Output JSON only.";
const DEFAULT_FINALIZATION_JSON: &str =
    "Return the final output as JSON matching the output schema. Output JSON only.";
const DEFAULT_FINALIZATION_TEXT: &str = "Return the final output as plain text.";

/// Execution mode for OpenCode CLI delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCodeMode {
    /// Run the OpenCode TUI for human-in-the-loop interaction.
    Interactive,
    /// Run OpenCode in a single headless call.
    Headless,
}

/// Session continuation strategy for OpenCode CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenCodeSession {
    /// Start a new session.
    New,
    /// Continue the most recent session.
    ContinueLast,
    /// Continue a specific session ID.
    SessionId(String),
}

/// Errors that can occur during OpenCode CLI delegation.
#[derive(Debug, Error)]
pub enum OpenCodeDelegationError {
    /// Expected state JSON missing from the pipeline state.
    #[error("missing pipeline state for key '{key}'")]
    MissingState { key: String },

    /// Failed to serialize JSON for prompt embedding.
    #[error("failed to serialize JSON payload: {0}")]
    JsonSerializeFailed(String),

    /// OpenCode binary was not found.
    #[error("opencode CLI not found: {0}")]
    BinaryNotFound(String),

    /// Failed to spawn the OpenCode process.
    #[error("failed to spawn opencode: {0}")]
    SpawnFailed(#[from] io::Error),

    /// OpenCode exited with a non-zero status code.
    #[error("opencode exited with status {status}: {stderr}")]
    NonZeroExit { status: i32, stderr: String },

    /// OpenCode emitted no events to parse.
    #[error("opencode returned no JSON events")]
    EmptyOutput,

    /// Failed to parse OpenCode JSON events.
    #[error("failed to parse opencode event JSON: {0}")]
    InvalidEventJson(String),

    /// OpenCode did not yield assistant output.
    #[error("opencode did not emit an assistant response")]
    MissingAssistantOutput,

    /// Structured output could not be parsed as JSON.
    #[error("assistant output is not valid JSON: {0}")]
    InvalidStructuredOutput(String),

    /// OpenCode reported a session error.
    #[error("opencode session error: {0}")]
    SessionError(String),
}

/// Delegates work to the OpenCode CLI using a prompt plus pipeline state payloads.
#[derive(Debug, Clone)]
pub struct OpenCodeDelegation {
    prompt: String,
    state_key: Option<StateKey<Value>>,
    state_schema: Option<Value>,
    output_schema: Option<Value>,
    mode: OpenCodeMode,
    session: OpenCodeSession,
    binary: String,
    agent: Option<String>,
    model: Option<String>,
    variant: Option<String>,
    title: Option<String>,
    state_instructions: String,
    finalization_prompt: Option<String>,
    capture_output: bool,
    session_key: Option<StateKey<String>>,
    read_keys: Vec<&'static str>,
    write_keys: Vec<&'static str>,
}

impl OpenCodeDelegation {
    /// Creates a new OpenCode delegation step with the given prompt.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            state_key: None,
            state_schema: None,
            output_schema: None,
            mode: OpenCodeMode::Headless,
            session: OpenCodeSession::New,
            binary: "opencode".to_string(),
            agent: None,
            model: None,
            variant: None,
            title: None,
            state_instructions: DEFAULT_STATE_INSTRUCTIONS.to_string(),
            finalization_prompt: None,
            capture_output: true,
            session_key: None,
            read_keys: Vec::new(),
            write_keys: Vec::new(),
        }
    }

    /// Sets the pipeline state key that contains JSON state for the prompt.
    #[must_use]
    pub fn with_state_key(mut self, key: StateKey<Value>) -> Self {
        self.state_key = Some(key);
        self.read_keys = vec![key.name()];
        self
    }

    /// Stores the OpenCode session ID in pipeline state after execution.
    #[must_use]
    pub fn with_session_key(mut self, key: StateKey<String>) -> Self {
        self.session_key = Some(key);
        self.write_keys = vec![key.name()];
        self
    }

    /// Sets the JSON schema for the pipeline state payload.
    #[must_use]
    pub fn with_state_schema(mut self, schema: Value) -> Self {
        self.state_schema = Some(schema);
        self
    }

    /// Sets the JSON schema for the expected output.
    #[must_use]
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Switches the delegation to interactive mode.
    #[must_use]
    pub fn interactive(mut self) -> Self {
        self.mode = OpenCodeMode::Interactive;
        self
    }

    /// Switches the delegation to headless mode.
    #[must_use]
    pub fn headless(mut self) -> Self {
        self.mode = OpenCodeMode::Headless;
        self
    }

    /// Sets the OpenCode session continuation strategy.
    #[must_use]
    pub fn with_session(mut self, session: OpenCodeSession) -> Self {
        self.session = session;
        self
    }

    /// Overrides the OpenCode binary name or path.
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Sets the OpenCode agent to use.
    #[must_use]
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Sets the model identifier in `provider/model` format.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the provider-specific model variant.
    #[must_use]
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Sets the session title for headless runs.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Overrides the default state instructions embedded in the prompt.
    #[must_use]
    pub fn with_state_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.state_instructions = instructions.into();
        self
    }

    /// Overrides the follow-up prompt used after interactive sessions.
    #[must_use]
    pub fn with_finalization_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.finalization_prompt = Some(prompt.into());
        self
    }

    /// Enables or disables the follow-up output capture after interactive runs.
    #[must_use]
    pub fn capture_output(mut self, capture: bool) -> Self {
        self.capture_output = capture;
        self
    }

    fn step_error(&self, err: OpenCodeDelegationError) -> StepError {
        StepError::new(self.name(), err.to_string())
            .with_source(err)
            .fatal()
    }

    fn build_prompt(&self, state_payload: &Value) -> Result<String, OpenCodeDelegationError> {
        let mut sections = Vec::new();
        sections.push(self.prompt.clone());

        let state_json = self.serialize_json(state_payload)?;
        sections.push(format!("Pipeline State (JSON):\n{state_json}"));

        if let Some(schema) = &self.state_schema {
            let schema_json = self.serialize_json(schema)?;
            sections.push(format!("State Schema (JSON):\n{schema_json}"));
        }

        if let Some(schema) = &self.output_schema {
            let schema_json = self.serialize_json(schema)?;
            sections.push(format!("Output Schema (JSON):\n{schema_json}"));
        }

        sections.push(format!("Instructions:\n{}", self.state_instructions));

        Ok(sections.join("\n\n"))
    }

    fn build_finalization_prompt(&self) -> Result<String, OpenCodeDelegationError> {
        let instructions = if let Some(prompt) = &self.finalization_prompt {
            prompt.clone()
        } else if self.output_schema.is_some() {
            DEFAULT_FINALIZATION_JSON.to_string()
        } else {
            DEFAULT_FINALIZATION_TEXT.to_string()
        };

        let mut sections = Vec::new();
        if let Some(schema) = &self.output_schema {
            let schema_json = self.serialize_json(schema)?;
            sections.push(format!("Output Schema (JSON):\n{schema_json}"));
        }
        sections.push(format!("Instructions:\n{instructions}"));
        Ok(sections.join("\n\n"))
    }

    fn serialize_json(&self, value: &Value) -> Result<String, OpenCodeDelegationError> {
        serde_json::to_string_pretty(value)
            .map_err(|err| OpenCodeDelegationError::JsonSerializeFailed(err.to_string()))
    }

    fn resolve_state_payload(
        &self,
        state: &PipelineState,
    ) -> Result<Value, OpenCodeDelegationError> {
        if let Some(key) = self.state_key {
            state
                .get(key)
                .cloned()
                .ok_or_else(|| OpenCodeDelegationError::MissingState {
                    key: key.name().to_string(),
                })
        } else {
            Ok(Value::Object(Map::new()))
        }
    }

    fn run_interactive(&self, prompt: &str) -> Result<(), OpenCodeDelegationError> {
        let mut command = Command::new(&self.binary);
        command.stdin(Stdio::inherit());
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        if let Some(agent) = &self.agent {
            command.args(["--agent", agent]);
        }
        if let Some(model) = &self.model {
            command.args(["--model", model]);
        }
        match &self.session {
            OpenCodeSession::ContinueLast => {
                command.arg("--continue");
            }
            OpenCodeSession::SessionId(session_id) => {
                command.args(["--session", session_id]);
            }
            OpenCodeSession::New => {}
        }

        command.args(["--prompt", prompt]);

        let status = match command.status() {
            Ok(status) => status,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(OpenCodeDelegationError::BinaryNotFound(self.binary.clone()));
            }
            Err(err) => return Err(OpenCodeDelegationError::SpawnFailed(err)),
        };

        if status.success() {
            Ok(())
        } else {
            let code = status.code().unwrap_or(-1);
            Err(OpenCodeDelegationError::NonZeroExit {
                status: code,
                stderr: "interactive session failed".to_string(),
            })
        }
    }

    fn run_headless(
        &self,
        prompt: &str,
        session: OpenCodeSession,
    ) -> Result<OpenCodeRunResult, OpenCodeDelegationError> {
        let mut command = Command::new(&self.binary);
        command.arg("run");
        command.args(["--format", "json"]);

        if let Some(agent) = &self.agent {
            command.args(["--agent", agent]);
        }
        if let Some(model) = &self.model {
            command.args(["--model", model]);
        }
        if let Some(variant) = &self.variant {
            command.args(["--variant", variant]);
        }
        if let Some(title) = &self.title {
            command.args(["--title", title]);
        }

        match session {
            OpenCodeSession::ContinueLast => {
                command.arg("--continue");
            }
            OpenCodeSession::SessionId(session_id) => {
                command.args(["--session", &session_id]);
            }
            OpenCodeSession::New => {}
        }

        command.arg(prompt);

        let output = match command.output() {
            Ok(output) => output,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(OpenCodeDelegationError::BinaryNotFound(self.binary.clone()));
            }
            Err(err) => return Err(OpenCodeDelegationError::SpawnFailed(err)),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(OpenCodeDelegationError::NonZeroExit {
                status: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Self::parse_run_output(&stdout)?;
        if self.output_schema.is_some() {
            let parsed: Value = serde_json::from_str(&result.output)
                .map_err(|err| OpenCodeDelegationError::InvalidStructuredOutput(err.to_string()))?;
            result.output = serde_json::to_string(&parsed)
                .map_err(|err| OpenCodeDelegationError::JsonSerializeFailed(err.to_string()))?;
        }
        Ok(result)
    }

    fn parse_run_output(output: &str) -> Result<OpenCodeRunResult, OpenCodeDelegationError> {
        let mut session_id = None;
        let mut message_roles: HashMap<String, String> = HashMap::new();
        let mut message_text: HashMap<String, String> = HashMap::new();
        let mut assistant_message_ids: HashSet<String> = HashSet::new();
        let mut last_message_id: Option<String> = None;
        let mut last_assistant_id: Option<String> = None;
        let mut saw_event = false;

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            saw_event = true;

            let event: Value = serde_json::from_str(trimmed)
                .map_err(|err| OpenCodeDelegationError::InvalidEventJson(err.to_string()))?;

            let event_type = event.get("type").and_then(Value::as_str);
            match event_type {
                Some("session.created") => {
                    if let Some(id) = event.pointer("/properties/info/id").and_then(Value::as_str) {
                        session_id = Some(id.to_string());
                    }
                }
                Some("session.error") => {
                    if let Some(message) = event
                        .pointer("/properties/error/data/message")
                        .and_then(Value::as_str)
                    {
                        return Err(OpenCodeDelegationError::SessionError(message.to_string()));
                    }
                }
                Some("message.updated") => {
                    if let Some(info) = event.get("properties").and_then(|p| p.get("info")) {
                        if let Some(message_id) = info.get("id").and_then(Value::as_str) {
                            if let Some(role) = info.get("role").and_then(Value::as_str) {
                                message_roles.insert(message_id.to_string(), role.to_string());
                                if role == "assistant" {
                                    assistant_message_ids.insert(message_id.to_string());
                                    if message_text.contains_key(message_id) {
                                        last_assistant_id = Some(message_id.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                Some("message.part.updated") => {
                    let part = event.pointer("/properties/part");
                    let part_type = part.and_then(|p| p.get("type")).and_then(Value::as_str);
                    if part_type != Some("text") {
                        continue;
                    }

                    let message_id = part
                        .and_then(|p| p.get("messageID"))
                        .and_then(Value::as_str)
                        .map(|id| id.to_string());

                    let message_id = match message_id {
                        Some(id) => id,
                        None => continue,
                    };

                    let entry = message_text.entry(message_id.clone()).or_default();
                    if let Some(delta) = event.pointer("/properties/delta").and_then(Value::as_str)
                    {
                        entry.push_str(delta);
                    } else if let Some(text) =
                        part.and_then(|p| p.get("text")).and_then(Value::as_str)
                    {
                        *entry = text.to_string();
                    }

                    last_message_id = Some(message_id.clone());
                    if assistant_message_ids.contains(&message_id)
                        || message_roles
                            .get(&message_id)
                            .map(|role| role == "assistant")
                            .unwrap_or(false)
                    {
                        last_assistant_id = Some(message_id);
                    }
                }
                _ => {}
            }
        }

        if !saw_event {
            return Err(OpenCodeDelegationError::EmptyOutput);
        }

        let selected_id = last_assistant_id.or(last_message_id);
        let output = match selected_id.and_then(|id| message_text.get(&id).cloned()) {
            Some(text) if !text.trim().is_empty() => text.trim().to_string(),
            _ => return Err(OpenCodeDelegationError::MissingAssistantOutput),
        };

        Ok(OpenCodeRunResult { output, session_id })
    }

    fn followup_session(&self) -> OpenCodeSession {
        match &self.session {
            OpenCodeSession::New => OpenCodeSession::ContinueLast,
            OpenCodeSession::ContinueLast => OpenCodeSession::ContinueLast,
            OpenCodeSession::SessionId(session_id) => {
                OpenCodeSession::SessionId(session_id.clone())
            }
        }
    }
}

impl AgentDelegation for OpenCodeDelegation {
    fn is_interactive(&self) -> bool {
        matches!(self.mode, OpenCodeMode::Interactive)
    }
}

impl Runnable for OpenCodeDelegation {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        let state_payload = self
            .resolve_state_payload(state)
            .map_err(|err| self.step_error(err))?;

        let prompt = self
            .build_prompt(&state_payload)
            .map_err(|err| self.step_error(err))?;

        let result = match self.mode {
            OpenCodeMode::Headless => self
                .run_headless(&prompt, self.session.clone())
                .map_err(|err| self.step_error(err))?,
            OpenCodeMode::Interactive => {
                self.run_interactive(&prompt)
                    .map_err(|err| self.step_error(err))?;
                if !self.capture_output {
                    return Ok(String::new());
                }
                let followup_prompt = self
                    .build_finalization_prompt()
                    .map_err(|err| self.step_error(err))?;
                self.run_headless(&followup_prompt, self.followup_session())
                    .map_err(|err| self.step_error(err))?
            }
        };

        if let Some(session_key) = self.session_key {
            if let Some(session_id) = result.session_id {
                state.set(session_key, session_id);
            }
        }

        Ok(result.output)
    }

    fn execute_readonly(&self, _state: &PipelineState) -> Result<Self::Output, StepError> {
        Err(StepError::new(self.name(), "OpenCode delegation requires mutable state").fatal())
    }

    fn name(&self) -> &str {
        "OpenCodeDelegation"
    }

    fn declares_reads(&self) -> &[&'static str] {
        self.read_keys.as_slice()
    }

    fn declares_writes(&self) -> &[&'static str] {
        self.write_keys.as_slice()
    }
}

#[derive(Debug, Clone)]
struct OpenCodeRunResult {
    output: String,
    session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_prompt_includes_state_and_schema() {
        let delegation = OpenCodeDelegation::new("Summarize")
            .with_state_schema(json!({"type": "object"}))
            .with_output_schema(json!({"type": "object"}));
        let prompt = delegation
            .build_prompt(&json!({"topic": "apples"}))
            .expect("prompt should build");

        assert!(prompt.contains("Pipeline State"));
        assert!(prompt.contains("State Schema"));
        assert!(prompt.contains("Output Schema"));
        assert!(prompt.contains("Instructions"));
    }

    #[test]
    fn test_parse_run_output_collects_assistant_text() {
        let output = r#"{"type":"session.created","properties":{"info":{"id":"sess-1"}}}
{"type":"message.updated","properties":{"info":{"id":"msg-1","role":"assistant"}}}
{"type":"message.part.updated","properties":{"part":{"id":"part-1","messageID":"msg-1","type":"text","text":"Hello"},"delta":"Hello"}}
{"type":"message.part.updated","properties":{"part":{"id":"part-1","messageID":"msg-1","type":"text","text":"Hello world"},"delta":" world"}}"#;

        let result = OpenCodeDelegation::parse_run_output(output).expect("parse should succeed");
        assert_eq!(result.output, "Hello world");
        assert_eq!(result.session_id, Some("sess-1".to_string()));
    }

    #[test]
    fn test_followup_session_defaults_to_continue() {
        let delegation = OpenCodeDelegation::new("test");
        assert_eq!(delegation.followup_session(), OpenCodeSession::ContinueLast);
    }
}
