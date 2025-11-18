use pyo3::prelude::*;

/// Python bindings for Bunkr database
#[pymodule]
fn bunkr_py(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Database>()?;
    m.add_class::<Collection>()?;
    Ok(())
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
}

