use crate::lexer_grammar::LexerGrammar;

pub fn make_lexer_grammar() -> LexerGrammar {
    let mut lg = LexerGrammar::new();
    lg.add_rule("WHITESPACE", r"\s+", true, false);
    lg.add_rule("COMMENT", r"\-\-.*", true, false);
    lg.add_rule("QUESTION_MARK", r"\?", false, false);
    lg.add_rule("ASTERISK", r"\*", false, false);
    lg.add_rule("PLUS", r"\+", false, false);
    lg.add_rule("LPAREN", r"\(", false, false);
    lg.add_rule("RPAREN", r"\)", false, false);
    lg.add_rule("LBRACKET", r"<", false, false);
    lg.add_rule("RBRACKET", r">", false, false);
    lg.add_rule("PIPE", r"\|", false, false);
    lg.add_rule("HASH", r"#", false, false);
    lg.add_rule("SEMICOLON", r";", false, false);
    lg.add_rule("COMMA", r",", false, false);
    lg.add_rule("RARROW", r"\->", false, false);
    lg.add_rule("IDENT", r"[a-z][a-z0-9_]*", false, false);
    lg.add_rule("TOKEN", r"[A-Z][A-Z0-9_]*", false, false);
    lg.add_rule("NUMBER", r"\-\-", false, false);
    lg.add_rule("START_ANNOTATION", r"@start", false, false);
    lg
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::meta::lexer_grammar::make_lexer_grammar;

    #[test]
    fn scan_grammar_def() {
        let grammar_code = r#"
        -- Expression grammar
        @start
        sum -> term ((op#ADD | op#SUB) term)*;

        term -> product ((op#MUL | op#DIV) product)*;

        product -> NUMBER | IDENTIFIER | group;

        group -> LPAREN sum RPAREN;
        "#;

        let lexer_grammar = make_lexer_grammar();
        let mut lexer = Lexer::new(&lexer_grammar, grammar_code);

        loop {
            match lexer.next_token() {
                Ok(Some(token)) => println!("{:?}", token),
                Ok(None) => break, // End of input
                Err(e) => panic!("Lex error: {:?}", e),
            }
        }
    }
}
