use crate::error::{Error, Result};
use std::io::{Read, Seek, SeekFrom, Write};

/// Page size in bytes (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Page number type
pub type PageId = u32;

/// Page types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageType {
    /// Free page (available for allocation)
    Free = 0,
    /// Data page (contains document data)
    Data = 1,
    /// Index page (contains index data)
    Index = 2,
    /// Metadata page (contains collection/index metadata)
    Metadata = 3,
}

impl PageType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PageType::Free),
            1 => Some(PageType::Data),
            2 => Some(PageType::Index),
            3 => Some(PageType::Metadata),
            _ => None,
        }
    }
}

/// Page header structure (16 bytes)
/// 
/// Layout:
/// - page_type: u8 (1 byte)
/// - flags: u8 (1 byte)
/// - next_page: u32 (4 bytes) - for multi-page documents
/// - data_size: u32 (4 bytes) - size of data in this page
/// - reserved: u8[6] (6 bytes)
#[derive(Debug, Clone)]
pub struct PageHeader {
    /// Type of page
    pub page_type: PageType,
    /// Flags (currently unused, reserved for future use)
    pub flags: u8,
    /// Next page ID if this is part of a multi-page structure (0 = none)
    pub next_page: PageId,
    /// Size of data in this page (excluding header)
    pub data_size: u32,
    /// Reserved bytes
    pub reserved: [u8; 6],
}

impl PageHeader {
    /// Size of page header in bytes
    pub const SIZE: usize = 16;

    /// Create a new page header
    pub fn new(page_type: PageType) -> Self {
        Self {
            page_type,
            flags: 0,
            next_page: 0,
            data_size: 0,
            reserved: [0; 6],
        }
    }

    /// Create a free page header
    pub fn free() -> Self {
        Self::new(PageType::Free)
    }

    /// Create a data page header
    pub fn data() -> Self {
        Self::new(PageType::Data)
    }

    /// Read page header from file at given page offset
    pub fn read_from(file: &mut std::fs::File, page_id: PageId) -> Result<Self> {
        let offset = Self::page_offset(page_id);
        file.seek(SeekFrom::Start(offset))?;

        let mut page_type_bytes = [0u8; 1];
        file.read_exact(&mut page_type_bytes)?;
        let page_type = PageType::from_u8(page_type_bytes[0])
            .ok_or_else(|| Error::CorruptedDatabase {
                reason: format!("Invalid page type: {}", page_type_bytes[0]),
            })?;

        let mut flags_bytes = [0u8; 1];
        file.read_exact(&mut flags_bytes)?;
        let flags = flags_bytes[0];

        let mut next_page_bytes = [0u8; 4];
        file.read_exact(&mut next_page_bytes)?;
        let next_page = u32::from_le_bytes(next_page_bytes);

        let mut data_size_bytes = [0u8; 4];
        file.read_exact(&mut data_size_bytes)?;
        let data_size = u32::from_le_bytes(data_size_bytes);

        let mut reserved = [0u8; 6];
        file.read_exact(&mut reserved)?;

        Ok(Self {
            page_type,
            flags,
            next_page,
            data_size,
            reserved,
        })
    }

    /// Write page header to file at given page offset
    pub fn write_to(&self, file: &mut std::fs::File, page_id: PageId) -> Result<()> {
        let offset = Self::page_offset(page_id);
        file.seek(SeekFrom::Start(offset))?;

        file.write_all(&[self.page_type as u8])?;
        file.write_all(&[self.flags])?;
        file.write_all(&self.next_page.to_le_bytes())?;
        file.write_all(&self.data_size.to_le_bytes())?;
        file.write_all(&self.reserved)?;

        Ok(())
    }

    /// Calculate file offset for a given page ID
    pub fn page_offset(page_id: PageId) -> u64 {
        crate::storage::HEADER_SIZE as u64 + (page_id as u64 * PAGE_SIZE as u64)
    }

    /// Get the maximum data size that can fit in a page
    pub fn max_data_size() -> usize {
        PAGE_SIZE - Self::SIZE
    }
}

/// Represents a single page in the database
#[derive(Debug, Clone)]
pub struct Page {
    /// Page ID
    pub id: PageId,
    /// Page header
    pub header: PageHeader,
    /// Page data (excluding header)
    pub data: Vec<u8>,
}

impl Page {
    /// Create a new empty page
    pub fn new(id: PageId, page_type: PageType) -> Self {
        Self {
            id,
            header: PageHeader::new(page_type),
            data: Vec::new(),
        }
    }

