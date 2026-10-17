use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use stt_parakeet::{DownloadError, FetchOutcome, FetchSink, ModelFile, RangeFetcher, RangeRequest};

pub const ALPHA_BYTES: &[u8] = b"alpha-model-bytes";
pub const BETA_BYTES: &[u8] = b"beta-model-bytes-longer";
pub const ALPHA_URL: &str = "https://models.example.test/alpha";
pub const BETA_URL: &str = "https://models.example.test/beta";
pub const TOTAL_BYTES: u64 = 40;
const CHUNK_BYTES: usize = 4;

pub static TWO_FILE_MODEL: [ModelFile; 2] = [
    ModelFile {
        name: "alpha.bin",
        url: ALPHA_URL,
        sha256: "383ff4d3744afd2558a49623333f7b339058b0cda7148a8cbad1c86bcfca6ace",
        bytes: 17,
    },
    ModelFile {
        name: "beta.bin",
        url: BETA_URL,
        sha256: "afed75b09c97372e67437d376e80f56a2cacc0224f45016770ecb155fa767104",
        bytes: 23,
    },
];

struct Behaviour {
    bodies: HashMap<&'static str, Vec<u8>>,
    honours_ranges: bool,
    drop_connection_after: Option<usize>,
    requests: Vec<(String, u64)>,
}

#[derive(Clone)]
pub struct FakeFetcher {
    behaviour: Arc<Mutex<Behaviour>>,
}

impl FakeFetcher {
    pub fn serving_both_files() -> FakeFetcher {
        let bodies = HashMap::from([
            (ALPHA_URL, ALPHA_BYTES.to_vec()),
            (BETA_URL, BETA_BYTES.to_vec()),
        ]);
        FakeFetcher {
            behaviour: Arc::new(Mutex::new(Behaviour {
                bodies,
                honours_ranges: true,
                drop_connection_after: None,
                requests: Vec::new(),
            })),
        }
    }

    pub fn serving_instead(self, url: &'static str, body: &[u8]) -> FakeFetcher {
        self.behaviour().bodies.insert(url, body.to_vec());
        self
    }

    pub fn ignoring_range_requests(self) -> FakeFetcher {
        self.behaviour().honours_ranges = false;
        self
    }

    pub fn dropping_the_connection_after(self, bytes: usize) -> FakeFetcher {
        self.behaviour().drop_connection_after = Some(bytes);
        self
    }

    pub fn reconnecting(&self) {
        self.behaviour().drop_connection_after = None;
    }

    pub fn requests(&self) -> Vec<(String, u64)> {
        self.behaviour().requests.clone()
    }

    fn behaviour(&self) -> MutexGuard<'_, Behaviour> {
        self.behaviour
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl RangeFetcher for FakeFetcher {
    fn fetch_from(
        &self,
        request: &RangeRequest,
        sink: &mut dyn FetchSink,
    ) -> Result<(), DownloadError> {
        let (body, outcome, mut budget) = self.plan_response(request)?;
        sink.begin(outcome)?;
        for chunk in body.chunks(CHUNK_BYTES) {
            let served = budget.map_or(chunk.len(), |left| left.min(chunk.len()));
            sink.write(&chunk[..served])?;
            budget = budget.map(|left| left - served);
            if served < chunk.len() {
                return Err(DownloadError::Network("connection reset".to_owned()));
            }
        }
        Ok(())
    }
}

impl FakeFetcher {
    fn plan_response(
        &self,
        request: &RangeRequest,
    ) -> Result<(Vec<u8>, FetchOutcome, Option<usize>), DownloadError> {
        let mut behaviour = self.behaviour();
        behaviour
            .requests
            .push((request.url.to_owned(), request.offset));
        let body = behaviour
            .bodies
            .get(request.url)
            .cloned()
            .ok_or_else(|| DownloadError::Network("not found".to_owned()))?;
        let budget = behaviour.drop_connection_after;
        if behaviour.honours_ranges {
            let start = usize::try_from(request.offset).map_err(|_| io::Error::other("offset"))?;
            Ok((body[start..].to_vec(), FetchOutcome::Resumed, budget))
        } else {
            Ok((body, FetchOutcome::Restarted, budget))
        }
    }
}
