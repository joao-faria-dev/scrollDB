use crate::error::{Error, Result};
use crate::storage::page::{Page, PageHeader, PageId, PageType};
use crate::types::{ObjectId, Value};

/// Document metadata stored at the beginning of document data
#[derive(Debug, Clone)]
struct DocumentHeader {
    /// Document ObjectId (_id field)
    object_id: ObjectId,
    /// Total size of document data (across all pages)
    total_size: u32,
}

impl DocumentHeader {
    const SIZE: usize = 16; // ObjectId (12 bytes) + total_size (4 bytes)

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(self.object_id.as_bytes());
        bytes.extend_from_slice(&self.total_size.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(Error::CorruptedDatabase {
                reason: "Document header too short".to_string(),
            });
        }

        let mut id_bytes = [0u8; 12];
        id_bytes.copy_from_slice(&bytes[0..12]);
        let object_id = ObjectId::from_bytes(id_bytes);

        let mut size_bytes = [0u8; 4];
        size_bytes.copy_from_slice(&bytes[12..16]);
        let total_size = u32::from_le_bytes(size_bytes);

        Ok(Self {
            object_id,
            total_size,
        })
    }
}

/// Serialize a document value to bytes using bincode
pub fn serialize_document(value: &Value) -> Result<Vec<u8>> {
    // Use bincode with serde compatibility
    // bincode 2.0 requires Encode/Decode traits, but we can use serde_json as intermediate
    // For now, use a simple approach: serialize to JSON then to bytes
    // In production, we'd want to use bincode directly with proper trait implementations
    let json = serde_json::to_vec(value).map_err(|e| Error::CorruptedDatabase {
        reason: format!("Failed to serialize document to JSON: {}", e),
    })?;
    Ok(json)
}

/// Deserialize bytes to a document value using bincode
pub fn deserialize_document(bytes: &[u8]) -> Result<Value> {
    // Deserialize from JSON bytes
    serde_json::from_slice(bytes).map_err(|e| Error::CorruptedDatabase {
        reason: format!("Failed to deserialize document from JSON: {}", e),
    })
}

/// Write a document to pages
/// 
/// Returns the first page ID where the document was written
pub fn write_document(
    file: &mut std::fs::File,
    object_id: ObjectId,
    value: &Value,
    allocate_page: &mut dyn FnMut() -> Result<PageId>,
) -> Result<PageId> {
    // Serialize document
    let doc_bytes = serialize_document(value)?;
    let doc_size = doc_bytes.len();

    // Create document header
    let doc_header = DocumentHeader {
        object_id,
        total_size: doc_size as u32,
    };
    let header_bytes = doc_header.to_bytes();

    // Total size needed: header + document data
    let total_needed = header_bytes.len() + doc_bytes.len();
    let max_page_data = PageHeader::max_data_size();

    // Calculate how many pages we need
    let pages_needed = if total_needed <= max_page_data {
        1
    } else {
        (total_needed + max_page_data - 1) / max_page_data
    };

    // Allocate pages
    let mut page_ids = Vec::with_capacity(pages_needed);
    for _ in 0..pages_needed {
        page_ids.push(allocate_page()?);
    }

    // Write data across pages
    let mut data_to_write = Vec::with_capacity(total_needed);
    data_to_write.extend_from_slice(&header_bytes);
    data_to_write.extend_from_slice(&doc_bytes);

    let mut offset = 0;
    for (i, &page_id) in page_ids.iter().enumerate() {
        let is_last = i == page_ids.len() - 1;
        let next_page = if is_last { 0 } else { page_ids[i + 1] };

        // Calculate how much data fits in this page
        let page_data_size = if is_last {
            data_to_write.len() - offset
        } else {
            max_page_data
        };

        // Create page
        let mut page = Page::new(page_id, PageType::Data);
        page.data = data_to_write[offset..offset + page_data_size].to_vec();
        page.header.next_page = if is_last { 0 } else { next_page };

        // Write page
        page.write_to(file)?;

        offset += page_data_size;
    }

    Ok(page_ids[0])
}

/// Read a document from pages starting at the given page ID
pub fn read_document(
    file: &mut std::fs::File,
    first_page_id: PageId,
) -> Result<(ObjectId, Value)> {
    // Read first page
    let page = Page::read_from(file, first_page_id)?;
    let mut all_data = page.data.clone();

    // Follow page chain if needed
    let mut current_page_id = page.header.next_page;
    while current_page_id != 0 {
        let next_page = Page::read_from(file, current_page_id)?;
        all_data.extend_from_slice(&next_page.data);
        current_page_id = next_page.header.next_page;
    }

    // Parse document header
    if all_data.len() < DocumentHeader::SIZE {
        return Err(Error::CorruptedDatabase {
            reason: "Document data too short for header".to_string(),
        });
    }

    let doc_header = DocumentHeader::from_bytes(&all_data[0..DocumentHeader::SIZE])?;
    let doc_data = &all_data[DocumentHeader::SIZE..];

    // Verify size matches
    if doc_data.len() != doc_header.total_size as usize {
        return Err(Error::CorruptedDatabase {
            reason: format!(
                "Document size mismatch: expected {}, got {}",
                doc_header.total_size,
                doc_data.len()
            ),
        });
    }

    // Deserialize document
    let value = deserialize_document(doc_data)?;

    Ok((doc_header.object_id, value))
}

