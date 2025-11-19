#!/usr/bin/env python3
"""Basic usage example for ScrollDB Python bindings"""

import scrolldb
import os

# Create a database
db_path = "example.scrolldb"
if os.path.exists(db_path):
    os.unlink(db_path)

db = scrolldb.Database.open(db_path)
collection = db.collection("users")

# Insert some documents
print("Inserting documents...")
alice_id = collection.insert_one({
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com",
    "tags": ["developer", "python"]
})

bob_id = collection.insert_one({
    "name": "Bob",
    "age": 25,
    "email": "bob@example.com",
    "tags": ["designer", "ui"]
})

charlie_id = collection.insert_one({
    "name": "Charlie",
    "age": 35,
    "email": "charlie@example.com",
    "tags": ["developer", "rust"]
})

print(f"Inserted Alice with ID: {alice_id}")
print(f"Inserted Bob with ID: {bob_id}")
print(f"Inserted Charlie with ID: {charlie_id}")

# Find all documents
print("\nAll users:")
for doc in collection.find({}):
    print(f"  - {doc['name']} (age {doc['age']})")

# Find by query
print("\nUsers older than 28:")
for doc in collection.find({"age": {"$gt": 28}}):
    print(f"  - {doc['name']} (age {doc['age']})")

# Find by ID
print("\nFinding Alice by ID:")
alice = collection.find_by_id(alice_id)
if alice:
    print(f"  Found: {alice['name']} - {alice['email']}")

# Update a document
print("\nUpdating Bob's age...")
count = collection.update_one(
    {"name": "Bob"},
    {"$set": {"age": 26}}
)
print(f"Updated {count} document(s)")

# Verify update
bob = collection.find_by_id(bob_id)
print(f"Bob's new age: {bob['age']}")

# Delete a document
print("\nDeleting Charlie...")
count = collection.delete_one({"name": "Charlie"})
print(f"Deleted {count} document(s)")

# List remaining users
print("\nRemaining users:")
for doc in collection.find({}):
    print(f"  - {doc['name']}")

# Close database
db.close()
print("\nDone!")

