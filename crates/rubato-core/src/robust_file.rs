use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{error, warn};

/// Error type for parse failures, analogous to Java's ParseException.
#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseError: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

fn backup_path(original: &Path) -> PathBuf {
    let mut name = original.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    original.with_file_name(name)
}

fn temporary_path(original: &Path) -> PathBuf {
    let mut name = original.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    original.with_file_name(name)
}

/// Reads and parses the file. In case of failure, falls back to trying the backup file.
///
/// The `parser` function takes raw bytes and returns either a parsed value or a `ParseError`.
pub fn load<T, F>(file: &Path, parser: F) -> Result<T>
where
    F: Fn(&[u8]) -> std::result::Result<T, ParseError>,
{
    // reads and parses the file
    // in case of failure, falls back to trying the backup file
    match fs::read(file) {
        Ok(data) => match parser(&data) {
            Ok(value) => Ok(value),
            Err(e) => {
                // the read reported no errors but, for some reason, parsing the received
                // data failed (possible corruption) - log the problem, then restore from backup
                error!("{}", e);
                load_backup(file, parser)
            }
        },
        Err(e) => {
            // could not read the original file - try the backup
            error!("{}", e);
            load_backup(file, parser)
        }
    }
}

/// Reads and parses the backup file (.bak) for the given original path.
pub fn load_backup<T, F>(original: &Path, parser: F) -> Result<T>
where
    F: Fn(&[u8]) -> std::result::Result<T, ParseError>,
{
    let file = backup_path(original);
    if !file.is_file() {
        anyhow::bail!(
            "File load failed: No backup file. \nPath: {}",
            original.display()
        );
    }

    let data = fs::read(&file).with_context(|| {
        format!(
            "File load failed.\nPath: {}\nReason: IOException",
            original.display()
        )
    })?;

    parser(&data).map_err(|e| {
        error!("{}", e);
        anyhow::anyhow!(
            "File load failed.\nPath: {}\nReason: ParseError\n{}",
            original.display(),
            e
        )
    })
}

/// Writes data to a file using a backup-then-rename scheme.
///
/// 1. Write backup (.bak) & fsync
/// 2. Write temporary file (.tmp) & fsync
/// 3. Rename temporary to original
pub fn write(file: &Path, data: &[u8]) -> Result<()> {
    //  write backup & fsync
    //  write temporary file & fsync
    //  rename temporary to original

    // each of these writes can individually throw, aborting the operation
    // we don't perform any retries, since the error might be persistent
    write_file(&backup_path(file), data)?;
    write_file(&temporary_path(file), data)?;
    // we only perform the final rename if both writes completed successfully

    // Note that, even though we request an atomic rename, this is not actually an atomic
    // operation with respect to system crashes, and not at all on certain filesystems.

    // That's the reason for the double-write scheme, where we first create
    // a backup, then a temporary copy and rename the temporary into the original.
    // Even if replacing the original with the temporary fails, we should
    // still be able to read the new data from its backup; if creating
    // the backup fails, the original file will remain untouched.

    let temp = temporary_path(file);
    match fs::rename(&temp, file) {
        Ok(()) => {}
        Err(e) => {
            // On some filesystems/platforms, rename may not be atomic.
            // Java catches AtomicMoveNotSupportedException and retries without ATOMIC_MOVE.
            // std::fs::rename is already the non-atomic fallback on most systems,
            // so we just warn and propagate if it truly fails.
            warn!(
                "RobustFile.write: Could not perform an atomic move to {}",
                file.display()
            );
            // In Java, the fallback is Files.move without ATOMIC_MOVE.
            // std::fs::rename is already the equivalent of that fallback.
            // If rename itself failed, propagate the error.
            return Err(e)
                .with_context(|| format!("Failed to rename temporary file to {}", file.display()));
        }
    }

    // This approach does nothing whatsoever to protect against in-memory data corruption,
    // or erroneous writes; which means this operation can complete successfully,
    // but as a result overwrites the config file with unusable data.
    // Checksumming each file and verifying after the write would be an expensive operation,
    // and possibly unproductive on systems where we can't ensure that the data we read
    // back actually comes from the device, rather than from cache.

    // In the case that both the original and backup files become damaged
    // and cannot be loaded, we might want to consider entirely preventing
    // the game from launching and inadvertently resetting the config file
    // to default values, as minor corruption might still be manually fixable.

    Ok(())
}

