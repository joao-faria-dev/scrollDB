use crate::types::Value;
use crate::error::{Error, Result};
use super::path::{parse_path, get_value_at_path, set_value_at_path};

/// Update modifier operations
#[derive(Debug, Clone)]
pub enum UpdateModifier {
    /// $set: Set field to value
    Set(String, Value),
    /// $unset: Remove field
    Unset(String),
    /// $inc: Increment numeric field
    Inc(String, Value),
}

impl UpdateModifier {
    /// Parse update operations from a Value
    /// 
    /// Example: {"$set": {"age": 31}} -> [UpdateModifier::Set("age", Value::Int(31))]
    pub fn from_value(value: &Value) -> Result<Vec<Self>> {
        match value {
            Value::Object(map) => {
                let mut modifiers = Vec::new();
                
                for (key, val) in map {
                    match key.as_str() {
                        "$set" => {
                            match val {
                                Value::Object(fields) => {
                                    for (field, field_val) in fields {
                                        modifiers.push(UpdateModifier::Set(field.clone(), field_val.clone()));
                                    }
                                }
                                _ => {
                                    return Err(Error::CorruptedDatabase {
                                        reason: "$set requires an object value".to_string(),
                                    });
                                }
                            }
                        }
                        "$unset" => {
                            match val {
                                Value::Object(fields) => {
                                    for (field, _) in fields {
                                        modifiers.push(UpdateModifier::Unset(field.clone()));
                                    }
                                }
                                _ => {
                                    return Err(Error::CorruptedDatabase {
                                        reason: "$unset requires an object value".to_string(),
                                    });
                                }
                            }
                        }
                        "$inc" => {
                            match val {
                                Value::Object(fields) => {
                                    for (field, field_val) in fields {
                                        modifiers.push(UpdateModifier::Inc(field.clone(), field_val.clone()));
                                    }
                                }
                                _ => {
                                    return Err(Error::CorruptedDatabase {
                                        reason: "$inc requires an object value".to_string(),
                                    });
                                }
                            }
                        }
                        _ => {
                            return Err(Error::CorruptedDatabase {
                                reason: format!("Unknown update modifier: {}", key),
                            });
                        }
                    }
                }
                
                Ok(modifiers)
            }
            _ => Err(Error::CorruptedDatabase {
                reason: "Update operations must be an Object".to_string(),
            }),
        }
    }

    /// Apply this modifier to a document
    pub fn apply(&self, doc: &mut Value) -> Result<()> {
        match self {
            UpdateModifier::Set(path_str, new_value) => {
                let path = parse_path(path_str);
                set_value_at_path(doc, &path, new_value.clone())
            }
            UpdateModifier::Unset(path_str) => {
                let path = parse_path(path_str);
                if path.is_empty() {
                    return Err(Error::CorruptedDatabase {
                        reason: "Cannot unset root document".to_string(),
                    });
                }
                
                // Navigate to parent and remove the field
                match doc {
                    Value::Object(map) => {
                        if path.len() == 1 {
                            // Simple field removal
                            map.remove(&path[0]);
                            Ok(())
                        } else {
                            // Nested field removal - need to navigate and modify
                            let parent_path = &path[..path.len() - 1];
                            let field_name = &path[path.len() - 1];
                            
                            // Get parent value mutably
                            let mut current: &mut Value = doc;
                            for segment in parent_path {
                                match current {
                                    Value::Object(parent_map) => {
                                        current = parent_map.get_mut(segment).ok_or_else(|| Error::CorruptedDatabase {
                                            reason: format!("Path {} does not exist", path_str),
                                        })?;
                                    }
                                    _ => {
                                        return Err(Error::CorruptedDatabase {
                                            reason: format!("Cannot unset field in non-object at path {}", path_str),
                                        });
                                    }
                                }
                            }
                            
                            // Remove the field
                            match current {
                                Value::Object(parent_map) => {
                                    parent_map.remove(field_name);
                                    Ok(())
                                }
                                _ => Err(Error::CorruptedDatabase {
                                    reason: format!("Cannot unset field in non-object at path {}", path_str),
                                }),
                            }
                        }
                    }
                    _ => Err(Error::CorruptedDatabase {
                        reason: "Cannot unset field in non-object document".to_string(),
                    }),
                }
            }
            UpdateModifier::Inc(path_str, increment) => {
                let path = parse_path(path_str);
                let current_value = get_value_at_path(doc, &path);
                
                let new_value = match (current_value, increment) {
                    (Some(Value::Int(current)), Value::Int(inc)) => Value::Int(current + inc),
                    (Some(Value::Float(current)), Value::Float(inc)) => Value::Float(current + inc),
                    (Some(Value::Int(current)), Value::Float(inc)) => Value::Float(current as f64 + inc),
                    (Some(Value::Float(current)), Value::Int(inc)) => Value::Float(current + *inc as f64),
                    (None, Value::Int(inc)) => Value::Int(*inc), // Treat missing as 0
                    (None, Value::Float(inc)) => Value::Float(*inc),
                    _ => {
                        return Err(Error::CorruptedDatabase {
                            reason: format!("Cannot increment non-numeric value at path {}", path_str),
                        });
                    }
                };
                
                set_value_at_path(doc, &path, new_value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_update_modifier_set() {
        let mut doc = Value::Object(HashMap::new());
        let modifier = UpdateModifier::Set("age".to_string(), Value::Int(31));
        modifier.apply(&mut doc).unwrap();
        
        assert_eq!(get_value_at_path(&doc, &parse_path("age")), Some(Value::Int(31)));
    }

    #[test]
    fn test_update_modifier_unset() {
        let mut doc = Value::Object({
            let mut map = HashMap::new();
            map.insert("age".to_string(), Value::Int(30));
            map
        });
        
        let modifier = UpdateModifier::Unset("age".to_string());
        modifier.apply(&mut doc).unwrap();
        
        assert_eq!(get_value_at_path(&doc, &parse_path("age")), None);
    }

    #[test]
    fn test_update_modifier_inc() {
        let mut doc = Value::Object({
            let mut map = HashMap::new();
            map.insert("age".to_string(), Value::Int(30));
            map
        });
        
        let modifier = UpdateModifier::Inc("age".to_string(), Value::Int(1));
        modifier.apply(&mut doc).unwrap();
        
        assert_eq!(get_value_at_path(&doc, &parse_path("age")), Some(Value::Int(31)));
    }
}

