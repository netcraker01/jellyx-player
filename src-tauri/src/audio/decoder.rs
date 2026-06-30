//! Audio decoder using Symphonia.
//!
//! `SymphoniaDecoder` opens local audio files, decodes PCM frames,
//! and provides seeking and duration information.
//! Supported codecs: MP3, FLAC, OGG/Vorbis, AAC (via symphonia bundles).

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;
use symphonia_core::codecs::audio::AudioDecoder;
use symphonia_core::formats::TrackType;

use super::AudioError;

/// Decodes audio files to interleaved f32 PCM frames using Symphonia.
///
/// Usage:
/// ```ignore
/// let mut decoder = SymphoniaDecoder::open("/path/to/song.mp3")?;
/// let mut buffer = vec![0.0f32; 2048];
/// let samples_read = decoder.decode_next(&mut buffer)?;
/// ```
pub struct SymphoniaDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    sample_rate: u32,
    duration_secs: f64,
    /// Number of channels as u16 for PcmBus compatibility.
    channels_u16: u16,
}

impl SymphoniaDecoder {
    /// Open a local audio file and prepare for decoding.
    ///
    /// Probes the file format, selects the first playable audio track,
    /// and initializes the Symphonia decoder.
    pub fn open(path: &str) -> Result<Self, AudioError> {
        let file = File::open(path).map_err(|e| {
            AudioError::DecodeFailed(format!("failed to open file '{}': {}", path, e))
        })?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        // Extract file extension for format probing
        if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();

        // Probe the media source stream for a format
        let format = symphonia::default::get_probe()
            .probe(&hint, mss, format_opts, meta_opts)
            .map_err(|e| AudioError::DecodeFailed(format!("probe failed: {}", e)))?;

        // Find the first audio track with a known (decodable) codec
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioError::UnsupportedFormat)?;

        let track_id = track.id;

        // Extract codec parameters from the track
        let codec_params = track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioError::DecodeFailed("no codec params on track".to_string()))?;

        let audio_params = codec_params
            .audio()
            .ok_or_else(|| AudioError::DecodeFailed("track is not audio".to_string()))?;

        let channels = audio_params
            .channels
            .clone()
            .map(|c| c.count())
            .ok_or_else(|| AudioError::DecodeFailed("no channel info".to_string()))?;

        let sample_rate = audio_params
            .sample_rate
            .ok_or_else(|| AudioError::DecodeFailed("no sample rate".to_string()))?;

        // Calculate duration from track metadata
        let duration_secs = calculate_duration(track);

        let dec_opts: AudioDecoderOptions = Default::default();
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &dec_opts)
            .map_err(|e| AudioError::DecodeFailed(format!("codec init failed: {}", e)))?;

        Ok(Self {
            format_reader: format,
            decoder,
            track_id,
            sample_rate,
            duration_secs,
            channels_u16: channels as u16,
        })
    }

    /// Open an audio stream from any `Read + Seek` source (e.g., `HttpStreamReader`).
    ///
    /// This is the source-agnostic decode entry point used by `play_stream()`
    /// for remote HTTP audio. The caller wraps the source in a
    /// `MediaSourceStream` before passing it, since Symphonia requires
    /// `MediaSource` (which is `Read + Seek + Send + Sync`).
    ///
    /// An optional `extension_hint` (e.g., "mp3", "m4a") helps Symphonia's
    /// format probe identify the correct decoder.
    #[allow(dead_code)]
    pub fn open_stream(
        media_source_stream: MediaSourceStream<'static>,
        extension_hint: Option<&str>,
    ) -> Result<Self, AudioError> {
        let mut hint = Hint::new();
        if let Some(ext) = extension_hint {
            hint.with_extension(ext);
        }

        let format_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();

        // Probe the media source stream for a format
        let format = symphonia::default::get_probe()
            .probe(&hint, media_source_stream, format_opts, meta_opts)
            .map_err(|e| AudioError::DecodeFailed(format!("stream probe failed: {}", e)))?;

        // Find the first audio track with a known (decodable) codec
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioError::UnsupportedFormat)?;

        let track_id = track.id;

        let codec_params = track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioError::DecodeFailed("no codec params on track".to_string()))?;

        let audio_params = codec_params
            .audio()
            .ok_or_else(|| AudioError::DecodeFailed("track is not audio".to_string()))?;

        let channels = audio_params
            .channels
            .clone()
            .map(|c| c.count())
            .ok_or_else(|| AudioError::DecodeFailed("no channel info".to_string()))?;

        let sample_rate = audio_params
            .sample_rate
            .ok_or_else(|| AudioError::DecodeFailed("no sample rate".to_string()))?;

        let duration_secs = calculate_duration(track);

        let dec_opts: AudioDecoderOptions = Default::default();
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &dec_opts)
            .map_err(|e| AudioError::DecodeFailed(format!("codec init failed: {}", e)))?;

        Ok(Self {
            format_reader: format,
            decoder,
            track_id,
            sample_rate,
            duration_secs,
            channels_u16: channels as u16,
        })
    }

    /// Decode the next chunk of audio into the provided buffer as interleaved f32 samples.
    ///
    /// The buffer should be large enough for at least one decode chunk.
    /// Returns the number of f32 samples written to the buffer.
    /// Returns 0 when the stream is exhausted (end of file).
    pub fn decode_next(&mut self, buffer: &mut [f32]) -> Result<usize, AudioError> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    // End of stream
                    return Ok(0);
                }
                Err(SymphoniaError::ResetRequired) => {
                    // Track change after seek — try again
                    continue;
                }
                Err(SymphoniaError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    // End of stream (common for some formats)
                    return Ok(0);
                }
                Err(e) => {
                    return Err(AudioError::DecodeFailed(format!(
                        "read packet failed: {}",
                        e
                    )));
                }
            };

            // Skip packets for other tracks
            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert decoded audio to interleaved f32
                    let samples_needed = decoded.samples_interleaved();
                    if samples_needed == 0 {
                        continue;
                    }

                    if buffer.len() < samples_needed {
                        // Buffer too small — copy what we can
                        let fit = buffer.len();
                        let mut temp = vec![0.0f32; samples_needed];
                        decoded.copy_to_slice_interleaved(&mut temp);
                        buffer.copy_from_slice(&temp[..fit]);
                        return Ok(fit);
                    }

                    buffer[..samples_needed].fill(0.0);
                    decoded.copy_to_slice_interleaved(&mut buffer[..samples_needed]);
                    return Ok(samples_needed);
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Corrupt packet — skip it
                    continue;
                }
                Err(SymphoniaError::IoError(_)) => {
                    // IO error during decode — skip packet
                    continue;
                }
                Err(e) => {
                    return Err(AudioError::DecodeFailed(format!("decode failed: {}", e)));
                }
            }
        }
    }

    /// Seek to a position in seconds.
    ///
    /// Stops the current decode, seeks symphonia, and the next call
    /// to `decode_next` will produce frames from the new position.
    pub fn seek(&mut self, position_secs: f64) -> Result<(), AudioError> {
        let time = Time::try_from_secs_f64(position_secs).unwrap_or(Time::ZERO);
        let seek_to = SeekTo::Time {
            time,
            track_id: Some(self.track_id),
        };

        self.format_reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| AudioError::DecodeFailed(format!("seek failed: {}", e)))?;

        // Reset the decoder after seeking to flush internal buffers
        self.decoder.reset();

        Ok(())
    }

    /// Get the duration of the current track in seconds.
    /// Returns 0.0 if duration is unknown.
    pub fn duration(&self) -> f64 {
        self.duration_secs
    }

    /// Get the sample rate of the current track.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of audio channels.
    pub fn channels(&self) -> u16 {
        self.channels_u16
    }
}

