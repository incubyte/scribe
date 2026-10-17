mod support;

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use stt_parakeet::{DownloadError, DownloadProgress, ModelProvisioner};
use support::{
    ALPHA_BYTES, ALPHA_URL, BETA_BYTES, BETA_URL, FakeFetcher, TOTAL_BYTES, TWO_FILE_MODEL,
};
use tempfile::TempDir;

fn provisioner_in(dir: &Path, fetcher: &FakeFetcher) -> ModelProvisioner {
    ModelProvisioner::with_manifest(
        dir.to_path_buf(),
        Box::new(fetcher.clone()),
        &TWO_FILE_MODEL,
    )
}

fn provision_quietly(provisioner: &ModelProvisioner) -> Result<(), DownloadError> {
    provisioner.provision(&|_| {})
}

fn installed_bytes(dir: &TempDir, name: &str) -> Option<Vec<u8>> {
    fs::read(dir.path().join(name)).ok()
}

fn is_absent(dir: &TempDir, name: &str) -> bool {
    !dir.path().join(name).exists()
}

fn mismatched_file(outcome: &Result<(), DownloadError>) -> Option<&str> {
    match outcome {
        Err(DownloadError::HashMismatch { file }) => Some(file),
        _ => None,
    }
}

fn is_network_failure(outcome: &Result<(), DownloadError>) -> bool {
    matches!(outcome, Err(DownloadError::Network(_)))
}

fn never_goes_backwards(reported: &[DownloadProgress]) -> bool {
    reported
        .windows(2)
        .all(|pair| pair[0].received_bytes <= pair[1].received_bytes)
}

fn models_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn downloads_every_model_file_on_first_launch() {
    let dir = models_dir();
    let provisioner = provisioner_in(dir.path(), &FakeFetcher::serving_both_files());

    provision_quietly(&provisioner).unwrap();

    assert_eq!(installed_bytes(&dir, "alpha.bin").unwrap(), ALPHA_BYTES);
    assert_eq!(installed_bytes(&dir, "beta.bin").unwrap(), BETA_BYTES);
    assert!(provisioner.is_ready());
}

#[test]
fn reports_progress_until_the_whole_model_has_arrived() {
    let dir = models_dir();
    let provisioner = provisioner_in(dir.path(), &FakeFetcher::serving_both_files());
    let reported = RefCell::new(Vec::new());

    provisioner
        .provision(&|progress| reported.borrow_mut().push(progress))
        .unwrap();

    let reported = reported.into_inner();
    let everything = DownloadProgress {
        received_bytes: TOTAL_BYTES,
        total_bytes: TOTAL_BYTES,
    };
    assert_eq!(reported.last(), Some(&everything));
    assert!(never_goes_backwards(&reported));
}

#[test]
fn is_not_ready_before_anything_is_downloaded() {
    let dir = models_dir();
    let provisioner = provisioner_in(dir.path(), &FakeFetcher::serving_both_files());

    assert!(!provisioner.is_ready());
}

#[test]
fn resumes_an_interrupted_download_from_where_it_stopped() {
    let dir = models_dir();
    let fetcher = FakeFetcher::serving_both_files().dropping_the_connection_after(6);
    let provisioner = provisioner_in(dir.path(), &fetcher);

    let interrupted = provision_quietly(&provisioner);
    fetcher.reconnecting();
    provision_quietly(&provisioner).unwrap();

    assert!(is_network_failure(&interrupted));
    assert_eq!(
        fetcher.requests(),
        vec![
            (ALPHA_URL.to_owned(), 0),
            (ALPHA_URL.to_owned(), 6),
            (BETA_URL.to_owned(), 0)
        ]
    );
    assert_eq!(installed_bytes(&dir, "alpha.bin").unwrap(), ALPHA_BYTES);
    assert!(provisioner.is_ready());
}

#[test]
fn restarts_the_file_cleanly_when_the_server_ignores_the_resume_request() {
    let dir = models_dir();
    let fetcher = FakeFetcher::serving_both_files()
        .ignoring_range_requests()
        .dropping_the_connection_after(6);
    let provisioner = provisioner_in(dir.path(), &fetcher);

    let _ = provision_quietly(&provisioner);
    fetcher.reconnecting();
    provision_quietly(&provisioner).unwrap();

    assert_eq!(installed_bytes(&dir, "alpha.bin").unwrap(), ALPHA_BYTES);
    assert!(provisioner.is_ready());
}

fn provision_corrupted_alpha(dir: &TempDir) -> (ModelProvisioner, Result<(), DownloadError>) {
    let fetcher =
        FakeFetcher::serving_both_files().serving_instead(ALPHA_URL, b"alpha-model-bytez");
    let provisioner = provisioner_in(dir.path(), &fetcher);
    let outcome = provision_quietly(&provisioner);
    (provisioner, outcome)
}

#[test]
fn reports_a_corrupted_file_as_failed() {
    let dir = models_dir();

    let (provisioner, outcome) = provision_corrupted_alpha(&dir);

    assert_eq!(mismatched_file(&outcome), Some("alpha.bin"));
    assert!(!provisioner.is_ready());
}

