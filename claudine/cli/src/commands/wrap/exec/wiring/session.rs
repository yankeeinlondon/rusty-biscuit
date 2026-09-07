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
    debug_assert!(
        config
            .env
            .get(&OsString::from(claudine::child_environment::AGENT_CWD_ENV))
            .is_some_and(|value| Path::new(value).is_absolute())
    );

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
    claudine::child_environment::contribute_child_environment(&mut command)?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
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

                parser.feed_line(&line);

                // Feed any synthetic diagnostic envelope produced by the
                // request-dispatch path (e.g. hook dispatch failures) so
                // the semantic parser surfaces it as a `SemanticEvent::Warning`.
                if let Some(envelope) = synthetic
                    && let Ok(serialized) = serde_json::to_string(&envelope)
                {
                    parser.feed_line(&serialized);
                }

                // Signal the wait loop the moment the prompt response
                // arrives. `close_stdin` is the graceful path (Kimi exits
                // on EOF), and `prompt_finished` is the hard fallback so
                // the wait loop forces exit if Kimi keeps the channel
                // open after responding. An error response to `init-1` is
                // equally terminal — a server that rejects the handshake
                // never processes the prompt, so waiting on `prompt-2`
                // would hang until the wall-clock timeout.
                if is_prompt_response_line(trimmed) {
                    info!("kimi prompt response received; closing wire stdin");
                    writer_for_reader.close_stdin();
                    prompt_finished_for_reader.store(true, Ordering::SeqCst);
                } else if is_initialize_error_line(trimmed) {
                    info!("kimi initialize error response received; closing wire stdin");
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

    let (early_tx, early_rx) = std::sync::mpsc::channel();
    let (completion_tx, completion_rx) = std::sync::mpsc::channel();
    let wait_done = Arc::new(AtomicBool::new(false));

    if let Some(content_early_rx) = wiring.content_early_rx {
        let tx = early_tx.clone();
        let done = Arc::clone(&wait_done);
        thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match content_early_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(early) => {
                        let _ = tx.send(early);
                        return;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        });
    }

    {
        let done = Arc::clone(&wait_done);
        let prompt_finished = Arc::clone(&prompt_finished);
        thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                if prompt_finished.load(Ordering::SeqCst) {
                    thread::sleep(PROMPT_FINISHED_GRACE);
                    if !done.load(Ordering::SeqCst) {
                        let _ = completion_tx.send(
                            super::super::termination::CompletionTermination,
                        );
                    }
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
        });
    }

    if let Some(timeout) = config.timeout {
        let tx = early_tx.clone();
        let done = Arc::clone(&wait_done);
        let writer = writer.clone();
        thread::spawn(move || {
            thread::sleep(timeout);
            if done.load(Ordering::SeqCst) {
                return;
            }
            let _cancel_span = info_span!("kimi_wire_cancel").entered();
            let envelope = build_cancel_request();
            match writer.send_value(&envelope) {
                Ok(_) => info!("sent kimi wire cancel"),
                Err(error) => warn!(error = %error, "failed to send kimi wire cancel"),
            }
            thread::sleep(Duration::from_secs(5));
            if !done.load(Ordering::SeqCst) {
                let _ = tx.send(EarlyTermination::Timeout {
                    message: "kimi wire timeout elapsed; child did not exit after cancel"
                        .to_string(),
                });
            }
        });
    }

    let kill_grace = super::super::timeouts::TimeoutConfig::resolve(None, None).kill_grace;
    let (exit_code, termination, early_termination) = match super::super::termination::wait_with_signal_early_termination_and_completion(
        &mut child,
        true,
        early_rx,
        None,
        Some(completion_rx),
        kill_grace,
        false,
    ) {
        Ok(result) => result,
        Err(error) => {
            warn!(error = %error, "kimi wire wait loop failed");
            (
                -1,
                claudine::harness::ProcessTermination::Completed,
                None,
            )
        }
    };
    wait_done.store(true, Ordering::SeqCst);

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
        signals: Vec::new(),
    })
}
/// Grace period after the `prompt-2` response arrives before tree termination.
///
/// Kimi's wire session is persistent — it does not exit on its own when a
/// prompt completes. Stdin is already closed (EOF) at this point so a
/// well-behaved Kimi build will quit promptly; the grace period covers
/// any final stderr flush or async cleanup. After the grace period
/// elapses, the shared wait loop terminates the child tree while preserving
/// a completed process outcome.
const PROMPT_FINISHED_GRACE: Duration = Duration::from_millis(750);
