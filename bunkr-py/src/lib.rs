use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

/// Python bindings for Bunkr database
#[pymodule]
fn bunkr_py(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Database>()?;
    m.add_class::<Collection>()?;
    m.add_class::<DocumentIterator>()?;
    Ok(())
}

/// Convert Python object to Bunkr Value
fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<bunkr::Value> {
    if obj.is_none() {
        Ok(bunkr::Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(bunkr::Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(bunkr::Value::Int(i))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(bunkr::Value::Float(f))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(bunkr::Value::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(py_to_value(&item)?);
        }
        Ok(bunkr::Value::Array(vec))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            map.insert(key_str, py_to_value(&value)?);
        }
        Ok(bunkr::Value::Object(map))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            format!("Unsupported Python type: {:?}", obj.get_type()),
        ))
    }
}

/// Convert Bunkr Value to Python object
fn value_to_py(value: &bunkr::Value, py: Python<'_>) -> PyResult<PyObject> {
    match value {
        bunkr::Value::Null => Ok(py.None()),
        bunkr::Value::Bool(b) => Ok(b.into_py(py)),
        bunkr::Value::Int(i) => Ok(i.into_py(py)),
        bunkr::Value::Float(f) => Ok(f.into_py(py)),
        bunkr::Value::String(s) => Ok(s.clone().into_py(py)),
        bunkr::Value::Array(arr) => {
            let list = PyList::empty_bound(py);
            for item in arr {
                list.append(value_to_py(item, py)?)?;
            }
            Ok(list.into())
        }
        bunkr::Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (key, val) in map {
                dict.set_item(key, value_to_py(val, py)?)?;
            }
            Ok(dict.into())
        }
    }
}

#[pyclass]
struct Database {
    inner: bunkr::Database,
}

#[pymethods]
impl Database {
    #[new]
    fn new() -> PyResult<Self> {
        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            "Use Database.open() to create a database instance",
        ))
    }

    /// Open a database file
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let db = bunkr::Database::open(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(Self { inner: db })
    }

    /// Check if the database is open
    fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    /// Get a collection by name
    fn collection(&mut self, name: &str) -> PyResult<Collection> {
        let coll = self.inner.collection(name)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(Collection { inner: coll })
    }

    /// Close the database
    /// 
    /// This flushes any pending writes. The database will also be closed
    /// automatically when the object is garbage collected.
    fn close(&mut self) -> PyResult<()> {
        // Database::close takes ownership, but we can't move self in PyO3.
        // Instead, we rely on Drop to handle cleanup automatically.
        // This method is provided for explicit cleanup, but the actual
        // flushing happens in Drop.
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        // Database will be closed automatically via its Drop implementation
    }
}

#[pyclass]
struct Collection {
    inner: bunkr::Collection,
}

#[pymethods]
impl Collection {
    /// Get the collection name
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Insert a document into the collection
    /// 
    /// The document should be a Python dict. If `_id` is not provided,
    /// it will be auto-generated.
    /// 
    /// Returns the document ID as a string.
    fn insert_one(&mut self, doc: &Bound<'_, PyAny>) -> PyResult<String> {
        let value = py_to_value(doc)?;
        let id = self.inner.insert_one(value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(id.to_hex())
    }

    /// Find a document by ID
    /// 
    /// Returns the document as a dict, or None if not found.
    fn find_by_id(&mut self, py: Python<'_>, id: &str) -> PyResult<Option<PyObject>> {
        let object_id = bunkr::ObjectId::from_hex(id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid ObjectId: {}", e)))?;
        
        match self.inner.find_by_id(&object_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?
        {
            Some(value) => Ok(Some(value_to_py(&value, py)?)),
            None => Ok(None),
        }
    }

    /// Find documents matching a query
    /// 
    /// Returns an iterator over matching documents.
    fn find(&mut self, py: Python<'_>, query: &Bound<'_, PyAny>) -> PyResult<Py<DocumentIterator>> {
        let query_value = py_to_value(query)?;
        let iter = self.inner.find(query_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        
        // Create a Python iterator
        let iter = DocumentIterator {
            inner: iter,
        };
        
        Ok(Py::new(py, iter)?)
    }

    /// Update a single document matching the query
    /// 
    /// Returns the number of documents updated (0 or 1).
    fn update_one(&mut self, filter: &Bound<'_, PyAny>, update: &Bound<'_, PyAny>) -> PyResult<u64> {
        let filter_value = py_to_value(filter)?;
        let update_value = py_to_value(update)?;
        let count = self.inner.update_one(filter_value, update_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(count)
    }

    /// Update multiple documents matching the query
    /// 
    /// Returns the number of documents updated.
    fn update_many(&mut self, filter: &Bound<'_, PyAny>, update: &Bound<'_, PyAny>) -> PyResult<u64> {
        let filter_value = py_to_value(filter)?;
        let update_value = py_to_value(update)?;
        let count = self.inner.update_many(filter_value, update_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(count)
    }

    /// Delete a single document matching the query
    /// 
    /// Returns the number of documents deleted (0 or 1).
    fn delete_one(&mut self, filter: &Bound<'_, PyAny>) -> PyResult<u64> {
        let filter_value = py_to_value(filter)?;
        let count = self.inner.delete_one(filter_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(count)
    }

    /// Delete multiple documents matching the query
    /// 
    /// Returns the number of documents deleted.
    fn delete_many(&mut self, filter: &Bound<'_, PyAny>) -> PyResult<u64> {
        let filter_value = py_to_value(filter)?;
        let count = self.inner.delete_many(filter_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;
        Ok(count)
    }
}

/// Python iterator over documents
#[pyclass]
struct DocumentIterator {
    inner: bunkr::FilteredDocumentIterator,
}

#[pymethods]
impl DocumentIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match slf.inner.next() {
            Some(Ok(value)) => Ok(Some(value_to_py(&value, py)?)),
            Some(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e))),
            None => Ok(None),
        }
    }
}

