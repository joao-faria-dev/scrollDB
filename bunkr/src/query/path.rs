use crate::types::Value;
use crate::error::{Error, Result};

/// Parse a dotted path string into segments
/// 
/// Example: "profile.age" -> ["profile", "age"]
pub fn parse_path(path: &str) -> Vec<String> {
    path.split('.').map(|s| s.to_string()).collect()
}

/// Navigate a nested object using a path
/// 
/// Returns the value at the path, or None if the path doesn't exist
pub fn get_value_at_path(value: &Value, path: &[String]) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }

    match value {
        Value::Object(map) => {
            let first = &path[0];
            if let Some(next_value) = map.get(first) {
                get_value_at_path(next_value, &path[1..])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Set a value at a path in a nested object
/// 
/// Creates intermediate objects as needed
pub fn set_value_at_path(value: &mut Value, path: &[String], new_value: Value) -> Result<()> {
    if path.is_empty() {
        *value = new_value;
        return Ok(());
    }

    match value {
        Value::Object(map) => {
            let first = &path[0];
            if path.len() == 1 {
                // Last segment, set the value
                map.insert(first.clone(), new_value);
            } else {
                // Need to navigate deeper
                if let Some(existing) = map.get_mut(first) {
                    // Recurse into existing object
                    set_value_at_path(existing, &path[1..], new_value)?;
                } else {
                    // Create new nested object
                    let mut nested = Value::Object(std::collections::HashMap::new());
                    set_value_at_path(&mut nested, &path[1..], new_value)?;
                    map.insert(first.clone(), nested);
                }
            }
            Ok(())
        }
        _ => Err(Error::CorruptedDatabase {
            reason: format!("Cannot set path on non-object value"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_path() {
        assert_eq!(parse_path("name"), vec!["name"]);
        assert_eq!(parse_path("profile.age"), vec!["profile", "age"]);
        assert_eq!(parse_path("a.b.c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_get_value_at_path() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("João".to_string()));
        
        let mut profile = HashMap::new();
        profile.insert("age".to_string(), Value::Int(30));
        map.insert("profile".to_string(), Value::Object(profile));

        let value = Value::Object(map);

        // Test simple path
        let path = parse_path("name");
        assert_eq!(get_value_at_path(&value, &path), Some(Value::String("João".to_string())));

        // Test dotted path
        let path = parse_path("profile.age");
        assert_eq!(get_value_at_path(&value, &path), Some(Value::Int(30)));

        // Test non-existent path
        let path = parse_path("profile.email");
        assert_eq!(get_value_at_path(&value, &path), None);
    }

    #[test]
    fn test_set_value_at_path() {
        let mut value = Value::Object(HashMap::new());

        // Set simple path
        let path = parse_path("name");
        set_value_at_path(&mut value, &path, Value::String("João".to_string())).unwrap();
        assert_eq!(get_value_at_path(&value, &path), Some(Value::String("João".to_string())));

        // Set dotted path
        let path = parse_path("profile.age");
        set_value_at_path(&mut value, &path, Value::Int(30)).unwrap();
        assert_eq!(get_value_at_path(&value, &path), Some(Value::Int(30)));
    }
}

