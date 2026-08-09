use crate::config;
use crate::daemon::client::ping_socket;
use crate::daemon::protocol::{parse_request, serialize_event, Event, Request};
use crate::daemon::stream::{self, StreamSession};
use crate::engine::{self, EngineSettings};
use master_voice_audio::{PlaybackOutcome, PlaybackThread};
use master_voice_linguistics::lang::Language;
use master_voice_linguistics::overrides::Overrides;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{mpsc as std_mpsc, Arc};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot, Notify};

pub struct DaemonConfig {
    pub device: Option<String>,
    pub queue_limit: usize,
    pub idle_timeout: Duration,
}

struct State {
    settings: EngineSettings,
    overrides: Overrides,
    config: DaemonConfig,
    controller: Mutex<Option<PlaybackThread>>,
    /// In-flight utterances (one-shot or stream), keyed by utterance id.
    active: Mutex<HashMap<u64, StreamSession>>,
    /// Stream key -> utterance id, for the word-append path.
    streams: Mutex<HashMap<String, u64>>,
    last_activity: Mutex<Instant>,
    shutting_down: AtomicBool,
    shutdown: Notify,
}

fn push_to_controller(
    state: &State,
    id: u64,
    owner: u64,
    samples: Vec<f32>,
    rate: u32,
    interrupt: bool,
) -> Result<Receiver<master_voice_audio::PlaybackOutcome>, String> {
    let mut guard = state.controller.lock();
    if guard.is_none() {
        *guard = Some(PlaybackThread::spawn(
            state.config.device.clone(),
            state.config.queue_limit,
        ));
    }
    guard
        .as_ref()
        .unwrap()
        .push(id, owner, samples, rate, interrupt)
        .map_err(|e| e.to_string())
}

fn queue_len(state: &State) -> usize {
    state
        .controller
        .lock()
        .as_ref()
        .map(|thread| thread.queue_len())
        .unwrap_or(0)
}

fn interrupt_utterance(state: &State, id: u64) {
    if let Some(thread) = state.controller.lock().as_ref() {
        thread.interrupt_utterance(id);
    }
}

/// Cancel an utterance: flag it, wake its producer, interrupt its queued
/// audio, and unregister its stream key.
fn cancel_utterance(state: &State, id: u64) {
    let stream_key = {
        let active = state.active.lock();
        match active.get(&id) {
            Some(session) => {
                session.cancelled.store(true, Ordering::SeqCst);
                session.notify.notify_one();
                session.stream_key.clone()
            }
            None => None,
        }
    };
    if let Some(key) = stream_key {
        let mut streams = state.streams.lock();
        if streams.get(&key) == Some(&id) {
            streams.remove(&key);
        }
    }
    interrupt_utterance(state, id);
}

fn cancel_connection_utterances(state: &State, conn_id: u64) {
    // Only one-shot utterances are cancelled on connection EOF. Stream
    // sessions survive their owner's disconnect: the per-word MCP path
    // opens a fresh connection per word and closes it after `Queued`, and
    // the producer's auto-finalize (STREAM_IDLE_MS) already protects
    // against crashed clients wedging a session.
    let ids: Vec<u64> = state
        .active
        .lock()
        .iter()
        .filter(|(_, s)| s.conn_id == conn_id && s.stream_key.is_none())
        .map(|(id, _)| *id)
        .collect();
    for id in ids {
        cancel_utterance(state, id);
    }
}

async fn idle_wait(state: &State) {
    loop {
        let deadline = *state.last_activity.lock() + state.config.idle_timeout;
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        tokio::time::sleep(deadline - now).await;
        if state.shutting_down.load(Ordering::SeqCst) {
            return;
        }
    }
}

