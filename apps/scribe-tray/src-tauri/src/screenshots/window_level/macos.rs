#![allow(unsafe_code)]

use std::ffi::c_void;

use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};
use tauri::{Runtime, WebviewWindow};

pub fn raise_above_menu_bar<R: Runtime>(window: &WebviewWindow<R>) {
    let target = window.clone();
    let scheduled = window.run_on_main_thread(move || match target.ns_window() {
        Ok(ns_window) => raise(ns_window),
        Err(error) => tracing::warn!(%error, "could not reach the region selector's native window"),
    });
    if let Err(error) = scheduled {
        tracing::warn!(%error, "could not raise the region selector above the menu bar");
    }
}

fn raise(ns_window: *mut c_void) {
    let Some(window) = (unsafe { ns_window.cast::<NSWindow>().as_ref() }) else {
        return;
    };
    window.setLevel(NSScreenSaverWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary,
    );
}
