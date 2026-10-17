use std::io;

use crate::fetcher::{FetchOutcome, FetchSink};
use crate::part_file::PartFile;
use crate::progress::ProgressTally;

pub(crate) struct PartSink<'a, 'b> {
    part: &'a mut PartFile,
    tally: &'a mut ProgressTally<'b>,
    pinned_bytes: u64,
}

impl<'a, 'b> PartSink<'a, 'b> {
    pub(crate) fn new(
        part: &'a mut PartFile,
        tally: &'a mut ProgressTally<'b>,
        pinned_bytes: u64,
    ) -> PartSink<'a, 'b> {
        PartSink {
            part,
            tally,
            pinned_bytes,
        }
    }
}

impl FetchSink for PartSink<'_, '_> {
    fn begin(&mut self, outcome: FetchOutcome) -> io::Result<()> {
        if outcome == FetchOutcome::Restarted {
            self.part.restart()?;
            self.tally.file_started_at(0);
        }
        Ok(())
    }

    fn write(&mut self, chunk: &[u8]) -> io::Result<()> {
        let chunk_bytes = chunk.len() as u64;
        if self.part.length() + chunk_bytes > self.pinned_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the server sent more bytes than the manifest allows",
            ));
        }
        self.part.append(chunk)?;
        self.tally.file_advanced(chunk_bytes);
        Ok(())
    }
}
