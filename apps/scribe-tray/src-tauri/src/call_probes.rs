#[cfg(target_os = "macos")]
mod macos {
    use audio_macos::CoreAudioCallProbe;
    use scribe_core::ports::CallProbe;

    pub fn call_probe() -> Box<dyn CallProbe> {
        Box::new(CoreAudioCallProbe)
    }
}

#[cfg(target_os = "macos")]
pub use macos::call_probe;

#[cfg(windows)]
mod windows {
    use audio_windows::WasapiCallProbe;
    use scribe_core::ports::CallProbe;

    pub fn call_probe() -> Box<dyn CallProbe> {
        Box::new(WasapiCallProbe)
    }
}

#[cfg(windows)]
pub use windows::call_probe;

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported {
    use scribe_core::ports::{CallProbe, CaptureError};

    struct NoCallProbe;

    impl CallProbe for NoCallProbe {
        fn call_app_holds_microphone(&self) -> Result<bool, CaptureError> {
            Ok(false)
        }
    }

    pub fn call_probe() -> Box<dyn CallProbe> {
        Box::new(NoCallProbe)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::call_probe;