#[test]
fn deletes_a_corrupted_file_instead_of_keeping_it() {
    let dir = models_dir();

    let _ = provision_corrupted_alpha(&dir);

    assert!(is_absent(&dir, "alpha.bin"));
    assert!(is_absent(&dir, "alpha.bin.part"));
}

#[test]
fn never_marks_a_corrupted_file_as_verified() {
    let dir = models_dir();

    let _ = provision_corrupted_alpha(&dir);

    assert!(is_absent(&dir, "alpha.bin.verified"));
}

#[test]
fn never_installs_a_file_whose_hash_is_wrong_even_after_a_resume() {
    let dir = models_dir();
    let fetcher = FakeFetcher::serving_both_files()
        .serving_instead(ALPHA_URL, b"alpha-model-bytez")
        .dropping_the_connection_after(6);
    let provisioner = provisioner_in(dir.path(), &fetcher);

    let _ = provision_quietly(&provisioner);
    fetcher.reconnecting();
    let outcome = provision_quietly(&provisioner);

    assert_eq!(mismatched_file(&outcome), Some("alpha.bin"));
    assert!(is_absent(&dir, "alpha.bin"));
    assert!(is_absent(&dir, "alpha.bin.part"));
}

#[test]
fn does_not_download_again_once_the_model_is_verified() {
    let dir = models_dir();
    provision_quietly(&provisioner_in(
        dir.path(),
        &FakeFetcher::serving_both_files(),
    ))
    .unwrap();
    let later_launch_fetcher = FakeFetcher::serving_both_files();
    let provisioner = provisioner_in(dir.path(), &later_launch_fetcher);

    provision_quietly(&provisioner).unwrap();

    assert!(provisioner.is_ready());
    assert!(later_launch_fetcher.requests().is_empty());
}

#[test]
fn fetches_a_file_again_when_its_verification_marker_is_missing() {
    let dir = models_dir();
    provision_quietly(&provisioner_in(
        dir.path(),
        &FakeFetcher::serving_both_files(),
    ))
    .unwrap();
    fs::remove_file(dir.path().join("beta.bin.verified")).unwrap();
    let fetcher = FakeFetcher::serving_both_files();
    let provisioner = provisioner_in(dir.path(), &fetcher);

    assert!(!provisioner.is_ready());
    provision_quietly(&provisioner).unwrap();

    assert_eq!(fetcher.requests(), vec![(BETA_URL.to_owned(), 0)]);
    assert!(provisioner.is_ready());
}

#[test]
fn installs_a_file_that_was_fully_downloaded_before_the_app_stopped() {
    let dir = models_dir();
    fs::write(dir.path().join("alpha.bin.part"), ALPHA_BYTES).unwrap();
    let fetcher = FakeFetcher::serving_both_files();
    let provisioner = provisioner_in(dir.path(), &fetcher);

    provision_quietly(&provisioner).unwrap();

    assert_eq!(fetcher.requests(), vec![(BETA_URL.to_owned(), 0)]);
    assert!(provisioner.is_ready());
}

#[test]
fn discards_a_leftover_partial_file_that_is_larger_than_the_pinned_size() {
    let dir = models_dir();
    fs::write(dir.path().join("alpha.bin.part"), vec![7_u8; 100]).unwrap();
    let fetcher = FakeFetcher::serving_both_files();
    let provisioner = provisioner_in(dir.path(), &fetcher);

    provision_quietly(&provisioner).unwrap();

    assert_eq!(fetcher.requests()[0], (ALPHA_URL.to_owned(), 0));
    assert_eq!(installed_bytes(&dir, "alpha.bin").unwrap(), ALPHA_BYTES);
}

#[test]
fn refuses_a_server_that_sends_more_than_the_pinned_size() {
    let dir = models_dir();
    let fetcher =
        FakeFetcher::serving_both_files().serving_instead(ALPHA_URL, b"alpha-model-bytes-and-more");
    let provisioner = provisioner_in(dir.path(), &fetcher);

    let outcome = provision_quietly(&provisioner);

    assert!(matches!(outcome, Err(DownloadError::Io(_))));
    assert!(is_absent(&dir, "alpha.bin"));
}

#[test]
fn reports_download_progress_as_a_whole_percentage() {
    let progress = DownloadProgress {
        received_bytes: 425,
        total_bytes: 1000,
    };

    assert_eq!(progress.percent(), 42);
}

#[test]
fn reports_zero_percent_when_nothing_is_expected() {
    let progress = DownloadProgress {
        received_bytes: 0,
        total_bytes: 0,
    };

    assert_eq!(progress.percent(), 0);
}

#[test]
fn names_the_kind_of_failure_for_the_tray() {
    let mismatch = DownloadError::HashMismatch {
        file: "alpha.bin".to_owned(),
    };

    assert_eq!(mismatch.kind(), "hash_mismatch");
    assert_eq!(DownloadError::Network(String::new()).kind(), "network");
}
