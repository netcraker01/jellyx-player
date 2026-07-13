//! PCM Bus — internal pub/sub for audio pipeline.
//!
//! The PcmBus uses crossbeam bounded channels to distribute PCM frames
//! from the decoder to audio output. FFT receives a separate tap fed by the
//! audio callback only after PCM is consumed for playback.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};

use super::AudioError;

/// Maximum number of PCM frames buffered for a single consumer.
const DEFAULT_BUS_BOUND: usize = 16;
/// Maximum sample frames in an FFT tap message.
const FFT_TAP_CHUNK_SAMPLE_FRAMES: usize = 1024;

/// A single PCM frame: interleaved f32 samples.
/// Frames are interleaved according to the stream channel count.
pub type PcmFrame = Vec<f32>;

/// PCM Bus — the central pub/sub for decoded audio frames.
///
/// The producer (decoder thread) sends frames via `PcmBusProducer` to the
/// primary audio output subscriber.
pub struct PcmBus {
    /// The sender side for the primary audio output consumer.
    #[allow(dead_code)]
    output_tx: Sender<PcmFrame>,
    /// Shared metadata about the stream (sample rate, channels).
    #[allow(dead_code)]
    stream_info: Arc<StreamInfo>,
}

/// Metadata about the audio stream being distributed through the bus.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
}

/// The producer (sender) side of the PCM Bus.
///
/// Used by the decoder thread to push PCM frames into the pipeline.
/// Each call to `send` pushes a frame to ALL subscribers.
pub struct PcmBusProducer {
    /// The primary (audio output) sender. Uses a *blocking* send so the
    /// decoder naturally paces itself to real-time playback. Never drops
    /// frames — backpressure slows decode speed to match output speed.
    output_tx: Sender<PcmFrame>,
    /// Shared stream info.
    #[allow(dead_code)]
    stream_info: Arc<StreamInfo>,
}

/// A subscriber (receiver) for PCM frames.
///
/// Call `try_recv` to get the next available frame.
pub struct PcmBusSubscriber {
    rx: Receiver<PcmFrame>,
}

/// Non-blocking PCM tap fed by the audio callback after samples are consumed.
pub struct PcmOutputTap {
    tx: Sender<PcmFrame>,
    channels: usize,
}

impl PcmBus {
    /// Create a new PcmBus with default buffer size.
    ///
    /// Returns a `(PcmBusProducer, PcmBusSubscriber)` pair for the primary
    /// audio output consumer.
    pub fn new(sample_rate: u32, channels: u16) -> (PcmBusProducer, PcmBusSubscriber) {
        Self::with_bound(sample_rate, channels, DEFAULT_BUS_BOUND)
    }

    /// Create a new PcmBus with a custom buffer size.
    pub fn with_bound(
        sample_rate: u32,
        channels: u16,
        bound: usize,
    ) -> (PcmBusProducer, PcmBusSubscriber) {
        let (output_tx, output_rx) = bounded(bound);
        let stream_info = Arc::new(StreamInfo {
            sample_rate,
            channels,
        });

        let subscriber = PcmBusSubscriber { rx: output_rx };
        let producer = PcmBusProducer {
            output_tx,
            stream_info: stream_info.clone(),
        };

        (producer, subscriber)
    }

    /// Create a bounded, lossy FFT tap for PCM consumed by audio output.
    pub fn output_tap(channels: u16) -> (PcmOutputTap, PcmBusSubscriber) {
        let (tx, rx) = bounded(DEFAULT_BUS_BOUND);
        (
            PcmOutputTap {
                tx,
                channels: usize::from(channels),
            },
            PcmBusSubscriber { rx },
        )
    }
}

impl PcmBusProducer {
    /// Get a reference to the stream info.
    #[allow(dead_code)]
    pub fn stream_info(&self) -> &StreamInfo {
        &self.stream_info
    }

