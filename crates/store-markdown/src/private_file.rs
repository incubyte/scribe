use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

#[cfg(unix)]
const OWNER_READ_WRITE: u32 = 0o600;

pub(crate) fn create_new(path: &Path, contents: &str) -> io::Result<()> {
    let mut file = private_options().create_new(true).open(path)?;
    write_durably(&mut file, contents.as_bytes())
}

pub(crate) fn create_new_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = private_options().create_new(true).open(path)?;
    write_durably(&mut file, bytes).inspect_err(|_| {
        let _ = fs::remove_file(path);
    })
}

pub(crate) fn copy_new(source: &Path, destination: &Path) -> io::Result<()> {
    let mut reader = File::open(source)?;
    let mut file = private_options().create_new(true).open(destination)?;
    io::copy(&mut reader, &mut file)?;
    file.sync_data()
}

pub(crate) fn append(path: &Path, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;
    write_durably(&mut file, line.as_bytes())
}

pub(crate) fn replace_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let staging = path.with_extension("md.tmp");
    let mut file = private_options()
        .create(true)
        .truncate(true)
        .open(&staging)?;
    write_durably(&mut file, contents.as_bytes())?;
    fs::rename(&staging, path)
}

fn write_durably(file: &mut File, contents: &[u8]) -> io::Result<()> {
    file.write_all(contents)?;
    file.sync_data()
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, OWNER_READ_WRITE);
    options
}
