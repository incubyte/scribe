use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

const HASH_BUFFER_BYTES: usize = 64 * 1024;

pub(crate) struct PartFile {
    file: File,
    hasher: Sha256,
    length: u64,
}

impl PartFile {
    pub(crate) fn open(path: &Path, pinned_bytes: u64) -> io::Result<PartFile> {
        discard_if_longer_than(path, pinned_bytes)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let mut hasher = Sha256::new();
        let length = hash_from_start(&mut file, &mut hasher)?;
        Ok(PartFile {
            file,
            hasher,
            length,
        })
    }

    pub(crate) fn length(&self) -> u64 {
        self.length
    }

    pub(crate) fn append(&mut self, chunk: &[u8]) -> io::Result<()> {
        self.file.write_all(chunk)?;
        self.hasher.update(chunk);
        self.length += chunk.len() as u64;
        Ok(())
    }

    pub(crate) fn restart(&mut self) -> io::Result<()> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.hasher = Sha256::new();
        self.length = 0;
        Ok(())
    }

    pub(crate) fn finish(self) -> io::Result<String> {
        self.file.sync_all()?;
        Ok(lowercase_hex(&self.hasher.finalize()))
    }
}

fn discard_if_longer_than(path: &Path, pinned_bytes: u64) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.len() > pinned_bytes => fs::remove_file(path),
        _ => Ok(()),
    }
}

fn hash_from_start(file: &mut File, hasher: &mut Sha256) -> io::Result<u64> {
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            return Ok(total);
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
}

fn lowercase_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}
