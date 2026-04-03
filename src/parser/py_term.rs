use std::{ffi::CString, sync::OnceLock};

use pyo3::{prelude::*, types::PyDict};

/// Python Term class definition, injected into Python environment before user
/// code. Provides a clean API over the JSON-dict term representation.
///
/// API (must match future PyO3 pyclass version):
///   Term("set", [child1, child2])   — symbol node with children
///   Term.number(n)                  — numeric leaf
///   Term.variable(name)             — variable leaf
///   Term.param(name)                — parameter leaf
///   term.symbol                     — symbol name or None
///   term.is_symbol                  — True if Symbol node
///   term.is_number                  — True if Number node
///   term.is_variable                — True if Variable node
///   term.is_param                   — True if Param node
///   term.value                      — numeric value (int or float)
///   term.name                       — name for Variable/Param
///   term[i]                         — i-th child as Term
///   len(term)                       — number of children
///   term.children                   — list of children as Terms
static PY_TERM_CLASS: OnceLock<Py<PyAny>> = OnceLock::new();

/// Returns the Python `Term` class, executing its definition once on first
/// call.
pub fn get_term_class(py: Python<'_>) -> Bound<'_, PyAny> {
    PY_TERM_CLASS
        .get_or_init(|| {
            let locals = PyDict::new(py);
            let code = CString::new(TERM_CLASS).expect("TERM_CLASS contains null byte");
            py.run(&code, Some(&locals), None)
                .expect("Failed to execute TERM_CLASS");
            locals
                .get_item("Term")
                .expect("Failed to get Term from locals")
                .expect("Term not found in locals")
                .unbind()
        })
        .bind(py)
        .clone()
}

const TERM_CLASS: &str = r#"
class Term:
    __slots__ = ('_data', '_children')

    def __init__(self, symbol=None, children=None, *, _raw=None):
        if _raw is not None:
            self._data = _raw.get("data", {})
            self._children = [Term(_raw=c) for c in _raw.get("children", [])]
        elif symbol is not None:
            self._data = {"Symbol": str(symbol)}
            self._children = list(children) if children else []
        else:
            raise ValueError("Term requires either symbol or _raw")

    @staticmethod
    def number(n):
        t = object.__new__(Term)
        t._data = {"Number": str(n)}
        t._children = []
        return t

    @staticmethod
    def variable(name):
        t = object.__new__(Term)
        t._data = {"Variable": str(name)}
        t._children = []
        return t

    @staticmethod
    def param(name):
        t = object.__new__(Term)
        t._data = {"Param": str(name)}
        t._children = []
        return t

    @property
    def symbol(self):
        return self._data.get("Symbol")

    @property
    def is_symbol(self):
        return "Symbol" in self._data

    @property
    def is_number(self):
        return "Number" in self._data

    @property
    def is_variable(self):
        return "Variable" in self._data

    @property
    def is_param(self):
        return "Param" in self._data

    @property
    def value(self):
        s = self._data.get("Number")
        if s is None:
            raise TypeError("Term is not a number")
        if "." in s or "e" in s or "E" in s:
            return float(s)
        return int(s)

    @property
    def name(self):
        return self._data.get("Variable") or self._data.get("Param")

    @property
    def children(self):
        return list(self._children)

    def __getitem__(self, i):
        return self._children[i]

    def __len__(self):
        return len(self._children)

    def __repr__(self):
        if self.is_number:
            return f"Term.number({self._data['Number']})"
        if self.is_variable:
            return f"Term.variable({self._data['Variable']!r})"
        if self.is_param:
            return f"Term.param({self._data['Param']!r})"
        sym = self._data.get("Symbol", "?")
        if self._children:
            args = ", ".join(repr(c) for c in self._children)
            return f"Term({sym!r}, [{args}])"
        return f"Term({sym!r})"

    def _to_json(self):
        d = {"data": dict(self._data)}
        if self._children:
            d["children"] = [c._to_json() for c in self._children]
        return d
"#;
