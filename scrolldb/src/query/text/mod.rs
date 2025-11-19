use crate::error::{Error, Result};
use crate::types::Value;

/// Text search query parameters
#[derive(Debug, Clone)]
pub struct TextSearchQuery {
    /// Search terms
    pub search: String,
    /// Case sensitive search (default: false)
    pub case_sensitive: bool,
    /// Fields to search in (empty = all string fields)
    pub fields: Vec<String>,
}

impl TextSearchQuery {
    /// Parse a $text query from a Value
    pub fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Object(map) => {
                let search = map
                    .get("$search")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| Error::CorruptedDatabase {
                        reason: "$text query requires $search field".to_string(),
                    })?;

                let case_sensitive = map
                    .get("$caseSensitive")
                    .and_then(|v| match v {
                        Value::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(false);

                let fields = map
                    .get("$fields")
                    .and_then(|v| match v {
                        Value::Array(arr) => Some(
                            arr.iter()
                                .filter_map(|item| match item {
                                    Value::String(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
                    .unwrap_or_else(Vec::new);

                Ok(Self {
                    search,
                    case_sensitive,
                    fields,
                })
            }
            _ => Err(Error::CorruptedDatabase {
                reason: "$text query must be an Object".to_string(),
            }),
        }
    }

    /// Get search terms (tokenized)
    pub fn terms(&self) -> Vec<String> {
        if self.case_sensitive {
            self.search
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else {
            crate::index::text::TextIndex::tokenize(&self.search)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_text_search_query_from_value() {
        let mut map = HashMap::new();
        map.insert(
            "$search".to_string(),
            Value::String("rust database".to_string()),
        );
        map.insert("$caseSensitive".to_string(), Value::Bool(false));
        let value = Value::Object(map);

        let query = TextSearchQuery::from_value(&value).unwrap();
        assert_eq!(query.search, "rust database");
        assert_eq!(query.case_sensitive, false);
    }

    #[test]
    fn test_text_search_query_terms() {
        let query = TextSearchQuery {
            search: "Rust Database".to_string(),
            case_sensitive: false,
            fields: vec![],
        };

        let terms = query.terms();
        assert_eq!(terms, vec!["rust", "database"]);
    }
}
