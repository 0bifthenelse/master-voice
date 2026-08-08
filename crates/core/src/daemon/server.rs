use crate::config;
use crate::daemon::client::ping_socket;
use crate::daemon::protocol::{parse_request, serialize_event, Event, Request};
use crate::engine::{self, EngineSettings};
use master_voice_audio::PlaybackThread;
use master_voice_linguistics::overrides::Overrides;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{mpsc as std_mpsc, Arc};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
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

fn current_owner(state: &State) -> Option<(u64, u64)> {
    state
        .controller
        .lock()
        .as_ref()
        .and_then(|thread| thread.current())
}

fn interrupt_current(state: &State) {
    if let Some(thread) = state.controller.lock().as_ref() {
        thread.interrupt_current();
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
    let (line_tx, mut line_rx) = mpsc::channel::<String>(16);
    let (eof_tx, mut eof_rx) = oneshot::channel::<()>();

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
                        send_event(&mut writer, &Event::Pong { id }).await;
                    }
                    Some(Request::Shutdown { id }) => {
                        send_event(&mut writer, &Event::ShutdownAck { id }).await;
                        state.shutting_down.store(true, Ordering::SeqCst);
                        state.shutdown.notify_waiters();
                        return;
                    }
                    Some(Request::Cancel { id }) => {
                        if current_owner(&state).is_some_and(|(current_id, _)| current_id == id) {
                            interrupt_current(&state);
                        }
                    }
                    Some(Request::Speak { id, text, language, interrupt }) => {
                        *state.last_activity.lock() = Instant::now();
                        handle_speak(
                            id,
                            text,
                            language,
                            interrupt,
                            conn_id,
                            &state,
                            &mut writer,
                            &mut line_rx,
                            &mut eof_rx,
                        )
                        .await;
                    }
                    None => {
                        send_event(
                            &mut writer,
                            &Event::Error {
                                id: 0,
                                message: "invalid request".to_string(),
                                code: 1,
                            },
                        )
                        .await;
                    }
                }
            }
            _ = &mut eof_rx => {
                if current_owner(&state).is_some_and(|(_, owner)| owner == conn_id) {
                    interrupt_current(&state);
                }
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_speak<W: AsyncWrite + Unpin>(
    id: u64,
    text: String,
    language: Option<String>,
    interrupt: bool,
    conn_id: u64,
    state: &Arc<State>,
    writer: &mut W,
    line_rx: &mut mpsc::Receiver<String>,
    eof_rx: &mut oneshot::Receiver<()>,
) {
    let language_parsed = language
        .as_deref()
        .and_then(master_voice_linguistics::lang::Language::from_code);
    if language.is_some() && language_parsed.is_none() {
        send_event(
            writer,
            &Event::Error {
                id,
                message: format!("unsupported language {language:?}"),
                code: 3,
            },
        )
        .await;
        return;
    }

    let settings = state.settings.clone();
    let overrides = state.overrides.clone();
    let text_clone = text.clone();
    let synthesized = tokio::task::spawn_blocking(move || {
        engine::synthesize_text(&text_clone, language_parsed, &settings, &overrides)
    })
    .await;

    let (language_used, buffer, synth_ms) = match synthesized {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            let code = e.exit_code();
            send_event(
                writer,
                &Event::Error {
                    id,
                    message: e.to_string(),
                    code,
                },
            )
            .await;
            return;
        }
        Err(e) => {
            send_event(
                writer,
                &Event::Error {
                    id,
                    message: format!("synthesis task failed: {e}"),
                    code: 4,
                },
            )
            .await;
            return;
        }
    };

    let duration_s = buffer.samples.len() as f32 / buffer.sample_rate as f32;

    let receiver = match push_to_controller(
        state,
        id,
        conn_id,
        buffer.samples,
        buffer.sample_rate,
        interrupt,
    ) {
        Ok(receiver) => receiver,
        Err(message) => {
            let code = if message.contains("queue is full") {
                7
            } else {
                5
            };
            send_event(writer, &Event::Error { id, message, code }).await;
            return;
        }
    };

    send_event(
        writer,
        &Event::Accepted {
            id,
            language: language_used.code().to_string(),
            duration_s,
            synth_ms,
        },
    )
    .await;

    let status = loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(5)) => {
                match receiver.try_recv() {
                    Ok(outcome) => {
                        break match outcome {
                            master_voice_audio::PlaybackOutcome::Played => "played",
                            master_voice_audio::PlaybackOutcome::Interrupted => "interrupted",
                        }
                        .to_string();
                    }
                    Err(std_mpsc::TryRecvError::Empty) => {}
                    Err(std_mpsc::TryRecvError::Disconnected) => {
                        break "interrupted".to_string();
                    }
                }
            }
            Some(line) = line_rx.recv() => {
                if let Some(Request::Cancel { id: cancel_id }) = parse_request(&line) {
                    if cancel_id == id
                        && current_owner(state).is_some_and(|(current_id, _)| current_id == cancel_id)
                    {
                        interrupt_current(state);
                    }
                }
            }
            _ = &mut *eof_rx => {
                interrupt_current(state);
                break "interrupted".to_string();
            }
        }
    };

    send_event(
        writer,
        &Event::Done {
            id,
            status,
            error: None,
        },
    )
    .await;
}

async fn send_event<W: AsyncWrite + Unpin>(writer: &mut W, event: &Event) {
    let _ = writer.write_all(serialize_event(event).as_bytes()).await;
    let _ = writer.write_all(b"\n").await;
    let _ = writer.flush().await;
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
