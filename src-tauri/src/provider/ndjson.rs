//! Newline-delimited JSON decoder.
//!
//! The Claude Code CLI emits one JSON object per line on stdout. A pipe hands
//! us arbitrary byte chunks, so the two things that actually break are frames
//! split across reads and multi-byte characters split across reads. Keeping
//! this free of any CLI knowledge means both are unit-testable without
//! spawning a process.

#[derive(Default)]
pub struct LineDecoder {
    /// Bytes received but not yet forming a complete line.
    buf: Vec<u8>,
}

impl LineDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk; returns every complete line it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();

        let mut start = 0usize;
        let mut i = 0usize;
        while i < self.buf.len() {
            if self.buf[i] == b'\n' {
                let mut end = i;
                // Tolerate CRLF, which a Windows build would produce.
                if end > start && self.buf[end - 1] == b'\r' {
                    end -= 1;
                }
                if end > start {
                    // Lossy is safe: we only ever cut on an ASCII newline, so
                    // a multi-byte character can never be split here.
                    out.push(String::from_utf8_lossy(&self.buf[start..end]).into_owned());
                }
                start = i + 1;
            }
            i += 1;
        }
        self.buf.drain(..start);
        out
    }

    /// Whatever is left when the stream ends without a trailing newline.
    pub fn finish(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            return None;
        }
        let line = String::from_utf8_lossy(&self.buf).trim().to_string();
        self.buf.clear();
        (!line.is_empty()).then_some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(chunks: &[&str]) -> Vec<String> {
        let mut d = LineDecoder::new();
        let mut out = Vec::new();
        for c in chunks {
            out.extend(d.push(c.as_bytes()));
        }
        out
    }

    #[test]
    fn splits_complete_lines() {
        assert_eq!(
            all(&["{\"a\":1}\n{\"b\":2}\n"]),
            vec!["{\"a\":1}", "{\"b\":2}"]
        );
    }

    #[test]
    fn reassembles_a_line_split_across_chunks() {
        // The realistic failure mode: the pipe hands us half an object.
        assert_eq!(
            all(&["{\"ty", "pe\":\"resu", "lt\"}\n"]),
            vec!["{\"type\":\"result\"}"]
        );
    }

    #[test]
    fn handles_a_multibyte_character_split_across_chunks() {
        let mut d = LineDecoder::new();
        assert!(d.push("{\"t\":\"caf".as_bytes()).is_empty());
        assert!(d.push(&[0xC3]).is_empty());
        let out = d.push(&[0xA9, b'"', b'}', b'\n']);
        assert_eq!(out, vec!["{\"t\":\"café\"}"]);
    }

    #[test]
    fn skips_blank_lines() {
        assert_eq!(all(&["\n\n{\"a\":1}\n\n"]), vec!["{\"a\":1}"]);
    }

    #[test]
    fn accepts_crlf() {
        assert_eq!(all(&["{\"a\":1}\r\n"]), vec!["{\"a\":1}"]);
    }

    #[test]
    fn withholds_an_incomplete_trailing_line_until_terminated() {
        let mut d = LineDecoder::new();
        assert!(d.push(b"{\"a\":1}").is_empty());
        assert_eq!(d.push(b"\n"), vec!["{\"a\":1}"]);
    }

    #[test]
    fn finish_returns_a_trailing_line_with_no_newline() {
        // A process can exit without a final newline.
        let mut d = LineDecoder::new();
        assert!(d.push(b"{\"last\":true}").is_empty());
        assert_eq!(d.finish().as_deref(), Some("{\"last\":true}"));
        assert_eq!(d.finish(), None);
    }

    #[test]
    fn emits_several_lines_from_one_chunk() {
        assert_eq!(all(&["a\nb\nc\n"]).len(), 3);
    }
}
