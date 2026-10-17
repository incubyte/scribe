use std::fs;
use std::io;
use std::path::PathBuf;

use crate::download_error::DownloadError;
use crate::fetcher::{RangeFetcher, RangeRequest};
use crate::model_manifest::{ModelFile, PARAKEET_MODEL};
use crate::part_file::PartFile;
use crate::part_sink::PartSink;
use crate::progress::{DownloadProgress, ProgressTally};

const PART_SUFFIX: &str = "part";
const VERIFIED_SUFFIX: &str = "verified";

pub struct ModelProvisioner {
    models_dir: PathBuf,
    fetcher: Box<dyn RangeFetcher>,
    manifest: &'static [ModelFile],
}

impl ModelProvisioner {
    #[must_use]
    pub fn new(models_dir: PathBuf, fetcher: Box<dyn RangeFetcher>) -> ModelProvisioner {
        ModelProvisioner::with_manifest(models_dir, fetcher, PARAKEET_MODEL)
    }

    #[must_use]
    pub fn with_manifest(
        models_dir: PathBuf,
        fetcher: Box<dyn RangeFetcher>,
        manifest: &'static [ModelFile],
    ) -> ModelProvisioner {
        ModelProvisioner {
            models_dir,
            fetcher,
            manifest,
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.manifest.iter().all(|file| self.is_installed(file))
    }

    pub fn provision(&self, progress: &dyn Fn(DownloadProgress)) -> Result<(), DownloadError> {
        fs::create_dir_all(&self.models_dir)?;
        let mut tally = ProgressTally::new(self.total_bytes(), progress);
        self.manifest
            .iter()
            .try_for_each(|file| self.provision_file(file, &mut tally))
    }

    fn provision_file(
        &self,
        file: &ModelFile,
        tally: &mut ProgressTally,
    ) -> Result<(), DownloadError> {
        if !self.is_installed(file) {
            self.download(file, tally)?;
        }
        tally.file_finished(file.bytes);
        Ok(())
    }

    fn download(&self, file: &ModelFile, tally: &mut ProgressTally) -> Result<(), DownloadError> {
        let mut transfer = Transfer::open(file, self.path_of(file, PART_SUFFIX))?;
        tally.file_started_at(transfer.part.length());
        self.fetch_missing_bytes(&mut transfer, tally)?;
        self.install_if_verified(transfer)
    }

    fn fetch_missing_bytes(
        &self,
        transfer: &mut Transfer,
        tally: &mut ProgressTally,
    ) -> Result<(), DownloadError> {
        let Transfer { file, part, .. } = transfer;
        if part.length() < file.bytes {
            let request = RangeRequest {
                url: file.url,
                offset: part.length(),
            };
            let mut sink = PartSink::new(part, tally, file.bytes);
            self.fetcher.fetch_from(&request, &mut sink)?;
        }
        if part.length() == file.bytes {
            return Ok(());
        }
        Err(DownloadError::Network(
            "the connection closed before the whole file arrived".to_owned(),
        ))
    }

    fn install_if_verified(&self, transfer: Transfer) -> Result<(), DownloadError> {
        let Transfer {
            file,
            part,
            part_path,
        } = transfer;
        if part.finish()? != file.sha256 {
            fs::remove_file(&part_path)?;
            tracing::warn!(
                file = file.name,
                "a model file did not match its pinned hash"
            );
            return Err(DownloadError::HashMismatch {
                file: file.name.to_owned(),
            });
        }
        fs::rename(&part_path, self.installed_path(file))?;
        fs::write(self.path_of(file, VERIFIED_SUFFIX), file.sha256)?;
        tracing::info!(file = file.name, "a model file was downloaded and verified");
        Ok(())
    }

    fn is_installed(&self, file: &ModelFile) -> bool {
        let has_pinned_size = fs::metadata(self.installed_path(file))
            .is_ok_and(|metadata| metadata.len() == file.bytes);
        let marked_verified = fs::read_to_string(self.path_of(file, VERIFIED_SUFFIX))
            .is_ok_and(|marker| marker == file.sha256);
        has_pinned_size && marked_verified
    }

    fn total_bytes(&self) -> u64 {
        self.manifest.iter().map(|file| file.bytes).sum()
    }

    fn installed_path(&self, file: &ModelFile) -> PathBuf {
        self.models_dir.join(file.name)
    }

    fn path_of(&self, file: &ModelFile, suffix: &str) -> PathBuf {
        self.models_dir.join(format!("{}.{suffix}", file.name))
    }
}

struct Transfer<'a> {
    file: &'a ModelFile,
    part: PartFile,
    part_path: PathBuf,
}

impl<'a> Transfer<'a> {
    fn open(file: &'a ModelFile, part_path: PathBuf) -> io::Result<Transfer<'a>> {
        let part = PartFile::open(&part_path, file.bytes)?;
        Ok(Transfer {
            file,
            part,
            part_path,
        })
    }
}
