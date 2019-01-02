pub mod lang;
pub mod syntax_tree;

#[test]
fn lang() {
    let states = lang::StatementsParser::new()
        .parse("x is Real; x is Unknown; x + y == 0 => y + x == 0;")
        .unwrap();
    // assert_eq!(&format!("{:?}", expr), "soso");
    for s in states {
        s.visual();
        // println!("{:?}", s);
    }
}
