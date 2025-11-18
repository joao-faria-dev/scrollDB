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
use bunkr::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open or create a database
    let db = Database::open("myapp.bunkr")?;
    
    println!("Bunkr v0.1 rodando!");
    
    // Close the database
    db.close()?;
    Ok(())
}
```

## Current Status (v0.1.0)

This is the initial release with basic file format and open/close functionality:

- ✅ File format with magic header
- ✅ Database open/close API
- ✅ Header validation
- ✅ Error handling for corrupted files

**Coming soon:**
- Document insert/find/update/delete
- Query parser
- Automatic indexes
- Transactions and WAL

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
