use evc::{lexer::Lexer, parser::Parser};

fn main() {
    let source = r#"
        fn main() -> u64 {
            3
        }
    "#;

    let parser = Parser::new(Lexer::new(source));

    let statements = parser.collect::<Result<Vec<_>, _>>().expect("Some error idk");

    for statement in statements {
        println!("{}", statement.span().as_slice(source));
    }

    unimplemented!("Cli missing!")
}
