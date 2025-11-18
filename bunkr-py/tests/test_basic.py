import tempfile
import os
import bunkr

def test_database_open():
    """Test opening a new database"""
    with tempfile.NamedTemporaryFile(delete=False, suffix='.bunkr') as f:
        path = f.name
    
    try:
        db = bunkr.Database.open(path)
        assert db.is_open()
        db.close()
    finally:
        if os.path.exists(path):
            os.unlink(path)

def test_collection_insert_find():
    """Test inserting and finding documents"""
    with tempfile.NamedTemporaryFile(delete=False, suffix='.bunkr') as f:
        path = f.name
    
    try:
        db = bunkr.Database.open(path)
        collection = db.collection("test")
        
        # Insert a document
        doc_id = collection.insert_one({
            "name": "Alice",
            "age": 30
        })
        assert doc_id is not None
        assert len(doc_id) == 24  # ObjectId hex string length
        
        # Find by ID
        doc = collection.find_by_id(doc_id)
        assert doc is not None
        assert doc["name"] == "Alice"
        assert doc["age"] == 30
        
        # Find by query
        results = list(collection.find({"name": "Alice"}))
        assert len(results) == 1
        assert results[0]["name"] == "Alice"
        
        db.close()
    finally:
        if os.path.exists(path):
            os.unlink(path)

def test_update_delete():
    """Test updating and deleting documents"""
    with tempfile.NamedTemporaryFile(delete=False, suffix='.bunkr') as f:
        path = f.name
    
    try:
        db = bunkr.Database.open(path)
        collection = db.collection("test")
        
        # Insert
        doc_id = collection.insert_one({"name": "Bob", "age": 25})
        
        # Update
        count = collection.update_one(
            {"name": "Bob"},
            {"$set": {"age": 26}}
        )
        assert count == 1
        
        # Verify update
        doc = collection.find_by_id(doc_id)
        assert doc["age"] == 26
        
        # Delete
        count = collection.delete_one({"name": "Bob"})
        assert count == 1
        
        # Verify deletion
        doc = collection.find_by_id(doc_id)
        assert doc is None
        
        db.close()
    finally:
        if os.path.exists(path):
            os.unlink(path)

