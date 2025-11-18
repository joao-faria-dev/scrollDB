use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// MongoDB-compatible ObjectId (12 bytes)
///
/// Structure: [timestamp (4 bytes)][random (8 bytes)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId([u8; 12]);

impl ObjectId {
    /// Create a new ObjectId with current timestamp and random bytes
    pub fn new() -> Self {
        let timestamp = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()) as u32;

        // Use UUID v4 to generate 8 random bytes
        let uuid = Uuid::new_v4();
        let uuid_bytes = uuid.as_bytes();

        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&timestamp.to_be_bytes());
        bytes[4..12].copy_from_slice(&uuid_bytes[0..8]);

        Self(bytes)
    }

    /// Create ObjectId from 12 bytes
    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        Self(bytes)
    }

    /// Get the 12-byte array
    pub fn as_bytes(&self) -> &[u8; 12] {
        &self.0
    }

    /// Convert to hexadecimal string (24 characters)
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create ObjectId from hexadecimal string
    pub fn from_hex(hex: &str) -> Result<Self, ObjectIdError> {
        if hex.len() != 24 {
            return Err(ObjectIdError::InvalidLength);
        }

        let bytes = hex::decode(hex).map_err(|_| ObjectIdError::InvalidHex)?;
        if bytes.len() != 12 {
            return Err(ObjectIdError::InvalidLength);
        }

        let mut id_bytes = [0u8; 12];
        id_bytes.copy_from_slice(&bytes);
        Ok(Self::from_bytes(id_bytes))
    }

    /// Get the timestamp portion (first 4 bytes as u32)
    pub fn timestamp(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for ObjectId {
    type Err = ObjectIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

/// Errors for ObjectId operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectIdError {
    InvalidLength,
    InvalidHex,
}

impl fmt::Display for ObjectIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectIdError::InvalidLength => write!(f, "ObjectId must be 24 hex characters"),
            ObjectIdError::InvalidHex => write!(f, "Invalid hexadecimal string"),
        }
    }
}

impl std::error::Error for ObjectIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_object_id() {
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();
        
        // Should be different
        assert_ne!(id1, id2);
        
        // Should be 12 bytes
        assert_eq!(id1.as_bytes().len(), 12);
        assert_eq!(id2.as_bytes().len(), 12);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = [0u8; 12];
        let id = ObjectId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_to_hex() {
        let id = ObjectId::from_bytes([
            0x12, 0x34, 0x56, 0x78,
            0x9a, 0xbc, 0xde, 0xf0,
            0x11, 0x22, 0x33, 0x44,
        ]);
        assert_eq!(id.to_hex(), "123456789abcdef011223344");
    }

    #[test]
    fn test_from_hex() {
        let hex = "123456789abcdef011223344";
        let id = ObjectId::from_hex(hex).unwrap();
        assert_eq!(id.to_hex(), hex);
    }

    #[test]
    fn test_from_hex_invalid_length() {
        assert!(ObjectId::from_hex("123").is_err());
        assert!(ObjectId::from_hex("123456789abcdef01122334455").is_err());
    }

    #[test]
    fn test_from_hex_invalid_hex() {
        assert!(ObjectId::from_hex("123456789abcdef01122334g").is_err());
    }

    #[test]
    fn test_display() {
        let id = ObjectId::from_bytes([0x12; 12]);
        let hex = format!("{}", id);
        assert_eq!(hex, "121212121212121212121212");
    }

    #[test]
    fn test_from_str() {
        let hex = "123456789abcdef011223344";
        let id: ObjectId = hex.parse().unwrap();
        assert_eq!(id.to_hex(), hex);
    }

    #[test]
    fn test_timestamp() {
        let id = ObjectId::new();
        let timestamp = id.timestamp();
        // Should be a reasonable timestamp (after 2020)
        assert!(timestamp > 1577836800); // Jan 1, 2020
    }

    #[test]
    fn test_serialization() {
        let id = ObjectId::new();
        let json = serde_json::to_string(&id).unwrap();
        let id2: ObjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, id2);
    }
}

