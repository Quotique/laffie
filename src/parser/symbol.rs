use std::ffi::CString;

use log::*;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{IntoPyDict, PyDict},
};

use solver::{
    NormLevel,
    term::{SymbolAttr, SymbolAttrValue, SymbolProgram, TermBuf, TermMut},
};

use crate::{
    ParserError,
    py_term::get_term_class,
    source::Source,
    syntax::{self, Expr},
};

impl syntax::Symbol {
    /// Executes the `py` block, if any.
    pub(crate) fn build(&self, src: &Source) -> Result<SymbolProgram, ParserError> {
        let mut program = SymbolProgram {
            name: self.name.clone(),
            ..Default::default()
        };
        if let Some(py) = &self.py &&
            let Some(calculator) = py.calculator(src)?
        {
            program.calculator = Box::new(calculator);
        }
        for e in &self.attrs {
            let (key, value) = attr(e, src)?;
            program.attrs.insert(key, value);
        }
        Ok(program)
    }
}

impl syntax::PySource {
    /// Runs the code once and takes the `calculator` it defines.
    fn calculator(
        &self,
        src: &Source,
    ) -> Result<Option<impl Fn(&mut TermMut, NormLevel) -> bool + Send + Sync + use<>>, ParserError>
    {
        info!("parsing py program {}", self.code);

        Python::initialize();

        let (locals, has_calculator) = Python::attach(|py| -> PyResult<(Py<PyDict>, bool)> {
            let locals = PyDict::new(py);
            locals.set_item("Term", get_term_class(py))?;
            // `.sym` делают `exec(SYMPY_CONVERT_SRC)`: исходник вкомпилен, CWD не важен.
            locals.set_item(
                "SYMPY_CONVERT_SRC",
                include_str!("../../symbols/py/sympy_convert.py"),
            )?;
            let code = CString::new(self.code.as_str())?;
            py.run(&code, Some(&locals), None)?;

            let has_calculator = locals.contains("calculator")?;
            Ok((locals.unbind(), has_calculator))
        })
        .inspect_err(|e| error!("python program error {e}"))
        .map_err(|e| src.error(self.span, e))?;

        if !has_calculator {
            return Ok(None);
        }
        Ok(Some(move |root: &mut TermMut, level: NormLevel| {
            Python::attach(|py| -> PyResult<bool> {
                let l = locals.bind(py);
                let calc_fn = l.get_item("calculator").unwrap();
                let calc_fn = calc_fn.as_ref().unwrap();

                let json_str = serde_json::to_string(&root.as_ref())
                    .map_err(|e| PyValueError::new_err(format!("serialize term: {e}")))?;
                let json_mod = py.import("json")?;
                let json_dict = json_mod.call_method1("loads", (json_str.as_str(),))?;
                let term_cls = get_term_class(py);
                let py_term = term_cls.call((), Some(&[("_raw", json_dict)].into_py_dict(py)?))?;

                let result = calc_fn.call1((py_term, level.to_string()))?;

                // None — без изменений, Term — новый терм
                if result.is_none() {
                    return Ok(false);
                }

                let json_dict = result.call_method0("_to_json")?;
                let json_str: String = json_mod.call_method1("dumps", (json_dict,))?.extract()?;
                let mut term: TermBuf = serde_json::from_str(&json_str)
                    .map_err(|e| PyValueError::new_err(format!("parse procedure result: {e}")))?;
                root.swap(&mut term.term_mut());

                Ok(true)
            })
            // A failing procedural symbol is a no-op, not a crash.
            .unwrap_or_else(|e| {
                error!("Procedural symbol failed, skipping: {e}");
                false
            })
        }))
    }
}

fn attr(e: &Expr, src: &Source) -> Result<(SymbolAttr, SymbolAttrValue), ParserError> {
    let (name, args) = e
        .as_call()
        .ok_or_else(|| src.error(e.span, "unknown attribute"))?;
    match (name, args.first()) {
        ("infix", Some(arg)) => Ok((
            SymbolAttr::Infix,
            SymbolAttrValue::UInt(src.uint(arg.span)?),
        )),
        ("display", Some(arg)) => Ok((
            SymbolAttr::Display,
            SymbolAttrValue::UStr(src.ident(arg).into()),
        )),
        ("infix" | "display", None) => Err(src.error(e.span, "must have an argument")),
        ("associative", _) => Ok((SymbolAttr::Associative, SymbolAttrValue::None)),
        ("commutative", _) => Ok((SymbolAttr::Commutative, SymbolAttrValue::None)),
        _ => Err(src.error(e.span, "unknown attribute")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_reads_its_infix_precedence() {
        let test_str = "symbol + { attr infix(10) }";
        let sym = crate::symbol(test_str).unwrap();
        assert_eq!(sym.name, "+");
        assert_eq!(
            sym.attrs.get(&SymbolAttr::Infix),
            Some(&SymbolAttrValue::UInt(10))
        );
    }

    #[test]
    fn a_py_block_runs() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "print(1)";
        }"#;
        crate::symbol(test_str).unwrap();
    }

    #[test]
    fn a_py_string_unescapes_quotes() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "print(\"Hello world\")";
        }"#;
        crate::symbol(test_str).unwrap();
    }

    #[test]
    fn a_py_string_spans_lines() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "def hello():
    print(\"Hello world\")
";
        }"#;
        crate::symbol(test_str).unwrap();
    }

    #[test]
    fn malformed_calculator_result_is_a_noop() {
        use solver::NormLevel;

        // A procedural calculator whose result can't be deserialized must be a
        // no-op, not a panic: the term stays put and the solver keeps running.
        let test_str = r#"symbol foo {
            py "
def calculator(root, level):
    return Term.number('not_a_number')
";
        }"#;
        let program = crate::symbol(test_str).unwrap();

        let mut term = TermBuf::variable("x");
        let original = term.clone();
        let changed = (program.calculator)(&mut term.term_mut(), NormLevel::Full);

        assert!(!changed, "a failing calculator must report no change");
        assert_eq!(term, original, "the term must be left untouched");
    }
}
