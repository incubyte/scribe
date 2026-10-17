use stt_parakeet::{
    DownloadError, FetchOutcome, FetchSink, HttpsFetcher, RangeFetcher, RangeRequest,
};

struct DiscardingSink;

impl FetchSink for DiscardingSink {
    fn begin(&mut self, _outcome: FetchOutcome) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, _chunk: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn refuses_a_clear_text_url_without_contacting_the_server() {
    let fetcher = HttpsFetcher::new().unwrap();

    let outcome = fetcher.fetch_from(
        &RangeRequest {
            url: "http://127.0.0.1:9/model.onnx",
            offset: 0,
        },
        &mut DiscardingSink,
    );

    assert!(matches!(outcome, Err(DownloadError::Network(_))));
}
