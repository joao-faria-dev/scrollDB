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

    #[test]
    fn test_insert_one_auto_id() {
        let mut page_manager = create_test_page_manager();
        let mut collection = Collection::new("users".to_string(), page_manager);

        let mut doc = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("name".to_string(), Value::String("João".to_string()));
            map.insert("age".to_string(), Value::Int(30));
        }

        let id = collection.insert_one(doc).unwrap();
        
        // Verify _id was added
        assert!(!id.to_string().is_empty());
        
        // Verify document was written (we can't easily read it back yet, but we can check it doesn't error)
    }

    #[test]
    fn test_insert_one_with_existing_id() {
        let mut page_manager = create_test_page_manager();
        let mut collection = Collection::new("users".to_string(), page_manager);

        let existing_id = ObjectId::new();
        let mut doc = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("_id".to_string(), Value::String(existing_id.to_string()));
            map.insert("name".to_string(), Value::String("João".to_string()));
        }

        let id = collection.insert_one(doc).unwrap();
        
        // Verify the provided _id was used
        assert_eq!(id, existing_id);
    }

    #[test]
    fn test_insert_one_multiple_documents() {
        let mut page_manager = create_test_page_manager();
        let mut collection = Collection::new("users".to_string(), page_manager);

        let mut doc1 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc1 {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
            map.insert("age".to_string(), Value::Int(25));
        }

        let mut doc2 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc2 {
            map.insert("name".to_string(), Value::String("Bob".to_string()));
            map.insert("age".to_string(), Value::Int(30));
        }

        let id1 = collection.insert_one(doc1).unwrap();
        let id2 = collection.insert_one(doc2).unwrap();
        
        // Verify both documents got unique IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_insert_one_invalid_document() {
        let mut page_manager = create_test_page_manager();
        let mut collection = Collection::new("users".to_string(), page_manager);

        // Try to insert a non-Object value
        let doc = Value::String("not an object".to_string());
        
        let result = collection.insert_one(doc);
        assert!(result.is_err());
        if let Err(Error::CorruptedDatabase { reason }) = result {
            assert!(reason.contains("Object"));
        } else {
            panic!("Expected CorruptedDatabase error");
        }
    }

    #[test]
    fn test_insert_one_invalid_id_format() {
        let mut page_manager = create_test_page_manager();
        let mut collection = Collection::new("users".to_string(), page_manager);

        let mut doc = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("_id".to_string(), Value::String("invalid-id".to_string()));
            map.insert("name".to_string(), Value::String("Test".to_string()));
        }

        let result = collection.insert_one(doc);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::Database;
    use crate::types::Value;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_database_insert_one() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bunkr");

        let mut db = Database::open(&path).unwrap();
        let mut collection = db.collection("users").unwrap();

        let mut doc = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("name".to_string(), Value::String("João".to_string()));
            map.insert("age".to_string(), Value::Int(30));
            map.insert("active".to_string(), Value::Bool(true));
        }

        let id = collection.insert_one(doc).unwrap();
        assert!(!id.to_string().is_empty());

        db.close().unwrap();
    }

    #[test]
    fn test_database_insert_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bunkr");

        let mut db = Database::open(&path).unwrap();
        let mut collection = db.collection("users").unwrap();

        // Insert first document
        let mut doc1 = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc1 {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
        }
        let id1 = collection.insert_one(doc1).unwrap();

        // Insert second document
        let mut doc2 = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc2 {
            map.insert("name".to_string(), Value::String("Bob".to_string()));
        }
        let id2 = collection.insert_one(doc2).unwrap();

        // Verify unique IDs
        assert_ne!(id1, id2);

        db.close().unwrap();
    }
}

