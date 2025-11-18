use crate::error::{Error, Result};
use crate::storage::page_manager::PageManager;
use crate::storage::{write_document, PageType};
use crate::types::{ObjectId, Value};

/// Collection handle for a named collection in the database
pub struct Collection {
    name: String,
    page_manager: PageManager,
}

impl Collection {
    /// Create a new collection with the given name and page manager
    pub(crate) fn new(name: String, page_manager: PageManager) -> Self {
        Self { name, page_manager }
    }

    /// Get the collection name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Insert a document into the collection
    ///
    /// If the document doesn't have an `_id` field, one will be automatically generated.
    /// Returns the ObjectId of the inserted document.
    pub fn insert_one(&mut self, mut doc: Value) -> Result<ObjectId> {
        // Ensure document is an Object
        let doc_map = match &mut doc {
            Value::Object(map) => map,
            _ => {
                return Err(Error::CorruptedDatabase {
                    reason: "Document must be an Object".to_string(),
                });
            }
        };

        // Auto-generate _id if missing
        let object_id = if let Some(id_value) = doc_map.get("_id") {
            // Extract ObjectId from _id field
            match id_value {
                Value::String(hex) => hex.parse().map_err(|_| Error::CorruptedDatabase {
                    reason: "Invalid _id format".to_string(),
                })?,
                _ => {
                    return Err(Error::CorruptedDatabase {
                        reason: "_id must be a string".to_string(),
                    });
                }
            }
        } else {
            // Generate new ObjectId
            let new_id = ObjectId::new();
            doc_map.insert("_id".to_string(), Value::String(new_id.to_string()));
            new_id
        };

        // Serialize document to determine size
        let doc_bytes = crate::storage::serialize_document(&doc)?;
        let doc_header_size = 16; // DocumentHeader::SIZE
        let total_size = doc_header_size + doc_bytes.len();
        let max_page_data = crate::storage::page::PageHeader::max_data_size();
        let pages_needed = if total_size <= max_page_data {
            1
        } else {
            (total_size + max_page_data - 1) / max_page_data
        };

        // Allocate pages first
        let mut page_ids = Vec::with_capacity(pages_needed);
        for _ in 0..pages_needed {
            page_ids.push(self.page_manager.allocate_page(PageType::Data)?);
        }

        // Now write document using allocated pages
        let file = self.page_manager.file();
        let mut page_id_counter = 0u32;
        let mut allocate = || {
            let id = page_ids[page_id_counter as usize];
            page_id_counter += 1;
            Ok(id)
        };

        write_document(file, object_id, &doc, &mut allocate)?;
        self.page_manager.flush()?;

        Ok(object_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Header, PageManager};
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_test_page_manager() -> PageManager {
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
        
        PageManager::new(file)
    }

    #[test]
    fn test_collection_new() {
        let page_manager = create_test_page_manager();
        let collection = Collection::new("users".to_string(), page_manager);
        assert_eq!(collection.name(), "users");
    }
}

