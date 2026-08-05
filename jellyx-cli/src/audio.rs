//! Minimal CPAL + Symphonia audio backend for the TUI.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender, unbounded};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia_core::codecs::audio::AudioDecoder;
use symphonia_core::formats::TrackType;

use jellyx_engine::audio_backend::{AudioBackend, AudioError};
use jellyx_engine::http_stream::HttpStreamReader;
use jellyx_engine::playback_models::PlaybackState;

fn codec_registry() -> &'static CodecRegistry {
    static REGISTRY: std::sync::OnceLock<CodecRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry
    })
}

pub struct TuiAudioBackend {
    state: Arc<Mutex<PlaybackState>>,
    position: Arc<Mutex<f64>>,
    duration: Arc<Mutex<f64>>,
    volume: Arc<Mutex<f32>>,
    stream: Option<Stream>,
    stop_flag: Arc<AtomicBool>,
}

impl TuiAudioBackend {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            position: Arc::new(Mutex::new(0.0)),
            duration: Arc::new(Mutex::new(0.0)),
            volume: Arc::new(Mutex::new(1.0)),
            stream: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    fn set_state(&self, state: PlaybackState) {
        *self.state.lock().unwrap() = state;
    }
}

impl Default for TuiAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for TuiAudioBackend {
    fn play(&mut self, url: &str) -> Result<(), AudioError> {
        self.stop()?;

        // Download the remote stream into memory
        let reader = HttpStreamReader::from_url(url)
            .map_err(|e| AudioError::DecodeError(format!("stream download: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(reader), Default::default());

        // Probe the format from the downloaded data
        let format = symphonia::default::get_probe()
            .probe(
                &Hint::new(),
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| AudioError::DecodeError(format!("probe: {e}")))?;

        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioError::UnsupportedFormat)?;

        let track_id = track.id;
        let codec_params = track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioError::DecodeError("no codec params".into()))?;
        let audio_params = codec_params
            .audio()
            .ok_or_else(|| AudioError::DecodeError("not audio".into()))?;

        let channels = audio_params
            .channels
            .clone()
            .map(|c| c.count())
            .unwrap_or(2)
            .max(1) as u16;
        let sample_rate = audio_params.sample_rate.unwrap_or(44100);

        let decoder = codec_registry()
            .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
            .map_err(|e| AudioError::DecodeError(format!("codec: {e}")))?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::NoAudioDevice("no output".into()))?;

        let config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let (tx, rx): (Sender<f32>, Receiver<f32>) = unbounded();

        let state = self.state.clone();
        let decoder_stop = self.stop_flag.clone();
        let format_reader: Arc<Mutex<Box<dyn FormatReader>>> = Arc::new(Mutex::new(format));

        std::thread::spawn(move || {
            let mut decoder = decoder;
            let mut buffer = vec![0.0f32; 4096];

            loop {
                if decoder_stop.load(Ordering::Relaxed) {
                    break;
                }

                let packet = {
                    let mut reader = format_reader.lock().unwrap();
                    match reader.next_packet() {
                        Ok(Some(pkt)) => pkt,
                        Ok(None) => break,
                        Err(SymphoniaError::IoError(ref e))
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            *state.lock().unwrap() = PlaybackState::Stopped;
                            break;
                        }
                        Err(SymphoniaError::ResetRequired) => continue,
                        Err(e) => {
                            eprintln!("[tui-stream] packet: {e}");
                            break;
                        }
                    }
                };

                if packet.track_id != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let n = decoded.samples_interleaved();
                        if n == 0 {
                            continue;
                        }
                        if n > buffer.len() {
                            buffer.resize(n, 0.0);
                        }
                        buffer[..n].fill(0.0);
                        decoded.copy_to_slice_interleaved(&mut buffer[..n]);
                        for &s in &buffer[..n] {
                            let _ = tx.try_send(s);
                        }
                    }
                    Err(SymphoniaError::DecodeError(_)) => continue,
                    Err(SymphoniaError::IoError(_)) => continue,
                    Err(e) => {
                        eprintln!("[tui-stream] decode: {e}");
                        break;
                    }
                }
            }
        });

        let volume = self.volume.clone();
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let vol = *volume.lock().unwrap();
                    for sample in data.iter_mut() {
                        *sample = rx.try_recv().unwrap_or(0.0) * vol;
                    }
                },
                |e| eprintln!("[tui-stream] stream: {e}"),
                None,
            )
            .map_err(|e| AudioError::DeviceError(format!("stream: {e}")))?;

        stream
            .play()
            .map_err(|e| AudioError::DeviceError(format!("play: {e}")))?;

        self.stream = Some(stream);
        self.set_state(PlaybackState::Playing);
        Ok(())
    }

    fn play_local(&mut self, path: &Path) -> Result<(), AudioError> {
        self.stop()?;

        let file =
            std::fs::File::open(path).map_err(|e| AudioError::DecodeError(format!("open: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| AudioError::DecodeError(format!("probe: {e}")))?;

        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioError::UnsupportedFormat)?;

        let track_id = track.id;

        let codec_params = track
            .codec_params
            .as_ref()
            .ok_or_else(|| AudioError::DecodeError("no codec params".into()))?;

        let audio_params = codec_params
            .audio()
            .ok_or_else(|| AudioError::DecodeError("not audio".into()))?;

        let channels = audio_params
            .channels
            .clone()
            .map(|c| c.count())
            .unwrap_or(2)
            .max(1) as u16;

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);

        let decoder = codec_registry()
            .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
            .map_err(|e| AudioError::DecodeError(format!("codec: {e}")))?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::NoAudioDevice("no output".into()))?;

        let config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let (tx, rx): (Sender<f32>, Receiver<f32>) = unbounded();

        let state = self.state.clone();
        let decoder_stop = self.stop_flag.clone();
        let format_reader: Arc<Mutex<Box<dyn FormatReader>>> = Arc::new(Mutex::new(format));

        std::thread::spawn(move || {
            let mut decoder = decoder;
            let mut buffer = vec![0.0f32; 4096];

            loop {
                if decoder_stop.load(Ordering::Relaxed) {
                    break;
                }

                let packet = {
                    let mut reader = format_reader.lock().unwrap();
                    match reader.next_packet() {
                        Ok(Some(pkt)) => pkt,
                        Ok(None) => break,
                        Err(SymphoniaError::IoError(ref e))
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            *state.lock().unwrap() = PlaybackState::Stopped;
                            break;
                        }
                        Err(SymphoniaError::ResetRequired) => continue,
                        Err(e) => {
                            eprintln!("[tui-audio] packet: {e}");
                            break;
                        }
                    }
                };

                if packet.track_id != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let n = decoded.samples_interleaved();
                        if n == 0 {
                            continue;
                        }
                        if n > buffer.len() {
                            buffer.resize(n, 0.0);
                        }
                        buffer[..n].fill(0.0);
                        decoded.copy_to_slice_interleaved(&mut buffer[..n]);
                        for &s in &buffer[..n] {
                            let _ = tx.try_send(s);
                        }
                    }
                    Err(SymphoniaError::DecodeError(_)) => continue,
                    Err(SymphoniaError::IoError(_)) => continue,
                    Err(e) => {
                        eprintln!("[tui-audio] decode: {e}");
                        break;
                    }
                }
            }
        });

        let volume = self.volume.clone();
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let vol = *volume.lock().unwrap();
                    for sample in data.iter_mut() {
                        *sample = rx.try_recv().unwrap_or(0.0) * vol;
                    }
                },
                |e| eprintln!("[tui-audio] stream: {e}"),
                None,
            )
            .map_err(|e| AudioError::DeviceError(format!("stream: {e}")))?;

        stream
            .play()
            .map_err(|e| AudioError::DeviceError(format!("play: {e}")))?;

        self.stream = Some(stream);
        self.set_state(PlaybackState::Playing);
        Ok(())
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        if let Some(ref stream) = self.stream {
            stream
                .pause()
                .map_err(|e| AudioError::DeviceError(format!("pause: {e}")))?;
        }
        self.set_state(PlaybackState::Paused);
        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        if let Some(ref stream) = self.stream {
            stream
                .play()
                .map_err(|e| AudioError::DeviceError(format!("resume: {e}")))?;
        }
        self.set_state(PlaybackState::Playing);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.stream.take();
        self.stop_flag = Arc::new(AtomicBool::new(false));
        *self.position.lock().unwrap() = 0.0;
        *self.duration.lock().unwrap() = 0.0;
        self.set_state(PlaybackState::Stopped);
        Ok(())
    }

    fn seek(&mut self, position: f64) -> Result<(), AudioError> {
        *self.position.lock().unwrap() = position;
        Ok(())
    }

    fn volume(&mut self, level: f32) -> Result<(), AudioError> {
        *self.volume.lock().unwrap() = level;
        Ok(())
    }

    fn position(&self) -> f64 {
        *self.position.lock().unwrap()
    }

    fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }
}
