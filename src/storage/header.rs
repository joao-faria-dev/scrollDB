use crate::error::{Error, Result};
use std::io::{Read, Seek, SeekFrom, Write};

pub const MAGIC_BYTES: [u8; 8] = *b"scrol\0\0\0";

pub const FILE_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct Header {
    pub magic: [u8; 8],
    pub version: u32,
    pub reserved: [u8; 20],
}

impl Header {
    pub fn new() -> Self {
        Self {
            magic: MAGIC_BYTES,
            version: FILE_VERSION,
            reserved: [0; 20],
        }
    }

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

    pub fn write_to(&self, file: &mut std::fs::File) -> Result<()> {
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&self.magic)?;
        file.write_all(&self.version.to_le_bytes())?;
        file.write_all(&self.reserved)?;
        file.flush()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.magic != MAGIC_BYTES {
            return Err(Error::InvalidFormatError {
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
