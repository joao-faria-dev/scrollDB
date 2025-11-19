`ScrollDB`
=========

**Embedded Document Database** — Zero dependencies, single binary.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://www.python.org/)

## What is ScrollDB?

ScrollDB is a lightweight document database that designed
for applications that need simple, reliable data storage. Instead of running a separate database server, everything is stored in a single file on disk. You can use it in desktop applications, embedded systems, serverless functions, or anywhere else you need to store structured data without the complexity of managing a full database setup.

It's built with with Rust, but we added Python bindings so you can use it in either language. The API is simple: you store documents, query them, update them, delete them, etc. Just open a file and start using it.


## Installation

### Rust

Add ScrollDB to your `Cargo.toml`:

```toml
[dependencies]
scrolldb = "0.1.0"
```

### Python

Install from PyPI (coming soon):

```bash
pip install scrolldb
```

Or build from source:

```bash
cd scrolldb-py
pip install -e .
```

## Quick Start

### Rust

```rust
use scrolldb::{Database, Value};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::open("myapp.scrolldb")?;
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
import scrolldb

db = scrolldb.Database.open("myapp.scrolldb")
collection = db.collection("users")

user_id = collection.insert_one({
    "name": "John",
    "age": 30,
    "email": "john@example.com"
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

See the [examples](scrolldb-py/examples/) directory for more complete examples.

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

## Contributing

We welcome contributions! ScrollDB is open-source, so you want to fix a bug, add a feature, improve documentation, or just ask questions, we'd love to hear from you.

**Getting started:**
- Read our [Contributing Guide](CONTRIBUTING.md) for development setup and guidelines
- Check out open issues for things to work on
- Submit pull requests for any improvements

Let's make this tool better for everyone!

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
