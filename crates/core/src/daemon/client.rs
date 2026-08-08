use crate::config;
use crate::daemon::protocol::{Event, Request};
use crate::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

pub struct DaemonClient {
    stream: UnixStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakStatus {
    Played,
    Interrupted,
}

type AcceptedCallback = Box<dyn FnOnce(&SpeakReport) + Send>;

pub struct SpeakReport {
    pub status: SpeakStatus,
    pub duration_s: f32,
    pub synth_ms: f64,
    pub language: String,
    pub accepted_ms: f64,
}

impl DaemonClient {
    pub fn connect() -> std::io::Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(config::socket_path())?,
        })
    }

    pub fn spawn_daemon() -> std::io::Result<()> {
        let exe = std::env::current_exe()?;
        let _child = std::process::Command::new(exe)
            .arg("serve")
            .arg("--daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(())
    }

    pub fn connect_or_spawn() -> Result<Self, Error> {
        let mut spawned = false;
        for _ in 0..80 {
            if let Ok(client) = Self::connect() {
                return Ok(client);
            }
            if !spawned {
                Self::spawn_daemon().map_err(|e| Error::Daemon(format!("spawn daemon: {e}")))?;
                spawned = true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(Error::Daemon(
            "cannot connect to playback daemon".to_string(),
        ))
    }

    pub fn speak(
        &mut self,
        text: &str,
        language: Option<&str>,
        interrupt: bool,
    ) -> Result<SpeakReport, Error> {
        self.speak_with_id(1, text, language, interrupt, None)
    }

    /// Speak and block until playback ends (CLI path).
    pub fn speak_with_id(
        &mut self,
        id: u64,
        text: &str,
        language: Option<&str>,
        interrupt: bool,
        robotic: Option<f32>,
    ) -> Result<SpeakReport, Error> {
        let mut report = SpeakReport::default();
        self.speak_impl(
            id,
            text,
            language,
            interrupt,
            robotic,
            None,
            &mut report,
            None,
        )?;
        Ok(report)
    }

    /// Speak and call `on_accepted` as soon as the daemon accepts the
    /// utterance (speech has started), then keep draining to `Done`.
    pub fn speak_streaming(
        &mut self,
        id: u64,
        text: &str,
        language: Option<&str>,
        interrupt: bool,
        robotic: Option<f32>,
        on_accepted: impl FnOnce(&SpeakReport) + Send + 'static,
    ) -> Result<SpeakReport, Error> {
        let mut report = SpeakReport::default();
        self.speak_impl(
            id,
            text,
            language,
            interrupt,
            robotic,
            None,
            &mut report,
            Some(Box::new(on_accepted)),
        )?;
        Ok(report)
    }

    /// Append one word/chunk to the live stream `stream_key`; returns as
    /// soon as the daemon queues it (`Event::Queued`).
    pub fn speak_chunk(
        &mut self,
        id: u64,
        stream_key: &str,
        text: &str,
        language: Option<&str>,
        last: bool,
    ) -> Result<(), Error> {
        let mut report = SpeakReport::default();
        self.speak_impl(
            id,
            text,
            language,
            false,
            None,
            Some((stream_key, last)),
            &mut report,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn speak_impl(
        &mut self,
        id: u64,
        text: &str,
        language: Option<&str>,
        interrupt: bool,
        robotic: Option<f32>,
        stream: Option<(&str, bool)>,
        report: &mut SpeakReport,
        on_accepted: Option<AcceptedCallback>,
    ) -> Result<(), Error> {
        let mut on_accepted = on_accepted;
        let request = Request::Speak {
            id,
            text: text.to_string(),
            language: language.map(|l| l.to_string()),
            interrupt,
            robotic,
            stream: stream.map(|(key, _)| key.to_string()),
            last: stream.map(|(_, last)| last),
        };
        let line = serde_json::to_string(&request)
            .map_err(|e| Error::Daemon(format!("serialize: {e}")))?;
        self.stream
            .write_all(line.as_bytes())
            .map_err(Error::from)?;
        self.stream.write_all(b"\n").map_err(Error::from)?;
        self.stream.flush().map_err(Error::from)?;

        let mut reader = BufReader::new(self.stream.try_clone().map_err(Error::from)?);
        let mut buffer = String::new();
        let sent_at = std::time::Instant::now();
        loop {
            buffer.clear();
            let n = reader
                .read_line(&mut buffer)
                .map_err(|e| Error::Daemon(format!("read: {e}")))?;
            if n == 0 {
                return Err(Error::Daemon("daemon closed connection".to_string()));
            }
            match serde_json::from_str::<Event>(buffer.trim()) {
                Ok(Event::Accepted {
                    language,
                    duration_s,
                    synth_ms,
                    ..
                }) => {
                    report.language = language;
                    report.duration_s = duration_s;
                    report.synth_ms = synth_ms;
                    report.accepted_ms = sent_at.elapsed().as_secs_f64() * 1000.0;
                    if let Some(cb) = on_accepted.take() {
                        cb(report);
                    }
                }
                Ok(Event::Queued { .. }) => return Ok(()),
                Ok(Event::Done { status, error, .. }) => {
                    if let Some(message) = error {
                        return Err(Error::Audio(message));
                    }
                    report.status = if status == "interrupted" {
                        SpeakStatus::Interrupted
                    } else {
                        SpeakStatus::Played
                    };
                    return Ok(());
                }
                Ok(Event::Error { message, code, .. }) => {
                    let error = match code {
                        1 => Error::Usage(message),
                        2 => Error::Config(message),
                        3 => Error::Language(message),
                        4 => Error::Synthesis(message),
                        5 => Error::Audio(message),
                        7 => Error::QueueFull(message),
                        _ => Error::Daemon(format!("{message} (code {code})")),
                    };
                    return Err(error);
                }
                _ => {}
            }
        }
    }

    pub fn cancel(&mut self, id: u64) {
        let request = Request::Cancel { id };
        if let Ok(line) = serde_json::to_string(&request) {
            let _ = self.stream.write_all(line.as_bytes());
            let _ = self.stream.write_all(b"\n");
            let _ = self.stream.flush();
        }
    }

    pub fn ping(&mut self) -> bool {
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(300)));
        let request = Request::Ping { id: 0 };
        let line = serde_json::to_string(&request).unwrap_or_default();
        if self.stream.write_all(line.as_bytes()).is_err() {
            return false;
        }
        if self.stream.write_all(b"\n").is_err() {
            return false;
        }
        if self.stream.flush().is_err() {
            return false;
        }
        let mut reader = BufReader::new(self.stream.try_clone().unwrap_or_else(|_| {
            UnixStream::connect(config::socket_path()).unwrap_or_else(|_| {
                let path = config::socket_path();
                let _ = path;
                panic!("no socket")
            })
        }));
        let mut buffer = String::new();
        match reader.read_line(&mut buffer) {
            Ok(_) => matches!(
                serde_json::from_str::<Event>(buffer.trim()),
                Ok(Event::Pong { .. })
            ),
            Err(_) => false,
        }
    }

    pub fn shutdown(&mut self) -> Result<(), Error> {
        let request = Request::Shutdown { id: 0 };
        let line = serde_json::to_string(&request).map_err(|e| Error::Daemon(e.to_string()))?;
        self.stream
            .write_all(line.as_bytes())
            .map_err(Error::from)?;
        self.stream.write_all(b"\n").map_err(Error::from)?;
        self.stream.flush().map_err(Error::from)?;
        Ok(())
    }
}

impl Default for SpeakReport {
    fn default() -> Self {
        Self {
            status: SpeakStatus::Played,
            duration_s: 0.0,
            synth_ms: 0.0,
            language: String::new(),
            accepted_ms: 0.0,
        }
    }
}

pub fn ping_socket(path: &Path) -> bool {
    let Ok(mut stream) = UnixStream::connect(path) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
    let request = Request::Ping { id: 0 };
    let line = serde_json::to_string(&request).unwrap_or_default();
    if stream.write_all(line.as_bytes()).is_err() {
        return false;
    }
    if stream.write_all(b"\n").is_err() {
        return false;
    }
    if stream.flush().is_err() {
        return false;
    }
    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();
    reader.read_line(&mut buffer).is_ok_and(|_| {
        matches!(
            serde_json::from_str::<Event>(buffer.trim()),
            Ok(Event::Pong { .. })
        )
    })
}
