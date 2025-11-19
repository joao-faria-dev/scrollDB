use crate::collection::Collection;
use crate::error::{Error, Result};
use crate::storage::{Header, PageManager};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

/// Database handle for ScrollDB
pub struct Database {
    path: PathBuf,
    page_manager: Option<PageManager>,
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
                .truncate(true)
                .open(&path)
                .map_err(Error::Io)?
        };

        if file_exists {
            // Check if file is empty
            let metadata = file.metadata().map_err(Error::Io)?;
            if metadata.len() == 0 {
                return Err(Error::CorruptedDatabase {
                    reason: "File exists but is empty".to_string(),
                });
            }

            // Validate existing file
            let header = Header::read_from(&mut file)?;
            header.validate()?;
        } else {
            // Create new file with header
            let header = Header::new();
            header.write_to(&mut file)?;
        }

        // Create page manager
        let page_manager = if file_exists {
            PageManager::from_file(file)?
        } else {
            PageManager::new(file)
        };

        Ok(Self {
            path,
            page_manager: Some(page_manager),
        })
    }

    /// Check if the database is currently open
    pub fn is_open(&self) -> bool {
        self.page_manager.is_some()
    }

    /// Get a collection by name
    ///
    /// Collections are created on-demand when first accessed.
    /// Note: This creates a new PageManager for the collection.
    /// In a production system, we'd share the PageManager across collections.
    pub fn collection(&mut self, name: &str) -> Result<Collection> {
        if self.page_manager.is_none() {
            return Err(Error::DatabaseNotOpen);
        }

        // Create a new page manager from the file for this collection
        // This is not ideal but works for the initial implementation
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(Error::Io)?;

        let page_manager = PageManager::from_file(file)?;
        let collection = Collection::new(name.to_string(), page_manager, self.path.clone());

        Ok(collection)
    }

    /// Close the database
    ///
    /// This flushes any pending writes and closes the file handle.
    pub fn close(mut self) -> Result<()> {
        if let Some(mut page_manager) = self.page_manager.take() {
            page_manager.flush()?;
        }
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        if let Some(mut page_manager) = self.page_manager.take() {
            let _ = page_manager.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_open_new_file_creates_header() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

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
        let path = temp_dir.path().join("test.scrolldb");

        // Create database
        let db1 = Database::open(&path).unwrap();
        db1.close().unwrap();

        // Reopen database
        let db2 = Database::open(&path).unwrap();
        db2.close().unwrap();
    }

    #[test]
    fn test_is_open() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        let db = Database::open(&path).unwrap();
        assert!(db.is_open());
        db.close().unwrap();
    }

    #[test]
    fn test_open_empty_file_errors() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        // Create empty file
        std::fs::File::create(&path).unwrap();

        let result = Database::open(&path);
        assert!(result.is_err());
        if let Err(Error::CorruptedDatabase { reason }) = result {
            assert!(reason.contains("empty"));
        } else {
            panic!("Expected CorruptedDatabase error");
        }
    }

    #[test]
    fn test_open_corrupted_file_errors() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        // Create file with invalid magic bytes
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"INVALID\0").unwrap();
        file.write_all(&0u32.to_le_bytes()).unwrap();
        file.write_all(&[0u8; 20]).unwrap();
        drop(file);

        let result = Database::open(&path);
        assert!(result.is_err());
        if let Err(Error::InvalidFileFormat { .. }) = result {
            // Expected error
        } else {
            panic!("Expected InvalidFileFormat error");
        }
    }

    #[test]
    fn test_open_invalid_version_errors() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        // Create file with valid magic but invalid version
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&crate::storage::MAGIC_BYTES).unwrap();
        file.write_all(&999u32.to_le_bytes()).unwrap(); // Invalid version
        file.write_all(&[0u8; 20]).unwrap();
        drop(file);

        let result = Database::open(&path);
        assert!(result.is_err());
        if let Err(Error::CorruptedDatabase { reason }) = result {
            assert!(reason.contains("version"));
        } else {
            panic!("Expected CorruptedDatabase error");
        }
    }

    #[test]
    fn test_close_flushes_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        let db = Database::open(&path).unwrap();
        assert!(db.is_open());
        db.close().unwrap();

        // Verify file still exists and has valid header
        let mut file = File::open(&path).unwrap();
        let header = Header::read_from(&mut file).unwrap();
        assert_eq!(header.magic, crate::storage::MAGIC_BYTES);
    }

    #[test]
    fn test_drop_closes_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        {
            let db = Database::open(&path).unwrap();
            assert!(db.is_open());
            // db is dropped here
        }

        // Verify file still exists and has valid header after drop
        let mut file = File::open(&path).unwrap();
        let header = Header::read_from(&mut file).unwrap();
        assert_eq!(header.magic, crate::storage::MAGIC_BYTES);
    }

    #[test]
    fn test_open_close_reopen_cycle() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.scrolldb");

        // First open
        let db1 = Database::open(&path).unwrap();
        assert!(db1.is_open());
        db1.close().unwrap();

        // Second open
        let db2 = Database::open(&path).unwrap();
        assert!(db2.is_open());
        db2.close().unwrap();

        // Third open
        let db3 = Database::open(&path).unwrap();
        assert!(db3.is_open());
        db3.close().unwrap();

        // Verify file is still valid
        let mut file = File::open(&path).unwrap();
        let header = Header::read_from(&mut file).unwrap();
        assert_eq!(header.magic, crate::storage::MAGIC_BYTES);
        assert_eq!(header.version, crate::storage::FILE_VERSION);
    }
}
