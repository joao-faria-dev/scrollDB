use crate::types::Value;
use crate::error::{Error, Result};

pub mod path;
pub mod matcher;
pub mod operators;

/// Query structure for matching documents
#[derive(Debug, Clone)]
pub struct Query {
    /// Fields to match (field path -> expected value)
    fields: std::collections::HashMap<String, Value>,
}

impl Query {
    /// Create a new empty query
    pub fn new() -> Self {
        Self {
            fields: std::collections::HashMap::new(),
        }
    }

    /// Parse a Value (JSON-like) into a Query
    pub fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Object(map) => {
                let mut query = Query::new();
                for (key, val) in map {
                    query.fields.insert(key, val);
                }
                Ok(query)
            }
            _ => Err(Error::CorruptedDatabase {
                reason: "Query must be an Object".to_string(),
            })
        }
    }

    /// Get all field paths in the query
    pub fn fields(&self) -> &std::collections::HashMap<String, Value> {
        &self.fields
    }

    /// Check if query is empty (matches all)
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_query_new() {
        let query = Query::new();
        assert!(query.is_empty());
    }

    #[test]
    fn test_query_from_value() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("João".to_string()));
        map.insert("age".to_string(), Value::Int(30));
        let value = Value::Object(map);

        let query = Query::from_value(value).unwrap();
        assert_eq!(query.fields().len(), 2);
        assert!(query.fields().contains_key("name"));
        assert!(query.fields().contains_key("age"));
    }

    #[test]
    fn test_query_from_value_invalid() {
        let value = Value::String("not an object".to_string());
        let result = Query::from_value(value);
        assert!(result.is_err());
    }
}

