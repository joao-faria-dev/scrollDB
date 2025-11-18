use crate::error::{Error, Result};
use crate::storage::page::{PageHeader, PageId, PageType, PAGE_SIZE};
use crate::storage::HEADER_SIZE;
use std::io::{Seek, SeekFrom, Write};

/// Manages page allocation and deallocation
pub struct PageManager {
    /// File handle (must be kept open)
    file: std::fs::File,
    /// First free page ID (None = no free pages)
    first_free_page: Option<PageId>,
    /// Next page ID to allocate (if no free pages available)
    next_page_id: PageId,
}

impl PageManager {
    /// Initialize page manager from existing file
    pub fn from_file(mut file: std::fs::File) -> Result<Self> {
        // Read free page list from header reserved space
        // For now, we'll use a simple approach: scan for free pages
        // In a production system, we'd store this in the header or a metadata page
        
        // Check file size to determine next_page_id
        let metadata = file.metadata().map_err(Error::Io)?;
        let file_size = metadata.len();
        
        let next_page_id = if file_size <= HEADER_SIZE as u64 {
            0
        } else {
            // Calculate how many pages exist
            let data_size = file_size - HEADER_SIZE as u64;
            (data_size / PAGE_SIZE as u64) as PageId
        };

        // Find first free page by scanning
        let first_free_page = Self::find_first_free_page(&mut file, next_page_id)?;

        Ok(Self {
            file,
            first_free_page: if first_free_page == Self::NO_NEXT_PAGE { None } else { Some(first_free_page) },
            next_page_id,
        })
    }

    /// Create new page manager for a new file
    pub fn new(file: std::fs::File) -> Self {
        Self {
            file,
            first_free_page: None,
            next_page_id: 0,
        }
    }

    /// Find the first free page by scanning
    fn find_first_free_page(file: &mut std::fs::File, max_page: PageId) -> Result<PageId> {
        for page_id in 0..max_page {
            let header = match PageHeader::read_from(file, page_id) {
                Ok(h) => h,
                Err(_) => continue,
            };
            
            if header.page_type == PageType::Free {
                return Ok(page_id);
            }
        }
        Ok(Self::NO_NEXT_PAGE) // No free pages found
    }

    /// Sentinel value for "no next page" in free page chain
    const NO_NEXT_PAGE: PageId = u32::MAX;

    /// Allocate a new page
    pub fn allocate_page(&mut self, page_type: PageType) -> Result<PageId> {
        let page_id = if let Some(free_page_id) = self.first_free_page {
            // Reuse a free page
            let page_id = free_page_id;
            let header = PageHeader::read_from(&mut self.file, page_id)?;
            
            // Update first_free_page to next free page in chain
            self.first_free_page = if header.next_page == Self::NO_NEXT_PAGE {
                None
            } else {
                Some(header.next_page)
            };
            
            // Update page header
            let new_header = PageHeader::new(page_type);
            new_header.write_to(&mut self.file, page_id)?;
            
            page_id
        } else {
            // Allocate new page
            let page_id = self.next_page_id;
            self.next_page_id += 1;
            
            // Create and write new page header
            let header = PageHeader::new(page_type);
            header.write_to(&mut self.file, page_id)?;
            
            // Ensure file is large enough
            let offset = PageHeader::page_offset(page_id) + PAGE_SIZE as u64;
            self.file.seek(SeekFrom::Start(offset - 1))?;
            self.file.write_all(&[0])?;
            
            page_id
        };

        Ok(page_id)
    }

    /// Deallocate a page (mark as free)
    pub fn deallocate_page(&mut self, page_id: PageId) -> Result<()> {
        // Read current header
        let header = PageHeader::read_from(&mut self.file, page_id)?;
        
        // If already free, nothing to do
        if header.page_type == PageType::Free {
            return Ok(());
        }

        // Create free page header
        let mut free_header = PageHeader::free();
        
        // Link to first free page
        free_header.next_page = self.first_free_page.unwrap_or(Self::NO_NEXT_PAGE);
        self.first_free_page = Some(page_id);
        
        // Clear page data
        free_header.data_size = 0;
        free_header.write_to(&mut self.file, page_id)?;
        
        // Zero out the page data area
        let data_offset = PageHeader::page_offset(page_id) + PageHeader::SIZE as u64;
        self.file.seek(SeekFrom::Start(data_offset))?;
        self.file.write_all(&vec![0u8; PageHeader::max_data_size()])?;

        Ok(())
    }

