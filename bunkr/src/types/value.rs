use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// BSON-like Value type for documents
///
/// Similar to serde_json::Value but optimized for Bunkr's needs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer number (i64)
    Int(i64),
    /// Floating point number (f64)
    Float(f64),
    /// String value
    String(String),
    /// Array of values
    Array(Vec<Value>),
    /// Object (map of string to value)
    Object(HashMap<String, Value>),
}

impl Value {
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get as boolean, if it is one
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer, if it is one
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// Get as float, if it is one
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as string, if it is one
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array, if it is one
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get as object, if it is one
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Get value at a path (supports dotted paths like "profile.age")
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Object(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert!(Value::Null.is_null());
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
        assert_eq!(Value::String("hello".to_string()).as_str(), Some("hello"));
    }

    #[test]
    fn test_value_from() {
        let v: Value = 42.into();
        assert_eq!(v.as_int(), Some(42));

        let v: Value = "test".into();
        assert_eq!(v.as_str(), Some("test"));

        let v: Value = true.into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_get_path() {
        let mut map = HashMap::new();
        let mut profile = HashMap::new();
        profile.insert("age".to_string(), Value::Int(30));
        map.insert("profile".to_string(), Value::Object(profile));
        map.insert("name".to_string(), Value::String("João".to_string()));

        let doc = Value::Object(map);

        assert_eq!(doc.get_path("name"), Some(&Value::String("João".to_string())));
        assert_eq!(doc.get_path("profile.age"), Some(&Value::Int(30)));
        assert_eq!(doc.get_path("nonexistent"), None);
    }

    #[test]
    fn test_serialization() {
        let value = Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String("test".to_string()));
            map.insert("age".to_string(), Value::Int(30));
            map
        });

        let json = serde_json::to_string(&value).unwrap();
        let deserialized: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value, deserialized);
    }
}

