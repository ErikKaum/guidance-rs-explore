#[cfg(test)]
use pyo3::types::PyAnyMethods;
#[cfg(test)]
use pyo3::{types::PyModule, PyResult, Python};

#[cfg(test)]
pub fn python_build_regex_from_schema(schema: &str) -> PyResult<String> {
    // TODO this now requires system vide installation of outlines
    Python::with_gil(|py| {
        // calling outlines
        let outlines = PyModule::import_bound(py, "outlines.fsm.json_schema")?;
        let build_regex = outlines.getattr("build_regex_from_schema")?;
        let result: String = build_regex.call1((schema,))?.extract()?;
        Ok(result)
    })
}