static NEXT_CONN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub async fn run(
    settings: EngineSettings,
    overrides: Overrides,
    config: DaemonConfig,
) -> Result<(), String> {
    let socket = config::socket_path();
    if socket.exists() {
        if ping_socket(&socket) {
            return Ok(());
        }
        let _ = std::fs::remove_file(&socket);
    }

    let listener =
        UnixListener::bind(&socket).map_err(|e| format!("bind {}: {e}", socket.display()))?;
    tracing::info!("daemon listening on {}", socket.display());

    let state = Arc::new(State {
        settings,
        overrides,
        config,
        controller: Mutex::new(None),
        active: Mutex::new(HashMap::new()),
        streams: Mutex::new(HashMap::new()),
        last_activity: Mutex::new(Instant::now()),
        shutting_down: AtomicBool::new(false),
        shutdown: Notify::new(),
    });

    {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut sigterm =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                    Ok(s) => s,
                    Err(_) => return,
                };
            let mut sigint =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                    Ok(s) => s,
                    Err(_) => return,
                };
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sigint.recv() => {}
            }
            state.shutting_down.store(true, Ordering::SeqCst);
            state.shutdown.notify_waiters();
        });
    }

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(pair) => pair,
                    Err(e) => {
                        tracing::warn!("accept error: {e}");
                        continue;
                    }
                };
                *state.last_activity.lock() = Instant::now();
                let state = Arc::clone(&state);
                let conn_id = NEXT_CONN.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    handle_connection(stream, state, conn_id).await;
                });
            }
            _ = state.shutdown.notified() => break,
            _ = idle_wait(&state) => {
                tracing::info!("idle timeout, shutting down");
                break;
            }
        }
    }

    if let Some(mut thread) = state.controller.lock().take() {
        thread.stop();
    }
    let _ = std::fs::remove_file(&socket);
    Ok(())
}

