#![allow(unsafe_code)]

use cidre::core_audio::aggregate_device_keys as aggregate_keys;
use cidre::core_audio::hardware::StartedDevice;
use cidre::{arc, cat, cf, core_audio as ca, ns, os};
use ringbuf::traits::{Consumer, Split};
use ringbuf::{HeapCons, HeapRb};
use scribe_core::ports::CaptureError;

use crate::io_proc::{IoContext, copy_tap_audio};

const RING_CAPACITY_SAMPLES: usize = 1 << 18;
const AGGREGATE_DEVICE_NAME: &str = "Scribe system audio";
const FLOAT_SAMPLE_BITS: u32 = 32;

pub(crate) struct TapHandle {
    device: Option<StartedDevice<ca::AggregateDevice>>,
    io_proc: ca::DeviceIoProcId,
    tap: ca::TapGuard,
    _io_context: Box<IoContext>,
    consumer: HeapCons<f32>,
    format: TapFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TapFormat {
    pub(crate) sample_rate_hz: u32,
    pub(crate) channels: u16,
}

pub(crate) fn create_tap() -> Result<TapHandle, CaptureError> {
    let tap = ca::TapDesc::with_mono_global_tap_excluding_processes(&own_process_exclusion())
        .create_process_tap()
        .map_err(|status| failure("create the system audio tap", status))?;
    let format = read_format(&tap)?;
    let (producer, consumer) = HeapRb::<f32>::new(RING_CAPACITY_SAMPLES).split();
    let mut context = Box::new(IoContext { producer });
    let aggregate = aggregate_device_for(&tap)?;
    let io_proc = aggregate
        .create_io_proc_id(copy_tap_audio, Some(&mut *context))
        .map_err(|status| failure("attach to the system audio tap", status))?;
    let device = ca::device_start(aggregate, Some(io_proc))
        .map_err(|status| failure("start the system audio tap", status))?;
    Ok(TapHandle {
        device: Some(device),
        io_proc,
        tap,
        _io_context: context,
        consumer,
        format,
    })
}

impl TapHandle {
    pub(crate) fn format(&self) -> TapFormat {
        self.format
    }

    pub(crate) fn current_format(&self) -> Result<TapFormat, CaptureError> {
        read_format(&self.tap)
    }

    pub(crate) fn take_samples(&mut self) -> Vec<f32> {
        self.consumer.pop_iter().collect()
    }
}

impl Drop for TapHandle {
    fn drop(&mut self) {
        if let Some(aggregate) = self.device.take().and_then(stop_device) {
            destroy_io_proc(&aggregate, self.io_proc);
            drop_aggregate(aggregate);
        }
    }
}

fn own_process_exclusion() -> arc::R<ns::Array<ns::Number>> {
    match own_process_object() {
        Some(process) => ns::Array::from_slice(&[&*ns::Number::with_u32(process.0.0)]),
        None => ns::Array::new(),
    }
}

fn own_process_object() -> Option<ca::Process> {
    let pid = i32::try_from(std::process::id()).ok()?;
    ca::Process::with_pid(pid)
        .ok()
        .filter(|process| process.0 != ca::Obj::UNKNOWN)
}

fn read_format(tap: &ca::TapGuard) -> Result<TapFormat, CaptureError> {
    let description = tap
        .asbd()
        .map_err(|status| failure("read the system audio format", status))?;
    if !is_float_samples(&description) {
        return Err(CaptureError::StreamFailed(
            "the system audio tap does not deliver 32-bit float samples".to_owned(),
        ));
    }
    Ok(TapFormat {
        sample_rate_hz: whole_hertz(description.sample_rate)?,
        channels: u16::try_from(description.channels_per_frame).unwrap_or(1),
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_hertz(sample_rate: f64) -> Result<u32, CaptureError> {
    if (1.0..=f64::from(u32::MAX)).contains(&sample_rate) {
        Ok(sample_rate.round() as u32)
    } else {
        Err(CaptureError::StreamFailed(
            "the system audio tap reported no usable sample rate".to_owned(),
        ))
    }
}

fn is_float_samples(description: &cat::AudioStreamBasicDesc) -> bool {
    description.format_flags.0 & cat::AudioFormatFlags::IS_FLOAT.0 != 0
        && description.bits_per_channel == FLOAT_SAMPLE_BITS
}

fn aggregate_device_for(tap: &ca::TapGuard) -> Result<ca::AggregateDevice, CaptureError> {
    let tap_uid = tap
        .uid()
        .map_err(|status| failure("identify the system audio tap", status))?;
    ca::AggregateDevice::with_desc(&aggregate_description(&tap_uid))
        .map_err(|status| failure("create the system audio device", status))
}

fn aggregate_description(tap_uid: &cf::String) -> arc::R<cf::DictionaryOf<cf::String, cf::Type>> {
    let sub_tap =
        cf::DictionaryOf::with_keys_values(&[ca::sub_device_keys::uid()], &[tap_uid.as_type_ref()]);
    cf::DictionaryOf::with_keys_values(
        &[
            aggregate_keys::is_private(),
            aggregate_keys::tap_auto_start(),
            aggregate_keys::name(),
            aggregate_keys::uid(),
            aggregate_keys::tap_list(),
        ],
        &[
            cf::Boolean::value_true().as_type_ref(),
            cf::Boolean::value_false(),
            cf::String::from_str(AGGREGATE_DEVICE_NAME).as_ref(),
            &cf::Uuid::new().to_cf_string(),
            &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
        ],
    )
}

fn stop_device(device: StartedDevice<ca::AggregateDevice>) -> Option<ca::AggregateDevice> {
    device
        .stop()
        .inspect_err(|status| tracing::warn!(?status, "could not stop the system audio device"))
        .ok()
}

fn destroy_io_proc(aggregate: &ca::AggregateDevice, io_proc: ca::DeviceIoProcId) {
    let status = unsafe { AudioDeviceDestroyIOProcID(**aggregate, io_proc) };
    if status != os::Status::NO_ERR {
        tracing::warn!(?status, "could not detach from the system audio device");
    }
}

fn drop_aggregate(aggregate: ca::AggregateDevice) {
    drop(aggregate);
}

fn failure(action: &str, status: impl std::fmt::Debug) -> CaptureError {
    CaptureError::StreamFailed(format!("could not {action}: {status:?}"))
}

unsafe extern "C-unwind" {
    fn AudioDeviceDestroyIOProcID(device: ca::Device, io_proc: ca::DeviceIoProcId) -> os::Status;
}
