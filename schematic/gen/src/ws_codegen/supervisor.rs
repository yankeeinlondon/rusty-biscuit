//! The connection supervisor and setup session emitted into `ws_shared.rs`.
//!
//! Every generated endpoint client connects through `connect_session`: one
//! attempt (upgrade, retained opening message, setup callback) bounded by the
//! per-attempt timeout, then a spawned supervisor that serves the connection,
//! enforces the inbound idle timeout, and redials with the same attempt under
//! a consecutive-failure budget. The lifecycle is described in
//! `schematic/docs/websocket-definitions.md`.

use proc_macro2::TokenStream;
use quote::quote;

/// The supervisor, its attempt, the setup session types, and the opening
/// frame helper.
pub fn generate_supervisor() -> TokenStream {
    let setup = generate_setup_session();
    let attempt = generate_attempt();
    let supervisor = generate_supervisor_loop();
    quote! {
        /// The transport of one connection.
        pub(crate) type WsStream = tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >;

        /// One inbound event: a frame, or an error the caller is told about.
        type WsEvent = Result<WsInboundFrame, WsError>;

        /// How long a manual close waits for its Close frame to be written.
        const CLOSE_GRACE: std::time::Duration = std::time::Duration::from_millis(500);

        #setup
        #attempt
        #supervisor
    }
}

