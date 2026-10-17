use std::f32::consts::TAU;

use audio_resample::MonoResampler;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

const DEVICE_RATE_HZ: u32 = 48_000;
const DEVICE_FRAMES_PER_SECOND: usize = 48_000;
const DEVICE_BLOCK_FRAMES: usize = 4_800;
const LARGEST_HELD_BACK_SAMPLES: usize = 342;
const ANALYSIS_WINDOW: usize = 8_192;
const HZ_PER_ANALYSIS_BIN: f32 = 16_000.0 / 8_192.0;

fn stereo_tone(frequency_hz: f32, seconds: usize) -> Vec<f32> {
    let phase_step = TAU * frequency_hz / 48_000.0;
    std::iter::successors(Some(0.0_f32), |phase| Some((phase + phase_step) % TAU))
        .take(DEVICE_FRAMES_PER_SECOND * seconds)
        .map(|phase| phase.sin() * 0.5)
        .flat_map(|sample| [sample, sample])
        .collect()
}

fn resample_in_device_blocks(resampler: &mut MonoResampler, interleaved: &[f32]) -> Vec<f32> {
    interleaved
        .chunks(DEVICE_BLOCK_FRAMES * 2)
        .flat_map(|block| resampler.process(block, 2).unwrap())
        .collect()
}

fn peak_frequency_hz(samples: &[f32]) -> f32 {
    let window = &samples[samples.len() - ANALYSIS_WINDOW..];
    let mut spectrum: Vec<Complex<f32>> = window.iter().map(|&s| Complex::new(s, 0.0)).collect();
    FftPlanner::new()
        .plan_fft_forward(ANALYSIS_WINDOW)
        .process(&mut spectrum);
    let peak_bin = spectrum[..ANALYSIS_WINDOW / 2]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.norm().total_cmp(&b.norm()))
        .map(|(bin, _)| bin)
        .unwrap();
    f32::from(u16::try_from(peak_bin).unwrap()) * HZ_PER_ANALYSIS_BIN
}

#[test]
fn stereo_audio_at_48_khz_becomes_a_third_as_many_mono_samples() {
    let mut resampler = MonoResampler::new(DEVICE_RATE_HZ).unwrap();

    let output = resample_in_device_blocks(&mut resampler, &stereo_tone(440.0, 3));

    assert!(output.len() <= 48_000);
    assert!(output.len() > 48_000 - LARGEST_HELD_BACK_SAMPLES);
}

#[test]
fn a_one_kilohertz_tone_keeps_its_pitch() {
    let mut resampler = MonoResampler::new(DEVICE_RATE_HZ).unwrap();

    let output = resample_in_device_blocks(&mut resampler, &stereo_tone(1_000.0, 2));

    assert!((peak_frequency_hz(&output) - 1_000.0).abs() < 4.0);
}

#[test]
fn no_audio_comes_out_before_a_full_block_has_arrived() {
    let mut resampler = MonoResampler::new(DEVICE_RATE_HZ).unwrap();

    let output = resampler.process(&[0.1, 0.1, 0.2, 0.2], 2).unwrap();

    assert!(output.is_empty());
}

#[test]
fn a_device_reporting_no_sample_rate_is_rejected() {
    assert!(MonoResampler::new(0).is_err());
}
