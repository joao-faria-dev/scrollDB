use crate::error::{Error, Result};
use crate::storage::page_manager::PageManager;
use crate::storage::{read_document, write_document, PageType};
use crate::storage::page::{Page, PageId};
use crate::types::{ObjectId, Value};

/// Collection handle for a named collection in the database
pub struct Collection {
    name: String,
    page_manager: PageManager,
    file_path: std::path::PathBuf,
}

impl Collection {
    /// Create a new collection with the given name and page manager
    pub(crate) fn new(name: String, page_manager: PageManager, file_path: std::path::PathBuf) -> Self {
        Self { name, page_manager, file_path }
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

/// Iterator over all documents in a collection
pub struct DocumentIterator {
    page_manager: PageManager,
    current_page_id: PageId,
    max_page_id: PageId,
    visited_pages: std::collections::HashSet<PageId>,
}

impl DocumentIterator {
    fn new(mut page_manager: PageManager) -> Result<Self> {
        // Get max page ID
        let metadata = page_manager.file().metadata().map_err(Error::Io)?;
        let file_size = metadata.len();
        let header_size = crate::storage::HEADER_SIZE as u64;
        let page_size = crate::storage::PAGE_SIZE as u64;
        
        let max_page_id = if file_size <= header_size {
            0
        } else {
            let data_size = file_size - header_size;
            (data_size / page_size) as PageId
        };

        Ok(Self {
            page_manager,
            current_page_id: 0,
            max_page_id,
            visited_pages: std::collections::HashSet::new(),
        })
    }
}

impl Iterator for DocumentIterator {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        const NO_NEXT_PAGE: PageId = u32::MAX;
        
        loop {
            // Check if we've exhausted all pages
            if self.current_page_id >= self.max_page_id {
                return None;
            }

            // Skip if we've already visited this page (part of a multi-page document)
            if self.visited_pages.contains(&self.current_page_id) {
                self.current_page_id += 1;
                continue;
            }

            // Try to read the page
            let page = match Page::read_from(self.page_manager.file(), self.current_page_id) {
                Ok(p) => p,
                Err(_) => {
                    // Page doesn't exist, try next
                    self.current_page_id += 1;
                    continue;
                }
            };

            // Check if this is a data page with document data
            if page.header.page_type == PageType::Data && !page.data.is_empty() {
                // Check if this page starts a document (has document header)
                if page.data.len() >= 16 {
                    // Try to read the document
                    match read_document(self.page_manager.file(), self.current_page_id) {
                        Ok((_, value)) => {
                            // Mark all pages in this document as visited
                            let mut page_id = self.current_page_id;
                            loop {
                                self.visited_pages.insert(page_id);
                                
                                // Check if there's a next page in the chain
                                let next_page = match Page::read_from(self.page_manager.file(), page_id) {
                                    Ok(p) => p,
                                    Err(_) => break,
                                };
                                
                                if next_page.header.next_page == NO_NEXT_PAGE {
                                    break;
                                }
                                page_id = next_page.header.next_page;
                            }
                            
                            // Move to next page for next iteration
                            self.current_page_id += 1;
                            
                            return Some(Ok(value));
                        }
                        Err(_) => {
                            // Not a valid document, skip this page
                            self.current_page_id += 1;
                            continue;
                        }
                    }
                } else {
                    // Page doesn't have enough data for a document header, skip
                    self.current_page_id += 1;
                    continue;
                }
            } else {
                // Not a data page or empty, skip
                self.current_page_id += 1;
                continue;
            }
        }
    }
}

impl Collection {
    /// Create an iterator over all documents in the collection
    pub fn iter(&mut self) -> Result<DocumentIterator> {
        // Flush any pending writes
        self.page_manager.flush()?;
        
        // Create a new PageManager for reading (iterator needs its own file handle)
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)
            .map_err(Error::Io)?;
        
        let page_manager = PageManager::from_file(file)?;
        DocumentIterator::new(page_manager)
    }

    /// Find a document by its ObjectId
    ///
    /// Returns `Some(document)` if found, `None` if not found.
    pub fn find_by_id(&mut self, id: &ObjectId) -> Result<Option<Value>> {
        // Flush any pending writes
        self.page_manager.flush()?;
        
        // Create a new PageManager for reading
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)
            .map_err(Error::Io)?;
        
        let mut page_manager = PageManager::from_file(file)?;
        