async fn handle_connection(stream: UnixStream, state: Arc<State>, conn_id: u64) {
    let (reader, mut writer) = tokio::io::split(stream);
    let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);
    let (line_tx, mut line_rx) = mpsc::channel::<String>(16);
    let (eof_tx, mut eof_rx) = oneshot::channel::<()>();

    // Single writer task serializes events from the main loop and any
    // number of producer tasks.
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = writer.write_all(serialize_event(&event).as_bytes()).await;
            let _ = writer.write_all(b"\n").await;
            let _ = writer.flush().await;
        }
    });

    {
        let line_tx = line_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => {
                        let _ = eof_tx.send(());
                        break;
                    }
                    Ok(_) => {
                        if line_tx.send(line.trim().to_string()).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    loop {
        tokio::select! {
            Some(line) = line_rx.recv() => {
                if line.is_empty() {
                    continue;
                }
                match parse_request(&line) {
                    Some(Request::Ping { id }) => {
                        let _ = event_tx.send(Event::Pong { id }).await;
                    }
                    Some(Request::Shutdown { id }) => {
                        let _ = event_tx.send(Event::ShutdownAck { id }).await;
                        state.shutting_down.store(true, Ordering::SeqCst);
                        state.shutdown.notify_waiters();
                        return;
                    }
                    Some(Request::Cancel { id }) => {
                        cancel_utterance(&state, id);
                    }
                    Some(Request::Speak {
                        id,
                        text,
                        language,
                        interrupt,
                        robotic,
                        stream,
                        last,
                    }) => {
                        *state.last_activity.lock() = Instant::now();
                        dispatch_speak(
                            id,
                            text,
                            language,
                            interrupt,
                            robotic,
                            stream,
                            last,
                            conn_id,
                            &state,
                            event_tx.clone(),
                        )
                        .await;
                    }
                    None => {
                        let _ = event_tx
                            .send(Event::Error {
                                id: 0,
                                message: "invalid request".to_string(),
                                code: 1,
                            })
                            .await;
                    }
                }
            }
            _ = &mut eof_rx => {
                cancel_connection_utterances(&state, conn_id);
                return;
            }
        }
    }
}

/// Route a `Speak` request: one-shot utterance, stream start, or
/// word-append into a live stream.
#[allow(clippy::too_many_arguments)]
async fn dispatch_speak(
    id: u64,
    text: String,
    language: Option<String>,
    interrupt: bool,
    robotic: Option<f32>,
    stream: Option<String>,
    last: Option<bool>,
    conn_id: u64,
    state: &Arc<State>,
    event_tx: mpsc::Sender<Event>,
) {
    let language_parsed = language.as_deref().and_then(Language::from_code);
    if language.is_some() && language_parsed.is_none() {
        let _ = event_tx
            .send(Event::Error {
                id,
                message: format!("unsupported language {language:?}"),
                code: 3,
            })
            .await;
        return;
    }

    if let Some(key) = &stream {
        // Word-append path: push into a live session, or start one.
        let utterance_id = state.streams.lock().get(key).copied();
        let append = match utterance_id {
            Some(utterance_id) => {
                let mut active = state.active.lock();
                match active.get_mut(&utterance_id) {
                    Some(session) if session.text_tx.is_some() => {
                        let is_final = last.unwrap_or(false);
                        let tx = session.text_tx.clone();
                        if is_final {
                            // Close the stream: dropping the stored sender
                            // lets the producer drain + finish.
                            session.text_tx = None;
                            state.streams.lock().remove(key);
                        }
                        Some((tx, is_final))
                    }
                    _ => None,
                }
            }
            None => None,
        };
        match append {
            Some((tx, _is_final)) => {
                let mut padded = String::new();
                if !text.is_empty() {
                    padded.push_str(&text);
                    if !padded.ends_with(char::is_whitespace) {
                        // Keep a word gap between appends.
                        padded.push(' ');
                    }
                }
                if let Some(tx) = &tx {
                    if !padded.trim().is_empty() {
                        let _ = tx.send(padded).await;
                    }
                }
                let _ = event_tx.send(Event::Queued { id }).await;
                tracing::debug!("stream {key} append queued");
            }
            None => {
                start_stream(
                    id,
                    text,
                    language_parsed,
                    interrupt,
                    robotic,
                    Some(key.clone()),
                    last,
                    conn_id,
                    state,
                    event_tx,
                )
                .await;
            }
        }
        return;
    }

    // One-shot utterance.
    if text.trim().is_empty() {
        let _ = event_tx
            .send(Event::Error {
                id,
                message: "no speakable text".to_string(),
                code: 3,
            })
            .await;
        return;
    }
    start_stream(
        id,
        text,
        language_parsed,
        interrupt,
        robotic,
        None,
        None,
        conn_id,
        state,
        event_tx,
    )
    .await;
}

/// Register an utterance and spawn its chunked producer.
#[allow(clippy::too_many_arguments)]
async fn start_stream(
    id: u64,
    text: String,
    language: Option<Language>,
    interrupt: bool,
    robotic: Option<f32>,
    stream_key: Option<String>,
    last: Option<bool>,
    conn_id: u64,
    state: &Arc<State>,
    event_tx: mpsc::Sender<Event>,
) {
    let (text_tx, text_rx) = mpsc::channel::<String>(64);
    let cancelled = Arc::new(AtomicBool::new(false));
    let notify = Arc::new(Notify::new());
    let session = Arc::new(StreamSession::new(
        id,
        conn_id,
        stream_key.clone(),
        Some(text_tx.clone()),
        Arc::clone(&cancelled),
        Arc::clone(&notify),
    ));

    {
        let mut active = state.active.lock();
        let _ = active.insert(id, (*session).clone());
        if let Some(key) = &stream_key {
            state.streams.lock().insert(key.clone(), id);
        }
    }

    if !text.is_empty() {
        let _ = text_tx.send(text).await;
    }
    // One-shot utterances and final streams have no further input: close
    // the channel so the producer drains to Done immediately.
    if stream_key.is_none() || last.unwrap_or(false) {
        if let Some(s) = state.active.lock().get_mut(&id) {
            s.text_tx = None;
        }
        if let Some(key) = &stream_key {
            state.streams.lock().remove(key);
        }
    }

    if let Some(key) = &stream_key {
        tracing::debug!("stream {key} started (utterance {id})");
    }

    let state = Arc::clone(state);
    let producer_tx = event_tx.clone();
    tokio::spawn(async move {
        run_producer(
            state,
            id,
            conn_id,
            interrupt,
            language,
            robotic,
            session,
            Some(text_rx),
            producer_tx,
        )
        .await;
    });

    // The word-append contract: every stream call (start or append)
    // answers `Queued` as soon as the text is accepted. One-shot
    // utterances never see it — their clients wait for `Done`.
    if stream_key.is_some() {
        let _ = event_tx.send(Event::Queued { id }).await;
    }
}

/// The chunked producer (Step 7d): synthesize and push chunks as they
/// become available, backpressure to `QUEUE_AHEAD`, honour cancel, and
/// emit `Accepted` once and `Done` once.
#[allow(clippy::too_many_arguments)]
async fn run_producer(
    state: Arc<State>,
    id: u64,
    conn_id: u64,
    interrupt: bool,
    language: Option<Language>,
    robotic: Option<f32>,
    session: Arc<StreamSession>,
    mut text_rx: Option<mpsc::Receiver<String>>,
    event_tx: mpsc::Sender<Event>,
) {
    let mut settings = state.settings.clone();
    if let Some(v) = robotic {
        settings.robotic_depth = v.clamp(0.0, 1.0);
    }
    let synth = Arc::new(tokio::sync::Mutex::new(engine::StreamSynth::new(&settings)));
    let overrides = state.overrides.clone();

    let mut pending = String::new();
    let mut first = true;
    let mut closed = text_rx.is_none();
    let mut accepted_sent = false;
    let mut language_used = language;
    let mut duration_s = 0.0f32;
    let mut final_rx: Option<Receiver<PlaybackOutcome>> = None;

    loop {
        // Drain appended text (if any).
        if let Some(rx) = &mut text_rx {
            loop {
                match rx.try_recv() {
                    Ok(text) => pending.push_str(&text),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                }
            }
        }

        // Produce chunks while text is available.

        while !pending.trim().is_empty() && !session.cancelled.load(Ordering::SeqCst) {
            let (chunk, rest) = stream::next_chunk(&pending, first);
            let chunk_owned = chunk.to_string();
            let rest_owned = rest.to_string();
            if chunk_owned.trim().is_empty() {
                pending = rest_owned;
                continue;
            }
            pending = rest_owned;
            let chunk_text = chunk_owned;
            let chunk_language = language_used;
            let chunk_overrides = overrides.clone();
            let chunk_first = first;
            let chunk_last = closed && pending.trim().is_empty();
            first = false;

            // Synthesize the chunk off the async workers (cancellable).
            let synth_clone = Arc::clone(&synth);
            let synth_result = tokio::select! {
                biased;
                _ = session.notify.notified() => {
                    if session.cancelled.load(Ordering::SeqCst) {
                        finish(id, &state, &session, None, event_tx.clone()).await;
                        return;
                    }
                    continue;
                }
                result = tokio::task::spawn_blocking(move || {
                    let mut guard = synth_clone.blocking_lock();
                    guard.chunk(&chunk_text, chunk_language, &chunk_overrides, chunk_last)
                }) => match result {
                    Ok(out) => out,
                    Err(e) => {
                        let _ = event_tx
                            .send(Event::Error {
                                id,
                                message: format!("synthesis task failed: {e}"),
                                code: 4,
                            })
                            .await;
                        finish(id, &state, &session, None, event_tx).await;
                        return;
                    }
                },
            };
            let (language_new, samples, synth_ms) = match synth_result {
                Ok(v) => v,
                Err(e) => {
                    let code = e.exit_code();
                    let _ = event_tx
                        .send(Event::Error {
                            id,
                            message: e.to_string(),
                            code,
                        })
                        .await;
                    finish(id, &state, &session, None, event_tx).await;
                    return;
                }
            };
            if language_used.is_none() {
                language_used = Some(language_new);
            }
            duration_s += samples.len() as f32 / master_voice_synth::params::SAMPLE_RATE as f32;
            if samples.is_empty() && !chunk_last {
                continue;
            }

            // Backpressure: keep at most QUEUE_AHEAD chunks queued ahead.
            while !session.cancelled.load(Ordering::SeqCst)
                && queue_len(&state) >= stream::QUEUE_AHEAD
            {
                tokio::select! {
                    biased;
                    _ = session.notify.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(5)) => {}
                }
            }
            if session.cancelled.load(Ordering::SeqCst) {
                finish(id, &state, &session, None, event_tx).await;
                return;
            }

            // Push (first chunk honours the request's interrupt flag).
            let receiver = match push_with_retry(
                &state,
                id,
                conn_id,
                samples,
                chunk_first && interrupt,
                &session,
                &event_tx,
            )
            .await
            {
                Some(receiver) => receiver,
                None => return,
            };

            if !accepted_sent {
                accepted_sent = true;
                let _ = event_tx
                    .send(Event::Accepted {
                        id,
                        language: language_used
                            .unwrap_or(Language::English)
                            .code()
                            .to_string(),
                        duration_s,
                        synth_ms,
                    })
                    .await;
            }

            if chunk_last {
                final_rx = Some(receiver);
            }
        }

        if session.cancelled.load(Ordering::SeqCst) {
            finish(id, &state, &session, None, event_tx).await;
            return;
        }

        if pending.trim().is_empty() && closed {
            let synth_clone = Arc::clone(&synth);
            let flush_overrides = overrides.clone();
            let flush_language = language_used;
            let flushed = tokio::task::spawn_blocking(move || {
                let mut guard = synth_clone.blocking_lock();
                guard.chunk("", flush_language, &flush_overrides, true)
            })
            .await;
            if let Ok(Ok((language_new, samples, synth_ms))) = flushed {
                if language_used.is_none() {
                    language_used = Some(language_new);
                }
                if !samples.is_empty() {
                    duration_s +=
                        samples.len() as f32 / master_voice_synth::params::SAMPLE_RATE as f32;
                    match push_with_retry(
                        &state,
                        id,
                        conn_id,
                        samples,
                        !accepted_sent && interrupt,
                        &session,
                        &event_tx,
                    )
                    .await
                    {
                        Some(receiver) => {
                            if !accepted_sent {
                                let _ = event_tx
                                    .send(Event::Accepted {
                                        id,
                                        language: language_used
                                            .unwrap_or(Language::English)
                                            .code()
                                            .to_string(),
                                        duration_s,
                                        synth_ms,
                                    })
                                    .await;
                            }
                            final_rx = Some(receiver);
                        }
                        None => return,
                    }
                }
            }
            break;
        }

        if !pending.trim().is_empty() {
            // More text to chunk: loop back to the producer.
            continue;
        }

        // Pending is empty and the stream is still open: wait for input,
        // stream close, idle finalize, or cancel.
        match &mut text_rx {
            None => break,
            Some(rx) => {
                tokio::select! {
                    biased;
                    _ = session.notify.notified() => {
                        if session.cancelled.load(Ordering::SeqCst) {
                            finish(id, &state, &session, None, event_tx).await;
                            return;
                        }
                    }
                    res = rx.recv() => match res {
                        Some(text) => pending.push_str(&text),
                        None => closed = true,
                    },
                    _ = tokio::time::sleep(Duration::from_millis(stream::STREAM_IDLE_MS)) => {
                        // Auto-finalize: a crashed client can never wedge us.
                        closed = true;
                    }
                }
            }
        }
    }

    // All text consumed: poll the final chunk's receiver to Done.
    let status = poll_final(&session, final_rx).await;
    finish(id, &state, &session, Some(status), event_tx).await;
}