/// Writes data to a file and calls fsync to ensure it is flushed to device.
///
/// Equivalent to Java's `FileChannel.open(file, CREATE, TRUNCATE_EXISTING, WRITE)`
/// followed by `FileChannel.force(true)`.
pub fn write_file(file: &Path, data: &[u8]) -> Result<()> {
    let mut f = fs::File::create(file)
        .with_context(|| format!("Failed to create file: {}", file.display()))?;
    f.write_all(data)
        .with_context(|| format!("Failed to write data to file: {}", file.display()))?;

    f.sync_all()
        .with_context(|| format!("Failed to fsync file: {}", file.display()))?;

    // force / sync_all corresponds to:
    // on linux, fsync(fd)
    // on macOS, fcntl(fd, F_FULLFSYNC)
    // on windows, FlushFileBuffers(hFile)

    // all of these should request that the data is
    // actually written to device before proceeding

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_parser(data: &[u8]) -> std::result::Result<String, ParseError> {
        String::from_utf8(data.to_vec()).map_err(|e| ParseError(e.to_string()))
    }

    fn failing_parser(_data: &[u8]) -> std::result::Result<String, ParseError> {
        Err(ParseError("always fails".to_string()))
    }

    #[test]
    fn test_backup_path() {
        let p = Path::new("/some/dir/config.json");
        assert_eq!(backup_path(p), PathBuf::from("/some/dir/config.json.bak"));
    }

    #[test]
    fn test_temporary_path() {
        let p = Path::new("/some/dir/config.json");
        assert_eq!(
            temporary_path(p),
            PathBuf::from("/some/dir/config.json.tmp")
        );
    }

    #[test]
    fn test_load_reads_original_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello").unwrap();

        let result = load(&file, test_parser).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_load_falls_back_to_backup() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        let backup = dir.path().join("test.txt.bak");
        // original does not exist
        fs::write(&backup, b"backup_data").unwrap();

        let result = load(&file, test_parser).unwrap();
        assert_eq!(result, "backup_data");
    }

    #[test]
    fn test_load_falls_back_on_parse_error() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        let backup = dir.path().join("test.txt.bak");
        fs::write(&file, b"original").unwrap();
        fs::write(&backup, b"backup_data").unwrap();

        // Parser that fails on "original" but succeeds on "backup_data"
        let parser = |data: &[u8]| -> std::result::Result<String, ParseError> {
            let s = String::from_utf8(data.to_vec()).unwrap();
            if s == "original" {
                Err(ParseError("bad data".to_string()))
            } else {
                Ok(s)
            }
        };

        let result = load(&file, parser).unwrap();
        assert_eq!(result, "backup_data");
    }

    #[test]
    fn test_load_backup_no_backup_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");

        let result = load_backup(&file, test_parser);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No backup file"));
    }

    #[test]
    fn test_load_backup_parse_error() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        let backup = dir.path().join("test.txt.bak");
        fs::write(&backup, b"anything").unwrap();

        let result = load_backup(&file, failing_parser);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("ParseError"));
    }

    #[test]
    fn test_write_creates_backup_and_original() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");

        write(&file, b"test_data").unwrap();

        // Original file should exist with correct content
        assert_eq!(fs::read(&file).unwrap(), b"test_data");
        // Backup should exist with correct content
        let backup = dir.path().join("test.txt.bak");
        assert_eq!(fs::read(&backup).unwrap(), b"test_data");
        // Temporary file should have been renamed away
        let temp = dir.path().join("test.txt.tmp");
        assert!(!temp.exists());
    }

    #[test]
    fn test_write_file_syncs() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("synced.txt");

        write_file(&file, b"synced_data").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"synced_data");
    }

    #[test]
    fn test_round_trip_write_then_load() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("config.json");

        let original_data = b"{\"key\": \"value\"}";
        write(&file, original_data).unwrap();

        let loaded = load(&file, test_parser).unwrap();
        assert_eq!(loaded, "{\"key\": \"value\"}");
    }
}