fn generate_setup_session() -> TokenStream {
    quote! {
        /// Why a setup callback, or one of its session operations, failed.
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        pub enum WsSetupError {
            /// The server refused the session, for example its
            /// authentication. Terminal: the client closes without trying
            /// another credential or connection.
            #[error("setup rejected: {0}")]
            Rejected(String),
            /// A failure a new connection may not repeat; it uses the
            /// reconnect budget.
            #[error("setup failed: {0}")]
            Transient(String),
            /// The peer closed the connection during setup.
            #[error("the connection closed during setup")]
            Closed,
            /// The transport failed during setup.
            #[error("transport error during setup: {0}")]
            Io(String),
            /// A setup message did not encode or decode.
            #[error("setup message codec error: {0}")]
            Codec(String),
            /// The attempt this session belongs to has ended (it finished,
            /// timed out, or the client closed), so the session can no longer
            /// reach the connection.
            #[error("the setup session is no longer valid")]
            SessionInvalid,
        }

        impl WsSetupError {
            /// Whether the failure ends the client: only [`Self::Rejected`].
            #[must_use]
            pub fn is_terminal(&self) -> bool {
                matches!(self, Self::Rejected(_))
            }
        }

        /// Where an attempt's connection is while setup runs.
        enum SetupSlot {
            Idle(Box<WsStream>),
            Busy,
            Revoked,
        }

        type SharedSetupSlot = Arc<Mutex<SetupSlot>>;

        fn lock_slot(slot: &SharedSetupSlot) -> std::sync::MutexGuard<'_, SetupSlot> {
            slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
        }

        /// The untyped half of a setup session: exclusive access to one
        /// attempt's connection until the attempt ends.
        ///
        /// Each operation checks the connection out of a slot shared with the
        /// attempt (no lock is held across an await) and races its I/O
        /// against revocation, so a session moved into another task cannot
        /// keep the connection once its attempt is over.
        pub struct WsSetupChannel {
            slot: SharedSetupSlot,
            revoked: watch::Receiver<bool>,
            credential: schematic_runtime::HandshakeCredential,
            binary: bool,
        }

        impl WsSetupChannel {
            async fn send_message(
                &mut self,
                message: tokio_tungstenite::tungstenite::Message,
            ) -> Result<(), WsSetupError> {
                use futures_util::SinkExt;
                let mut stream = self.checkout()?;
                let sent = tokio::select! {
                    biased;
                    () = revoked(&mut self.revoked) => None,
                    sent = stream.send(message) => Some(sent),
                };
                match sent {
                    None => Err(WsSetupError::SessionInvalid),
                    Some(Err(error)) => Err(WsSetupError::Io(error.to_string())),
                    Some(Ok(())) => self.checkin(stream),
                }
            }

            async fn send_json<T: serde::Serialize + ?Sized>(&mut self, payload: &T) -> Result<(), WsSetupError> {
                let codec = |error: serde_json::Error| WsSetupError::Codec(error.to_string());
                let message = if self.binary {
                    tokio_tungstenite::tungstenite::Message::Binary(serde_json::to_vec(payload).map_err(codec)?.into())
                } else {
                    tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(payload).map_err(codec)?.into())
                };
                self.send_message(message).await
            }

            /// The next data frame; pings and pongs are skipped.
            async fn next_frame(&mut self) -> Result<WsInboundFrame, WsSetupError> {
                let mut stream = self.checkout()?;
                let received = tokio::select! {
                    biased;
                    () = revoked(&mut self.revoked) => None,
                    received = next_data_frame(&mut stream) => Some(received),
                };
                match received {
                    None => Err(WsSetupError::SessionInvalid),
                    Some(Err(error)) => Err(error),
                    Some(Ok(raw)) => {
                        self.checkin(stream)?;
                        WsInboundFrame::parse(raw).map_err(|error| WsSetupError::Codec(error.to_string()))
                    }
                }
            }

            fn checkout(&self) -> Result<Box<WsStream>, WsSetupError> {
                let mut slot = lock_slot(&self.slot);
                match std::mem::replace(&mut *slot, SetupSlot::Busy) {
                    SetupSlot::Idle(stream) => Ok(stream),
                    other => {
                        *slot = other;
                        Err(WsSetupError::SessionInvalid)
                    }
                }
            }

            fn checkin(&self, stream: Box<WsStream>) -> Result<(), WsSetupError> {
                let mut slot = lock_slot(&self.slot);
                if matches!(*slot, SetupSlot::Busy) {
                    *slot = SetupSlot::Idle(stream);
                    Ok(())
                } else {
                    // Revoked while checked out: the stream is dropped here.
                    Err(WsSetupError::SessionInvalid)
                }
            }
        }

        async fn revoked(revoked: &mut watch::Receiver<bool>) {
            // A dropped sender also ends the session.
            let _ = revoked.wait_for(|revoked| *revoked).await;
        }

        async fn next_data_frame(stream: &mut WsStream) -> Result<WsRawFrame, WsSetupError> {
            use futures_util::StreamExt;
            use tokio_tungstenite::tungstenite::Message;
            loop {
                match stream.next().await {
                    None | Some(Ok(Message::Close(_))) => return Err(WsSetupError::Closed),
                    Some(Err(error)) => return Err(WsSetupError::Io(error.to_string())),
                    Some(Ok(Message::Text(text))) => return Ok(WsRawFrame::Text(text.to_string())),
                    Some(Ok(Message::Binary(data))) => return Ok(WsRawFrame::Binary(data.to_vec())),
                    Some(Ok(_)) => {}
                }
            }
        }

        /// The supervisor's side of a setup session's slot. Dropping it (the
        /// attempt failed, timed out, or the client closed) revokes the
        /// session and drops a connection the session is not using.
        struct SetupGuard {
            slot: SharedSetupSlot,
            revoke: watch::Sender<bool>,
        }

        impl SetupGuard {
            fn new(
                stream: WsStream,
                credential: schematic_runtime::HandshakeCredential,
                binary: bool,
            ) -> (WsSetupChannel, Self) {
                let slot = Arc::new(Mutex::new(SetupSlot::Idle(Box::new(stream))));
                let (revoke, revoked) = watch::channel(false);
                let channel = WsSetupChannel { slot: Arc::clone(&slot), revoked, credential, binary };
                (channel, Self { slot, revoke })
            }

            /// Takes the connection back once setup has succeeded.
            fn reclaim(self) -> Result<WsStream, WsError> {
                match std::mem::replace(&mut *lock_slot(&self.slot), SetupSlot::Revoked) {
                    SetupSlot::Idle(stream) => Ok(*stream),
                    _ => Err(WsError::Setup(
                        "setup returned while one of its session operations still held the connection".to_string(),
                    )),
                }
            }
        }

        impl Drop for SetupGuard {
            fn drop(&mut self) {
                let taken = std::mem::replace(&mut *lock_slot(&self.slot), SetupSlot::Revoked);
                drop(taken);
                self.revoke.send_replace(true);
            }
        }

        /// A setup callback's typed access to one connection attempt, after
        /// the upgrade and the opening message and before the client reports
        /// the connection ready. `C` is the endpoint's client message enum,
        /// `S` its server message enum.
        ///
        /// The session is valid only while its callback's attempt lasts; an
        /// operation after that returns [`WsSetupError::SessionInvalid`].
        /// Frames the callback does not read are delivered as events once the
        /// connection is ready.
        pub struct WsSetupSession<C, S> {
            channel: WsSetupChannel,
            decode: fn(WsInboundFrame) -> Result<S, WsError>,
            _client: std::marker::PhantomData<fn(C)>,
        }

        impl<C: serde::Serialize, S> WsSetupSession<C, S> {
            pub(crate) fn new(channel: WsSetupChannel, decode: fn(WsInboundFrame) -> Result<S, WsError>) -> Self {
                Self { channel, decode, _client: std::marker::PhantomData }
            }

            /// The credential selected for this connect, the one the
            /// handshake carried. Its `Debug` never shows the value; read the
            /// key or token with
            /// [`schematic_runtime::HandshakeCredential::expose_secret`].
            #[must_use]
            pub fn credential(&self) -> &schematic_runtime::HandshakeCredential {
                &self.channel.credential
            }

            /// Sends one message, written as its payload alone.
            ///
            /// ## Errors
            ///
            /// [`WsSetupError::Codec`], [`WsSetupError::Io`], or
            /// [`WsSetupError::SessionInvalid`].
            pub async fn send(&mut self, message: C) -> Result<(), WsSetupError> {
                self.channel.send_json(&message).await
            }

            /// Sends an arbitrary JSON value, bypassing the declared messages.
            ///
            /// ## Errors
            ///
            /// As [`Self::send`].
            pub async fn send_value(&mut self, value: serde_json::Value) -> Result<(), WsSetupError> {
                self.channel.send_json(&value).await
            }

            /// Receives the next data frame, decoded as the message its
            /// selectors name. Pings and pongs are skipped.
            ///
            /// ## Errors
            ///
            /// [`WsSetupError::Closed`] when the peer closes,
            /// [`WsSetupError::Codec`] when the frame does not decode,
            /// [`WsSetupError::Io`], or [`WsSetupError::SessionInvalid`].
            pub async fn next_message(&mut self) -> Result<S, WsSetupError> {
                let frame = self.channel.next_frame().await?;
                (self.decode)(frame).map_err(|error| WsSetupError::Codec(error.to_string()))
            }

            /// Receives the next data frame as a JSON value.
            ///
            /// ## Errors
            ///
            /// As [`Self::next_message`].
            pub async fn next_value(&mut self) -> Result<serde_json::Value, WsSetupError> {
                self.channel.next_frame().await.map(WsInboundFrame::into_value)
            }
        }

        impl<C, S> std::fmt::Debug for WsSetupSession<C, S> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("WsSetupSession")
                    .field("credential", &self.channel.credential)
                    .finish_non_exhaustive()
            }
        }

        /// The boxed future a setup callback returns.
        pub(crate) type SetupFuture =
            std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), WsSetupError>> + Send>>;

        /// A setup callback, called once per attempt with a fresh session.
        pub(crate) type SetupFactory = Arc<dyn Fn(WsSetupChannel) -> SetupFuture + Send + Sync>;
    }
}