    /// Get file handle (for reading/writing page data)
    pub fn file(&mut self) -> &mut std::fs::File {
        &mut self.file
    }

    /// Flush all writes to disk
    pub fn flush(&mut self) -> Result<()> {
        self.file.flush().map_err(Error::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Header;
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_test_file() -> (NamedTempFile, std::fs::File) {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        
        // Write database header
        let header = Header::new();
        header.write_to(&mut file).unwrap();
        
        (temp_file, file)
    }

    #[test]
    fn test_allocate_first_page() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);
        
        let page_id = manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(page_id, 0);
        
        // Verify page header
        let header = PageHeader::read_from(manager.file(), page_id).unwrap();
        assert_eq!(header.page_type, PageType::Data);
    }

    #[test]
    fn test_allocate_multiple_pages() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);
        
        let page1 = manager.allocate_page(PageType::Data).unwrap();
        let page2 = manager.allocate_page(PageType::Data).unwrap();
        let page3 = manager.allocate_page(PageType::Index).unwrap();
        
        assert_eq!(page1, 0);
        assert_eq!(page2, 1);
        assert_eq!(page3, 2);
        
        // Verify headers
        let header1 = PageHeader::read_from(manager.file(), page1).unwrap();
        let header2 = PageHeader::read_from(manager.file(), page2).unwrap();
        let header3 = PageHeader::read_from(manager.file(), page3).unwrap();
        
        assert_eq!(header1.page_type, PageType::Data);
        assert_eq!(header2.page_type, PageType::Data);
        assert_eq!(header3.page_type, PageType::Index);
    }

    #[test]
    fn test_deallocate_page() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);
        
        let page_id = manager.allocate_page(PageType::Data).unwrap();
        
        // Deallocate
        manager.deallocate_page(page_id).unwrap();
        
        // Verify page is marked as free
        let header = PageHeader::read_from(manager.file(), page_id).unwrap();
        assert_eq!(header.page_type, PageType::Free);
    }

    #[test]
    fn test_reuse_free_page() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);
        
        // Allocate and deallocate a page
        let page1 = manager.allocate_page(PageType::Data).unwrap();
        manager.deallocate_page(page1).unwrap();
        
        // Allocate again - should reuse the free page
        let page2 = manager.allocate_page(PageType::Data).unwrap();
        assert_eq!(page1, page2);
        
        // Verify it's no longer free
        let header = PageHeader::read_from(manager.file(), page2).unwrap();
        assert_eq!(header.page_type, PageType::Data);
    }

    #[test]
    fn test_free_page_chain() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);
        
        // Allocate and deallocate multiple pages
        let page1 = manager.allocate_page(PageType::Data).unwrap();
        let page2 = manager.allocate_page(PageType::Data).unwrap();
        let page3 = manager.allocate_page(PageType::Data).unwrap();
        
        manager.deallocate_page(page1).unwrap();
        manager.deallocate_page(page2).unwrap();
        manager.deallocate_page(page3).unwrap();
        
        // Allocate again - should reuse pages in reverse order (LIFO)
        // The chain is: page3 -> page2 -> page1, so we should get them in that order
        let reused1 = manager.allocate_page(PageType::Data).unwrap();
        let reused2 = manager.allocate_page(PageType::Data).unwrap();
        let reused3 = manager.allocate_page(PageType::Data).unwrap();
        
        // Verify all three pages were reused (they should be page3, page2, page1 in LIFO order)
        let reused_pages = vec![reused1, reused2, reused3];
        assert!(reused_pages.contains(&page1), "page1 should be reused");
        assert!(reused_pages.contains(&page2), "page2 should be reused");
        assert!(reused_pages.contains(&page3), "page3 should be reused");
        assert_eq!(reused_pages.len(), 3, "Should have exactly 3 reused pages");
    }
}

