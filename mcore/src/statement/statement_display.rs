use super::{
    symbols::{Symbol, SymbolAttr},
    term::StatementNode,
};

pub fn display_string(node: &StatementNode) -> String {
    match node.data().symbol() {
        Some(symbol) => symbol_display(symbol, node),
        _ => node.data().to_string(),
    }
}

fn symbol_display(symbol: Symbol, node: &StatementNode) -> String {
    match symbol.display_weight() {
        Some(weight) if node.degree() < 2 => format!(
            "{}{}",
            symbol,
            node.iter()
                .map(|x| argument_display(
                    weight,
                    x,
                    symbol.attrs.contains_key(&SymbolAttr::Associative)
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
                    symbol.attrs.contains_key(&SymbolAttr::Associative),
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

fn argument_display(parent_weight: u64, node: &StatementNode, is_associative: bool) -> String {
    if let Some(symbol) = node.data().symbol() {
        if let Some(weight) = symbol.display_weight() {
            if weight > parent_weight || (weight == parent_weight && !is_associative) {
                return format!("({})", display_string(node));
            }
        }
    }
    display_string(node)
}

fn prefix_symbol_display(symbol: Symbol, node: &StatementNode) -> String {
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
        format!("{}", symbol,)
    }
}

#[cfg(test)]
mod tests {
    use crate::statement::statement_with_params;

    #[test]
    fn symbol_display_test() {
        for (statement, display) in &[
            ("a + b + c", "a+b+c"),
            ("a*(b+c)", "a*(b+c)"),
            ("a*b + c", "a*b+c"),
            ("a*b/2 + c", "(a*b)/2+c"),
            ("a + b - c", "(a+b)-c"),
            ("x == -3", "x==-3"),
            ("-(-x + 2)", "-(-x+2)"), //("-(-1)", "-(-1)"),
        ] {
            let statement = statement_with_params(statement);

            assert_eq!(statement.to_string(), *display);
        }
    }
}