fn generate_attempt() -> TokenStream {
    quote! {
        /// How a correlated endpoint's answers are recognized.
        #[derive(Clone, Copy)]
        pub(crate) struct CorrelationConfig {
            pub(crate) id_field: &'static str,
            pub(crate) string_ids: bool,
            pub(crate) selected_message: fn(&WsInboundFrame) -> Option<&'static str>,
        }

        /// Everything one explicit connect retains for its redials.
        pub(crate) struct WsSession {
            pub(crate) handshake: WsHandshake,
            /// The opening message, serialized once when connecting and
            /// written first on every connection.
            pub(crate) opening: Option<tokio_tungstenite::tungstenite::Message>,
            pub(crate) setup: Option<SetupFactory>,
            pub(crate) correlation: Option<CorrelationConfig>,
            /// Whether messages are written as binary frames.
            pub(crate) binary: bool,
        }

        /// Serializes an endpoint's opening payload once, binding its
        /// declared credential field to the handshake credential: an absent,
        /// null, or empty field is filled from the selected key or token, an
        /// equal value is kept, and a different value is refused.
        ///
        /// ## Errors
        ///
        /// [`WsError::InvalidCredential`] for a conflicting value (never
        /// showing either value), [`WsError::Protocol`] when a payload that
        /// must carry the credential is not a JSON object, and
        /// [`WsError::Serde`].
        pub(crate) fn opening_message<T: serde::Serialize + ?Sized>(
            payload: &T,
            credential_field: Option<&str>,
            credential: &schematic_runtime::HandshakeCredential,
            binary: bool,
        ) -> Result<tokio_tungstenite::tungstenite::Message, WsError> {
            use tokio_tungstenite::tungstenite::Message;
            let bound = credential_field.zip(credential.expose_secret());
            let encoded = match bound {
                None if binary => return Ok(Message::Binary(serde_json::to_vec(payload)?.into())),
                None => return Ok(Message::Text(serde_json::to_string(payload)?.into())),
                Some((field, secret)) => {
                    let mut value = serde_json::to_value(payload)?;
                    let Some(object) = value.as_object_mut() else {
                        return Err(WsError::Protocol(format!(
                            "the opening payload must be a JSON object to carry the credential in `{field}`"
                        )));
                    };
                    match object.get(field) {
                        None | Some(serde_json::Value::Null) => {}
                        Some(serde_json::Value::String(given)) if given.is_empty() || given == secret => {}
                        Some(_) => {
                            return Err(WsError::InvalidCredential(format!(
                                "the opening payload's `{field}` differs from the credential selected for the handshake; supply the credential once"
                            )));
                        }
                    }
                    object.insert(field.to_string(), serde_json::Value::String(secret.to_string()));
                    value
                }
            };
            if binary {
                Ok(Message::Binary(serde_json::to_vec(&encoded)?.into()))
            } else {
                Ok(Message::Text(serde_json::to_string(&encoded)?.into()))
            }
        }

        /// One attempt within the per-attempt timeout.
        async fn run_attempt(
            session: &WsSession,
            options: &WsClientOptions,
            state: &watch::Sender<WsConnectionState>,
        ) -> Result<WsStream, WsError> {
            let limit = options.handshake_timeout;
            match tokio::time::timeout(limit, attempt(session, options, state)).await {
                Ok(result) => result,
                Err(_) => Err(WsError::HandshakeTimeout(limit)),
            }
        }

        /// Upgrade, write the opening message, run setup, and take the
        /// connection back. Dropping this future revokes the setup session.
        async fn attempt(
            session: &WsSession,
            options: &WsClientOptions,
            state: &watch::Sender<WsConnectionState>,
        ) -> Result<WsStream, WsError> {
            use futures_util::SinkExt;
            let mut stream = session.handshake.dial(options).await?;
            if session.opening.is_some() || session.setup.is_some() {
                state.send_replace(WsConnectionState::Authenticating);
            }
            if let Some(opening) = &session.opening {
                stream
                    .send(opening.clone())
                    .await
                    .map_err(|error| WsError::OpeningWrite(error.to_string()))?;
            }
            if let Some(setup) = &session.setup {
                let (channel, guard) = SetupGuard::new(stream, session.handshake.credential().clone(), session.binary);
                setup(channel).await.map_err(|error| match error {
                    WsSetupError::Rejected(reason) => WsError::SetupRejected(reason),
                    other => WsError::Setup(other.to_string()),
                })?;
                stream = guard.reclaim()?;
            }
            Ok(stream)
        }

        /// Makes the explicit connect's one attempt inline (dropping the
        /// returned future leaves nothing running), then spawns the
        /// supervisor for the ready connection.
        ///
        /// ## Errors
        ///
        /// The attempt's error: a handshake failure,
        /// [`WsError::OpeningWrite`], [`WsError::SetupRejected`],
        /// [`WsError::Setup`], or [`WsError::HandshakeTimeout`].
        pub(crate) async fn connect_session(
            session: WsSession,
            options: WsClientOptions,
        ) -> Result<(WsTransportHandle, mpsc::Receiver<WsEvent>), WsError> {
            let (state_tx, state_rx) = watch::channel(WsConnectionState::Connecting);
            let stream = run_attempt(&session, &options, &state_tx).await?;
            state_tx.send_replace(WsConnectionState::Ready);

            let (writer_tx, writer_rx) = mpsc::channel(options.outbound_capacity);
            let connection = Arc::new(AtomicU64::new(1));
            let (event_tx, event_rx) = mpsc::channel(options.inbound_capacity);
            let (shutdown, shutdown_rx) = watch::channel(false);
            let pending = PendingRequests::default();
            let supervisor = Supervisor {
                session,
                options,
                state: state_tx,
                events: event_tx,
                held: None,
                writer_rx,
                connection: Arc::clone(&connection),
                shutdown: shutdown_rx,
                pending: pending.clone(),
            };
            let supervisor_handle = tokio::spawn(supervisor.run(stream));
            let transport = WsTransportHandle {
                writer_tx,
                connection,
                state_rx,
                shutdown,
                supervisor_handle,
                next_id: Arc::new(AtomicU64::new(1)),
                pending,
            };
            Ok((transport, event_rx))
        }
    }
}