    /// Send a PCM frame to the primary output.
    ///
    /// The **primary** output channel uses a *blocking* send — if it is full,
    /// the decoder thread waits. This backpressures the decoder so decode
    /// speed matches real-time playback speed.
    ///
    #[allow(dead_code)]
    pub fn send(&self, frame: PcmFrame) -> Result<(), AudioError> {
        // Primary output — BLOCKING. This is the key pacing mechanism:
        // the decoder cannot run faster than the audio callback consumes data.
        self.output_tx
            .send(frame)
            .map_err(|_| AudioError::DeviceError("PCM bus closed".to_string()))
    }

    /// Send without dropping PCM while allowing a stopped backend to interrupt
    /// bounded-channel backpressure. A full channel still paces decoding; only
    /// an explicit stop ends the wait.
    pub fn send_interruptible(
        &self,
        mut frame: PcmFrame,
        stopped: &AtomicBool,
    ) -> Result<(), AudioError> {
        loop {
            if stopped.load(Ordering::Acquire) {
                return Err(AudioError::DeviceError("audio backend stopped".to_string()));
            }
            match self
                .output_tx
                .send_timeout(frame, Duration::from_millis(10))
            {
                Ok(()) => return Ok(()),
                Err(crossbeam_channel::SendTimeoutError::Timeout(returned)) => frame = returned,
                Err(crossbeam_channel::SendTimeoutError::Disconnected(_)) => {
                    return Err(AudioError::DeviceError("PCM bus closed".to_string()));
                }
            }
        }
    }
}

impl PcmOutputTap {
    /// Publish PCM that the output callback actually consumed. This never
    /// blocks playback; a lagging FFT drops new tap chunks instead.
    pub fn send_consumed(&self, interleaved: &[f32]) {
        debug_assert!(self.channels > 0);
        debug_assert_eq!(interleaved.len() % self.channels, 0);
        for chunk in interleaved.chunks(FFT_TAP_CHUNK_SAMPLE_FRAMES * self.channels) {
            let _ = self.tx.try_send(chunk.to_vec());
        }
    }
}

impl PcmBusSubscriber {
    /// Try to receive the next PCM frame.
    ///
    /// Returns `Some(frame)` if a frame is available, `None` if the channel
    /// is empty. This is non-blocking.
    pub fn try_recv(&self) -> Option<PcmFrame> {
        self.rx.try_recv().ok()
    }

    /// Block until a PCM frame is available.
    ///
    /// Used by the audio output thread which must block to maintain
    /// continuous audio playback.
    #[allow(dead_code)]
    pub fn recv(&self) -> Option<PcmFrame> {
        self.rx.recv().ok()
    }

    /// Wait up to `timeout` for a frame. Used for stop-state polling, not
    /// playback pacing.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<PcmFrame> {
        self.rx.recv_timeout(timeout).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_bus_send_and_recv() {
        let (producer, subscriber) = PcmBus::new(44100, 2);
        let frame: PcmFrame = vec![0.0, 0.1, 0.2, 0.3];
        producer.send(frame.clone()).unwrap();
        let received = subscriber.try_recv().unwrap();
        assert_eq!(received, frame);
    }

    #[test]
    fn pcm_bus_primary_blocks_on_full() {
        // The primary channel is blocking and never drops. With no receiver
        // draining it, the primary channel will block on the third send. Verify that the
        // first two sends succeed and the third blocks (detected via
        // thread join timeout).
        let (producer, _subscriber) = PcmBus::with_bound(44100, 2, 2);

        // Fill the channel
        producer.send(vec![0.1; 4]).unwrap();
        producer.send(vec![0.2; 4]).unwrap();

        // Third send blocks because primary channel is full with no receiver.
        // Spawn it in a thread and enforce a short timeout — if it returns
        // immediately, the assertion fails.
        let handle = std::thread::spawn(move || producer.send(vec![0.3; 4]));
        // If primary were lossy (old behavior), this would return instantly.
        // With blocking primary, the thread should still be alive after 50ms.
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(
            !handle.is_finished(),
            "Primary channel should block when full (backpressure)"
        );
    }