/// Push one chunk, retrying `QueueFull` with 5 ms backoff for up to 2 s.
/// Returns `None` (and sends `Error`) on failure or cancellation.
async fn push_with_retry(
    state: &State,
    id: u64,
    conn_id: u64,
    samples: Vec<f32>,
    interrupt: bool,
    session: &Arc<StreamSession>,
    event_tx: &mpsc::Sender<Event>,
) -> Option<Receiver<PlaybackOutcome>> {
    for attempt in 0..400 {
        match push_to_controller(
            state,
            id,
            conn_id,
            samples.clone(),
            master_voice_synth::params::SAMPLE_RATE,
            interrupt && attempt == 0,
        ) {
            Ok(receiver) => return Some(receiver),
            Err(message) => {
                if !message.contains("queue is full") {
                    let code = 5;
                    let _ = event_tx.send(Event::Error { id, message, code }).await;
                    return None;
                }
                if attempt == 399 {
                    let _ = event_tx
                        .send(Event::Error {
                            id,
                            message,
                            code: 7,
                        })
                        .await;
                    return None;
                }
                tokio::select! {
                    biased;
                    _ = session.notify.notified() => {
                        if session.cancelled.load(Ordering::SeqCst) {
                            return None;
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(5)) => {}
                }
            }
        }
    }
    None
}

/// Poll the final chunk's playback receiver (5 ms cadence, as before).
async fn poll_final(
    session: &Arc<StreamSession>,
    final_rx: Option<Receiver<PlaybackOutcome>>,
) -> String {
    let Some(receiver) = final_rx else {
        return "played".to_string();
    };
    loop {
        tokio::select! {
            biased;
            _ = session.notify.notified() => {
                if session.cancelled.load(Ordering::SeqCst) {
                    return "interrupted".to_string();
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(5)) => {
                match receiver.try_recv() {
                    Ok(outcome) => {
                        return match outcome {
                            PlaybackOutcome::Played => "played".to_string(),
                            PlaybackOutcome::Interrupted => "interrupted".to_string(),
                        };
                    }
                    Err(std_mpsc::TryRecvError::Empty) => {}
                    Err(std_mpsc::TryRecvError::Disconnected) => {
                        return "interrupted".to_string();
                    }
                }
            }
        }
    }
}

/// Unregister the utterance and emit `Done` (unless the producer already
/// sent an error; `done_status` overrides the cancellation-derived one).
async fn finish(
    id: u64,
    state: &Arc<State>,
    session: &Arc<StreamSession>,
    done_status: Option<String>,
    event_tx: mpsc::Sender<Event>,
) {
    state.active.lock().remove(&id);
    if let Some(key) = &session.stream_key {
        let mut streams = state.streams.lock();
        if streams.get(key) == Some(&id) {
            streams.remove(key);
        }
    }
    let status = done_status.unwrap_or_else(|| {
        if session.cancelled.load(Ordering::SeqCst) {
            "interrupted".to_string()
        } else {
            "played".to_string()
        }
    });
    let _ = event_tx
        .send(Event::Done {
            id,
            status,
            error: None,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_config_defaults() {
        let config = DaemonConfig {
            device: None,
            queue_limit: 8,
            idle_timeout: Duration::from_secs(300),
        };
        assert_eq!(config.queue_limit, 8);
    }
}