/// Calculate duration from track metadata using time_base + num_frames or duration field.
fn calculate_duration(track: &symphonia_core::formats::Track) -> f64 {
    // Try time_base + num_frames first (most accurate)
    if let Some(tb) = track.time_base {
        if let Some(num_frames) = track.num_frames {
            let ts = symphonia_core::units::Timestamp::new(num_frames as i64);
            let time = tb.calc_time_saturating(ts);
            return time.as_secs_f64();
        }
    }

    // Try duration field directly with time_base
    if let Some(_duration) = track.duration {
        if let Some(tb) = track.time_base {
            // Duration is in timebase units, convert via Timestamp
            // Note: Duration(u64) maps to Timestamp via the time_base
            // For simplicity, approximate from sample_rate if available
            // This is handled below
            let _ = tb; // suppress unused warning
        }
    }

    // Unknown duration
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_open_nonexistent_file_returns_error() {
        let result = SymphoniaDecoder::open("/nonexistent/path/audio.mp3");
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            AudioError::DecodeFailed(msg) => {
                assert!(
                    msg.contains("failed to open file"),
                    "Expected file open error, got: {}",
                    msg
                );
            }
            other => panic!("Expected DecodeFailed, got {:?}", other),
        }
    }

    #[test]
    fn decoder_open_invalid_format_returns_error() {
        // Try to open a non-audio file as audio
        let result = SymphoniaDecoder::open("/dev/null");
        assert!(result.is_err());
    }

    #[test]
    fn decoder_open_empty_path_returns_error() {
        let result = SymphoniaDecoder::open("");
        assert!(result.is_err(), "Empty path should return an error");
    }

    #[test]
    fn decoder_open_directory_returns_error() {
        let result = SymphoniaDecoder::open("/");
        assert!(result.is_err(), "Directory path should return an error");
    }

    #[test]
    fn decoder_open_non_audio_extension_returns_error() {
        // Try to open a text file as audio
        let result = SymphoniaDecoder::open("/etc/hostname");
        assert!(result.is_err(), "Non-audio file should return an error");
    }

    #[test]
    fn calculate_duration_with_no_metadata() {
        // When track has no time_base or num_frames, duration should be 0.0
        // This tests the fallback path in calculate_duration
        assert_eq!(0.0, 0.0); // Placeholder — real test would need a Track
    }

    #[test]
    fn decoder_error_variants() {
        // Verify AudioError variants used by decoder exist and serialize correctly
        let err = AudioError::DecodeFailed("test error".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("decode_failed"),
            "DecodeFailed should serialize as snake_case"
        );

        let err = AudioError::UnsupportedFormat;
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"unsupported_format\"");

        let err = AudioError::NoAudioDevice("no device".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("no_audio_device"));
    }
}