    #[test]
    fn pcm_bus_full_send_is_interrupted_when_backend_stops() {
        let (producer, _subscriber) = PcmBus::with_bound(44_100, 2, 1);
        producer.send(vec![0.1; 4]).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let stopper = stopped.clone();
        let sender =
            std::thread::spawn(move || producer.send_interruptible(vec![0.2; 4], &stopper));
        std::thread::sleep(Duration::from_millis(30));
        assert!(
            !sender.is_finished(),
            "backpressure must hold while backend is healthy"
        );
        stopped.store(true, Ordering::Release);
        assert!(
            sender.join().unwrap().is_err(),
            "stop must interrupt a full-channel send"
        );
    }

    #[test]
    fn pcm_bus_try_recv_empty_returns_none() {
        let (_, subscriber) = PcmBus::new(44100, 2);
        assert!(subscriber.try_recv().is_none());
    }

    #[test]
    fn stream_info_preserved() {
        let (producer, _) = PcmBus::new(48000, 2);
        let info = producer.stream_info();
        assert_eq!(info.sample_rate, 48000);
        assert_eq!(info.channels, 2);
    }

    // --- Additional PcmBus tests (Task 4.2) ---

    #[test]
    fn pcm_bus_primary_send_errors_when_receiver_dropped() {
        // When the primary output subscriber is dropped, the blocking
        // send should fail with an error (channel disconnected).
        let (producer, subscriber) = PcmBus::new(44100, 2);
        drop(subscriber);
        let result = producer.send(vec![1.0; 4]);
        assert!(
            result.is_err(),
            "Primary send should error when receiver dropped"
        );
    }

    #[test]
    fn pcm_bus_recv_all_frames_in_order() {
        let (producer, subscriber) = PcmBus::with_bound(44100, 2, 16);

        // Send multiple frames and verify they arrive in order
        for i in 0u8..5 {
            producer.send(vec![i as f32; 4]).unwrap();
        }

        for i in 0u8..5 {
            let frame = subscriber.try_recv().unwrap();
            assert_eq!(frame[0], i as f32, "Frame {} should have value {}", i, i);
        }

        // Channel should now be empty
        assert!(subscriber.try_recv().is_none());
    }

    #[test]
    fn pcm_bus_with_bound_custom_capacity() {
        // Verify that with_bound creates a bus with the specified capacity
        let (producer, subscriber) = PcmBus::with_bound(96000, 1, 3);

        // Should be able to send up to 3 frames without blocking
        for _ in 0..3 {
            producer.send(vec![0.5; 2]).unwrap();
        }

        // All 3 should be receivable
        let mut count = 0;
        while subscriber.try_recv().is_some() {
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn pcm_bus_large_frame() {
        // Verify that large frames (e.g., 4096 samples) can be sent
        let (producer, subscriber) = PcmBus::new(44100, 2);
        let large_frame: PcmFrame = (0..4096).map(|i| i as f32 / 4096.0).collect();
        producer.send(large_frame.clone()).unwrap();
        let received = subscriber.try_recv().unwrap();
        assert_eq!(received.len(), 4096);
        assert_eq!(received[0], 0.0);
        assert!((received[4095] - 4095.0 / 4096.0).abs() < 0.0001);
    }

    #[test]
    fn pcm_bus_stream_info_clone() {
        let (producer, _) = PcmBus::new(22050, 1);
        let info = producer.stream_info().clone();
        assert_eq!(info.sample_rate, 22050);
        assert_eq!(info.channels, 1);
    }

    #[test]
    fn pcm_bus_mono_channel() {
        // Verify mono (1-channel) configuration works
        let (producer, subscriber) = PcmBus::new(16000, 1);
        let frame: PcmFrame = vec![0.5]; // mono frame
        producer.send(frame.clone()).unwrap();
        let received = subscriber.try_recv().unwrap();
        assert_eq!(received, vec![0.5]);
    }
}
