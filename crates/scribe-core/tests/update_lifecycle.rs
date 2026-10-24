use std::time::{Duration, Instant};

use scribe_core::update::{CHECK_INTERVAL, ReleaseVersion, UpdateLifecycle};

const ONE_MILLISECOND_SHORT_OF_CHECK_INTERVAL: Duration = Duration::from_millis(3_599_999);

#[derive(Debug, PartialEq)]
struct TestPackage(&'static str);

fn version(text: &str) -> ReleaseVersion {
    ReleaseVersion::new(text)
}

fn ready_with_0_5_0() -> UpdateLifecycle<TestPackage> {
    let mut lifecycle = UpdateLifecycle::new();
    lifecycle.begin_download(&version("0.5.0"));
    lifecycle.finish_download(TestPackage("0.5.0 bytes"));
    lifecycle
}

#[test]
fn nothing_waits_to_install_before_any_release_is_found() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();

    assert!(!lifecycle.is_update_waiting(Instant::now()));
    assert_eq!(lifecycle.claim_install(), None);
}

#[test]
fn a_found_release_starts_a_download() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();

    assert!(lifecycle.begin_download(&version("0.5.0")));
}

#[test]
fn nothing_waits_to_install_while_the_first_download_runs() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();
    lifecycle.begin_download(&version("0.5.0"));

    assert!(!lifecycle.is_update_waiting(Instant::now()));
    assert_eq!(lifecycle.claim_install(), None);
}

#[test]
fn a_finished_download_waits_to_install_at_once() {
    let lifecycle = ready_with_0_5_0();

    assert!(lifecycle.is_update_waiting(Instant::now()));
}

#[test]
fn claiming_a_ready_update_hands_out_its_package() {
    let mut lifecycle = ready_with_0_5_0();

    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.5.0 bytes")));
}

#[test]
fn a_failed_download_leaves_nothing_to_install_and_allows_another_attempt() {
    let mut lifecycle = UpdateLifecycle::<TestPackage>::new();
    lifecycle.begin_download(&version("0.5.0"));

    lifecycle.abandon_download();

    assert_eq!(lifecycle.claim_install(), None);
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
    let mut lifecycle = ready_with_0_5_0();

    assert!(!lifecycle.begin_download(&version("0.5.0")));
    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.5.0 bytes")));
}

#[test]
fn a_newer_version_replaces_a_ready_one_once_downloaded() {
    let mut lifecycle = ready_with_0_5_0();

    assert!(lifecycle.begin_download(&version("0.6.0")));
    lifecycle.finish_download(TestPackage("0.6.0 bytes"));

    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.6.0 bytes")));
}

#[test]
fn a_ready_update_stays_installable_while_a_newer_one_downloads() {
    let mut lifecycle = ready_with_0_5_0();

    lifecycle.begin_download(&version("0.6.0"));

    assert!(lifecycle.is_update_waiting(Instant::now()));
    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.5.0 bytes")));
}

#[test]
fn a_failed_download_of_a_newer_version_falls_back_to_the_ready_update() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.begin_download(&version("0.6.0"));

    lifecycle.abandon_download();

    assert!(lifecycle.is_update_waiting(Instant::now()));
    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.5.0 bytes")));
}

#[test]
fn a_finished_download_nobody_asked_for_is_ignored() {
    let mut lifecycle = UpdateLifecycle::new();

    lifecycle.finish_download(TestPackage("stray bytes"));

    assert_eq!(lifecycle.claim_install(), None);
}

#[test]
fn a_version_displays_as_its_text() {
    assert_eq!(version("0.5.0").to_string(), "0.5.0");
}

#[test]
fn a_claimed_update_cannot_be_claimed_twice() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.claim_install();

    assert_eq!(lifecycle.claim_install(), None);
}

#[test]
fn an_update_being_installed_no_longer_waits_to_install() {
    let mut lifecycle = ready_with_0_5_0();

    lifecycle.claim_install();

    assert!(!lifecycle.is_update_waiting(Instant::now()));
}

#[test]
fn no_download_starts_while_an_update_is_being_installed() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.claim_install();

    assert!(!lifecycle.begin_download(&version("0.6.0")));
}

#[test]
fn a_failed_install_makes_the_update_installable_again() {
    let mut lifecycle = ready_with_0_5_0();
    let package = lifecycle.claim_install().expect("a ready update");

    lifecycle.release_failed_install(package, Instant::now());

    assert_eq!(lifecycle.claim_install(), Some(TestPackage("0.5.0 bytes")));
}

#[test]
fn a_failed_install_is_retried_only_after_the_check_interval() {
    let failed_at = Instant::now();
    let mut lifecycle = ready_with_0_5_0();
    let package = lifecycle.claim_install().expect("a ready update");

    lifecycle.release_failed_install(package, failed_at);

    assert!(!lifecycle.is_update_waiting(failed_at + ONE_MILLISECOND_SHORT_OF_CHECK_INTERVAL));
    assert!(lifecycle.is_update_waiting(failed_at + CHECK_INTERVAL));
}

#[test]
fn an_installed_update_waits_to_relaunch() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.claim_install();

    lifecycle.finish_install();

    assert!(lifecycle.is_installed());
    assert!(lifecycle.is_update_waiting(Instant::now()));
}

#[test]
fn an_installed_update_is_not_installed_again() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.claim_install();

    lifecycle.finish_install();

    assert_eq!(lifecycle.claim_install(), None);
}

#[test]
fn no_download_starts_once_an_update_is_installed() {
    let mut lifecycle = ready_with_0_5_0();
    lifecycle.claim_install();

    lifecycle.finish_install();

    assert!(!lifecycle.begin_download(&version("0.6.0")));
}

#[test]
fn an_install_nobody_claimed_does_not_count_as_installed() {
    let mut lifecycle = ready_with_0_5_0();

    lifecycle.finish_install();

    assert!(!lifecycle.is_installed());
}
