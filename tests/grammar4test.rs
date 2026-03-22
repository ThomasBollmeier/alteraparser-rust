//! Shared expression grammar used by all integration tests.
//!
//! Mirrors `tests/grammar4test.py` from the Python project exactly.
//!
//! Grammar (arithmetic + function calls):
//! ```text
//! sum    = term (('+' | '-') term)*         [start]
//! term   = factor (('*' | '/') factor)*
//! factor = NUMBER | IDENT | group | call
//! group  = '(' sum ')'
//! call   = IDENT '(' [sum (',' sum)*] ')'
//! ```

use alteraparser::ast::Ast;
use alteraparser::grammar::{choice, many, opt, seq, tok, tok_id, with_id, Grammar};
use alteraparser::lexer_grammar::LexerGrammar;

pub fn make_lexer_grammar() -> LexerGrammar {
    let mut lg = LexerGrammar::new();
    lg.add_rule("WHITESPACE", r"\s+", true,  false);
    lg.add_rule("COMMENT",    r"//.*", true, false);
    lg.add_rule("NUMBER",     r"\d+",  false, false);
    lg.add_rule("IDENT",      r"[a-zA-Z_][a-zA-Z0-9_]*", false, false);
    lg.add_rule("PLUS",       r"\+",   false, false);
    lg.add_rule("MINUS",      r"-",    false, false);
    lg.add_rule("MULTIPLY",   r"\*",   false, false);
    lg.add_rule("DIVIDE",     r"/",    false, false);
    lg.add_rule("LPAREN",     r"\(",   false, false);
    lg.add_rule("RPAREN",     r"\)",   false, false);
    lg.add_rule("COMMA",      r",",    false, false);
    lg
}

/// Collapse single-child rule nodes (mirrors `_trans_single_child` in Python).
fn trans_single_child(ast: Ast) -> Ast {
    if ast.children().len() == 1 {
        ast[0].clone()
    } else {
        ast
    }
}

pub fn make_grammar() -> Grammar {
    let mut g = Grammar::new();

    // sum = term (('+' | '-') term)*
    g.add_rule("sum", true, |h| {
        seq(vec![
            h.rule_ref("term"),
            many(seq(vec![
                choice(vec![tok("PLUS"), tok("MINUS")]),
                h.rule_ref("term"),
            ])),
        ])
    });

    // term = factor (('*' | '/') factor)*
    g.add_rule("term", false, |h| {
        seq(vec![
            h.rule_ref("factor"),
            many(seq(vec![
                choice(vec![tok("MULTIPLY"), tok("DIVIDE")]),
                h.rule_ref("factor"),
            ])),
        ])
    });

    // factor = NUMBER | IDENT | group | call
    g.add_rule("factor", false, |h| {
        choice(vec![
            tok("NUMBER"),
            tok("IDENT"),
            h.rule_ref("group"),
            h.rule_ref("call"),
        ])
    });

    // group = '(' sum ')'
    g.add_rule("group", false, |h| {
        seq(vec![tok("LPAREN"), h.rule_ref("sum"), tok("RPAREN")])
    });

    // group transformer: return the middle child (the sum), skip the parens
    g.add_ast_transformer("group", |ast| ast[1].clone());

    // call = IDENT '(' [sum (',' sum)*] ')'
    g.add_rule("call", false, |h| {
        let rest_args = many(seq(vec![tok("COMMA"), with_id(h.rule_ref("sum"), "arg")]));
        let args = opt(seq(vec![with_id(h.rule_ref("sum"), "arg"), rest_args]));
        seq(vec![
            tok_id("IDENT", "callee"),
            tok("LPAREN"),
            args,
            tok("RPAREN"),
        ])
    });

    // call transformer: restructure to { callee, arguments[] }
    g.add_ast_transformer("call", |ast| {
        let mut call_node = Ast::new("call");

        let callee_value = ast
            .child_by_id("callee", true)
            .expect("callee missing")
            .value
            .unwrap_or_default();
        call_node.add_child(Ast::with_value("callee", callee_value));

        let mut args_node = Ast::new("arguments");
        for arg in ast.children_by_id("arg", true) {
            args_node.add_child(arg);
        }
        call_node.add_child(args_node);

        call_node
    });

    g.add_ast_transformer("sum",    trans_single_child);
    g.add_ast_transformer("term",   trans_single_child);
    g.add_ast_transformer("factor", trans_single_child);

    g
}
