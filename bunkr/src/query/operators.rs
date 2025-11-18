use crate::types::Value;
use crate::error::{Error, Result};

/// Comparison operators for queries
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    /// Greater than: $gt
    Gt(Value),
    /// Greater than or equal: $gte
    Gte(Value),
    /// Less than: $lt
    Lt(Value),
    /// Less than or equal: $lte
    Lte(Value),
    /// Not equal: $ne
    Ne(Value),
    /// In array: $in
    In(Vec<Value>),
}

impl Operator {
    /// Parse an operator from a JSON-like Value object
    /// 
    /// Example: {"$gt": 25} -> Operator::Gt(Value::Int(25))
    pub fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Object(map) => {
                if map.len() != 1 {
                    return Err(Error::CorruptedDatabase {
                        reason: "Operator object must have exactly one key".to_string(),
                    });
                }

                let (key, val) = map.iter().next().unwrap();
                match key.as_str() {
                    "$gt" => Ok(Operator::Gt(val.clone())),
                    "$gte" => Ok(Operator::Gte(val.clone())),
                    "$lt" => Ok(Operator::Lt(val.clone())),
                    "$lte" => Ok(Operator::Lte(val.clone())),
                    "$ne" => Ok(Operator::Ne(val.clone())),
                    "$in" => {
                        match val {
                            Value::Array(arr) => Ok(Operator::In(arr.clone())),
                            _ => Err(Error::CorruptedDatabase {
                                reason: "$in operator requires an array value".to_string(),
                            }),
                        }
                    }
                    _ => Err(Error::CorruptedDatabase {
                        reason: format!("Unknown operator: {}", key),
                    }),
                }
            }
            _ => Err(Error::CorruptedDatabase {
                reason: "Operator must be an Object".to_string(),
            }),
        }
    }

    /// Check if a value matches this operator
    pub fn matches(&self, actual: &Value) -> Result<bool> {
        match self {
            Operator::Gt(expected) => compare_values(actual, expected, |a, b| a > b),
            Operator::Gte(expected) => compare_values(actual, expected, |a, b| a >= b),
            Operator::Lt(expected) => compare_values(actual, expected, |a, b| a < b),
            Operator::Lte(expected) => compare_values(actual, expected, |a, b| a <= b),
            Operator::Ne(expected) => {
                // Not equal: true if values don't match
                Ok(!values_equal(actual, expected)?)
            }
            Operator::In(arr) => {
                // Check if actual value is in the array
                for item in arr {
                    if values_equal(actual, item)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

/// Compare two values using a comparison function
fn compare_values<F>(actual: &Value, expected: &Value, cmp: F) -> Result<bool>
where
    F: Fn(f64, f64) -> bool,
{
    let actual_num = value_to_number(actual)?;
    let expected_num = value_to_number(expected)?;
    Ok(cmp(actual_num, expected_num))
}

/// Convert a value to a number for comparison
fn value_to_number(value: &Value) -> Result<f64> {
    match value {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        Value::String(s) => {
            // Try to parse string as number
            s.parse::<f64>().map_err(|_| Error::CorruptedDatabase {
                reason: format!("Cannot compare string '{}' as number", s),
            })
        }
        _ => Err(Error::CorruptedDatabase {
            reason: format!("Cannot compare value {:?} as number", value),
        }),
    }
}

/// Check if two values are equal
fn values_equal(a: &Value, b: &Value) -> Result<bool> {
    match (a, b) {
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
                if !values_equal(av, bv)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (key, av) in a {
                match b.get(key) {
                    Some(bv) => {
                        if !values_equal(av, bv)? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
            Ok(true)
        }
        // Type mismatch
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_from_value_gt() {
        let mut map = std::collections::HashMap::new();
        map.insert("$gt".to_string(), Value::Int(25));
        let value = Value::Object(map);
        
        let op = Operator::from_value(&value).unwrap();
        assert_eq!(op, Operator::Gt(Value::Int(25)));
    }

    #[test]
    fn test_operator_from_value_in() {
        let mut map = std::collections::HashMap::new();
        map.insert("$in".to_string(), Value::Array(vec![
            Value::String("admin".to_string()),
            Value::String("user".to_string()),
        ]));
        let value = Value::Object(map);
        
        let op = Operator::from_value(&value).unwrap();
        match op {
            Operator::In(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("Expected In operator"),
        }
    }

    #[test]
    fn test_operator_matches_gt() {
        let op = Operator::Gt(Value::Int(25));
        assert!(op.matches(&Value::Int(30)).unwrap());
        assert!(!op.matches(&Value::Int(20)).unwrap());
    }

    #[test]
    fn test_operator_matches_ne() {
        let op = Operator::Ne(Value::String("inactive".to_string()));
        assert!(op.matches(&Value::String("active".to_string())).unwrap());
        assert!(!op.matches(&Value::String("inactive".to_string())).unwrap());
    }

    #[test]
    fn test_operator_matches_in() {
        let op = Operator::In(vec![
            Value::String("admin".to_string()),
            Value::String("user".to_string()),
        ]);
        assert!(op.matches(&Value::String("admin".to_string())).unwrap());
        assert!(op.matches(&Value::String("user".to_string())).unwrap());
        assert!(!op.matches(&Value::String("guest".to_string())).unwrap());
    }
}

