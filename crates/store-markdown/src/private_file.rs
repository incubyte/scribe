use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

#[cfg(unix)]
const OWNER_READ_WRITE: u32 = 0o600;

pub(crate) fn create_new(path: &Path, contents: &str) -> io::Result<()> {
    let mut file = private_options().create_new(true).open(path)?;
    write_durably(&mut file, contents)
}

pub(crate) fn append(path: &Path, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;
    write_durably(&mut file, line)
}

pub(crate) fn replace_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let staging = path.with_extension("md.tmp");
    let mut file = private_options()
        .create(true)
        .truncate(true)
        .open(&staging)?;
    write_durably(&mut file, contents)?;
    fs::rename(&staging, path)
}

fn write_durably(file: &mut File, contents: &str) -> io::Result<()> {
    file.write_all(contents.as_bytes())?;
    file.sync_data()
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, OWNER_READ_WRITE);
    options
}