        // Use find_document_by_id to locate the document
        use crate::storage::find_document_by_id;
        if let Some(page_id) = find_document_by_id(page_manager.file(), *id, 0, 10000)? {
            // Read the document
            let (_, value) = read_document(page_manager.file(), page_id)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Find documents matching a query
    ///
    /// Currently only supports empty query `{}` which returns all documents.
    /// Returns an iterator over matching documents.
    pub fn find(&mut self, query: Value) -> Result<DocumentIterator> {
        // For now, only support empty query (find all)
        match query {
            Value::Object(map) if map.is_empty() => {
                // Empty query means find all
                self.iter()
            }
            _ => {
                Err(Error::CorruptedDatabase {
                    reason: "Only empty query {} is supported".to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Header, PageManager};
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_test_collection() -> (NamedTempFile, Collection) {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        
        let header = Header::new();
        header.write_to(&mut file).unwrap();
        
        let page_manager = PageManager::new(file);
        let collection = Collection::new("users".to_string(), page_manager, path);
        (temp_file, collection)
    }

    #[test]
    fn test_collection_new() {
        let (_temp_file, collection) = create_test_collection();
        assert_eq!(collection.name(), "users");
    }

    #[test]
    fn test_insert_one_auto_id() {
        let (_temp_file, mut collection) = create_test_collection();

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
        let (_temp_file, mut collection) = create_test_collection();

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
        let (_temp_file, mut collection) = create_test_collection();

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
        let (_temp_file, mut collection) = create_test_collection();

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
        let (_temp_file, mut collection) = create_test_collection();

        let mut doc = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("_id".to_string(), Value::String("invalid-id".to_string()));
            map.insert("name".to_string(), Value::String("Test".to_string()));
        }

        let result = collection.insert_one(doc);
        assert!(result.is_err());
    }

    #[test]
    fn test_document_iterator() {
        let (_temp_file, mut collection) = create_test_collection();

        // Insert a few documents
        let mut doc1 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc1 {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
        }
        collection.insert_one(doc1).unwrap();

        let mut doc2 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc2 {
            map.insert("name".to_string(), Value::String("Bob".to_string()));
        }
        collection.insert_one(doc2).unwrap();

        // Iterate over documents
        let mut iter = collection.iter().unwrap();
        let mut count = 0;
        while let Some(result) = iter.next() {
            let doc = result.unwrap();
            if let Value::Object(map) = doc {
                assert!(map.contains_key("name"));
                count += 1;
            }
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_document_iterator_empty() {
        let (_temp_file, mut collection) = create_test_collection();
        let mut iter = collection.iter().unwrap();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_find_by_id() {
        let (_temp_file, mut collection) = create_test_collection();

        // Insert a document
        let mut doc = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
        }
        let id = collection.insert_one(doc).unwrap();

        // Find the document
        let found = collection.find_by_id(&id).unwrap();
        assert!(found.is_some());
        if let Some(Value::Object(map)) = found {
            assert_eq!(map.get("name"), Some(&Value::String("Alice".to_string())));
        } else {
            panic!("Expected Object value");
        }
    }

    #[test]
    fn test_find_by_id_not_found() {
        let (_temp_file, mut collection) = create_test_collection();

        let non_existent_id = ObjectId::new();
        let found = collection.find_by_id(&non_existent_id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_find_all() {
        let (_temp_file, mut collection) = create_test_collection();

        // Insert multiple documents
        let mut doc1 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc1 {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
        }
        collection.insert_one(doc1).unwrap();

        let mut doc2 = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = doc2 {
            map.insert("name".to_string(), Value::String("Bob".to_string()));
        }
        collection.insert_one(doc2).unwrap();

        // Find all documents
        let empty_query = Value::Object(std::collections::HashMap::new());
        let mut iter = collection.find(empty_query).unwrap();
        let mut count = 0;
        while let Some(result) = iter.next() {
            let doc = result.unwrap();
            if let Value::Object(map) = doc {
                assert!(map.contains_key("name"));
                count += 1;
            }
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_find_invalid_query() {
        let (_temp_file, mut collection) = create_test_collection();

        // Try to use a non-empty query (not supported yet)
        let mut query = Value::Object(std::collections::HashMap::new());
        if let Value::Object(ref mut map) = query {
            map.insert("name".to_string(), Value::String("Alice".to_string()));
        }
        
        let result = collection.find(query);
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

