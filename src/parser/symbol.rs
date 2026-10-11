use std::{ffi::CString, str::FromStr};

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

use super::{Node, ParserError, Tree};
use crate::{
    py_term::get_term_class,
    source::Source,
    syntax::{self, Expr},
};

pub struct SymbolParser<'a> {
    ast: &'a Tree,
}

impl<'a> From<&'a Tree> for SymbolParser<'a> {
    fn from(syntax_tree: &'a Tree) -> Self {
        Self { ast: syntax_tree }
    }
}

impl<'a> SymbolParser<'a> {
    pub fn parse(self) -> Result<SymbolProgram, ParserError> {
        if self.ast.data().symbol != "Declare" {
            return Err(ParserError {
                loc: self.ast.data().location.clone(),
                msg: "expected 'symbol'".to_owned(),
            });
        }

        let mut symbol_program = SymbolProgram::default();

        for sym_child in self.ast.iter() {
            if sym_child.data().symbol == "Symbol" {
                symbol_program.name = sym_child.front().unwrap().data().symbol.clone();
            } else if sym_child.data().symbol == "PyProg" {
                let py_src = sym_child.front().unwrap().data().symbol.clone();
                if let Some(calculator) = python_calculator(&py_src).map_err(|e| ParserError {
                    loc: sym_child.data().location.clone(),
                    msg: e.to_string(),
                })? {
                    symbol_program.calculator = Box::new(calculator);
                }
            } else if sym_child.data().symbol == "Attrs" {
                for attr in sym_child.iter() {
                    let a = Self::parse_attr(attr)?;
                    symbol_program.attrs.insert(a.0, a.1);
                }
            }
        }
        if symbol_program.name.is_empty() {
            Err(ParserError {
                loc: self.ast.data().location.clone(),
                msg: "symbol name is mandatory".into(),
            })
        } else {
            Ok(symbol_program)
        }
    }

    fn parse_attr(data: &Node) -> Result<(SymbolAttr, SymbolAttrValue), ParserError> {
        match data.data().symbol.as_str() {
            "infix" => {
                let c = data.front().ok_or_else(|| ParserError {
                    loc: data.data().location.clone(),
                    msg: "infix(number) argument is required!".to_owned(),
                })?;
                let w = u64::from_str(&c.data().symbol).map_err(|_| ParserError {
                    loc: data.data().location.clone(),
                    msg: "infix argument must be u64".to_owned(),
                })?;
                Ok((SymbolAttr::Infix, SymbolAttrValue::UInt(w)))
            }
            "display" => {
                let s = data
                    .front()
                    .ok_or_else(|| ParserError {
                        loc: data.data().location.clone(),
                        msg: "display(s) argument is required!".to_owned(),
                    })?
                    .data();
                Ok((SymbolAttr::Display, SymbolAttrValue::UStr(s.symbol.clone())))
            }
            "associative" => Ok((SymbolAttr::Associative, SymbolAttrValue::None)),
            "commutative" => Ok((SymbolAttr::Commutative, SymbolAttrValue::None)),
            _ => Err(ParserError {
                loc: data.data().location.clone(),
                msg: "unknown attribute".to_owned(),
            }),
        }
    }
}

impl syntax::Symbol {
    /// Executes the `py` block, if any.
    pub(crate) fn build(&self, src: &Source) -> Result<SymbolProgram, ParserError> {
        let mut program = SymbolProgram {
            name: self.name.clone(),
            ..Default::default()
        };
        if let Some(py) = &self.py &&
            let Some(calculator) =
                python_calculator(&py.code).map_err(|e| src.error(py.span, e))?
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

/// Runs `py_src` once and takes the `calculator` it defines.
fn python_calculator(
    py_src: &str,
) -> PyResult<Option<impl Fn(&mut TermMut, NormLevel) -> bool + Send + Sync + use<>>> {
    info!("parsing py program {py_src}");

    Python::initialize();

    let (locals, has_calculator) = Python::attach(|py| -> PyResult<(Py<PyDict>, bool)> {
        let locals = PyDict::new(py);
        locals.set_item("Term", get_term_class(py))?;
        // `.sym` делают `exec(SYMPY_CONVERT_SRC)`: исходник вкомпилен, чтобы не зависеть от CWD.
        locals.set_item(
            "SYMPY_CONVERT_SRC",
            include_str!("../../symbols/py/sympy_convert.py"),
        )?;
        let code = CString::new(py_src)?;
        py.run(&code, Some(&locals), None)?;

        let has_calculator = locals.contains("calculator")?;
        Ok((locals.unbind(), has_calculator))
    })
    .inspect_err(|e| error!("python program error {e}"))?;

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

#[cfg(test)]
pub mod tests {
    use crate::lang;

    use super::*;

    #[test]
    fn parser_test() {
        let test_str = "symbol + { attr infix(10) }";
        let states = lang::symbol(test_str).unwrap();

        let sym = SymbolParser::from(&states).parse().unwrap();
        assert_eq!(sym.name, "+");
        assert_eq!(
            sym.attrs.get(&SymbolAttr::Infix),
            Some(&SymbolAttrValue::UInt(10))
        );
    }

    #[test]
    fn py_parser_test() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "print(1)";
        }"#;
        let _states = lang::symbol(test_str).unwrap();
    }

    #[test]
    fn py_parser_escape_test() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "print(\"Hello world\")";
        }"#;
        let _states = lang::symbol(test_str).unwrap();
    }

    #[test]
    fn py_parser_multiline_test() {
        let test_str = r#"symbol + {
            attr infix(10);
            py "def hello():
    print(\"Hello world\")
";
        }"#;
        let _states = lang::symbol(test_str).unwrap();
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
        let states = lang::symbol(test_str).unwrap();
        let program = SymbolParser::from(&states).parse().unwrap();

        let mut term = TermBuf::variable("x");
        let original = term.clone();
        let changed = (program.calculator)(&mut term.term_mut(), NormLevel::Full);

        assert!(!changed, "a failing calculator must report no change");
        assert_eq!(term, original, "the term must be left untouched");
    }
}
