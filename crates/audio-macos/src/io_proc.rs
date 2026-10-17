#![allow(unsafe_code)]

use cidre::{cat, core_audio as ca, os};
use ringbuf::HeapProd;
use ringbuf::traits::Producer;

pub(crate) struct IoContext {
    pub(crate) producer: HeapProd<f32>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) extern "C" fn copy_tap_audio(
    _device: ca::Device,
    _now: &cat::AudioTimeStamp,
    input: &cat::AudioBufList<1>,
    _input_time: &cat::AudioTimeStamp,
    _output: &mut cat::AudioBufList<1>,
    _output_time: &cat::AudioTimeStamp,
    context: Option<&mut IoContext>,
) -> os::Status {
    if let Some(context) = context {
        context.producer.push_slice(samples_in(&input.buffers[0]));
    }
    os::Status::NO_ERR
}

fn samples_in(buffer: &cat::AudioBuf) -> &[f32] {
    if buffer.data.is_null() {
        return &[];
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.data_bytes_size as usize) };
    match unsafe { bytes.align_to::<f32>() } {
        ([], samples, _) => samples,
        _ => &[],
    }
}
