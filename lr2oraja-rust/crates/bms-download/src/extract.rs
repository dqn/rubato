// Archive extraction
//
// Provides extraction for tar.gz, zip, and lzh/lha archives.
// 7z support is omitted for now (would require sevenz-rust).

use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::Path;

use anyhow::{Context, bail};
use flate2::read::GzDecoder;

/// Extract a .tar.gz archive to the destination directory.
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("failed to open {:?}", archive_path))?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest_dir)
        .with_context(|| format!("failed to extract {:?} to {:?}", archive_path, dest_dir))?;
    Ok(())
}

/// Extract a .zip archive to the destination directory.
///
/// Uses `enclosed_name()` to prevent path traversal attacks.
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("failed to open {:?}", archive_path))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read zip archive {:?}", archive_path))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let name = match entry.enclosed_name() {
            Some(name) => name.to_owned(),
            None => continue, // skip entries with unsafe paths
        };
        let out_path = dest_dir.join(&name);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = File::create(&out_path)?;
            io::copy(&mut entry, &mut out_file)?;
        }
    }

    Ok(())
}

/// Extract a .lzh/.lha archive to the destination directory.
///
/// Validates that extracted paths do not escape the destination directory.
pub fn extract_lzh(archive_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let mut decoder = delharc::parse_file(archive_path)
        .with_context(|| format!("failed to open lzh archive {:?}", archive_path))?;

    let dest_canonical = dest_dir
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {:?}", dest_dir))?;

    loop {
        let header = decoder.header();
        let entry_path = header.parse_pathname();

        let out_path = dest_dir.join(&entry_path);

        // Prevent path traversal: ensure the resolved path stays within dest_dir.
        // We need to create parent dirs first so canonicalize works on the parent.
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let parent_canonical = out_path
            .parent()
            .unwrap_or(dest_dir)
            .canonicalize()
            .unwrap_or_default();
        let resolved = parent_canonical.join(out_path.file_name().unwrap_or_default());

        if !resolved.starts_with(&dest_canonical) {
            bail!("path traversal detected in lzh archive: {:?}", entry_path);
        }

        if header.is_directory() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = File::create(&out_path)?;
            io::copy(&mut decoder, &mut out_file)?;
        }

        if !decoder
            .next_file()
            .map_err(|e| anyhow::anyhow!("failed to read next lzh entry: {:?}", e))?
        {
            break;
        }
    }

    Ok(())
}

