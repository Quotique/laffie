use std::{ffi::CString, str::FromStr};

use log::*;
use pyo3::{
    prelude::*,
    types::{IntoPyDict, PyDict},
};

use solver::term::{SymbolAttr, SymbolAttrValue, SymbolProgram, TermBuf};

use super::{Node, ParserError, Tree};
use crate::py_term::get_term_class;

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
                let program = sym_child.front().unwrap().data().symbol.clone();
                info!("parsing py program {program}");

                Python::initialize();

                let (locals, calc) = Python::attach(|py| -> PyResult<(Py<PyDict>, bool)> {
                    let locals = PyDict::new(py);
                    // Инжектируем класс Term в окружение пользовательского кода
                    locals.set_item("Term", get_term_class(py))?;
                    let code = CString::new(program.as_str())?;
                    py.run(&code, Some(&locals), None)?;

                    let calc = locals.contains("calculator")?;
                    Ok((locals.unbind(), calc))
                })
                .inspect_err(|e| error!("python program error {e}"))
                .map_err(|e| ParserError {
                    loc: sym_child.data().location.clone(),
                    msg: e.to_string(),
                })?;

                if calc {
                    symbol_program.calculator = Box::new(move |root, level| {
                        Python::attach(|py| -> PyResult<bool> {
                            let l = locals.bind(py);
                            let calc_fn = l.get_item("calculator").unwrap();
                            let calc_fn = calc_fn.as_ref().unwrap();

                            // Сериализуем терм в JSON-dict и оборачиваем в Term
                            let json_str = serde_json::to_string(&root.as_ref())
                                .expect("Unable to serialize term");
                            let json_mod = py.import("json")?;
                            let json_dict = json_mod.call_method1("loads", (json_str.as_str(),))?;
                            let term_cls = get_term_class(py);
                            let py_term = term_cls
                                .call((), Some(&[("_raw", json_dict)].into_py_dict(py)?))?;

                            let result = calc_fn.call1((py_term, level.to_string()))?;

                            // None — без изменений, Term — новый терм
                            if result.is_none() {
                                return Ok(false);
                            }

                            let json_dict = result.call_method0("_to_json")?;
                            let json_str: String =
                                json_mod.call_method1("dumps", (json_dict,))?.extract()?;
                            let mut term: TermBuf = serde_json::from_str(&json_str)
                                .expect("Unable to parse procedure result");
                            root.swap(&mut term.term_mut());

                            Ok(true)
                        })
                        // A failing procedural symbol is a no-op, not a crash.
                        .unwrap_or_else(|e| {
                            error!("Procedural symbol failed, skipping: {e}");
                            false
                        })
                    });
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
}
