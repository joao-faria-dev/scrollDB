use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// BSON-like Value type for documents
///
/// Similar to serde_json::Value but optimized for Bunkr's needs
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                // Handle NaN and infinity
                if a.is_nan() && b.is_nan() {
                    false // NaN != NaN
                } else {
                    a == b
                }
            }
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        // Type-based ordering: Null < Bool < Int < Float < String < Array < Object
        let type_order = |v: &Value| match v {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Int(_) => 2,
            Value::Float(_) => 3,
            Value::String(_) => 4,
            Value::Array(_) => 5,
            Value::Object(_) => 6,
        };

        let self_order = type_order(self);
        let other_order = type_order(other);

        match self_order.cmp(&other_order) {
            Ordering::Equal => {
                // Same type, compare values
                match (self, other) {
                    (Value::Null, Value::Null) => Ordering::Equal,
                    (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
                    (Value::Int(a), Value::Int(b)) => a.cmp(b),
                    (Value::Float(a), Value::Float(b)) => {
                        // Handle NaN and infinity
                        if a.is_nan() && b.is_nan() {
                            Ordering::Equal
                        } else if a.is_nan() {
                            Ordering::Less // NaN is less than everything
                        } else if b.is_nan() {
                            Ordering::Greater
                        } else {
                            a.partial_cmp(b).unwrap_or(Ordering::Equal)
                        }
                    }
                    (Value::String(a), Value::String(b)) => a.cmp(b),
                    (Value::Array(a), Value::Array(b)) => {
                        // Lexicographic comparison
                        for (a_val, b_val) in a.iter().zip(b.iter()) {
                            match a_val.cmp(b_val) {
                                Ordering::Equal => continue,
                                other => return other,
                            }
                        }
                        a.len().cmp(&b.len())
                    }
                    (Value::Object(a), Value::Object(b)) => {
                        // Compare as sorted key-value pairs
                        let mut a_pairs: Vec<_> = a.iter().collect();
                        let mut b_pairs: Vec<_> = b.iter().collect();
                        a_pairs.sort_by_key(|(k, _)| *k);
                        b_pairs.sort_by_key(|(k, _)| *k);
                        a_pairs.cmp(&b_pairs)
                    }
                    _ => unreachable!(), // Same type order, so this shouldn't happen
                }
            }
            other => other,
        }
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

        assert_eq!(
            doc.get_path("name"),
            Some(&Value::String("João".to_string()))
        );
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