/// Detect archive format by extension and extract.
///
/// Currently supports:
/// - `.tar.gz`, `.tgz` — tar + gzip
/// - `.zip` — zip
/// - `.lzh`, `.lha` — LHA/LZH
///
/// Returns an error for unsupported formats.
pub fn detect_and_extract(archive_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive_path, dest_dir)
    } else if name.ends_with(".zip") {
        extract_zip(archive_path, dest_dir)
    } else if name.ends_with(".lzh") || name.ends_with(".lha") {
        extract_lzh(archive_path, dest_dir)
    } else {
        bail!(
            "unsupported archive format: {:?} (supported: .tar.gz, .tgz, .zip, .lzh, .lha)",
            archive_path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    /// Create a minimal tar.gz archive containing a single file.
    fn create_test_tar_gz(
        dir: &Path,
        archive_name: &str,
        file_name: &str,
        content: &[u8],
    ) -> std::path::PathBuf {
        let archive_path = dir.join(archive_name);

        let file = File::create(&archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);

        let mut header = tar::Header::new_gnu();
        header.set_path(file_name).unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, content).unwrap();
        builder.finish().unwrap();

        archive_path
    }

    #[test]
    fn test_extract_tar_gz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_dir = tmp.path().join("archives");
        let extract_dir = tmp.path().join("output");
        std::fs::create_dir_all(&archive_dir).unwrap();
        std::fs::create_dir_all(&extract_dir).unwrap();

        let archive =
            create_test_tar_gz(&archive_dir, "test.tar.gz", "hello.txt", b"Hello, World!");

        extract_tar_gz(&archive, &extract_dir).unwrap();

        let extracted = extract_dir.join("hello.txt");
        assert!(extracted.exists());
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "Hello, World!"
        );
    }

    #[test]
    fn test_extract_tar_gz_with_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_dir = tmp.path().join("archives");
        let extract_dir = tmp.path().join("output");
        std::fs::create_dir_all(&archive_dir).unwrap();
        std::fs::create_dir_all(&extract_dir).unwrap();

        let archive_path = archive_dir.join("nested.tar.gz");
        let file = File::create(&archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);

        // Add a directory entry
        let mut dir_header = tar::Header::new_gnu();
        dir_header.set_path("subdir/").unwrap();
        dir_header.set_size(0);
        dir_header.set_mode(0o755);
        dir_header.set_entry_type(tar::EntryType::Directory);
        dir_header.set_cksum();
        builder.append(&dir_header, &[] as &[u8]).unwrap();

        // Add a file in the subdirectory
        let content = b"nested content";
        let mut file_header = tar::Header::new_gnu();
        file_header.set_path("subdir/file.bms").unwrap();
        file_header.set_size(content.len() as u64);
        file_header.set_mode(0o644);
        file_header.set_cksum();
        builder.append(&file_header, &content[..]).unwrap();

        // Finish the tar archive and flush the gzip encoder
        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();

        extract_tar_gz(&archive_path, &extract_dir).unwrap();

        let extracted = extract_dir.join("subdir/file.bms");
        assert!(extracted.exists());
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_detect_and_extract_tar_gz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = create_test_tar_gz(tmp.path(), "test.tar.gz", "data.txt", b"test data");

        let extract_dir = tmp.path().join("out");
        std::fs::create_dir_all(&extract_dir).unwrap();

        detect_and_extract(&archive, &extract_dir).unwrap();
        assert!(extract_dir.join("data.txt").exists());
    }

    #[test]
    fn test_detect_and_extract_tgz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = create_test_tar_gz(tmp.path(), "test.tgz", "data.txt", b"tgz data");

        let extract_dir = tmp.path().join("out");
        std::fs::create_dir_all(&extract_dir).unwrap();

        detect_and_extract(&archive, &extract_dir).unwrap();
        assert!(extract_dir.join("data.txt").exists());
    }

    #[test]
    fn test_detect_and_extract_unsupported() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.7z");
        File::create(&path).unwrap().write_all(b"fake").unwrap();

        let extract_dir = tmp.path().join("out");
        std::fs::create_dir_all(&extract_dir).unwrap();

        let result = detect_and_extract(&path, &extract_dir);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unsupported archive format"));
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = extract_tar_gz(&tmp.path().join("nonexistent.tar.gz"), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_invalid_tar_gz() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("invalid.tar.gz");
        File::create(&path)
            .unwrap()
            .write_all(b"not a real archive")
            .unwrap();

        let extract_dir = tmp.path().join("out");
        std::fs::create_dir_all(&extract_dir).unwrap();

        let result = extract_tar_gz(&path, &extract_dir);
        assert!(result.is_err());
    }

    // --- zip tests ---

    #[test]
    fn test_extract_zip_single_file() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("test.zip");
        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        // Create a zip archive with ZipWriter
        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("hello.txt", options).unwrap();
            writer.write_all(b"Hello from zip!").unwrap();
            writer.finish().unwrap();
        }

        extract_zip(&archive_path, &extract_dir).unwrap();

        let extracted = extract_dir.join("hello.txt");
        assert!(extracted.exists());
        assert_eq!(fs::read_to_string(&extracted).unwrap(), "Hello from zip!");
    }

    #[test]
    fn test_extract_zip_with_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("nested.zip");
        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            writer
                .add_directory("subdir/", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.start_file("subdir/data.bms", options).unwrap();
            writer.write_all(b"bms data").unwrap();
            writer.finish().unwrap();
        }

        extract_zip(&archive_path, &extract_dir).unwrap();

        let extracted = extract_dir.join("subdir/data.bms");
        assert!(extracted.exists());
        assert_eq!(fs::read_to_string(&extracted).unwrap(), "bms data");
    }

    #[test]
    fn test_extract_zip_path_traversal_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("evil.zip");
        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        // Create a zip with a path traversal entry using raw API
        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();

            // enclosed_name() will return None for "../evil.txt", so it gets skipped
            writer.start_file("../evil.txt", options).unwrap();
            writer.write_all(b"evil content").unwrap();

            // Also add a safe file
            writer.start_file("safe.txt", options).unwrap();
            writer.write_all(b"safe content").unwrap();

            writer.finish().unwrap();
        }

        extract_zip(&archive_path, &extract_dir).unwrap();

        // The evil file should not exist outside extract_dir
        assert!(!tmp.path().join("evil.txt").exists());
        // The safe file should exist
        assert!(extract_dir.join("safe.txt").exists());
    }

    #[test]
    fn test_detect_and_extract_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("test.zip");
        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("detected.txt", options).unwrap();
            writer.write_all(b"detected").unwrap();
            writer.finish().unwrap();
        }

        detect_and_extract(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.join("detected.txt").exists());
    }

    // --- lzh tests ---

    // lzh archives are hard to create programmatically, so we test with
    // a minimal binary fixture. For now, test the detect_and_extract routing.

    #[test]
    fn test_detect_and_extract_lzh_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.lzh");
        File::create(&path).unwrap().write_all(b"fake").unwrap();

        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        // Should attempt lzh extraction (and fail on invalid data)
        let result = detect_and_extract(&path, &extract_dir);
        assert!(result.is_err());
        // The error should be from lzh parsing, not "unsupported format"
        let err_msg = result.unwrap_err().to_string();
        assert!(!err_msg.contains("unsupported archive format"));
    }

    #[test]
    fn test_detect_and_extract_lha_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.lha");
        File::create(&path).unwrap().write_all(b"fake").unwrap();

        let extract_dir = tmp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();

        let result = detect_and_extract(&path, &extract_dir);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(!err_msg.contains("unsupported archive format"));
    }
}
