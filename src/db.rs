use crate::error::{Error, Result};
use crate::storage::Header;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Database {
    path: PathBuf,
    file: Option<File>,
}

impl Database {
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
            let header = Header::read_from(&mut file)?;
            header.validate()?;
        } else {
            let header = Header::new();
            header.write_to(&mut file)?;
        }

        Ok(Self {
            path,
            file: Some(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

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
