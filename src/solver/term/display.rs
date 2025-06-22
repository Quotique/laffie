use std::sync::Arc;

use crate::symbol::{FuncSymbol, Symbol, SymbolAttr};

use super::SymbolNode;

pub fn display_string(node: SymbolNode) -> String {
    let mul_sym_str = FuncSymbol::by_name("*")
        .map(|x| x.to_string())
        .unwrap_or("*".to_owned());

    match node.data() {
        Symbol::FuncSymbol(symbol) => symbol_display(symbol.clone(), node),
        Symbol::Number(num) => num.to_string(),
        _ => node.data().to_string(),
    }
    .replace(&format!("-1{mul_sym_str}"), "-")
    .replace("+-", "-")
}

fn symbol_display(symbol: Arc<FuncSymbol>, node: SymbolNode) -> String {
    match symbol.display_weight() {
        Some(weight) if node.degree() < 2 => format!(
            "{}{}",
            symbol,
            node.iter()
                .map(|x| argument_display(
                    weight,
                    x,
                    symbol.attrs.read().contains_key(&SymbolAttr::Associative)
                ))
                .collect::<Vec::<String>>()
                .join(", ")
        ),
        Some(weight) => node
            .iter()
            .map(|x| {
                argument_display(
                    weight,
                    x,
                    symbol.attrs.read().contains_key(&SymbolAttr::Associative),
                )
            })
            .collect::<Vec<String>>()
            .join(symbol.to_string().as_str()),
        None => {
            // Prefix notation is default
            prefix_symbol_display(symbol, node)
        }
    }
}

fn argument_display(parent_weight: u64, node: SymbolNode, is_associative: bool) -> String {
    if let Some(symbol) = node.data().func_symbol() {
        if let Some(weight) = symbol.display_weight() {
            if weight > parent_weight || (weight == parent_weight && !is_associative) {
                return format!("({})", display_string(node));
            }
        }
    }
    display_string(node)
}

fn prefix_symbol_display(symbol: Arc<FuncSymbol>, node: SymbolNode) -> String {
    if node.degree() > 0 {
        format!(
            "{}({})",
            symbol,
            node.iter()
                .map(display_string)
                .collect::<Vec::<String>>()
                .join(", ")
        )
    } else {
        format!("{symbol}")
    }
}

#[cfg(test)]
mod tests {
    use crate::term::term_with_params;

    #[test]
    fn symbol_display_test() {
        for (term, display) in &[
            ("a + b + c", "a+b+c"),
            ("a*(b+c)", "a*(b+c)"),
            ("a*b + c", "a*b+c"),
            ("a*b/2 + c", "(a*b)/2+c"),
            ("a + b - c", "a+b-c"),
            ("x == -3", "x==-3"),
            ("-(-x + 2)", "-(-x+2)"),
            ("-(-1)", "--1"),
            ("118*x^2 + 1389x - 1507 == 0", "118*x^2+1389*x-1507==0"),
            // TODO: ("(-3)*(x+2)", "-3*(x+2)"),
        ] {
            let term = term_with_params(term);

            assert_eq!(term.to_string(), *display);
        }
    }
}
