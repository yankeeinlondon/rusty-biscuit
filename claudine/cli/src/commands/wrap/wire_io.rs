//! Re-export bridge: the canonical wire I/O module now lives in
//! `exec::wiring`. This file preserves backward compatibility
//! for callers that still reference `wrap::wire_io` until the
//! migration is complete.
#![allow(unused_imports)]

pub(crate) use crate::commands::wrap::exec::wiring::{
    CANCEL_REQUEST_ID, HookDispatchResult, HookOutcome, INITIALIZE_REQUEST_ID, PROMPT_REQUEST_ID,
    WIRE_PROTOCOL_VERSION, WireClientCapabilities, WireRequestDispatch, WireSessionConfig,
    WireSessionWiring, WireWriter, build_approval_response, build_cancel_request,
    build_hook_response, build_initialize_request, build_prompt_request, build_question_response,
    build_synthetic_warning_envelope, build_tool_call_unsupported_error, dispatch_for_request,
    dispatch_hook_request, map_kimi_hook_event, run_kimi_wire_session,
};
