//! Server-Sent Events decoder.
//!
//! Deliberately free of any network or provider knowledge so the tricky part —
//! frame boundaries landing in the middle of a multi-byte character or between
//! `event:` and `data:` — is unit-testable without a server.
//!
//! Implements the parts of the WHATWG SSE grammar that matter here: `event:`,
//! `data:` (multi-line, newline-joined), comment lines, and CRLF or LF
//! terminators.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

#[derive(Default)]
pub struct SseDecoder {
    /// Bytes received but not yet forming a complete line.
    buf: Vec<u8>,
    event: String,
    data: String,
    /// True once any field of the current event has been seen, so a stray
    /// blank line does not emit a spurious empty event.
    seen_field: bool,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of bytes; returns every event completed by it.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();

        // Split on LF. A trailing CR is stripped per the spec, which also
        // handles CRLF without a separate pass.
        let mut start = 0usize;
        let mut i = 0usize;
        while i < self.buf.len() {
            if self.buf[i] == b'\n' {
                let mut end = i;
                if end > start && self.buf[end - 1] == b'\r' {
                    end -= 1;
                }
                // Lossy is safe: a split multi-byte char can never occur here
                // because we only ever cut on an ASCII newline boundary.
                let line = String::from_utf8_lossy(&self.buf[start..end]).into_owned();
                if let Some(ev) = self.line(&line) {
                    out.push(ev);
                }
                start = i + 1;
            }
            i += 1;
        }
        self.buf.drain(..start);
        out
    }

    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            // Dispatch on a blank line.
            if !self.seen_field {
                return None;
            }
            let ev = SseEvent {
                event: std::mem::take(&mut self.event),
                data: std::mem::take(&mut self.data),
            };
            self.seen_field = false;
            return Some(ev);
        }

        // A leading colon marks a comment (used for keep-alives).
        if line.starts_with(':') {
            return None;
        }

        let (field, raw) = match line.find(':') {
            Some(i) => (&line[..i], &line[i + 1..]),
            // A field with no colon is a field with an empty value.
            None => (line, ""),
        };
        // Exactly one optional leading space is removed from the value.
        let value = raw.strip_prefix(' ').unwrap_or(raw);

        match field {
            "event" => {
                self.event = value.to_string();
                self.seen_field = true;
            }
            "data" => {
                // Multiple data lines are joined with newlines.
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
                self.seen_field = true;
            }
            // `id` and `retry` are irrelevant: the Messages API does not
            // support resuming a stream, so there is nothing to reconnect to.
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(chunks: &[&str]) -> Vec<SseEvent> {
        let mut d = SseDecoder::new();
        let mut out = Vec::new();
        for c in chunks {
            out.extend(d.push(c.as_bytes()));
        }
        out
    }

    #[test]
    fn parses_a_simple_event() {
        let ev = all(&["event: ping\ndata: {\"a\":1}\n\n"]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].event, "ping");
        assert_eq!(ev[0].data, "{\"a\":1}");
    }

    #[test]
    fn reassembles_events_split_across_chunk_boundaries() {
        // The realistic failure mode: TCP hands us half a frame.
        let ev = all(&[
            "event: content_bl",
            "ock_delta\nda",
            "ta: {\"x\":",
            "1}\n",
            "\n",
        ]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].event, "content_block_delta");
        assert_eq!(ev[0].data, "{\"x\":1}");
    }

    #[test]
    fn handles_a_multibyte_character_split_across_chunks() {
        // "é" is two bytes; deliver them in separate chunks.
        let mut d = SseDecoder::new();
        assert!(d.push(b"data: caf").is_empty());
        assert!(d.push(&[0xC3]).is_empty());
        let ev = d.push(&[0xA9, b'\n', b'\n']);
        assert_eq!(ev[0].data, "café");
    }

    #[test]
    fn joins_multiple_data_lines_with_newlines() {
        let ev = all(&["data: line one\ndata: line two\n\n"]);
        assert_eq!(ev[0].data, "line one\nline two");
    }

    #[test]
    fn ignores_comments_and_keepalives() {
        let ev = all(&[": keep-alive\n\ndata: real\n\n"]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].data, "real");
    }

    #[test]
    fn accepts_crlf_terminators() {
        let ev = all(&["event: x\r\ndata: y\r\n\r\n"]);
        assert_eq!(ev[0].event, "x");
        assert_eq!(ev[0].data, "y");
    }

    #[test]
    fn preserves_significant_whitespace_after_the_single_optional_space() {
        let ev = all(&["data:  indented\n\n"]);
        assert_eq!(ev[0].data, " indented");
    }

    #[test]
    fn does_not_emit_for_blank_lines_alone() {
        assert!(all(&["\n\n\n"]).is_empty());
    }

    #[test]
    fn emits_several_events_from_one_chunk() {
        let ev = all(&["data: a\n\ndata: b\n\ndata: c\n\n"]);
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[2].data, "c");
    }

    #[test]
    fn incomplete_trailing_event_is_withheld_until_terminated() {
        let mut d = SseDecoder::new();
        assert!(d.push(b"data: partial\n").is_empty());
        let ev = d.push(b"\n");
        assert_eq!(ev[0].data, "partial");
    }
}
