use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Speak {
        id: u64,
        text: String,
        language: Option<String>,
        interrupt: bool,
        #[serde(default)]
        robotic: Option<f32>,
        /// Stream key: append this text to the live utterance with the
        /// same key (Step 7e). `None` = one self-contained utterance.
        #[serde(default)]
        stream: Option<String>,
        /// Serialized as `"final"` (Rust keyword). Marks the last chunk of
        /// a stream; default true when `stream` is absent, false otherwise.
        #[serde(default, rename = "final")]
        last: Option<bool>,
    },
    Cancel {
        id: u64,
    },
    Ping {
        id: u64,
    },
    Shutdown {
        id: u64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Accepted {
        id: u64,
        language: String,
        duration_s: f32,
        synth_ms: f64,
    },
    Pong {
        id: u64,
    },
    /// A word-append was accepted into a live stream (Step 7e); the caller
    /// returns immediately instead of waiting for `Done`.
    Queued {
        id: u64,
    },
    Done {
        id: u64,
        status: String,
        error: Option<String>,
    },
    Error {
        id: u64,
        message: String,
        code: i32,
    },
    ShutdownAck {
        id: u64,
    },
}

pub fn parse_request(line: &str) -> Option<Request> {
    serde_json::from_str(line).ok()
}

pub fn serialize_event(event: &Event) -> String {
    serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_speak() {
        let line = r#"{"op":"speak","id":7,"text":"hello","language":"en-US","interrupt":true}"#;
        match parse_request(line) {
            Some(Request::Speak {
                id,
                text,
                language,
                interrupt,
                robotic,
                stream,
                last,
            }) => {
                assert_eq!(id, 7);
                assert_eq!(text, "hello");
                assert_eq!(language.as_deref(), Some("en-US"));
                assert!(interrupt);
                // Old clients omit the new fields.
                assert_eq!(robotic, None);
                assert_eq!(stream, None);
                assert_eq!(last, None);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn speak_parses_optional_fields() {
        let line = r#"{"op":"speak","id":1,"text":"x","interrupt":false,"robotic":0.8,"stream":"s1","final":true}"#;
        match parse_request(line) {
            Some(Request::Speak {
                id,
                robotic,
                stream,
                last,
                ..
            }) => {
                assert_eq!(id, 1);
                assert_eq!(robotic, Some(0.8));
                assert_eq!(stream.as_deref(), Some("s1"));
                assert_eq!(last, Some(true));
            }
            other => panic!("unexpected: {other:?}"),
        }
        // `final` absent, stream present -> default handled by the server.
        let line = r#"{"op":"speak","id":2,"text":"x","interrupt":false,"stream":"s2"}"#;
        match parse_request(line) {
            Some(Request::Speak { stream, last, .. }) => {
                assert_eq!(stream.as_deref(), Some("s2"));
                assert_eq!(last, None);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn roundtrips_events() {
        let event = Event::Done {
            id: 3,
            status: "played".to_string(),
            error: None,
        };
        let json = serialize_event(&event);
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Event::Done { id: 3, .. }));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_request("not json").is_none());
        assert!(parse_request(r#"{"op":"explode"}"#).is_none());
    }
}
