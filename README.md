# Bunkr

**Embedded Document Database** — MongoDB feel, zero dependencies, single binary.

Bunkr is an embedded document database for Rust applications, designed to be the LiteDB/MongoDB Realm that Rust/Tauri/Electron/Bevy/Flutter developers have always dreamed of.

## Features

- ✅ **Single-file database** — No server, no configuration
- ✅ **MongoDB-style API** — Familiar `insert`, `find`, `update`, `delete` operations
- ✅ **Automatic indexes** — Created on first query (no upfront cost)

## Quick Start

Add Bunkr to your `Cargo.toml`:

```toml
[dependencies]
bunkr = "0.1.0"
```

Basic usage:

```rust
use bunkr::{Database, Value};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open or create a database
    let mut db = Database::open("myapp.bunkr")?;
    
    // Get a collection
    let mut users = db.collection("users")?;
    
    // Insert a document (auto-generates _id if not provided)
    let mut doc = Value::Object(HashMap::new());
    if let Value::Object(ref mut map) = doc {
        map.insert("name".to_string(), Value::String("João".to_string()));
        map.insert("age".to_string(), Value::Int(30));
        map.insert("active".to_string(), Value::Bool(true));
    }
    
    let id = users.insert_one(doc)?;
    println!("Inserted document with id: {}", id);
    
    // Close the database
    db.close()?;
    Ok(())
}
```

## Current Status (v0.2.0)

**Implemented:**
- ✅ File format with magic header
- ✅ Database open/close API
- ✅ Header validation
- ✅ Error handling for corrupted files
- ✅ Document insert with automatic `_id` generation
- ✅ Page-based storage system (4KB pages)
- ✅ Document serialization

**Coming soon:**
- Document find operations
- Query parser
- Automatic indexes
- Update and delete operations
- Transactions and WAL

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
