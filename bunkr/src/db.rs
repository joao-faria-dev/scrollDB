use crate::error::{Error, Result};
use crate::storage::Header;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Database handle for Bunkr
pub struct Database {
    path: PathBuf,
    file: Option<File>,
}

impl Database {
    /// Open or create a database at the given path
    ///
    /// If the file exists, it will be opened and the header validated.
    /// If the file doesn't exist, it will be created with a new header.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file_exists = path.exists();

        let mut file = if file_exists {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(Error::Io)?
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .map_err(Error::Io)?
        };

        if file_exists {
            // Validate existing file
            let header = Header::read_from(&mut file)?;
            header.validate()?;
        } else {
            // Create new file with header
            let header = Header::new();
            header.write_to(&mut file)?;
        }

        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// Close the database
    ///
    /// This flushes any pending writes and closes the file handle.
    pub fn close(mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush().map_err(Error::Io)?;
        }
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_new_file_creates_header() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bunkr");

        let db = Database::open(&path).unwrap();
        db.close().unwrap();

        // Verify header exists
        let mut file = File::open(&path).unwrap();
        let header = Header::read_from(&mut file).unwrap();
        assert_eq!(header.magic, crate::storage::MAGIC_BYTES);
        assert_eq!(header.version, crate::storage::FILE_VERSION);
    }

    #[test]
    fn test_open_existing_file_validates_header() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bunkr");

        // Create database
        let db1 = Database::open(&path).unwrap();
        db1.close().unwrap();

        // Reopen database
        let db2 = Database::open(&path).unwrap();
        db2.close().unwrap();
    }
}
