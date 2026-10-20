#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;

    use capture_macos::ScreenCaptureKitCapturer;
    use scribe_core::ports::ScreenCapturer;

    pub fn screen_capturer() -> Arc<dyn ScreenCapturer> {
        Arc::new(ScreenCaptureKitCapturer::new())
    }
}

#[cfg(target_os = "macos")]
pub use macos::screen_capturer;

#[cfg(windows)]
mod windows {
    use std::sync::Arc;

    use capture_windows::DesktopCapturer;
    use scribe_core::ports::ScreenCapturer;

    pub fn screen_capturer() -> Arc<dyn ScreenCapturer> {
        Arc::new(DesktopCapturer::new())
    }
}

#[cfg(windows)]
pub use windows::screen_capturer;

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported {
    use std::sync::Arc;

    use scribe_core::capture::MonitorShot;
    use scribe_core::ports::{CaptureAccess, ScreenCaptureError, ScreenCapturer};

    struct NoScreenCapture;

    impl ScreenCapturer for NoScreenCapture {
        fn access(&self) -> CaptureAccess {
            CaptureAccess::NotGranted
        }

        fn request_access(&self) {}

        fn capture_under_pointer(&self) -> Result<MonitorShot, ScreenCaptureError> {
            Err(ScreenCaptureError::Failed(
                "screen capture is not available on this system yet".to_owned(),
            ))
        }
    }

    pub fn screen_capturer() -> Arc<dyn ScreenCapturer> {
        Arc::new(NoScreenCapture)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::screen_capturer;