    /// Read a page from file
    pub fn read_from(file: &mut std::fs::File, page_id: PageId) -> Result<Self> {
        let header = PageHeader::read_from(file, page_id)?;
        
        let offset = PageHeader::page_offset(page_id) + PageHeader::SIZE as u64;
        file.seek(SeekFrom::Start(offset))?;

        let mut data = vec![0u8; header.data_size as usize];
        if !data.is_empty() {
            file.read_exact(&mut data)?;
        }

        Ok(Self {
            id: page_id,
            header,
            data,
        })
    }

    /// Write a page to file
    pub fn write_to(&self, file: &mut std::fs::File) -> Result<()> {
        // Update header with current data size
        let mut header = self.header.clone();
        header.data_size = self.data.len() as u32;
        
        header.write_to(file, self.id)?;

        // Write data
        let offset = PageHeader::page_offset(self.id) + PageHeader::SIZE as u64;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&self.data)?;

        // Pad remaining space with zeros if needed
        let remaining = PAGE_SIZE - PageHeader::SIZE - self.data.len();
        if remaining > 0 {
            file.write_all(&vec![0u8; remaining])?;
        }

        Ok(())
    }

    /// Get available space in this page
    pub fn available_space(&self) -> usize {
        PageHeader::max_data_size() - self.data.len()
    }

    /// Check if page has space for given size
    pub fn has_space(&self, size: usize) -> bool {
        self.available_space() >= size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_page_header_size() {
        assert_eq!(PageHeader::SIZE, 16);
    }

    #[test]
    fn test_page_header_new() {
        let header = PageHeader::new(PageType::Data);
        assert_eq!(header.page_type, PageType::Data);
        assert_eq!(header.flags, 0);
        assert_eq!(header.next_page, 0);
        assert_eq!(header.data_size, 0);
    }

    #[test]
    fn test_page_header_write_read() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write header
        let mut header = PageHeader::new(PageType::Data);
        header.next_page = 5;
        header.data_size = 100;

        // Create file with header space
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        
        // Write database header first
        let db_header = crate::storage::Header::new();
        db_header.write_to(&mut file).unwrap();

        header.write_to(&mut file, 0).unwrap();

        // Read header
        let read_header = PageHeader::read_from(&mut file, 0).unwrap();
        assert_eq!(read_header.page_type, PageType::Data);
        assert_eq!(read_header.next_page, 5);
        assert_eq!(read_header.data_size, 100);
    }

    #[test]
    fn test_page_offset() {
        assert_eq!(PageHeader::page_offset(0), crate::storage::HEADER_SIZE as u64);
        assert_eq!(PageHeader::page_offset(1), crate::storage::HEADER_SIZE as u64 + PAGE_SIZE as u64);
        assert_eq!(PageHeader::page_offset(10), crate::storage::HEADER_SIZE as u64 + (10 * PAGE_SIZE) as u64);
    }

    #[test]
    fn test_max_data_size() {
        let max = PageHeader::max_data_size();
        assert_eq!(max, PAGE_SIZE - PageHeader::SIZE);
        assert_eq!(max, 4080); // 4096 - 16
    }

    #[test]
    fn test_page_new() {
        let page = Page::new(0, PageType::Data);
        assert_eq!(page.id, 0);
        assert_eq!(page.header.page_type, PageType::Data);
        assert_eq!(page.data.len(), 0);
    }

    #[test]
    fn test_page_write_read() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Create file with header
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        
        let db_header = crate::storage::Header::new();
        db_header.write_to(&mut file).unwrap();

        // Write page
        let mut page = Page::new(0, PageType::Data);
        page.data = b"Hello, World!".to_vec();
        page.write_to(&mut file).unwrap();

        // Read page
        let read_page = Page::read_from(&mut file, 0).unwrap();
        assert_eq!(read_page.id, 0);
        assert_eq!(read_page.header.page_type, PageType::Data);
        assert_eq!(read_page.data, b"Hello, World!");
        assert_eq!(read_page.header.data_size, 13);
    }

    #[test]
    fn test_page_available_space() {
        let mut page = Page::new(0, PageType::Data);
        assert_eq!(page.available_space(), PageHeader::max_data_size());
        
        page.data = vec![0u8; 100];
        assert_eq!(page.available_space(), PageHeader::max_data_size() - 100);
    }

    #[test]
    fn test_page_has_space() {
        let mut page = Page::new(0, PageType::Data);
        assert!(page.has_space(100));
        assert!(page.has_space(PageHeader::max_data_size()));
        assert!(!page.has_space(PageHeader::max_data_size() + 1));
        
        page.data = vec![0u8; 100];
        assert!(page.has_space(PageHeader::max_data_size() - 100));
        assert!(!page.has_space(PageHeader::max_data_size() - 99));
    }
}

