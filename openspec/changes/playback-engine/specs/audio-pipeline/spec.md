# Delta for audio-pipeline

## ADDED Requirements

### Requirement: PE-001 SymphoniaDecoder reads local audio files

The system MUST decode local audio files (MP3, FLAC, OGG/Vorbis, AAC) into raw PCM frames using symphonia.

#### Scenario: Decode a valid MP3 file

- GIVEN a local MP3 file path that exists and is readable
- WHEN SymphoniaDecoder opens and decodes the file
- THEN it SHALL produce interleaved f32 PCM frames at the file's native sample rate and channel count

#### Scenario: Handle unsupported format

- GIVEN a file with an unsupported codec (e.g., WAV/ALAC)
- WHEN SymphoniaDecoder attempts to decode
- THEN it SHALL return AudioError::UnsupportedFormat

#### Scenario: Handle corrupted file

- GIVEN a corrupted audio file
- WHEN SymphoniaDecoder attempts to decode
- THEN it SHALL return AudioError::DecodeError with a descriptive message

### Requirement: PE-002 PCM Bus distributes frames to consumers

The system MUST provide a PCM Bus that distributes decoded PCM frames to multiple consumers (audio output and FFT engine) via bounded channels.

#### Scenario: Single consumer receives frames

- GIVEN a PCM Bus with one subscribed consumer
- WHEN the decoder produces PCM frames
- THEN the consumer SHALL receive those frames in order

#### Scenario: Slow consumer gets frame-dropped

- GIVEN an FFT consumer that processes slower than the audio output
- WHEN the FFT channel is full
- THEN the PCM Bus SHALL drop the oldest frame for the FFT consumer rather than blocking the audio output

### Requirement: PE-003 cpal AudioOutput plays PCM frames

The system MUST play PCM frames through the system audio device using cpal's Stream API.

#### Scenario: Play audio through default device

- GIVEN a valid cpal audio device and PCM stream
- WHEN AudioOutput starts playback
- THEN PCM frames SHALL be written to the cpal stream callback and audible through the device

#### Scenario: Handle missing audio device

- GIVEN no audio output device is available
- WHEN AudioOutput attempts to initialize
- THEN it SHALL return AudioError::DeviceError

#### Scenario: Device disconnect during playback

- GIVEN audio is playing and the device disconnects
- WHEN an error occurs on the cpal stream
- THEN the system SHALL emit an error event and transition to PlaybackState::Stopped

### Requirement: PE-004 PlaybackService.play() connects to real audio pipeline for local files

PlaybackService.play() MUST open a local file via SymphoniaDecoder, start the PCM Bus, and begin cpal audio output.

#### Scenario: Play a local file path

- GIVEN a valid local file path (e.g., `/music/song.flac`)
- WHEN PlaybackService.play() is called with that path
- THEN SymphoniaDecoder SHALL decode the file, PCM Bus SHALL distribute frames, and AudioOutput SHALL play them

#### Scenario: Play with invalid file path

- GIVEN a non-existent file path
- WHEN PlaybackService.play() is called
- THEN it SHALL return an error and PlaybackState SHALL remain Stopped

### Requirement: PE-005 PlaybackService pause/resume/seek/set_volume control the pipeline

PlaybackService control methods MUST control the real audio pipeline.

#### Scenario: Pause and resume

- GIVEN audio is Playing
- WHEN pause() is called, THEN PlaybackState transitions to Paused and audio output stops
- WHEN resume() is called, THEN PlaybackState transitions to Playing and audio output resumes

#### Scenario: Seek to position

- GIVEN audio is Playing or Paused
- WHEN seek(position) is called with a valid position in seconds
- THEN the decoder SHALL seek to that position and audio output SHALL resume from there

#### Scenario: Set volume

- GIVEN audio is Playing
- WHEN set_volume(level) is called with a value 0.0–1.0
- THEN audio output volume SHALL be adjusted accordingly

### Requirement: PE-006 PlaybackState reflects actual audio state

PlaybackState MUST transition based on actual audio pipeline state, not manual assignment.

#### Scenario: State transitions on play

- GIVEN PlaybackState is Stopped
- WHEN play() succeeds
- THEN PlaybackState transitions to Playing

#### Scenario: State transitions on error

- GIVEN PlaybackState is Playing
- WHEN a decode error or device error occurs
- THEN PlaybackState transitions to Stopped and an error event is emitted

#### Scenario: State transitions on end of track

- GIVEN audio is Playing and reaches the end of the file
- THEN PlaybackState transitions to Stopped and a track-ended event is emitted