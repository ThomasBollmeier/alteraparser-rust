mod grammar4test;

use alteraparser::ast::AstStrWriter;
use alteraparser::parser::Parser;
use alteraparser::token::{Token, TokenStreamFromList};

fn run_parser(tokens: Vec<Token>) -> alteraparser::ast::Ast {
    let grammar = grammar4test::make_grammar();
    let parser = Parser::new(grammar);
    let mut stream = TokenStreamFromList::new(tokens);
    let ast = parser.parse(&mut stream).expect("parse should succeed");
    let writer = AstStrWriter::default();
    println!("{}", writer.write_ast_to_str(&ast));
    ast
}

// "(3 + 5) * x"
#[test]
fn test_simple_expr() {
    let lg = grammar4test::make_lexer_grammar();
    let tokens = vec![
        Token::new(lg.token_type("LPAREN"),   "(",  1,  1),
        Token::new(lg.token_type("NUMBER"),   "3",  1,  2),
        Token::new(lg.token_type("PLUS"),     "+",  1,  4),
        Token::new(lg.token_type("NUMBER"),   "5",  1,  6),
        Token::new(lg.token_type("RPAREN"),   ")",  1,  7),
        Token::new(lg.token_type("MULTIPLY"), "*",  1,  9),
        Token::new(lg.token_type("IDENT"),    "x",  1, 11),
    ];
    let ast = run_parser(tokens);
    // After transformers: term with two factors
    assert_eq!(ast.name, "term");
}

// "foo(42, bar)"
#[test]
fn test_call_expr() {
    let lg = grammar4test::make_lexer_grammar();
    let tokens = vec![
        Token::new(lg.token_type("IDENT"),  "foo", 1,  1),
        Token::new(lg.token_type("LPAREN"), "(",   1,  4),
        Token::new(lg.token_type("NUMBER"), "42",  1,  5),
        Token::new(lg.token_type("COMMA"),  ",",   1,  7),
        Token::new(lg.token_type("IDENT"),  "bar", 1,  9),
        Token::new(lg.token_type("RPAREN"), ")",   1, 12),
    ];
    let ast = run_parser(tokens);
    assert_eq!(ast.name, "call");
    assert_eq!(ast[0].value.as_deref(), Some("foo")); // callee
    let args = &ast[1]; // arguments
    assert_eq!(args.children().len(), 2);
}
