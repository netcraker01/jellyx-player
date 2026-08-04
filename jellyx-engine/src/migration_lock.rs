use fs4::{FileExt, TryLockError};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum MigrationLockError {
    InvalidPath(String),
    Contended { waited: Duration },
    Io(io::Error),
}

impl std::fmt::Display for MigrationLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(reason) => write!(f, "unsafe database path: {reason}"),
            Self::Contended { waited } => {
                write!(f, "migration lock remained busy for {waited:?}")
            }
            Self::Io(error) => write!(f, "migration lock I/O error: {error}"),
        }
    }
}

impl std::error::Error for MigrationLockError {}

/// Exclusive cross-process guard for the desktop schema initialization path.
pub struct MigrationLock {
    file: File,
}

impl MigrationLock {
    pub fn acquire(database: &Path, timeout: Duration) -> Result<Self, MigrationLockError> {
        let lock_path = validated_lock_path(database)?;
        let file = open_lock_file(&lock_path).map_err(MigrationLockError::Io)?;
        if file
            .metadata()
            .map_err(MigrationLockError::Io)?
            .file_type()
            .is_symlink()
        {
            return Err(MigrationLockError::InvalidPath(
                "lock file is a symbolic link".into(),
            ));
        }

        let started = Instant::now();
        let deadline = started.checked_add(timeout);
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => return Ok(Self { file }),
                Err(TryLockError::Error(error)) => return Err(MigrationLockError::Io(error)),
                Err(TryLockError::WouldBlock) => {}
            }

            let now = Instant::now();
            let remaining = deadline.and_then(|end| end.checked_duration_since(now));
            if timeout.is_zero() || remaining.is_none() {
                return Err(MigrationLockError::Contended {
                    waited: started.elapsed(),
                });
            }
            thread::sleep(remaining.unwrap().min(Duration::from_millis(10)));
        }
    }
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

fn validated_lock_path(database: &Path) -> Result<PathBuf, MigrationLockError> {
    let name = database
        .file_name()
        .ok_or_else(|| MigrationLockError::InvalidPath("database must have a file name".into()))?;
    let parent = database.parent().ok_or_else(|| {
        MigrationLockError::InvalidPath("database must have a parent directory".into())
    })?;
    let parent = parent.canonicalize().map_err(|error| {
        MigrationLockError::InvalidPath(format!("database parent is unavailable: {error}"))
    })?;
    if !parent.is_dir() {
        return Err(MigrationLockError::InvalidPath(
            "database parent is not a directory".into(),
        ));
    }
    reject_symlink(database, "database")?;

    let mut lock_name = OsString::from(name);
    lock_name.push(".migration.lock");
    let lock_path = parent.join(lock_name);
    reject_symlink(&lock_path, "lock file")?;
    Ok(lock_path)
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), MigrationLockError> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(MigrationLockError::InvalidPath(
            format!("{label} is a symbolic link"),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(MigrationLockError::Io(error)),
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path)
}
