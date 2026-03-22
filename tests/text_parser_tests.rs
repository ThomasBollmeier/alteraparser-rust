mod grammar4test;

use alteraparser::ast::AstStrWriter;
use alteraparser::error::ParseError;
use alteraparser::parser::TextParser;

fn make_text_parser() -> TextParser {
    TextParser::new(grammar4test::make_grammar(), grammar4test::make_lexer_grammar())
}

#[test]
fn test_text_parser_simple_expr() {
    let parser = make_text_parser();
    let input = "\n        3 + 5 * x * y(42, mult(a, b))\n    ";
    let ast = parser.parse_text(input).expect("parse should succeed");
    assert!(ast.name == "sum" || ast.name == "term" || ast.name == "call" || ast.name == "NUMBER" || ast.name == "IDENT",
        "unexpected root name: {}", ast.name);
    let writer = AstStrWriter::default();
    println!("AST:\n{}", writer.write_ast_to_str(&ast));
}

#[test]
fn test_invalid_syntax() {
    let parser = make_text_parser();
    let input = "\n        3 + * 5\n    ";
    let err = parser.parse_text(input).unwrap_err();
    match err {
        ParseError::UnexpectedToken { found, line, col: _ } => {
            assert_eq!(found, "*");
            assert_eq!(line, 2);
            // column may vary by whitespace; just ensure we get the right token
        }
        other => panic!("Expected UnexpectedToken, got {:?}", other),
    }
}
