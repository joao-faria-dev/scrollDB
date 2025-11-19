use crate::storage::PageId;
use crate::types::ObjectId;
use std::collections::BTreeMap;

/// Index mapping ObjectId to PageId for O(log n) document lookups
/// Uses B-tree for better cache locality and ordered iteration support
pub struct IdIndex {
    /// Map from ObjectId to the first PageId where the document is stored
    index: BTreeMap<ObjectId, PageId>,
}

impl IdIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }

    /// Insert or update an entry in the index
    pub fn insert(&mut self, object_id: ObjectId, page_id: PageId) {
        self.index.insert(object_id, page_id);
    }

    /// Get the page ID for a given ObjectId
    pub fn get(&self, object_id: &ObjectId) -> Option<PageId> {
        self.index.get(object_id).copied()
    }

    /// Remove an entry from the index
    pub fn remove(&mut self, object_id: &ObjectId) -> bool {
        self.index.remove(object_id).is_some()
    }

    /// Check if the index contains an entry for the given ObjectId
    pub fn contains(&self, object_id: &ObjectId) -> bool {
        self.index.contains_key(object_id)
    }

    /// Get the number of entries in the index
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

impl Default for IdIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_index_new() {
        let index = IdIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_id_index_insert_get() {
        let mut index = IdIndex::new();
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();

        index.insert(id1, 0);
        index.insert(id2, 1);

        assert_eq!(index.get(&id1), Some(0));
        assert_eq!(index.get(&id2), Some(1));
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_id_index_remove() {
        let mut index = IdIndex::new();
        let id = ObjectId::new();

        index.insert(id, 0);
        assert_eq!(index.len(), 1);

        assert!(index.remove(&id));
        assert_eq!(index.len(), 0);
        assert_eq!(index.get(&id), None);
    }

    #[test]
    fn test_id_index_update() {
        let mut index = IdIndex::new();
        let id = ObjectId::new();

        index.insert(id, 0);
        assert_eq!(index.get(&id), Some(0));

        index.insert(id, 5);
        assert_eq!(index.get(&id), Some(5));
        assert_eq!(index.len(), 1);
    }
}
