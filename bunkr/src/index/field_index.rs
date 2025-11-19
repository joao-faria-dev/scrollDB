use crate::types::{ObjectId, Value};
use std::collections::BTreeMap;

/// Field-level index for efficient querying by document field values
/// Uses B-tree for ordered iteration and range queries
pub struct FieldIndex {
    /// Map from field value to set of ObjectIds
    /// For ordered types (numbers, strings), this enables range queries
    index: BTreeMap<Value, Vec<ObjectId>>,
    /// Field name this index is for
    field_name: String,
}

impl FieldIndex {
    /// Create a new field index for the given field name
    pub fn new(field_name: String) -> Self {
        Self {
            index: BTreeMap::new(),
            field_name,
        }
    }

    /// Get the field name this index is for
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Index a document's field value
    pub fn index_document(&mut self, object_id: ObjectId, value: &Value) {
        // Extract the field value from the document
        let field_value = match value {
            Value::Object(map) => map.get(&self.field_name),
            _ => None,
        };

        if let Some(field_value) = field_value {
            // Clone the value for use as a key (Value must be Clone)
            // Add ObjectId to the list for this value
            self.index
                .entry(field_value.clone())
                .or_default()
                .push(object_id);
        }
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, object_id: ObjectId, value: &Value) {
        let field_value = match value {
            Value::Object(map) => map.get(&self.field_name),
            _ => None,
        };

        if let Some(field_value) = field_value {
            if let Some(object_ids) = self.index.get_mut(field_value) {
                object_ids.retain(|&id| id != object_id);
                // Remove entry if empty
                if object_ids.is_empty() {
                    self.index.remove(field_value);
                }
            }
        }
    }

    /// Find all ObjectIds with the given field value (equality)
    pub fn find_equal(&self, value: &Value) -> Vec<ObjectId> {
        self.index.get(value).cloned().unwrap_or_default()
    }

    /// Find all ObjectIds with field values greater than the given value (range query)
    pub fn find_greater_than(&self, value: &Value) -> Vec<ObjectId> {
        use std::ops::Bound;
        let mut results = Vec::new();
        // Use exclusive bound to exclude the value itself
        for (_key, object_ids) in self
            .index
            .range((Bound::Excluded(value.clone()), Bound::Unbounded))
        {
            results.extend(object_ids.iter().copied());
        }
        results
    }

    /// Find all ObjectIds with field values less than the given value (range query)
    pub fn find_less_than(&self, value: &Value) -> Vec<ObjectId> {
        use std::ops::Bound;
        let mut results = Vec::new();
        // Use exclusive bound to exclude the value itself
        for (_key, object_ids) in self
            .index
            .range((Bound::Unbounded, Bound::Excluded(value.clone())))
        {
            results.extend(object_ids.iter().copied());
        }
        results
    }

    /// Find all ObjectIds with field values in the given range (inclusive)
    pub fn find_range(&self, min: &Value, max: &Value) -> Vec<ObjectId> {
        let mut results = Vec::new();
        for (_key, object_ids) in self.index.range(min.clone()..=max.clone()) {
            results.extend(object_ids.iter().copied());
        }
        results
    }

    /// Get the number of unique values in the index
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_field_index_new() {
        let index = FieldIndex::new("age".to_string());
        assert!(index.is_empty());
        assert_eq!(index.field_name(), "age");
    }

    #[test]
    fn test_field_index_index_document() {
        let mut index = FieldIndex::new("age".to_string());
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();

        let mut doc1 = HashMap::new();
        doc1.insert("age".to_string(), Value::Int(25));
        let value1 = Value::Object(doc1);

        let mut doc2 = HashMap::new();
        doc2.insert("age".to_string(), Value::Int(30));
        let value2 = Value::Object(doc2);

        index.index_document(id1, &value1);
        index.index_document(id2, &value2);

        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_field_index_find_equal() {
        let mut index = FieldIndex::new("age".to_string());
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();

        let mut doc1 = HashMap::new();
        doc1.insert("age".to_string(), Value::Int(25));
        let value1 = Value::Object(doc1);

        let mut doc2 = HashMap::new();
        doc2.insert("age".to_string(), Value::Int(25));
        let value2 = Value::Object(doc2);

        index.index_document(id1, &value1);
        index.index_document(id2, &value2);

        let results = index.find_equal(&Value::Int(25));
        assert_eq!(results.len(), 2);
        assert!(results.contains(&id1));
        assert!(results.contains(&id2));
    }

    #[test]
    fn test_field_index_find_greater_than() {
        let mut index = FieldIndex::new("age".to_string());
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();
        let id3 = ObjectId::new();

        let mut doc1 = HashMap::new();
        doc1.insert("age".to_string(), Value::Int(25));
        let value1 = Value::Object(doc1);

        let mut doc2 = HashMap::new();
        doc2.insert("age".to_string(), Value::Int(30));
        let value2 = Value::Object(doc2);

        let mut doc3 = HashMap::new();
        doc3.insert("age".to_string(), Value::Int(35));
        let value3 = Value::Object(doc3);

        index.index_document(id1, &value1);
        index.index_document(id2, &value2);
        index.index_document(id3, &value3);

        let results = index.find_greater_than(&Value::Int(25));
        assert_eq!(results.len(), 2);
        assert!(results.contains(&id2));
        assert!(results.contains(&id3));
    }

    #[test]
    fn test_field_index_remove_document() {
        let mut index = FieldIndex::new("age".to_string());
        let id1 = ObjectId::new();

        let mut doc1 = HashMap::new();
        doc1.insert("age".to_string(), Value::Int(25));
        let value1 = Value::Object(doc1.clone());

        index.index_document(id1, &value1);
        assert_eq!(index.find_equal(&Value::Int(25)).len(), 1);

        index.remove_document(id1, &Value::Object(doc1));
        assert_eq!(index.find_equal(&Value::Int(25)).len(), 0);
    }
}
