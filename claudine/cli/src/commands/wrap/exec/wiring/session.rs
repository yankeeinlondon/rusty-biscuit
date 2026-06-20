//! (split out of `wiring/mod.rs`; see that file for the protocol overview)
#![allow(unused_imports)]
use super::*;

/// Configuration for a Kimi wire-mode session.
pub(crate) struct WireSessionConfig<'a> {
    pub binary: &'a Path,
    pub args: &'a [String],
    pub env: &'a HashMap<OsString, OsString>,
    pub cwd: &'a Path,
    pub prompt: String,
    pub timeout: Option<Duration>,
    pub client_name: &'a str,
    pub client_version: &'a str,
    pub capabilities: WireClientCapabilities,
    pub env_context: EnvironmentContext,
}

/// Live wiring threaded into the session. The parser builder, stream
/// output coordinator, and metrics handle are owned by the same surface
/// that already powers `run_child_stream_semantic`, so wire mode reuses
/// stderr coordination, JSONL logging, and tracing spans.
pub(crate) struct WireSessionWiring {
    pub build_parser: SemanticParserBuilder,
    pub stream_output: Arc<StreamOutput>,
    pub live_metrics: LiveMetrics,
    pub runtime_context: claudine::dispatch::DispatchRuntimeContext,
    pub content_early_rx: Option<std::sync::mpsc::Receiver<EarlyTermination>>,
}

