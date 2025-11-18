use crate::types::Value;
use std::collections::{HashMap, HashSet};

/// Inverted index for text search
/// Maps terms (words) to sets of document IDs
pub struct TextIndex {
    /// Term -> Set of document IDs containing this term
    index: HashMap<String, HashSet<String>>,
    /// Fields that are indexed
    indexed_fields: HashSet<String>,
}

impl TextIndex {
    /// Create a new empty text index
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            indexed_fields: HashSet::new(),
        }
    }

    /// Add a document to the index
    pub fn index_document(&mut self, doc_id: &str, doc: &Value, fields: &[String]) {
        // Extract text from specified fields
        let mut text_parts = Vec::new();
        self.extract_text(doc, fields, &mut text_parts);
        
        // Tokenize and index
        for text in text_parts {
            let tokens = Self::tokenize(&text);
            for token in tokens {
                self.index.entry(token).or_insert_with(HashSet::new).insert(doc_id.to_string());
            }
        }
        
        // Track indexed fields
        for field in fields {
            self.indexed_fields.insert(field.clone());
        }
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) {
        for term_set in self.index.values_mut() {
            term_set.remove(doc_id);
        }
    }

    /// Search for documents containing the given terms
    pub fn search(&self, terms: &[String]) -> HashSet<String> {
        if terms.is_empty() {
            return HashSet::new();
        }

        // Start with documents containing the first term
        let mut result = self.index.get(&terms[0])
            .cloned()
            .unwrap_or_else(HashSet::new);

        // Intersect with documents containing other terms (AND logic)
        for term in &terms[1..] {
            if let Some(doc_set) = self.index.get(term) {
                result = result.intersection(doc_set).cloned().collect();
            } else {
                // Term not found, no matches
                return HashSet::new();
            }
        }

        result
    }

    /// Extract text from document fields
    fn extract_text(&self, value: &Value, fields: &[String], output: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    if fields.is_empty() || fields.contains(key) {
                        match val {
                            Value::String(s) => output.push(s.clone()),
                            Value::Array(arr) => {
                                for item in arr {
                                    self.extract_text(item, &[], output);
                                }
                            }
                            Value::Object(_) => {
                                self.extract_text(val, &[], output);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Value::String(s) => output.push(s.clone()),
            Value::Array(arr) => {
                for item in arr {
                    self.extract_text(item, &[], output);
                }
            }
            _ => {}
        }
    }

    /// Simple tokenizer: split by whitespace and lowercase
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get indexed fields
    pub fn indexed_fields(&self) -> &HashSet<String> {
        &self.indexed_fields
    }
}

impl Default for TextIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_text_index_tokenize() {
        let tokens = TextIndex::tokenize("Hello World Rust");
        assert_eq!(tokens, vec!["hello", "world", "rust"]);
    }

    #[test]
    fn test_text_index_document() {
        let mut index = TextIndex::new();
        let mut doc = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc {
            map.insert("title".to_string(), Value::String("Rust Database".to_string()));
            map.insert("content".to_string(), Value::String("Embedded storage".to_string()));
        }

        index.index_document("doc1", &doc, &["title".to_string(), "content".to_string()]);
        
        // Search for "rust"
        let results = index.search(&["rust".to_string()]);
        assert!(results.contains("doc1"));
    }

    #[test]
    fn test_text_index_search_and() {
        let mut index = TextIndex::new();
        let mut doc1 = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc1 {
            map.insert("text".to_string(), Value::String("rust database".to_string()));
        }
        index.index_document("doc1", &doc1, &["text".to_string()]);

        let mut doc2 = Value::Object(HashMap::new());
        if let Value::Object(ref mut map) = doc2 {
            map.insert("text".to_string(), Value::String("database embedded".to_string()));
        }
        index.index_document("doc2", &doc2, &["text".to_string()]);

        // Search for documents containing both "rust" and "database"
        let results = index.search(&["rust".to_string(), "database".to_string()]);
        assert!(results.contains("doc1"));
        assert!(!results.contains("doc2"));
    }
}

