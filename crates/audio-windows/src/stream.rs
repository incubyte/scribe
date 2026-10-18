use std::collections::VecDeque;

use scribe_core::ports::CaptureError;
use scribe_core::recording::SAMPLE_RATE_HZ;
use wasapi::{
    AudioCaptureClient, AudioClient, DeviceEnumerator, Direction, Handle, SampleType, StreamMode,
    WasapiError, WaveFormat,
};

const BYTES_PER_SAMPLE: usize = 4;
const BITS_PER_SAMPLE: usize = 32;
const MONO: usize = 1;
const BUFFER_DURATION_HNS: i64 = 1_000_000;
const EVENT_WAIT_MS: u32 = 200;

pub(crate) struct LoopbackStream {
    client: AudioClient,
    capture: AudioCaptureClient,
    data_ready: Handle,
    captured: VecDeque<u8>,
}

impl LoopbackStream {
    pub(crate) fn open() -> Result<LoopbackStream, CaptureError> {
        let client = loopback_client_for_default_output()?;
        LoopbackStream::start(client).map_err(|error| stream_failed(&error))
    }

    fn start(client: AudioClient) -> Result<LoopbackStream, WasapiError> {
        let data_ready = client.set_get_eventhandle()?;
        let capture = client.get_audiocaptureclient()?;
        client.start_stream()?;
        Ok(LoopbackStream {
            client,
            capture,
            data_ready,
            captured: VecDeque::new(),
        })
    }

    pub(crate) fn wait_and_capture(&mut self) -> Result<(), WasapiError> {
        match self.data_ready.wait_for_event(EVENT_WAIT_MS) {
            Ok(()) => self.capture_ready_packets(),
            Err(WasapiError::EventTimeout) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn take_samples(&mut self) -> Vec<f32> {
        let whole_bytes = self.captured.len() - self.captured.len() % BYTES_PER_SAMPLE;
        let bytes: Vec<u8> = self.captured.drain(..whole_bytes).collect();
        bytes
            .chunks_exact(BYTES_PER_SAMPLE)
            .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
            .collect()
    }

    fn capture_ready_packets(&mut self) -> Result<(), WasapiError> {
        while self.capture.get_next_packet_size()?.unwrap_or(0) > 0 {
            let before = self.captured.len();
            let packet = self.capture.read_from_device_to_deque(&mut self.captured)?;
            if packet.flags.silent {
                self.captured.range_mut(before..).for_each(|byte| *byte = 0);
            }
        }
        Ok(())
    }
}

impl Drop for LoopbackStream {
    fn drop(&mut self) {
        if let Err(error) = self.client.stop_stream() {
            tracing::warn!(%error, "could not stop the loopback stream");
        }
    }
}

fn loopback_client_for_default_output() -> Result<AudioClient, CaptureError> {
    let device = DeviceEnumerator::new()
        .and_then(|enumerator| enumerator.get_default_device(&Direction::Render))
        .map_err(|_| CaptureError::DeviceUnavailable)?;
    let mut client = device
        .get_iaudioclient()
        .map_err(|error| stream_failed(&error))?;
    client
        .initialize_client(&speech_format(), &Direction::Capture, &shared_events())
        .map_err(|error| stream_failed(&error))?;
    Ok(client)
}

fn speech_format() -> WaveFormat {
    WaveFormat::new(
        BITS_PER_SAMPLE,
        BITS_PER_SAMPLE,
        &SampleType::Float,
        SAMPLE_RATE_HZ as usize,
        MONO,
        None,
    )
}

fn shared_events() -> StreamMode {
    StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: BUFFER_DURATION_HNS,
    }
}

fn stream_failed(error: &WasapiError) -> CaptureError {
    CaptureError::StreamFailed(error.to_string())
}
