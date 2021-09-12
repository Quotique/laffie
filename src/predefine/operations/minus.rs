use crate::statement::{
    symbols::Symbol,
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("-")
        .with_calculator(Box::new(minus))
        .build()
        .unwrap()
}

pub fn minus(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("-") {
        return false;
    }

    let mut result = false;

    match root.degree() {
        1 => {
            let last = root.pop_back().unwrap();
            if let Term::Number(d) = &last.data() {
                *root.data_mut() = Term::Number(-d);
                result = true;
            } else {
                root.push_back(last);
            }
        }
        2 => {
            if let (Term::Number(d1), Term::Number(d2)) =
                (&root.front().unwrap().data(), &root.back().unwrap().data())
            {
                *root.data_mut() = Term::Number(d1 - d2);
                result = true;
                root.pop_back();
                root.pop_back();
            }
        }
        _ => {
            panic!("'-' is binary operator!");
        }
    }
    result
}
