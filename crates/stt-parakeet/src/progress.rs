#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: u64,
}

impl DownloadProgress {
    #[must_use]
    pub fn percent(&self) -> u8 {
        let whole = self
            .received_bytes
            .saturating_mul(100)
            .checked_div(self.total_bytes)
            .unwrap_or(0);
        u8::try_from(whole.min(100)).unwrap_or(100)
    }
}

pub(crate) struct ProgressTally<'a> {
    finished_bytes: u64,
    current_file_bytes: u64,
    total_bytes: u64,
    report: &'a dyn Fn(DownloadProgress),
}

impl<'a> ProgressTally<'a> {
    pub(crate) fn new(total_bytes: u64, report: &'a dyn Fn(DownloadProgress)) -> ProgressTally<'a> {
        ProgressTally {
            finished_bytes: 0,
            current_file_bytes: 0,
            total_bytes,
            report,
        }
    }

    pub(crate) fn file_started_at(&mut self, bytes: u64) {
        self.current_file_bytes = bytes;
        self.announce();
    }

    pub(crate) fn file_advanced(&mut self, bytes: u64) {
        self.current_file_bytes += bytes;
        self.announce();
    }

    pub(crate) fn file_finished(&mut self, file_bytes: u64) {
        self.finished_bytes += file_bytes;
        self.current_file_bytes = 0;
        self.announce();
    }

    fn announce(&self) {
        (self.report)(DownloadProgress {
            received_bytes: self.finished_bytes + self.current_file_bytes,
            total_bytes: self.total_bytes,
        });
    }
}
