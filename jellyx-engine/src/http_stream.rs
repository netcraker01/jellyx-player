//! HTTP stream reader for remote audio playback.
//!
//! Downloads remote audio via HTTP and provides a `Read + Seek` interface
//! for Symphonia's `MediaSourceStream`. Uses a full-buffer strategy: the
//! entire response body is read into memory before decoding begins.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

use symphonia::core::io::MediaSource;

/// Error type for HTTP stream operations.
#[derive(Debug)]
pub enum StreamError {
    /// The stream URL has expired (HTTP 403).
    UrlExpired,
    /// The HTTP request failed.
    StreamFailed(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UrlExpired => write!(f, "stream URL expired"),
            Self::StreamFailed(msg) => write!(f, "stream failed: {msg}"),
        }
    }
}

impl std::error::Error for StreamError {}

/// HTTP stream reader that downloads remote audio for Symphonia decoding.
pub struct HttpStreamReader {
    data: Cursor<Vec<u8>>,
}

impl HttpStreamReader {
    /// Fetch a remote audio URL and buffer the entire response body.
    pub fn from_url(url: &str) -> Result<Self, StreamError> {
        let response = reqwest::blocking::get(url)
            .map_err(|e| StreamError::StreamFailed(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 403 {
                return Err(StreamError::UrlExpired);
            }
            return Err(StreamError::StreamFailed(format!(
                "HTTP error {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        let body = response
            .bytes()
            .map_err(|e| StreamError::StreamFailed(format!("read body: {e}")))?;

        Ok(Self {
            data: Cursor::new(body.to_vec()),
        })
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }

    pub fn len(&self) -> u64 {
        self.data.get_ref().len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.data.get_ref().is_empty()
    }
}

impl Read for HttpStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }
}

impl Seek for HttpStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.data.seek(pos)
    }
}

impl MediaSource for HttpStreamReader {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.data.get_ref().len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_read() {
        let mut reader = HttpStreamReader::from_bytes(vec![0, 1, 2, 3]);
        let mut buf = [0u8; 2];
        assert_eq!(reader.read(&mut buf).unwrap(), 2);
        assert_eq!(&buf, &[0, 1]);
    }

    #[test]
    fn from_bytes_seek_and_read() {
        let mut reader = HttpStreamReader::from_bytes(vec![10, 20, 30, 40]);
        reader.seek(SeekFrom::Start(2)).unwrap();
        let mut buf = [0u8; 2];
        reader.read(&mut buf).unwrap();
        assert_eq!(&buf, &[30, 40]);
    }

    #[test]
    fn len_and_empty() {
        let reader = HttpStreamReader::from_bytes(vec![0; 50]);
        assert_eq!(reader.len(), 50);
        assert!(!reader.is_empty());
    }
}
