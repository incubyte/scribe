use std::fs;

use stt_parakeet::{
    FetchOutcome, FetchSink, HttpsFetcher, ModelFile, ModelProvisioner, PARAKEET_MODEL,
    RangeFetcher, RangeRequest,
};

const CONFIG_JSON: usize = 4;
const BYTES_ALREADY_ON_DISK: usize = 10;

static CONFIG_ONLY: [ModelFile; 1] = [PARAKEET_MODEL[CONFIG_JSON]];

#[derive(Default)]
struct CollectingSink {
    outcome: Option<FetchOutcome>,
    bytes: Vec<u8>,
}

impl FetchSink for CollectingSink {
    fn begin(&mut self, outcome: FetchOutcome) -> std::io::Result<()> {
        self.outcome = Some(outcome);
        Ok(())
    }

    fn write(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }
}

fn fetch(offset: usize) -> CollectingSink {
    let mut sink = CollectingSink::default();
    HttpsFetcher::new()
        .unwrap()
        .fetch_from(
            &RangeRequest {
                url: CONFIG_ONLY[0].url,
                offset: offset as u64,
            },
            &mut sink,
        )
        .unwrap();
    sink
}

#[test]
#[ignore = "talks to huggingface.co; run manually"]
fn the_real_server_honours_a_resume_request() {
    let whole = fetch(0);

    let tail = fetch(BYTES_ALREADY_ON_DISK);

    assert_eq!(tail.outcome, Some(FetchOutcome::Resumed));
    assert_eq!(tail.bytes, whole.bytes[BYTES_ALREADY_ON_DISK..]);
}

#[test]
#[ignore = "talks to huggingface.co; run manually"]
fn completes_the_real_config_file_from_a_partial_download_and_verifies_its_hash() {
    let file = &CONFIG_ONLY[0];
    let whole = fetch(0);
    let models_dir = tempfile::tempdir().unwrap();
    let part_path = models_dir.path().join(format!("{}.part", file.name));
    fs::write(&part_path, &whole.bytes[..BYTES_ALREADY_ON_DISK]).unwrap();
    let provisioner = ModelProvisioner::with_manifest(
        models_dir.path().to_path_buf(),
        Box::new(HttpsFetcher::new().unwrap()),
        &CONFIG_ONLY,
    );

    provisioner.provision(&|_| {}).unwrap();

    assert!(provisioner.is_ready());
    assert_eq!(
        fs::read(models_dir.path().join(file.name)).unwrap(),
        whole.bytes
    );
}
