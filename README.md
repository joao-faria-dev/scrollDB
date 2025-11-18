`Bunkr`
=========

**Embedded Document Database** — Zero dependencies, single binary.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://www.python.org/)

Bunkr is an embedded document database for Rust and Python applications. Perfect for desktop applications, embedded systems, and serverless environments where you need a lightweight, file-based database.

## Features

- **Single-file database** — No server, no configuration, just a file
- **Zero runtime dependencies** — Pure Rust implementation
- **Python bindings** — Full-featured Python API via PyO3
- **Document queries** — Find, update, and delete operations
- **Automatic indexing** — Indexes created on first query
- **Page-based storage** — Efficient 4KB page management
- **Type-safe** — Leverages Rust's type system
- **Cross-platform** — Works on Windows, macOS, and Linux

## Installation

### Rust

Add Bunkr to your `Cargo.toml`:

```toml
[dependencies]
bunkr = "0.1.0"
```

### Python

Install from PyPI (coming soon):

```bash
pip install bunkr
```

Or build from source:

```bash
cd bunkr-py
pip install -e .
```

## Quick Start

### Rust

```rust
use bunkr::{Database, Value};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::open("myapp.bunkr")?;
    let mut users = db.collection("users")?;
    
    let mut doc = Value::Object(HashMap::new());
    if let Value::Object(ref mut map) = doc {
        map.insert("name".to_string(), Value::String("John".to_string()));
        map.insert("age".to_string(), Value::Int(30));
    }
    
    let id = users.insert_one(doc)?;
    
    for doc in users.find(Value::Object(HashMap::new()))? {
        println!("Found: {:?}", doc);
    }
    
    db.close()?;
    Ok(())
}
```

### Python

```python
import bunkr

db = bunkr.Database.open("myapp.bunkr")
collection = db.collection("users")

user_id = collection.insert_one({
    "name": "John",
    "age": 30,
    "email": "John@example.com"
})

for doc in collection.find({"age": {"$gt": 25}}):
    print(f"Found: {doc['name']}")

collection.update_one(
    {"name": "John"},
    {"$set": {"age": 31}}
)

collection.delete_one({"name": "John"})
db.close()
```

See the [examples](bunkr-py/examples/) directory for more complete examples.

## API Reference

### Python API

- `Database.open(path)` - Open or create a database
- `Database.collection(name)` - Get a collection
- `Database.close()` - Close the database
- `Collection.insert_one(doc)` - Insert a document
- `Collection.find(query)` - Find documents matching a query
- `Collection.find_by_id(id)` - Find a document by ID
- `Collection.update_one(query, update)` - Update documents
- `Collection.delete_one(query)` - Delete documents

### Query Operators

- `$eq` - Equal to
- `$ne` - Not equal to
- `$gt` - Greater than
- `$gte` - Greater than or equal to
- `$lt` - Less than
- `$lte` - Less than or equal to
- `$in` - In array
- `$nin` - Not in array
- `$text` - Text search (creates index automatically)


## Status

**Implemented (v0.1.0):**
- File format with magic header
- Database open/close API
- Document insert, find, update, delete
- Query operators
- Text search with automatic indexing
- Python bindings

**Coming Soon:**
- Compound indexes

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