/// Run the full Kimi wire-mode lifecycle: spawn child, send initialize,
/// send prompt, route auto-responses, dispatch hooks, handle
/// cancellation, drain stdout/stderr, and finalize the parser into a
/// [`StreamExecutionSummary`].
pub(crate) fn run_kimi_wire_session(
    config: WireSessionConfig<'_>,
    wiring: WireSessionWiring,
    child_spawned: &mut bool,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(config.env.contains_key(&OsString::from("PATH")));
    debug_assert!(config.env.contains_key(&OsString::from("HOME")));

    let started_at = Instant::now();
    let span = info_span!("kimi_wire_session");
    let _guard = span.enter();
    let _ = wiring.live_metrics; // reserved for Phase 4 wiring of stall detection

    // Spawn child with stdin/stdout/stderr piped.
    let mut command = Command::new(config.binary);
    command
        .args(config.args)
        .env_clear()
        .envs(config.env)
        .current_dir(config.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn()?;
    let captured_pid = child.id();
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(captured_pid));

    let stdin = child
        .stdin
        .take()
        .expect("child stdin must be piped: Stdio::piped() set above");
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() set above");
    let stderr_pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() set above");

    let writer = WireWriter::from_child_stdin(stdin);

    // Forward stderr verbatim so kimi panics still surface.
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "{line}");
        }
        captured
    });

    // Stdout reader thread: feeds the semantic parser, also classifies
    // each envelope inline so the writer can auto-respond to requests
    // without round-tripping through the parser sink.
    let writer_for_reader = writer.clone();
    let runtime_handle = tokio::runtime::Handle::try_current().ok();
    let runtime_context_for_reader = wiring.runtime_context.clone();
    let env_context_for_reader = config.env_context.clone();
    let stream_span = Span::current();
    let stream_output = wiring.stream_output.clone();
    let prompt_finished = Arc::new(AtomicBool::new(false));
    let prompt_finished_for_reader = Arc::clone(&prompt_finished);
    let stdout_handle: thread::JoinHandle<Box<dyn SemanticStreamParser>> = {
        let build_parser = wiring.build_parser;
        thread::spawn(move || {
            let _stream_guard = stream_span.enter();
            let _parse_span = info_span!("kimi_wire_stdout").entered();
            let reader = BufReader::new(stdout_pipe);
            let mut out = stream_output.stdout_writer();

            let output_cb: OutputTextCallback = Box::new(move |chunk: &str| {
                if !chunk.is_empty() {
                    let _ = out.write_all(chunk.as_bytes());
                }
            });
            let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});
            let mut parser: Box<dyn SemanticStreamParser> =
                build_parser(output_cb, reasoning_cb, Some(captured_pid));

            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let synthetic = handle_request_dispatch(
                    trimmed,
                    &writer_for_reader,
                    runtime_handle.as_ref(),
                    &runtime_context_for_reader,
                    &env_context_for_reader,
                    Some(captured_pid),
                );

                match parser.feed_line(&line) {
                    Ok(()) => {}
                    Err(StreamParseError::MalformedLine { .. }) => {
                        debug!("skipping malformed kimi wire line: {line}");
                    }
                    Err(StreamParseError::Fatal(error)) => {
                        warn!(error = %error, "kimi wire parser fatal error; continuing");
                    }
                }

                // Feed any synthetic diagnostic envelope produced by the
                // request-dispatch path (e.g. hook dispatch failures) so
                // the semantic parser surfaces it as a `SemanticEvent::Warning`.
                if let Some(envelope) = synthetic
                    && let Ok(serialized) = serde_json::to_string(&envelope)
                    && let Err(error) = parser.feed_line(&serialized)
                {
                    debug!(?error, "failed to feed synthetic warning envelope");
                }

                // Signal the wait loop the moment the prompt response
                // arrives. `close_stdin` is the graceful path (Kimi exits
                // on EOF), and `prompt_finished` is the hard fallback so
                // the wait loop forces exit if Kimi keeps the channel
                // open after responding.
                if is_prompt_response_line(trimmed) {
                    info!("kimi prompt response received; closing wire stdin");
                    writer_for_reader.close_stdin();
                    prompt_finished_for_reader.store(true, Ordering::SeqCst);
                }
            }

            parser
        })
    };

    // Send initialize and prompt on the main thread so we hit a single
    // tracing span for handshake timing.
    {
        let _init_span = info_span!("kimi_wire_initialize").entered();
        let initialize = build_initialize_request(
            config.client_name,
            config.client_version,
            config.capabilities,
        );
        if let Err(error) = writer.send_value(&initialize) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }

    {
        let _prompt_span = info_span!("kimi_wire_prompt_send").entered();
        let prompt_request = build_prompt_request(&config.prompt);
        if let Err(error) = writer.send_value(&prompt_request) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }

    // SIGINT forwarder: flip the cancel flag so the wait loop sends the
    // JSON-RPC cancel before falling back to SIGTERM/SIGKILL.
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let signal_guard = install_sigint_forwarder(Arc::clone(&cancel_requested));

    let (exit_code, early_termination) = match wait_for_child_exit(
        &mut child,
        config.timeout,
        &cancel_requested,
        &prompt_finished,
        &writer,
        wiring.content_early_rx,
    ) {
        Ok(result) => result,
        Err(error) => {
            warn!(error = %error, "kimi wire wait loop failed");
            (-1, None)
        }
    };

    let _ = signal_guard;

    let parser = match stdout_handle.join() {
        Ok(parser) => parser,
        Err(_) => {
            return Err(color_eyre::eyre::eyre!(
                "kimi wire stdout reader thread panicked"
            ));
        }
    };

    let stderr_text = stderr_handle.join().unwrap_or_default();

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }
    if !stderr_text.is_empty() && summary.stderr_text.is_none() {
        summary.stderr_text = Some(stderr_text);
    }
    if let Some(termination) = early_termination.as_ref() {
        super::super::termination::apply_early_termination_to_summary(&mut summary, termination);
    }

    let total_elapsed = started_at.elapsed();
    let termination = if early_termination.is_some() {
        super::super::termination::early_termination_process_outcome(early_termination.as_ref())
    } else if cancel_requested.load(Ordering::SeqCst) {
        claudine::harness::ProcessTermination::Interrupted
    } else {
        claudine::harness::ProcessTermination::Completed
    };
    let guard_context = early_termination
        .as_ref()
        .and_then(super::super::termination::early_termination_guard_context);
    Ok(ProcessResult {
        data: summary,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: None,
        },
        agent_pid: Some(captured_pid),
        guard_context,
    })
}
#[cfg(unix)]
fn install_sigint_forwarder(flag: Arc<AtomicBool>) -> Option<signal_hook::SigId> {
    // SAFETY: `signal_hook::low_level::register` requires the closure to be
    // async-signal-safe; only an atomic store is performed.
    let register = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            flag.store(true, Ordering::SeqCst);
        })
    };
    match register {
        Ok(id) => Some(id),
        Err(error) => {
            warn!(error = %error, "failed to install SIGINT handler for kimi wire session");
            None
        }
    }
}

#[cfg(not(unix))]
fn install_sigint_forwarder(_flag: Arc<AtomicBool>) -> Option<()> {
    None
}

