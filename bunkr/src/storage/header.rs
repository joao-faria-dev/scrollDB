use crate::error::{Error, Result};
use std::io::{Read, Seek, SeekFrom, Write};

/// Magic bytes identifying a Bunkr database file
pub const MAGIC_BYTES: [u8; 8] = *b"BUNKR\0\0\0";

/// Current file format version
pub const FILE_VERSION: u32 = 1;

/// Total size of the file header in bytes
pub const HEADER_SIZE: usize = 32;

/// Database file header
#[derive(Debug, Clone)]
pub struct Header {
    /// Magic bytes to identify Bunkr files
    pub magic: [u8; 8],
    /// File format version
    pub version: u32,
    /// Reserved space for future use
    pub reserved: [u8; 20],
}

impl Header {
    /// Create a new header with default values
    pub fn new() -> Self {
        Self {
            magic: MAGIC_BYTES,
            version: FILE_VERSION,
            reserved: [0; 20],
        }
    }

    /// Read header from a file
    pub fn read_from(file: &mut std::fs::File) -> Result<Self> {
        file.seek(SeekFrom::Start(0))?;

        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;

        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);

        let mut reserved = [0u8; 20];
        file.read_exact(&mut reserved)?;

        Ok(Self {
            magic,
            version,
            reserved,
        })
    }

    /// Write header to a file
    pub fn write_to(&self, file: &mut std::fs::File) -> Result<()> {
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&self.magic)?;
        file.write_all(&self.version.to_le_bytes())?;
        file.write_all(&self.reserved)?;
        file.flush()?;
        Ok(())
    }

    /// Validate the header
    pub fn validate(&self) -> Result<()> {
        if self.magic != MAGIC_BYTES {
            return Err(Error::InvalidFileFormat {
                expected: format!("{:?}", MAGIC_BYTES),
                found: format!("{:?}", self.magic),
            });
        }

        if self.version != FILE_VERSION {
            return Err(Error::CorruptedDatabase {
                reason: format!(
                    "Unsupported file version: expected {}, found {}",
                    FILE_VERSION, self.version
                ),
            });
        }

        Ok(())
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_header_new() {
        let header = Header::new();
        assert_eq!(header.magic, MAGIC_BYTES);
        assert_eq!(header.version, FILE_VERSION);
    }

    #[test]
    fn test_header_write_read() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write header
        let header = Header::new();
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        header.write_to(&mut file).unwrap();

        // Read header
        let mut file = fs::File::open(path).unwrap();
        let read_header = Header::read_from(&mut file).unwrap();

        assert_eq!(read_header.magic, header.magic);
        assert_eq!(read_header.version, header.version);
    }

    #[test]
    fn test_header_validate_success() {
        let header = Header::new();
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_header_validate_invalid_magic() {
        let mut header = Header::new();
        header.magic = [0; 8];
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_header_validate_invalid_version() {
        let mut header = Header::new();
        header.version = 999;
        assert!(header.validate().is_err());
    }
}