fn generate_supervisor_loop() -> TokenStream {
    quote! {
        /// Why serving a connection ended.
        enum Served {
            /// A manual close or a dropped client.
            Shutdown,
            /// The connection was lost; the error, if any, is reported.
            Lost(Option<WsError>),
        }

        /// Owns the connection after the first `Ready`.
        ///
        /// Events go to a bounded queue. When it is full, at most one event
        /// is held and reading pauses until the consumer makes room; every
        /// other duty (manual close, the idle timeout, writes, state, and
        /// pending requests) carries on meanwhile. A transient error that
        /// arrives while an event is already held is dropped; the state still
        /// shows it.
        struct Supervisor {
            session: WsSession,
            options: WsClientOptions,
            state: watch::Sender<WsConnectionState>,
            events: mpsc::Sender<WsEvent>,
            held: Option<WsEvent>,
            writer_rx: mpsc::Receiver<(u64, WriterCommand)>,
            /// The number of the connection being served or awaited.
            connection: Arc<AtomicU64>,
            shutdown: watch::Receiver<bool>,
            pending: PendingRequests,
        }

        impl Supervisor {
            async fn run(mut self, mut stream: WsStream) {
                loop {
                    let served = self.serve(stream).await;
                    // Every pending request wakes with `Disconnected`; none is re-sent.
                    fail_pending(&self.pending);
                    let Served::Lost(error) = served else {
                        self.finish(None).await;
                        return;
                    };
                    self.state.send_replace(WsConnectionState::Disconnected);
                    // Queued commands belonged to the lost connection.
                    if discard_queued(&mut self.writer_rx) {
                        self.finish(None).await;
                        return;
                    }
                    if let Some(error) = error {
                        self.notify(Err(error));
                    }
                    match self.recover().await {
                        Some(next) => stream = next,
                        None => return,
                    }
                }
            }

            /// Serves one ready connection until it is lost or closed.
            async fn serve(&mut self, stream: WsStream) -> Served {
                use futures_util::StreamExt;
                use tokio_tungstenite::tungstenite::Message;

                let current = self.connection.load(std::sync::atomic::Ordering::SeqCst);
                let (mut write, mut read) = stream.split();
                let idle = self.options.receive_timeout;
                let mut idle_deadline = idle.map(|limit| tokio::time::Instant::now() + limit);
                let mut outgoing: Option<Message> = None;
                let mut flush = false;
                loop {
                    tokio::select! {
                        biased;
                        () = shutdown_requested(&mut self.shutdown) => {
                            self.state.send_replace(WsConnectionState::Closing);
                            let _ = tokio::time::timeout(CLOSE_GRACE, futures_util::SinkExt::close(&mut write)).await;
                            return Served::Shutdown;
                        }
                        () = idle_expired(idle_deadline) => {
                            let limit = idle.unwrap_or_default();
                            return Served::Lost(Some(WsError::ReceiveTimeout(limit)));
                        }
                        delivered = deliver_held(&self.events, &mut self.held), if self.held.is_some() => {
                            if !delivered {
                                return Served::Shutdown;
                            }
                        }
                        written = drive_write(&mut write, &mut outgoing, &mut flush), if outgoing.is_some() || flush => {
                            if let Err(error) = written {
                                return Served::Lost(Some(WsError::Transport(error)));
                            }
                        }
                        command = self.writer_rx.recv(), if outgoing.is_none() => {
                            match command {
                                Some((connection, command)) if connection == current => {
                                    outgoing = Some(command.into_message());
                                }
                                // Sent for a connection that has since been lost.
                                Some(_) => {}
                                None => return Served::Shutdown,
                            }
                        }
                        message = read.next(), if self.held.is_none() => {
                            // Any inbound frame, control frames included, resets the idle limit.
                            idle_deadline = idle.map(|limit| tokio::time::Instant::now() + limit);
                            match message {
                                Some(Ok(Message::Text(text))) => self.inbound(WsRawFrame::Text(text.to_string())),
                                Some(Ok(Message::Binary(data))) => self.inbound(WsRawFrame::Binary(data.to_vec())),
                                // The transport queues the pong; flush it now rather than with the next write.
                                Some(Ok(Message::Ping(_))) => flush = true,
                                Some(Ok(Message::Pong(_) | Message::Frame(_))) => {}
                                Some(Ok(Message::Close(_))) | None => return Served::Lost(None),
                                Some(Err(error)) => return Served::Lost(Some(WsError::Transport(error))),
                            }
                        }
                    }
                }
            }

            fn inbound(&mut self, raw: WsRawFrame) {
                let event = match WsInboundFrame::parse(raw) {
                    Ok(frame) => match self.session.correlation {
                        Some(correlation) => match answer_pending(
                            &self.pending,
                            frame,
                            correlation.id_field,
                            correlation.string_ids,
                            correlation.selected_message,
                        ) {
                            Some(frame) => Ok(frame),
                            None => return,
                        },
                        None => Ok(frame),
                    },
                    Err(error) => Err(error),
                };
                self.notify(event);
            }

            /// Queues `event`, holding it when the queue is full.
            fn notify(&mut self, event: WsEvent) {
                if self.held.is_some() {
                    return;
                }
                if let Err(mpsc::error::TrySendError::Full(event)) = self.events.try_send(event) {
                    self.held = Some(event);
                }
            }

            /// Redials under the reconnect policy. `None` once the client has
            /// closed: no policy, the budget is spent, a terminal rejection,
            /// or a manual close.
            async fn recover(&mut self) -> Option<WsStream> {
                let Some(policy) = self.options.reconnect.clone() else {
                    self.finish(None).await;
                    return None;
                };
                let mut failures: u32 = 0;
                loop {
                    if policy.max_attempts.is_some_and(|max| failures >= max) {
                        self.finish(None).await;
                        return None;
                    }
                    if !self.pause(reconnect_delay(&policy, failures)).await {
                        self.finish(None).await;
                        return None;
                    }
                    self.state.send_replace(WsConnectionState::Connecting);
                    let attempt = tokio::select! {
                        biased;
                        () = shutdown_requested(&mut self.shutdown) => None,
                        result = run_attempt(&self.session, &self.options, &self.state) => Some(result),
                    };
                    match attempt {
                        None => {
                            self.finish(None).await;
                            return None;
                        }
                        Some(Ok(stream)) => {
                            self.connection.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            self.state.send_replace(WsConnectionState::Ready);
                            return Some(stream);
                        }
                        Some(Err(error)) if error.is_terminal() => {
                            self.finish(Some(error)).await;
                            return None;
                        }
                        Some(Err(error)) => {
                            self.state.send_replace(WsConnectionState::Disconnected);
                            self.notify(Err(error));
                            failures = failures.saturating_add(1);
                        }
                    }
                }
            }

            /// Waits out a backoff delay while still delivering a held
            /// event. `false` when the client closed meanwhile.
            async fn pause(&mut self, delay: std::time::Duration) -> bool {
                let sleep = tokio::time::sleep(delay);
                tokio::pin!(sleep);
                loop {
                    tokio::select! {
                        biased;
                        () = shutdown_requested(&mut self.shutdown) => return false,
                        delivered = deliver_held(&self.events, &mut self.held), if self.held.is_some() => {
                            if !delivered {
                                return false;
                            }
                        }
                        () = &mut sleep => return true,
                    }
                }
            }

            /// Publishes `Closed`, then delivers the held event and `error`
            /// before the event stream ends, unless the client closes first.
            async fn finish(&mut self, error: Option<WsError>) {
                fail_pending(&self.pending);
                self.state.send_replace(WsConnectionState::Closed);
                let pending_events = self.held.take().into_iter().chain(error.map(Err));
                for event in pending_events {
                    tokio::select! {
                        biased;
                        () = shutdown_requested(&mut self.shutdown) => return,
                        sent = self.events.send(event) => {
                            if sent.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        }

        async fn shutdown_requested(shutdown: &mut watch::Receiver<bool>) {
            // A dropped client also counts as a close.
            let _ = shutdown.wait_for(|requested| *requested).await;
        }

        async fn idle_expired(deadline: Option<tokio::time::Instant>) {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await,
            }
        }

        /// Sends the held event once the queue has room. `false` when the
        /// consumer is gone.
        async fn deliver_held(events: &mpsc::Sender<WsEvent>, held: &mut Option<WsEvent>) -> bool {
            match events.reserve().await {
                Ok(permit) => {
                    if let Some(event) = held.take() {
                        permit.send(event);
                    }
                    true
                }
                Err(_) => {
                    *held = None;
                    false
                }
            }
        }

        /// Writes `outgoing`, if any, and flushes. Cancel-safe: a frame is
        /// taken only once the sink accepts it, and `flush` stays set until
        /// the flush completes, which also sends any queued pong.
        async fn drive_write(
            write: &mut futures_util::stream::SplitSink<WsStream, tokio_tungstenite::tungstenite::Message>,
            outgoing: &mut Option<tokio_tungstenite::tungstenite::Message>,
            flush: &mut bool,
        ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
            use futures_util::Sink;
            std::future::poll_fn(|cx| {
                let mut sink = std::pin::Pin::new(&mut *write);
                if outgoing.is_some() {
                    std::task::ready!(sink.as_mut().poll_ready(cx))?;
                    if let Some(message) = outgoing.take() {
                        sink.as_mut().start_send(message)?;
                    }
                    *flush = true;
                }
                std::task::ready!(sink.as_mut().poll_flush(cx))?;
                *flush = false;
                std::task::Poll::Ready(Ok(()))
            })
            .await
        }
    }
}
