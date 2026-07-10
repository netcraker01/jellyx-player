//! HTTP stream reader for remote audio playback.
//!
//! `HttpStreamReader` fetches audio data via reqwest and provides a
//! `Read + Seek` interface for Symphonia's `MediaSourceStream`.
//!
//! Design: The reader downloads the entire response into a `Cursor<Vec<u8>>`
//! first (full-buffer strategy), then serves `Read + Seek` from memory.
//! This is simpler than ring-buffer progressive streaming and avoids
//! seek complications on partial downloads. For v0.1, this trades latency
//! for reliability. Progressive streaming with backpressure can be added later.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

use symphonia::core::io::MediaSource;

use crate::errors::types::StreamError;

/// HTTP stream reader that downloads remote audio for Symphonia decoding.
///
/// Fetches a URL via `reqwest::blocking`, buffers the entire response body
/// in memory, and provides `Read + Seek` access for Symphonia's probe and
/// decode pipeline.
///
/// # Buffering Strategy
///
/// The current implementation uses a **full-download** strategy: the entire
/// HTTP response body is read into a `Vec<u8>` before any decoding begins.
/// This ensures:
/// - Seek is always available (Symphonia requires this for many formats)
/// - No backpressure or underrun issues during decode
/// - Simpler implementation with fewer failure modes
///
/// For very large files or slow connections, a progressive streaming
/// approach with a ring buffer could be substituted later.
pub struct HttpStreamReader {
    data: Cursor<Vec<u8>>,
}

impl HttpStreamReader {
    /// Fetch a remote audio URL and buffer the entire response body.
    ///
    /// Returns an `HttpStreamReader` that implements `Read + Seek`
    /// for use with Symphonia's `MediaSourceStream`.
    ///
    /// # Errors
    ///
    /// Returns `StreamError::StreamFailed` if the HTTP request fails
    /// or the response status is not successful.
    #[allow(dead_code)]
    pub fn from_url(url: &str) -> Result<Self, StreamError> {
        let response = reqwest::blocking::get(url).map_err(|e| {
            StreamError::StreamFailed(format!("HTTP request failed: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 403 {
                // 403 typically means the stream URL has expired
                return Err(StreamError::UrlExpired);
            }
            return Err(StreamError::StreamFailed(format!(
                "HTTP error {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        let body = response.bytes().map_err(|e| {
            StreamError::StreamFailed(format!("Failed to read response body: {}", e))
        })?;

        let data = body.to_vec();
        Ok(Self {
            data: Cursor::new(data),
        })
    }

    /// Create an `HttpStreamReader` from raw bytes (useful for testing).
    #[cfg(test)]
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }

    /// Returns the total length of the buffered data.
    #[allow(dead_code)]
    pub fn len(&self) -> u64 {
        self.data.get_ref().len() as u64
    }

    /// Returns true if the buffered data is empty.
    #[allow(dead_code)]
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
    fn http_stream_reader_read_from_bytes() {
        let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        let mut reader = HttpStreamReader::from_bytes(data);

        let mut buf = [0u8; 4];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 4);
        assert_eq!(&buf, &[0, 1, 2, 3]);
    }

    #[test]
    fn http_stream_reader_seek_and_read() {
        let data = vec![10u8, 20, 30, 40, 50, 60, 70, 80];
        let mut reader = HttpStreamReader::from_bytes(data);

        reader.seek(SeekFrom::Start(3)).unwrap();
        let mut buf = [0u8; 2];
        reader.read(&mut buf).unwrap();
        assert_eq!(&buf, &[40, 50]);
    }

    #[test]
    fn http_stream_reader_seek_from_end() {
        let data = vec![1u8, 2, 3, 4, 5];
        let mut reader = HttpStreamReader::from_bytes(data);

        reader.seek(SeekFrom::End(-2)).unwrap();
        let mut buf = [0u8; 1];
        reader.read(&mut buf).unwrap();
        assert_eq!(buf[0], 4);
    }

    #[test]
    fn http_stream_reader_len() {
        let data = vec![0u8; 100];
        let reader = HttpStreamReader::from_bytes(data);
        assert_eq!(reader.len(), 100);
    }

    #[test]
    fn http_stream_reader_empty() {
        let reader = HttpStreamReader::from_bytes(vec![]);
        assert!(reader.is_empty());
        assert_eq!(reader.len(), 0);
    }

    #[test]
    fn http_stream_reader_read_all_data() {
        let original = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut reader = HttpStreamReader::from_bytes(original.clone());

        let mut buf = vec![0u8; 4];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 4);
        assert_eq!(&buf, &original);
    }

    #[test]
    fn http_stream_reader_read_past_end_returns_zero() {
        let data = vec![1u8, 2, 3];
        let mut reader = HttpStreamReader::from_bytes(data);

        let mut buf = [0u8; 10];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 3);

        // Second read should return 0 (EOF)
        let read2 = reader.read(&mut buf).unwrap();
        assert_eq!(read2, 0);
    }

    #[test]
    fn stream_error_variants_match_design() {
        // Verify StreamError variants exist and can be constructed
        let _expired = StreamError::UrlExpired;
        let _failed = StreamError::StreamFailed("connection refused".to_string());
        let _underrun = StreamError::BufferUnderrun;
    }

    #[test]
    fn stream_error_url_expired_maps_to_stream_expired_code() {
        let err = crate::errors::types::AppError::from(StreamError::UrlExpired);
        assert_eq!(err.code, "STREAM_EXPIRED");
    }

    #[test]
    fn stream_error_stream_failed_maps_to_stream_error_code() {
        let err = crate::errors::types::AppError::from(StreamError::StreamFailed("timeout".into()));
        assert_eq!(err.code, "STREAM_ERROR");
    }

    #[test]
    fn stream_error_buffer_underrun_maps_to_stream_error_code() {
        let err = crate::errors::types::AppError::from(StreamError::BufferUnderrun);
        assert_eq!(err.code, "STREAM_ERROR");
    }
}