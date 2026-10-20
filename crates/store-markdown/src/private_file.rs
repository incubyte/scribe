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
    create_new_removing_on_failure(path, bytes, write_durably)
}

fn create_new_removing_on_failure(
    path: &Path,
    bytes: &[u8],
    write: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let mut file = private_options().create_new(true).open(path)?;
    write(&mut file, bytes).inspect_err(|_| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn failing_write(_file: &mut File, _bytes: &[u8]) -> io::Result<()> {
        Err(io::Error::other("disk full"))
    }

    #[test]
    fn a_failed_write_leaves_no_partial_file_behind() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("shot.png");

        let result = create_new_removing_on_failure(&path, b"png", failing_write);

        assert!(result.is_err());
        assert!(!path.exists());
    }

    #[test]
    fn a_successful_write_keeps_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("shot.png");

        create_new_bytes(&path, b"png").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"png");
    }
}
