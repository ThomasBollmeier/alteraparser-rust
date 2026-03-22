mod grammar4test;

use alteraparser::lexer::Lexer;
use alteraparser::token::TokenStream;

#[test]
fn test_lexer_simple_expr() {
    let lg = grammar4test::make_lexer_grammar();
    let mut lexer = Lexer::new(&lg, "(3 + 5) * x");

    let expected: &[(&str, &str, usize, usize)] = &[
        ("LPAREN",   "(",  1, 1),
        ("NUMBER",   "3",  1, 2),
        ("PLUS",     "+",  1, 4),
        ("NUMBER",   "5",  1, 6),
        ("RPAREN",   ")",  1, 7),
        ("MULTIPLY", "*",  1, 9),
        ("IDENT",    "x",  1, 11),
    ];

    for &(tok_type, value, line, col) in expected {
        let t = lexer.advance().expect("expected token");
        assert_eq!(t.token_type, tok_type, "token_type mismatch");
        assert_eq!(t.value,      value,    "value mismatch");
        assert_eq!(t.line,       line,     "line mismatch");
        assert_eq!(t.column,     col,      "column mismatch");
    }

    assert!(lexer.advance().is_none(), "stream should be exhausted");
}

#[test]
fn test_lexer_all_commented() {
    let lg = grammar4test::make_lexer_grammar();
    let mut lexer = Lexer::new(&lg, "// This is a comment\n   // Another comment");
    assert!(lexer.advance().is_none());
}
