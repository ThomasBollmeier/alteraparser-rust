use crate::ast::Ast;
use crate::grammar::{choice, many, opt, seq, tok, tok_id, Grammar};
use std::collections::HashSet;

pub fn make_grammar() -> Grammar {
    let mut grammar = Grammar::new();

    grammar.add_rule("grammar", true, |g| many(g.rule_ref("rule_or_macro")));

    grammar.add_rule("rule_or_macro", false, |g| {
        choice(vec![g.rule_ref("rule"), g.rule_ref("macro")])
    });

    grammar.add_ast_transformer("rule_or_macro", |ast| ast.children()[0].clone());

    grammar.add_rule("rule", false, |g| {
        seq(vec![
            opt(tok_id("START_ANNOTATION", "start_annotation")),
            tok_id("IDENT", "rule_name"),
            tok("RARROW"),
            g.rule_ref_id("alternatives", "alt"),
            tok("SEMICOLON"),
        ])
    });

    grammar.add_ast_transformer("rule", |ast| {
        let is_start_rule = ast.child_by_id("start_annotation", true).is_some();
        let rule_name = ast
            .child_by_id("rule_name", true)
            .unwrap()
            .value
            .clone()
            .unwrap();

        let mut ret = if !is_start_rule {
            Ast::new("rule")
        } else {
            Ast::new("start_rule")
        };
        ret.add_child(Ast::with_value("name", rule_name));
        ret.add_child(ast.child_by_id("alt", true).unwrap().clone());

        ret
    });

    grammar.add_rule("macro", false, |g| {
        seq(vec![
            tok_id("IDENT", "macro_name"),
            g.rule_ref_id("parameters", "params"),
            tok("RARROW"),
            g.rule_ref_id("alternatives", "alt"),
            tok("SEMICOLON"),
        ])
    });

    grammar.add_ast_transformer("macro", |ast| {
        let macro_name = ast
            .child_by_id("macro_name", true)
            .unwrap()
            .value
            .clone()
            .unwrap();

        let mut ret = Ast::new("macro");
        ret.add_child(Ast::with_value("name", macro_name));

        let params = ast.child_by_id("params", true).unwrap().clone();
        let param_names: Vec<String> = params
            .children()
            .iter()
            .map(|param| param.value.clone().unwrap())
            .collect();
        let param_names: HashSet<String> = HashSet::from_iter(param_names.into_iter());

        let mut body = ast.child_by_id("alt", true).unwrap().clone();
        replace_rule_refs_with_params(&mut body, &param_names);

        ret.add_child(params);
        ret.add_child(body);

        ret
    });

    fn replace_rule_refs_with_params(ast: &mut Ast, param_names: &HashSet<String>) {
        if ast.name == "rule_ref" {
            let name = ast.value.clone().unwrap();
            if param_names.contains(&name) {
                ast.name = "param_ref".to_string();
            }
        }

        for child in ast.children_mut() {
            replace_rule_refs_with_params(child, param_names);
        }
    }

    grammar.add_rule("parameters", false, |_| {
        seq(vec![
            tok("LBRACKET"),
            opt(seq(vec![
                tok_id("IDENT", "param"),
                many(seq(vec![tok("COMMA"), tok_id("IDENT", "param")])),
            ])),
            tok("RBRACKET"),
        ])
    });

    grammar.add_ast_transformer("parameters", |ast| {
        let mut ret = Ast::new("parameters");
        for param in ast.children_by_id("param", true) {
            let param_name = param.value.clone().unwrap();
            ret.add_child(Ast::with_value("param", param_name));
        }

        ret
    });

    grammar.add_rule("alternatives", false, |g| {
        seq(vec![
            g.rule_ref_id("alternative", "alt"),
            many(seq(vec![tok("PIPE"), g.rule_ref_id("alternative", "alt")])),
        ])
    });

    grammar.add_ast_transformer("alternatives", |ast| {
        let alternatives = ast.children_by_id("alt", true);
        if alternatives.len() == 1 {
            return alternatives[0].clone();
        }
        let mut ret = Ast::new("choice");
        for alternative in alternatives {
            ret.add_child(alternative);
        }

        ret
    });

    grammar.add_rule("alternative", false, |g| many(g.rule_ref("item")));

    grammar.add_ast_transformer("alternative", |ast| {
        if ast.children().len() == 1 {
            return ast[0].clone();
        }
        let mut ret = Ast::new("seq");
        for child in ast.children() {
            ret.add_child(child.clone());
        }

        ret
    });

    grammar.add_rule("item", false, |g| {
        seq(vec![
            g.rule_ref_id("atom", "atom"),
            opt(choice(vec![
                tok_id("QUESTION_MARK", "mult"),
                tok_id("ASTERISK", "mult"),
                tok_id("PLUS", "mult"),
            ])),
        ])
    });

    grammar.add_ast_transformer("item", |ast| {
        let multi_opt = ast.child_by_id("mult", true);
        let atom = ast.child_by_id("atom", true).unwrap();

        if let Some(multi) = multi_opt {
            match multi.name.as_str() {
                "QUESTION_MARK" => {
                    let mut node = Ast::new("opt");
                    node.add_child(atom);
                    node
                }
                "ASTERISK" => {
                    let mut node = Ast::new("many");
                    node.add_child(atom);
                    node
                }
                "PLUS" => {
                    let mut node = Ast::new("one_or_more");
                    node.add_child(atom);
                    node
                }
                _ => panic!("unexpected multiplier"),
            }
        } else {
            atom
        }
    });

    grammar.add_rule("atom", false, |g| {
        choice(vec![
            seq(vec![
                opt(seq(vec![tok_id("IDENT", "id"), tok("HASH")])),
                choice(vec![tok_id("IDENT", "rule_ref"), tok_id("TOKEN", "token")]),
            ]),
            g.rule_ref_id("group", "grp"),
            g.rule_ref_id("macro_call", "mac_call"),
        ])
    });

    grammar.add_ast_transformer("atom", |ast| {
        let token_opt = ast.child_by_id("token", true);
        let rule_ref_opt = ast.child_by_id("rule_ref", true);
        let group_opt = ast.child_by_id("grp", true);
        let macro_call_opt = ast.child_by_id("mac_call", true);

        let mut new_ast = if let Some(token) = token_opt {
            Ast::with_value("token", token.value.clone().unwrap())
        } else if let Some(rule_ref) = rule_ref_opt {
            Ast::with_value("rule_ref", rule_ref.value.clone().unwrap())
        } else if let Some(group) = group_opt {
            group.clone()
        } else if let Some(mac_call) = macro_call_opt {
            mac_call.clone()
        } else {
            ast.children()[0].clone()
        };

        let id_opt = ast.child_by_id("id", true);
        if let Some(id) = id_opt {
            new_ast.add_child(Ast::with_value("id", id.value.clone().unwrap()));
        }

        new_ast
    });

    grammar.add_rule("group", false, |g| {
        seq(vec![
            tok("LPAREN"),
            g.rule_ref("alternatives"),
            tok("RPAREN"),
        ])
    });

    grammar.add_ast_transformer("group", |ast| {
        // group -> LPAREN alternatives RPAREN
        // return the alternatives child, skipping the parens
        ast[1].clone()
    });

    grammar.add_rule("macro_call", false, |g| {
        seq(vec![
            tok_id("IDENT", "macro_name"),
            tok("LBRACKET"),
            opt(seq(vec![
                g.rule_ref_id("alternatives", "arg"),
                many(seq(vec![
                    tok("COMMA"),
                    g.rule_ref_id("alternatives", "arg"),
                ])),
            ])),
            tok("RBRACKET"),
        ])
    });

    grammar.add_ast_transformer("macro_call", |ast| {
        let mut ret = Ast::new("macro_call");
        let macro_name = ast
            .child_by_id("macro_name", true)
            .unwrap()
            .value
            .clone()
            .unwrap();
        ret.add_child(Ast::with_value("name", macro_name));

        let mut args = Ast::new("arguments");
        for arg in ast.children_by_id("arg", true) {
            args.add_child(arg.clone());
        }
        ret.add_child(args);

        ret
    });

    grammar
}

