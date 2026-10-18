use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use audio_resample::MonoResampler;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, SupportedStreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use scribe_core::ports::CaptureError;
use scribe_core::recording::{AudioChunk, SessionClock};

const RING_CAPACITY_SAMPLES: usize = 1 << 18;
const PUMP_INTERVAL: Duration = Duration::from_millis(100);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

type StartupReport = Result<(), CaptureError>;

pub(crate) struct Capture {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

impl Capture {
    pub(crate) fn spawn(
        clock: SessionClock,
        chunks: Sender<AudioChunk>,
    ) -> Result<Capture, CaptureError> {
        let (stop, stop_signal) = mpsc::channel();
        let (report, startup) = mpsc::sync_channel(1);
        let thread =
            thread::spawn(move || run_capture(&Delivery { clock, chunks }, &stop_signal, &report));
        match startup.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Capture { stop, thread }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(CaptureError::DeviceUnavailable),
        }
    }

    pub(crate) fn stop(self) {
        let _ = self.stop.send(());
        if self.thread.join().is_err() {
            tracing::error!("the microphone capture thread panicked");
        }
    }
}

struct Delivery {
    clock: SessionClock,
    chunks: Sender<AudioChunk>,
}

fn run_capture(
    delivery: &Delivery,
    stop_signal: &Receiver<()>,
    report: &SyncSender<StartupReport>,
) {
    match open_default_input() {
        Ok(input) => {
            let _ = report.send(Ok(()));
            let OpenInput { stream, pump } = input;
            pump.run(delivery, stop_signal);
            drop(stream);
        }
        Err(error) => {
            let _ = report.send(Err(error));
        }
    }
}

struct OpenInput {
    stream: Stream,
    pump: Pump,
}

fn open_default_input() -> Result<OpenInput, CaptureError> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or(CaptureError::DeviceUnavailable)?;
    let config = device
        .default_input_config()
        .map_err(|error| capture_error(&error))?;
    let resampler = MonoResampler::new(config.sample_rate())
        .map_err(|error| CaptureError::StreamFailed(error.to_string()))?;
    let (producer, consumer) = HeapRb::<f32>::new(RING_CAPACITY_SAMPLES).split();
    let stream = build_stream(&device, &config, producer)?;
    stream.play().map_err(|error| capture_error(&error))?;
    let pump = Pump {
        consumer,
        resampler,
        channels: config.channels(),
    };
    Ok(OpenInput { stream, pump })
}

fn build_stream(
    device: &cpal::Device,
    config: &SupportedStreamConfig,
    producer: HeapProd<f32>,
) -> Result<Stream, CaptureError> {
    match config.sample_format() {
        SampleFormat::I16 => build_typed_stream::<i16>(device, config, producer),
        SampleFormat::I32 => build_typed_stream::<i32>(device, config, producer),
        SampleFormat::F32 => build_typed_stream::<f32>(device, config, producer),
        unsupported => Err(CaptureError::StreamFailed(format!(
            "unsupported microphone sample format {unsupported}"
        ))),
    }
}

fn build_typed_stream<S>(
    device: &cpal::Device,
    config: &SupportedStreamConfig,
    mut producer: HeapProd<f32>,
) -> Result<Stream, CaptureError>
where
    S: SizedSample,
    f32: FromSample<S>,
{
    device
        .build_input_stream::<S, _, _>(
            config.config(),
            move |data: &[S], _| {
                producer.push_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
            },
            |error| tracing::warn!(%error, "the microphone stream reported an error"),
            None,
        )
        .map_err(|error| capture_error(&error))
}

const WINDOWS_ACCESS_DENIED: &str = "(os error -2147024891)";

fn capture_error(error: &cpal::Error) -> CaptureError {
    match error.kind() {
        cpal::ErrorKind::PermissionDenied => CaptureError::PermissionDenied,
        cpal::ErrorKind::BackendError if is_windows_access_denied(error) => {
            CaptureError::PermissionDenied
        }
        cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::HostUnavailable => {
            CaptureError::DeviceUnavailable
        }
        _ => CaptureError::StreamFailed(error.to_string()),
    }
}

fn is_windows_access_denied(error: &cpal::Error) -> bool {
    error
        .message()
        .is_some_and(|message| message.contains(WINDOWS_ACCESS_DENIED))
}

struct Pump {
    consumer: HeapCons<f32>,
    resampler: MonoResampler,
    channels: u16,
}

impl Pump {
    fn run(mut self, delivery: &Delivery, stop_signal: &Receiver<()>) {
        while let Err(RecvTimeoutError::Timeout) = stop_signal.recv_timeout(PUMP_INTERVAL) {
            if !self.forward_captured_audio(delivery) {
                return;
            }
        }
        self.forward_captured_audio(delivery);
    }

    fn forward_captured_audio(&mut self, delivery: &Delivery) -> bool {
        let interleaved: Vec<f32> = self.consumer.pop_iter().collect();
        let samples = match self.resampler.process(&interleaved, self.channels) {
            Ok(samples) => samples,
            Err(error) => {
                tracing::warn!(%error, "dropped microphone audio that could not be resampled");
                return true;
            }
        };
        samples.is_empty()
            || delivery
                .chunks
                .send(AudioChunk::ending_at(samples, delivery.clock.elapsed()))
                .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS_ACCESS_DENIED_MESSAGE: &str = "Access is denied. (os error -2147024891)";

    #[test]
    fn a_microphone_blocked_in_windows_privacy_settings_is_reported_as_permission_denied() {
        let blocked =
            cpal::Error::with_message(cpal::ErrorKind::BackendError, WINDOWS_ACCESS_DENIED_MESSAGE);

        assert_eq!(capture_error(&blocked), CaptureError::PermissionDenied);
    }

    #[test]
    fn other_windows_backend_failures_stay_stream_failures() {
        let failure = cpal::Error::with_message(
            cpal::ErrorKind::BackendError,
            "The device is in use. (os error -2004287478)",
        );

        assert!(matches!(
            capture_error(&failure),
            CaptureError::StreamFailed(_)
        ));
    }
}
