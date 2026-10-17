#![allow(unsafe_code)]

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{NSWorkspace, NSWorkspaceWillSleepNotification};
use objc2_foundation::NSNotification;

pub fn on_will_sleep(callback: impl Fn() + Send + 'static) {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| callback());
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceWillSleepNotification),
            None,
            None,
            &block,
        )
    };
    keep_observing_for_the_app_lifetime(observer);
}

fn keep_observing_for_the_app_lifetime(observer: Retained<ProtocolObject<dyn NSObjectProtocol>>) {
    std::mem::forget(observer);
}
