use std::time::{Duration, Instant};

use scribe_core::update::{CHECK_INTERVAL, ReleaseVersion, UpdateLifecycle};

const ONE_MILLISECOND_SHORT_OF_CHECK_INTERVAL: Duration = Duration::from_millis(3_599_999);

#[derive(Clone, Debug, PartialEq)]
struct TestPackage(&'static str);

fn version(text: &str) -> ReleaseVersion {
    ReleaseVersion::new(text)
}

#[test]
fn nothing_is_ready_before_any_release_is_found() {
    let lifecycle = UpdateLifecycle::<TestPackage>::new();

    assert!(!lifecycle.is_ready());
    assert_eq!(lifecycle.package_to_install(), None);
}

#[test]
fn a_found_release_starts_a_download() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();

    assert!(lifecycle.begin_download(&version("0.5.0")));
}

#[test]
fn a_finished_download_makes_the_update_ready_with_its_version() {
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));

    lifecycle.finish_download(TestPackage("0.5.0 bytes"));

    assert!(lifecycle.is_ready());
    assert_eq!(lifecycle.ready_version(), Some(&version("0.5.0")));
    assert_eq!(
        lifecycle.package_to_install(),
        Some(TestPackage("0.5.0 bytes"))
    );
}

#[test]
fn a_failed_download_returns_to_up_to_date_and_allows_another_attempt() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();
    lifecycle.begin_download(&version("0.5.0"));

    lifecycle.abandon_download();

    assert!(!lifecycle.is_ready());
    assert!(lifecycle.begin_download(&version("0.5.0")));
}

#[test]
fn no_second_download_starts_while_one_is_running() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();
    lifecycle.begin_download(&version("0.5.0"));

    assert!(!lifecycle.begin_download(&version("0.5.0")));
    assert!(!lifecycle.begin_download(&version("0.6.0")));
}

#[test]
fn the_same_version_is_not_downloaded_again_once_ready() {
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));
    lifecycle.finish_download(TestPackage("0.5.0 bytes"));

    assert!(!lifecycle.begin_download(&version("0.5.0")));
    assert!(lifecycle.is_ready());
}

#[test]
fn a_newer_version_replaces_a_ready_one() {
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));
    lifecycle.finish_download(TestPackage("0.5.0 bytes"));

    assert!(lifecycle.begin_download(&version("0.6.0")));
    lifecycle.finish_download(TestPackage("0.6.0 bytes"));

    assert_eq!(lifecycle.ready_version(), Some(&version("0.6.0")));
    assert_eq!(
        lifecycle.package_to_install(),
        Some(TestPackage("0.6.0 bytes"))
    );
}

#[test]
fn a_finished_download_nobody_asked_for_is_ignored() {
    let mut lifecycle = UpdateLifecycle::new();

    lifecycle.finish_download(TestPackage("stray bytes"));

    assert!(!lifecycle.is_ready());
}

#[test]
fn a_version_displays_as_its_text() {
    assert_eq!(version("0.5.0").to_string(), "0.5.0");
    assert_eq!(version("0.5.0").as_str(), "0.5.0");
}

#[test]
fn a_failed_install_is_retried_only_after_the_check_interval() {
    let failed_at = Instant::now();
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));
    lifecycle.finish_download(TestPackage("0.5.0 bytes"));

    lifecycle.record_failed_install(failed_at);

    assert!(!lifecycle.is_install_due(failed_at + ONE_MILLISECOND_SHORT_OF_CHECK_INTERVAL));
    assert!(lifecycle.is_install_due(failed_at + CHECK_INTERVAL));
}

#[test]
fn an_update_with_no_failed_install_is_due_at_once() {
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));
    lifecycle.finish_download(TestPackage("0.5.0 bytes"));

    assert!(lifecycle.is_install_due(Instant::now()));
}

#[test]
fn nothing_is_due_to_install_before_an_update_is_ready() {
    let lifecycle = UpdateLifecycle::<TestPackage>::new();

    assert!(!lifecycle.is_install_due(Instant::now()));
}