#[cfg(test)]
mod test {
    use crate::ast::AstStrWriter;
    use crate::lexer::Lexer;
    use crate::meta::grammar::make_grammar;
    use crate::meta::lexer_grammar::make_lexer_grammar;
    use crate::parser::Parser;

    #[test]
    fn test_grammar() {
        let grammar_code = r#"
        -- Expression grammar
        @start
        sum -> term ((op#ADD | op#SUB) term)*;

        term -> product ((op#MUL | op#DIV) product)*;

        product -> NUMBER | IDENTIFIER | group;

        group -> LPAREN sum RPAREN;
        "#;

        run(grammar_code);
    }

    #[test]
    fn test_grammar_with_macro() {
        let grammar_code = r#"
        @start
        program -> definition*;
        definition -> enclosed<DEF IDENTIFIER expr>;
        expr -> NUMBER;
        enclosed<content> ->
            LPAREN content RPAREN |
            LBRACE content RBRACE |
            LBRACKET content RBRACKET;
        "#;

        run(grammar_code);
    }

    fn run(grammar_code: &str) {
        let lexer_grammar = make_lexer_grammar();
        let mut lexer = Lexer::new(&lexer_grammar, grammar_code);

        let grammar = make_grammar();
        let parser = Parser::new(grammar);

        let ast = parser.parse(&mut lexer).unwrap();
        let writer = AstStrWriter::new(2);

        println!("{}", writer.write_ast_to_str(&ast));
    }
}
