pub mod lang;
pub mod syntax_tree;

#[test]
fn lang() {
    let test = "x in Real; x is Unknown;";
    println!("{}", test);
    let states = lang::StatementsParser::new().parse(test).unwrap();
    for s in states {
        s.visual();
    }

    let test2 = "x*(y+z) == 0 => y+z == 0 || x == 0;";
    println!("{}", test2);
    let states2 = lang::StatementsParser::new().parse(test2).unwrap();
    for s in states2 {
        s.visual();
    }
    let test3 = "-sin(x) == 0 => x == Pi*n && n in Z;";
    println!("{}", test3);
    let states3 = lang::StatementsParser::new().parse(test3).unwrap();
    for s in states3 {
        s.visual();
    }
}
