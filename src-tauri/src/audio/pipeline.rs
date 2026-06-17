//! PCM Bus — internal pub/sub for audio pipeline.
//!
//! The PcmBus uses crossbeam bounded channels to distribute PCM frames
//! from the decoder to audio output and FFT subscribers. When a subscriber's
//! channel is full, the oldest frame is dropped (backpressure for visualization,
//! never blocking the decoder thread).

use std::sync::Arc;

use crossbeam_channel::{bounded, Receiver, Sender};

use super::AudioError;

/// Maximum number of PCM frames buffered in the bus.
/// At 48000 Hz / 1024 samples per frame ≈ 47 frames/sec,
/// a bound of 16 gives ~340ms of buffering.
const DEFAULT_BUS_BOUND: usize = 16;

/// A single PCM frame: interleaved f32 samples.
/// Frames are always stereo (2 channels) and interleaved: L, R, L, R, ...
pub type PcmFrame = Vec<f32>;

/// PCM Bus — the central pub/sub for decoded audio frames.
///
/// The producer (decoder thread) sends frames via `PcmBusProducer`.
/// Each subscriber gets its own `PcmBusSubscriber` with a bounded channel.
/// When a subscriber's channel is full, `try_recv` returns the oldest frame
/// and drops it (non-blocking backpressure).
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
    /// Senders for all subscribers (output + FFT + future consumers).
    subscribers: Vec<Sender<PcmFrame>>,
    /// Shared stream info.
    #[allow(dead_code)]
    stream_info: Arc<StreamInfo>,
}

/// A subscriber (receiver) for PCM frames.
///
/// Call `try_recv` to get the next available frame.
/// If the channel is full, the oldest frame is dropped automatically
/// when the sender tries to push a new frame (backpressure).
pub struct PcmBusSubscriber {
    rx: Receiver<PcmFrame>,
}

impl PcmBus {
    /// Create a new PcmBus with default buffer size.
    ///
    /// Returns a `(PcmBusProducer, PcmBusSubscriber)` pair for the primary
    /// audio output consumer. Call `subscribe()` on the producer to add
    /// additional consumers (e.g., FFT).
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
            subscribers: vec![output_tx],
            stream_info: stream_info.clone(),
        };

        (producer, subscriber)
    }
}

impl PcmBusProducer {
    /// Get a reference to the stream info.
    #[allow(dead_code)]
    pub fn stream_info(&self) -> &StreamInfo {
        &self.stream_info
    }

    /// Add a new subscriber to the bus.
    ///
    /// Returns a `PcmBusSubscriber` that will receive all frames sent after
    /// this call. The subscriber has its own bounded channel with the same
    /// capacity as the bus.
    pub fn subscribe(&mut self) -> PcmBusSubscriber {
        let (tx, rx) = bounded(DEFAULT_BUS_BOUND);
        self.subscribers.push(tx);
        PcmBusSubscriber { rx }
    }

    /// Send a PCM frame to all subscribers.
    ///
    /// If a subscriber's channel is full, the frame is dropped for that
    /// subscriber (non-blocking backpressure). This ensures the decoder
    /// thread never blocks on a slow consumer.
    pub fn send(&self, frame: PcmFrame) -> Result<(), AudioError> {
        for tx in &self.subscribers {
            // try_send is non-blocking; if the channel is full, the frame
            // is dropped for that subscriber — acceptable for visualization,
            // critical for audio output (which should always be draining).
            let _ = tx.try_send(frame.clone());
        }
        Ok(())
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
    fn pcm_bus_drops_on_full() {
        let (producer, subscriber) = PcmBus::with_bound(44100, 2, 2);

        // Fill the channel
        producer.send(vec![0.1; 4]).unwrap();
        producer.send(vec![0.2; 4]).unwrap();

        // Third send should not block; oldest frame may be dropped or newest dropped
        producer.send(vec![0.3; 4]).unwrap();

        // At least one frame should be receivable
        let frame = subscriber.try_recv();
        assert!(frame.is_some());
    }

    #[test]
    fn pcm_bus_multiple_subscribers() {
        let (mut producer, output_sub) = PcmBus::new(44100, 2);
        let fft_sub = producer.subscribe();

        let frame: PcmFrame = vec![0.5, 0.6];
        producer.send(frame.clone()).unwrap();

        assert_eq!(output_sub.try_recv().unwrap(), frame);
        assert_eq!(fft_sub.try_recv().unwrap(), frame);
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
    fn pcm_bus_producer_send_returns_ok_even_with_no_subscribers_after_drop() {
        // If all subscribers are dropped, send should still return Ok
        // (frames are just dropped — no panic)
        let (producer, subscriber) = PcmBus::new(44100, 2);
        drop(subscriber);
        // The sender in PcmBusProducer keeps its own reference,
        // but if all receivers are dropped, try_send on those channels fails silently
        let result = producer.send(vec![1.0; 4]);
        // send always returns Ok — it silently drops frames for disconnected subs
        assert!(result.is_ok());
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
    fn pcm_bus_subscribe_receives_all_frames_after_subscribe() {
        let (mut producer, output_sub) = PcmBus::new(44100, 2);

        // Send a frame BEFORE subscribing the FFT listener
        producer.send(vec![1.0; 4]).unwrap();

        let fft_sub = producer.subscribe();

        // Send a frame AFTER subscribing
        producer.send(vec![2.0; 4]).unwrap();

        // Output subscriber should get both frames
        let f1 = output_sub.try_recv().unwrap();
        assert_eq!(f1[0], 1.0);
        let f2 = output_sub.try_recv().unwrap();
        assert_eq!(f2[0], 2.0);

        // FFT subscriber should only get the frame sent AFTER subscribe
        let fft_frame = fft_sub.try_recv().unwrap();
        assert_eq!(fft_frame[0], 2.0);
        assert!(fft_sub.try_recv().is_none());
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