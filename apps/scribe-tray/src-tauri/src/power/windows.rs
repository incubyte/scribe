#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr;

use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, PowerRegisterSuspendResumeNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_CALLBACK, PBT_APMSUSPEND};

type SleepCallback = Box<dyn Fn() + Send + Sync>;

pub fn on_will_sleep(callback: impl Fn() + Send + Sync + 'static) {
    let callback: SleepCallback = Box::new(callback);
    let callback: &'static SleepCallback = Box::leak(Box::new(callback));
    let subscription = Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
        Callback: Some(power_event),
        Context: ptr::from_ref(callback).cast_mut().cast(),
    }));
    let mut registration = ptr::null_mut();
    let status = unsafe {
        PowerRegisterSuspendResumeNotification(
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(ptr::from_mut(subscription).cast()),
            &raw mut registration,
        )
    };
    if status != ERROR_SUCCESS {
        tracing::warn!(
            ?status,
            "recordings will not be stopped before the PC sleeps"
        );
    }
}

unsafe extern "system" fn power_event(
    context: *const c_void,
    event: u32,
    _setting: *const c_void,
) -> u32 {
    if event == PBT_APMSUSPEND
        && let Some(callback) = unsafe { context.cast::<SleepCallback>().as_ref() }
    {
        callback();
    }
    ERROR_SUCCESS.0
}
