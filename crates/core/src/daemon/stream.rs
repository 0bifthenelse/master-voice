//! Chunked streaming: deterministic text chunking and stream sessions
//! (Step 7c/7e of the uplift plan).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

/// First chunk of an utterance: hold for four words so assembly receives
/// enough context for a phrase contour.
pub const FIRST_CHUNK_WORDS: usize = 4;
/// Later chunks: longest run of whole words under this many characters.
pub const MAX_CHUNK_CHARS: usize = 120;
/// Chunks queued ahead of the speaker before the producer backpressures.
pub const QUEUE_AHEAD: usize = 3;
/// Producer finalizes a stream after this long without new text (a crashed
/// client can never wedge a session).
pub const STREAM_IDLE_MS: u64 = 1500;

/// Segment boundaries: sentence/clause punctuation the chunker never
/// crosses (the same characters `sentence::split_sentences` breaks on).
fn is_segment_boundary(c: char) -> bool {
    matches!(c, '.' | '?' | '!' | ';' | ':' | ',' | '\n')
}

/// Take the next chunk from `text`. A non-final first chunk waits for four
/// words unless terminal punctuation arrives; every chunk respects
/// `MAX_CHUNK_CHARS` and segment boundaries. Returns `(chunk, rest)`.
pub fn next_chunk(text: &str, first: bool, last: bool) -> (&str, &str) {
    let text = text.trim_start();
    if text.is_empty() {
        return ("", "");
    }
    if first && !last {
        let mut words = 0usize;
        let mut in_word = false;
        let mut len = 0usize;
        let mut last_word_end = 0usize;
        for (i, c) in text.char_indices() {
            if is_segment_boundary(c) {
                let end = i + c.len_utf8();
                return (&text[..end], &text[end..]);
            }
            len += 1;
            if c.is_whitespace() {
                in_word = false;
                last_word_end = i + c.len_utf8();
                if words >= FIRST_CHUNK_WORDS {
                    return (&text[..last_word_end], &text[last_word_end..]);
                }
            } else if !in_word {
                words += 1;
                in_word = true;
            }
            if len > MAX_CHUNK_CHARS && last_word_end > 0 {
                return (&text[..last_word_end], &text[last_word_end..]);
            }
        }
        return if words >= FIRST_CHUNK_WORDS {
            (text, "")
        } else {
            ("", text)
        };
    }
    let mut len = 0usize;
    let mut last_word_end = 0usize;
    for (i, c) in text.char_indices() {
        if is_segment_boundary(c) {
            // Never cross a segment boundary: the chunk ends here.
            let end = i + c.len_utf8();
            return (&text[..end], &text[end..]);
        }
        len += 1;
        if c.is_whitespace() {
            last_word_end = i + c.len_utf8();
        }
        if len > MAX_CHUNK_CHARS {
            if last_word_end > 0 {
                return (&text[..last_word_end], &text[last_word_end..]);
            }
            // A single word longer than the cap: take the whole word.
            let end = text[i..]
                .find(char::is_whitespace)
                .map(|off| i + off)
                .unwrap_or(text.len());
            return (&text[..end], &text[end..]);
        }
    }
    (text, "")
}

/// A live stream session: appends push text onto `text_tx`; the producer
/// (which owns the receiver) synthesizes and plays chunks with
/// `utterance_id`. `cancelled` + `notify` wake the producer on cancel.
#[derive(Clone)]
pub struct StreamSession {
    pub utterance_id: u64,
    pub conn_id: u64,
    pub stream_key: Option<String>,
    /// `Some` while appends are accepted; dropped (or set to None) on
    /// final, which closes the producer's channel.
    pub text_tx: Option<mpsc::Sender<String>>,
    pub cancelled: Arc<AtomicBool>,
    pub notify: Arc<Notify>,
}

impl StreamSession {
    pub fn new(
        utterance_id: u64,
        conn_id: u64,
        stream_key: Option<String>,
        text_tx: Option<mpsc::Sender<String>>,
        cancelled: Arc<AtomicBool>,
        notify: Arc<Notify>,
    ) -> Self {
        Self {
            utterance_id,
            conn_id,
            stream_key,
            text_tx,
            cancelled,
            notify,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_chunk_waits_for_context_or_terminal_input() {
        assert_eq!(
            next_chunk("the system reports", true, false),
            ("", "the system reports")
        );
        let (chunk, rest) = next_chunk("the system reports nominal now", true, false);
        assert_eq!(chunk, "the system reports nominal ");
        assert_eq!(rest, "now");
        assert_eq!(next_chunk("ready.", true, false), ("ready.", ""));
        assert_eq!(
            next_chunk("short final input", true, true),
            ("short final input", "")
        );
    }

    #[test]
    fn chunk_never_crosses_segment_boundary() {
        let text = "one two three. four five six seven eight";
        let (chunk, rest) = next_chunk(text, false, false);
        assert_eq!(chunk, "one two three.");
        let (chunk2, _) = next_chunk(rest, false, false);
        assert_eq!(chunk2, "four five six seven eight");
    }

    #[test]
    fn chunk_respects_char_cap() {
        let text = "word ".repeat(100); // 500 chars, words of 5
        let (chunk, rest) = next_chunk(&text, false, false);
        assert!(chunk.len() <= MAX_CHUNK_CHARS);
        assert!(
            chunk.trim_end().ends_with("word"),
            "ends on a word boundary"
        );
        assert!(!rest.is_empty());
        // All chunks together reconstruct the text (modulo whitespace trim).
        let mut rebuilt = String::new();
        let mut remaining = text.as_str();
        let first = false;
        while !remaining.trim().is_empty() {
            let (c, r) = next_chunk(remaining, first, false);
            rebuilt.push_str(c);
            rebuilt.push(' ');
            remaining = r;
        }
        let words: Vec<&str> = rebuilt.split_whitespace().collect();
        assert_eq!(words.len(), 100);
    }

    #[test]
    fn long_single_word_taken_whole() {
        let long_word = "x".repeat(200);
        let text = format!("{long_word} and more");
        let (chunk, rest) = next_chunk(&text, false, false);
        assert_eq!(chunk, long_word);
        assert!(rest.contains("and more"));
    }

    #[test]
    fn empty_and_whitespace() {
        assert_eq!(next_chunk("", true, false), ("", ""));
        assert_eq!(next_chunk("   ", false, false), ("", ""));
    }
}