/// Grace period after the `prompt-2` response arrives before SIGKILL.
///
/// Kimi's wire session is persistent — it does not exit on its own when a
/// prompt completes. Stdin is already closed (EOF) at this point so a
/// well-behaved Kimi build will quit promptly; the grace period covers
/// any final stderr flush or async cleanup. After the grace period
/// elapses, the child is killed unconditionally.
const PROMPT_FINISHED_GRACE: Duration = Duration::from_millis(750);

/// Poll the child for exit, sending `cancel` when the cancel flag is set
/// or the wall-clock timeout elapses, and forcibly terminating the child
/// shortly after the prompt response arrives so the non-interactive
/// wrapper does not hang on Kimi's persistent JSON-RPC session.
fn wait_for_child_exit(
    child: &mut Child,
    timeout: Option<Duration>,
    cancel_flag: &Arc<AtomicBool>,
    prompt_finished: &Arc<AtomicBool>,
    writer: &WireWriter,
    content_early_rx: Option<std::sync::mpsc::Receiver<EarlyTermination>>,
) -> std::io::Result<(i32, Option<EarlyTermination>)> {
    let deadline = timeout.map(|d| Instant::now() + d);
    let mut cancel_sent = false;
    let mut cancel_sent_at: Option<Instant> = None;
    let mut prompt_finished_at: Option<Instant> = None;
    let mut content_early_rx = content_early_rx;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status_to_code(&status), None));
        }

        if let Some(rx) = content_early_rx.as_ref() {
            match rx.try_recv() {
                Ok(early) => {
                    info!("kimi wire content guard tripped; terminating child");
                    let status = terminate_child_for_content_trip(child)?;
                    return Ok((status_to_code(&status), Some(early)));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    content_early_rx = None;
                }
            }
        }

        let timeout_elapsed = deadline.is_some_and(|d| Instant::now() >= d);
        let user_canceled = cancel_flag.load(Ordering::SeqCst);
        let prompt_done = prompt_finished.load(Ordering::SeqCst);

        if prompt_done && prompt_finished_at.is_none() {
            prompt_finished_at = Some(Instant::now());
        }

        // Hard-stop fallback: Kimi's wire mode does not terminate when a
        // prompt completes — stdin EOF is the expected signal but some
        // builds keep async tasks alive. Once the grace period elapses,
        // kill the child directly. Report exit code 0 because the prompt
        // already completed; the semantic parser surfaces real errors
        // (auth-expired, cancelled, etc.) from the response payload, not
        // from the synthetic SIGKILL exit code.
        if let Some(at) = prompt_finished_at
            && Instant::now() >= at + PROMPT_FINISHED_GRACE
        {
            info!("kimi prompt finished; terminating child after grace period");
            let _ = child.kill();
            let _ = child.wait()?;
            return Ok((0, None));
        }

        if !cancel_sent && (timeout_elapsed || user_canceled) {
            let _cancel_span = info_span!("kimi_wire_cancel").entered();
            let envelope = build_cancel_request();
            match writer.send_value(&envelope) {
                Ok(_) => info!("sent kimi wire cancel"),
                Err(error) => warn!(error = %error, "failed to send kimi wire cancel"),
            }
            cancel_flag.store(true, Ordering::SeqCst);
            cancel_sent = true;
            cancel_sent_at = Some(Instant::now());
        }

        if let Some(sent_at) = cancel_sent_at
            && Instant::now() >= sent_at + Duration::from_secs(5)
        {
            warn!("kimi child did not exit 5s after cancel; sending SIGKILL");
            let _ = child.kill();
            let status = child.wait()?;
            return Ok((status_to_code(&status), None));
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn terminate_child_for_content_trip(
    child: &mut Child,
) -> std::io::Result<std::process::ExitStatus> {
    let child_pid = child.id();
    super::super::termination::send_signal_to_child(child_pid, true, libc::SIGTERM);
    let grace_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= grace_deadline {
            warn!("kimi child did not exit after content-guard SIGTERM; sending SIGKILL");
            let _ = child.kill();
            return child.wait();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(unix))]
fn terminate_child_for_content_trip(
    child: &mut Child,
) -> std::io::Result<std::process::ExitStatus> {
    let _ = child.kill();
    child.wait()
}

fn status_to_code(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}
