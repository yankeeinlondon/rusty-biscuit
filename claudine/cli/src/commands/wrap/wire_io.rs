//! Re-export bridge: the canonical wire I/O module now lives in
//! `exec::wiring`. This file preserves backward compatibility
//! for callers that still reference `wrap::wire_io` until the
//! migration is complete.
#![allow(unused_imports)]

pub(crate) use crate::commands::wrap::exec::wiring::{
    WIRE_PROTOCOL_VERSION, INITIALIZE_REQUEST_ID, PROMPT_REQUEST_ID, CANCEL_REQUEST_ID,
    WireClientCapabilities, build_initialize_request, build_prompt_request, build_cancel_request,
    build_approval_response, build_question_response, build_tool_call_unsupported_error,
    build_hook_response, HookOutcome, HookDispatchResult, WireWriter, WireRequestDispatch,
    dispatch_for_request, map_kimi_hook_event, dispatch_hook_request,
    validate_initialize_response, WireInitError, WireSessionConfig, WireSessionWiring,
    run_kimi_wire_session, build_synthetic_warning_envelope,
};
