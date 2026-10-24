#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr;

use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE, WIN32_ERROR};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, PowerRegisterSuspendResumeNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DEVICE_NOTIFY_CALLBACK, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND,
};

struct PowerListener {
    event: u32,
    callback: Box<dyn Fn() + Send + Sync>,
}

pub fn on_will_sleep(callback: impl Fn() + Send + Sync + 'static) {
    listen(PBT_APMSUSPEND, callback);
}

pub fn on_did_wake(callback: impl Fn() + Send + Sync + 'static) {
    listen(PBT_APMRESUMEAUTOMATIC, callback);
}

fn listen(event: u32, callback: impl Fn() + Send + Sync + 'static) {
    let listener: &'static PowerListener = Box::leak(Box::new(PowerListener {
        event,
        callback: Box::new(callback),
    }));
    let status = subscribe(listener);
    if status != ERROR_SUCCESS {
        tracing::warn!(
            ?status,
            event,
            "could not listen for the PC sleeping or waking"
        );
    }
}

fn subscribe(listener: &'static PowerListener) -> WIN32_ERROR {
    let subscription = Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
        Callback: Some(power_event),
        Context: ptr::from_ref(listener).cast_mut().cast(),
    }));
    let mut registration = ptr::null_mut();
    unsafe {
        PowerRegisterSuspendResumeNotification(
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(ptr::from_mut(subscription).cast()),
            &raw mut registration,
        )
    }
}

unsafe extern "system" fn power_event(
    context: *const c_void,
    event: u32,
    _setting: *const c_void,
) -> u32 {
    if let Some(listener) = unsafe { context.cast::<PowerListener>().as_ref() }
        && listener.event == event
    {
        (listener.callback)();
    }
    ERROR_SUCCESS.0
}
