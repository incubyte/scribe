use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const FOLDER_NAME: &str = "Claude";
const CONFIG_FILE: &str = "claude_desktop_config.json";
const BACKUP_FILE: &str = "claude_desktop_config.json.scribe-backup";
const STAGING_FILE: &str = "claude_desktop_config.json.scribe-tmp";
#[cfg(unix)]
const OWNER_READ_WRITE: u32 = 0o600;

pub struct ClaudeDesktopConfig {
    folder: PathBuf,
}

impl ClaudeDesktopConfig {
    #[must_use]
    pub fn default_location() -> Option<ClaudeDesktopConfig> {
        dirs::config_dir().map(|base| ClaudeDesktopConfig::in_folder(base.join(FOLDER_NAME)))
    }

    #[must_use]
    pub fn in_folder(folder: PathBuf) -> ClaudeDesktopConfig {
        ClaudeDesktopConfig { folder }
    }

    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.folder.is_dir()
    }

    pub fn read(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(self.config_path()) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn replace(&self, contents: &str) -> io::Result<()> {
        let config = self.config_path();
        let staging = self.folder.join(STAGING_FILE);
        if config.exists() {
            fs::copy(&config, self.folder.join(BACKUP_FILE))?;
        }
        stage_and_swap(&staging, &config, contents).inspect_err(|_| {
            let _ = fs::remove_file(&staging);
        })
    }

    fn config_path(&self) -> PathBuf {
        self.folder.join(CONFIG_FILE)
    }
}

fn stage_and_swap(staging: &Path, config: &Path, contents: &str) -> io::Result<()> {
    let mut file = open_staging(staging)?;
    file.write_all(contents.as_bytes())?;
    file.sync_data()?;
    if config.exists() {
        fs::set_permissions(staging, fs::metadata(config)?.permissions())?;
    }
    fs::rename(staging, config)
}

fn open_staging(staging: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, OWNER_READ_WRITE);
    options.open(staging)
}
