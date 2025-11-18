use crate::types::Value;
use crate::error::{Error, Result};
use super::Query;
use super::path::{parse_path, get_value_at_path};

/// Check if a document matches a query
pub fn matches(doc: &Value, query: &Query) -> Result<bool> {
    // Empty query matches all documents
    if query.is_empty() {
        return Ok(true);
    }

    // Check each field in the query
    for (field_path, expected_value) in query.fields() {
        let path_segments = parse_path(field_path);
        
        // Get the actual value at this path
        let actual_value = match get_value_at_path(doc, &path_segments) {
            Some(v) => v,
            None => {
                // Path doesn't exist, doesn't match
                return Ok(false);
            }
        };

        // Compare values (exact match for now)
        if !values_match(&actual_value, expected_value)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Check if two values match (exact match)
fn values_match(actual: &Value, expected: &Value) -> Result<bool> {
    match (actual, expected) {
        (Value::Null, Value::Null) => Ok(true),
        (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
        (Value::Int(a), Value::Int(b)) => Ok(a == b),
        (Value::Float(a), Value::Float(b)) => Ok(a == b),
        (Value::String(a), Value::String(b)) => Ok(a == b),
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (av, bv) in a.iter().zip(b.iter()) {
                if !values_match(av, bv)? {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        (Value::Object(a), Value::Object(b)) => {
            // For objects, check if all keys in expected exist in actual with matching values
            for (key, expected_val) in b {
                match a.get(key) {
                    Some(actual_val) => {
                        if !values_match(actual_val, expected_val)? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
            Ok(true)
        },
        // Type mismatch
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_matches_empty_query() {
        let query = Query::new();
        let doc = Value::Object(HashMap::new());
        assert!(matches(&doc, &query).unwrap());
    }

    #[test]
    fn test_matches_simple_field() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("João".to_string()));
        let doc = Value::Object(map);

        let query = Query::from_value(Value::Object({
            let mut q = HashMap::new();
            q.insert("name".to_string(), Value::String("João".to_string()));
            q
        })).unwrap();

        assert!(matches(&doc, &query).unwrap());
    }

    #[test]
    fn test_matches_dotted_path() {
        let mut map = HashMap::new();
        let mut profile = HashMap::new();
        profile.insert("age".to_string(), Value::Int(30));
        map.insert("profile".to_string(), Value::Object(profile));
        let doc = Value::Object(map);

        let query = Query::from_value(Value::Object({
            let mut q = HashMap::new();
            q.insert("profile.age".to_string(), Value::Int(30));
            q
        })).unwrap();

        assert!(matches(&doc, &query).unwrap());
    }

    #[test]
    fn test_matches_no_match() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("João".to_string()));
        let doc = Value::Object(map);

        let query = Query::from_value(Value::Object({
            let mut q = HashMap::new();
            q.insert("name".to_string(), Value::String("Bob".to_string()));
            q
        })).unwrap();

        assert!(!matches(&doc, &query).unwrap());
    }

    #[test]
    fn test_matches_missing_field() {
        let doc = Value::Object(HashMap::new());

        let query = Query::from_value(Value::Object({
            let mut q = HashMap::new();
            q.insert("name".to_string(), Value::String("João".to_string()));
            q
        })).unwrap();

        assert!(!matches(&doc, &query).unwrap());
    }
}

