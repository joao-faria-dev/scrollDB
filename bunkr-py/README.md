# Bunkr Python Bindings

Python bindings for Bunkr, a fast embedded document database.

## Installation

```bash
pip install bunkr
```

Or build from source:

```bash
maturin develop
```

## Quick Start

```python
import bunkr

# Open or create a database
db = bunkr.Database.open("mydb.bunkr")

# Get a collection
collection = db.collection("users")

# Insert a document
doc_id = collection.insert_one({
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com"
})
print(f"Inserted document with ID: {doc_id}")

# Find documents
for doc in collection.find({"age": {"$gt": 25}}):
    print(doc)

# Update a document
collection.update_one(
    {"name": "Alice"},
    {"$set": {"age": 31}}
)

# Delete a document
collection.delete_one({"name": "Alice"})

# Close the database
db.close()
```

## API Reference

### Database

- `Database.open(path: str) -> Database`: Open or create a database
- `is_open() -> bool`: Check if database is open
- `collection(name: str) -> Collection`: Get a collection by name
- `close()`: Close the database

### Collection

- `name() -> str`: Get collection name
- `insert_one(doc: dict) -> str`: Insert a document, returns ObjectId as hex string
- `find_by_id(id: str) -> dict | None`: Find document by ID
- `find(query: dict) -> Iterator[dict]`: Find documents matching query
- `update_one(filter: dict, update: dict) -> int`: Update one document
- `update_many(filter: dict, update: dict) -> int`: Update multiple documents
- `delete_one(filter: dict) -> int`: Delete one document
- `delete_many(filter: dict) -> int`: Delete multiple documents

## Query Operators

- `{"field": value}`: Exact match
- `{"field": {"$gt": value}}`: Greater than
- `{"field": {"$gte": value}}`: Greater than or equal
- `{"field": {"$lt": value}}`: Less than
- `{"field": {"$lte": value}}`: Less than or equal
- `{"field": {"$ne": value}}`: Not equal
- `{"field": {"$in": [value1, value2]}}`: In array
- `{"$text": {"$search": "query"}}`: Text search

## Update Modifiers

- `{"$set": {"field": value}}`: Set field value
- `{"$unset": {"field": ""}}`: Remove field
- `{"$inc": {"field": 1}}`: Increment numeric field