/// Find a document by ObjectId (linear search)
/// 
/// Returns the first page ID if found, None otherwise
pub fn find_document_by_id(
    file: &mut std::fs::File,
    target_id: ObjectId,
    start_page: PageId,
    max_pages: usize,
) -> Result<Option<PageId>> {
    let mut current_page_id = start_page;
    let mut pages_checked = 0;

    while current_page_id != 0 && pages_checked < max_pages {
        let page = Page::read_from(file, current_page_id)?;
        
        if page.header.page_type == PageType::Data && !page.data.is_empty() {
            // Try to read document header
            if page.data.len() >= DocumentHeader::SIZE {
                if let Ok(doc_header) = DocumentHeader::from_bytes(&page.data[0..DocumentHeader::SIZE]) {
                    if doc_header.object_id == target_id {
                        return Ok(Some(current_page_id));
                    }
                }
            }
        }

        // Move to next page in chain or next sequential page
        if page.header.next_page != 0 {
            current_page_id = page.header.next_page;
        } else {
            // No next page, try next sequential page
            current_page_id += 1;
        }
        pages_checked += 1;
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Header, PageManager};
    use crate::types::Value;
    use std::collections::HashMap;
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
        
        let header = Header::new();
        header.write_to(&mut file).unwrap();
        
        (temp_file, file)
    }

    #[test]
    fn test_serialize_deserialize_document() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("João".to_string()));
        map.insert("age".to_string(), Value::Int(30));
        let value = Value::Object(map);

        let bytes = serialize_document(&value).unwrap();
        let deserialized = deserialize_document(&bytes).unwrap();

        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_write_read_single_page_document() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);

        let object_id = ObjectId::new();
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("Test".to_string()));
        let value = Value::Object(map);

        // Allocate page first
        let page_id = manager.allocate_page(PageType::Data).unwrap();
        let mut page_id_counter = 0u32;
        let mut allocate = || {
            let id = if page_id_counter == 0 { page_id } else { page_id_counter };
            page_id_counter += 1;
            Ok(id)
        };

        let first_page = write_document(manager.file(), object_id, &value, &mut allocate).unwrap();

        // Read back
        let (read_id, read_value) = read_document(manager.file(), first_page).unwrap();
        assert_eq!(read_id, object_id);
        assert_eq!(read_value, value);
    }

    #[test]
    fn test_write_read_multi_page_document() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);

        let object_id = ObjectId::new();
        // Create a large document that spans multiple pages
        let mut map = HashMap::new();
        let large_string = "x".repeat(5000); // Large enough to span pages
        map.insert("data".to_string(), Value::String(large_string));
        let value = Value::Object(map);

        // Allocate pages first (estimate we need 2-3 pages)
        let page1 = manager.allocate_page(PageType::Data).unwrap();
        let page2 = manager.allocate_page(PageType::Data).unwrap();
        let page3 = manager.allocate_page(PageType::Data).unwrap();
        let page_ids = vec![page1, page2, page3];
        let mut page_id_counter = 0usize;
        let mut allocate = || {
            let id = page_ids[page_id_counter];
            page_id_counter += 1;
            Ok(id)
        };

        let first_page = write_document(manager.file(), object_id, &value, &mut allocate).unwrap();

        // Read back
        let (read_id, read_value) = read_document(manager.file(), first_page).unwrap();
        assert_eq!(read_id, object_id);
        assert_eq!(read_value, value);
    }

    #[test]
    fn test_find_document_by_id() {
        let (_temp_file, file) = create_test_file();
        let mut manager = PageManager::new(file);

        let id1 = ObjectId::new();
        let id2 = ObjectId::new();
        let id3 = ObjectId::new();

        let mut map1 = HashMap::new();
        map1.insert("name".to_string(), Value::String("Doc1".to_string()));
        let value1 = Value::Object(map1);

        let mut map2 = HashMap::new();
        map2.insert("name".to_string(), Value::String("Doc2".to_string()));
        let value2 = Value::Object(map2);

        // Allocate pages first
        let page1_id = manager.allocate_page(PageType::Data).unwrap();
        let page2_id = manager.allocate_page(PageType::Data).unwrap();
        
        // First document
        let page_ids1 = vec![page1_id];
        let mut page_id_counter1 = 0usize;
        let mut allocate1 = || {
            let id = page_ids1[page_id_counter1];
            page_id_counter1 += 1;
            Ok(id)
        };
        let page1 = write_document(manager.file(), id1, &value1, &mut allocate1).unwrap();
        
        // Second document
        let page_ids2 = vec![page2_id];
        let mut page_id_counter2 = 0usize;
        let mut allocate2 = || {
            let id = page_ids2[page_id_counter2];
            page_id_counter2 += 1;
            Ok(id)
        };
        let page2 = write_document(manager.file(), id2, &value2, &mut allocate2).unwrap();

        // Find existing document
        let found = find_document_by_id(manager.file(), id1, 0, 100).unwrap();
        assert_eq!(found, Some(page1));

        let found = find_document_by_id(manager.file(), id2, 0, 100).unwrap();
        assert_eq!(found, Some(page2));

        // Find non-existent document
        let found = find_document_by_id(manager.file(), id3, 0, 100).unwrap();
        assert_eq!(found, None);
    }
}

