use std::rc::Rc;

use crate::ast::Ast;
use crate::error::ParseError;
use crate::grammar::{find_epsilon_path, find_follow_tokens, Grammar, GrammarNodeKind, NodeRef};
use crate::lexer::Lexer;
use crate::lexer_grammar::LexerGrammar;
use crate::token::{Token, TokenStream};

/// Parses a [`TokenStream`] according to a [`Grammar`] and produces an [`Ast`].
///
/// The algorithm maintains **all** valid parse paths in parallel (breadth-first)
/// so it can detect ambiguous grammars.  It raises an error for:
/// - an unexpected token (no matching continuation)
/// - an ambiguous parse (multiple valid paths after consuming all tokens)
/// - an incomplete parse (no path reaches the grammar end)
pub struct Parser {
    grammar: Grammar,
}

impl Parser {
    pub fn new(grammar: Grammar) -> Self {
        let mut p = Parser { grammar };
        p.grammar.compile();
        p
    }

    /// Parse all tokens from `stream` and return the root [`Ast`] node.
    pub fn parse(&self, stream: &mut dyn TokenStream) -> Result<Ast, ParseError> {
        self._parse(self.grammar.start_node(), self.grammar.end_node(), stream)
    }

    // Parse a rule
    pub fn parse_rule(&self, rule_name: &str, stream: &mut dyn TokenStream) -> Result<Ast, ParseError> {
        let rule_element = self.grammar.get_rule_element(rule_name)?;
        self._parse(rule_element.in_node(), rule_element.out_node(), stream)
    }

    fn _parse(&self, start: NodeRef, end: NodeRef, stream: &mut dyn TokenStream) -> Result<Ast, ParseError> {
        let mut active_paths: Vec<Vec<NodeRef>> = vec![vec![start]];
        let mut consumed_tokens: Vec<Token> = Vec::new();

        loop {
            let token = stream.advance();
            match token {
                None => break,
                Some(t) => {
                    consumed_tokens.push(t.clone());
                    let mut next_paths: Vec<Vec<NodeRef>> = Vec::new();

                    for path in &active_paths {
                        let curr = path.last().unwrap();
                        for (token_type, follow_path) in find_follow_tokens(curr) {
                            if token_type == t.token_type {
                                let mut new_path = path.clone();
                                // Avoid duplicating the last node
                                let first_new = &follow_path[0];
                                if !Rc::ptr_eq(new_path.last().unwrap(), first_new) {
                                    new_path.push(Rc::clone(first_new));
                                }
                                for node in &follow_path[1..] {
                                    new_path.push(Rc::clone(node));
                                }
                                next_paths.push(new_path);
                            }
                        }
                    }

                    if next_paths.is_empty() {
                        return Err(ParseError::UnexpectedToken {
                            found: t.value.clone(),
                            line: t.line,
                            col: t.column,
                        });
                    }
                    active_paths = next_paths;
                }
            }
        }

        // Filter paths that can reach the grammar end via epsilon transitions
        let end_ptr = Rc::as_ptr(&end);
        let mut valid_paths: Vec<Vec<NodeRef>> = Vec::new();
        for path in &active_paths {
            let last = path.last().unwrap();
            let pred = |n: &NodeRef| Rc::as_ptr(n) == end_ptr;
            if let Some(tail) = find_epsilon_path(last, &pred) {
                let mut full_path = path.clone();
                if !tail.is_empty() {
                    // tail[0] is last itself (already in path)
                    for node in &tail[1..] {
                        full_path.push(Rc::clone(node));
                    }
                }
                valid_paths.push(full_path);
            }
        }

        if valid_paths.len() > 1 {
            return Err(ParseError::AmbiguousGrammar);
        }
        if valid_paths.is_empty() {
            return Err(ParseError::IncompleteParse);
        }

        Ok(build_ast(&valid_paths[0], &consumed_tokens, &self.grammar)
            .expect("AST construction failed"))
    }

    fn _path_to_string(path: &[NodeRef]) -> String {
        let mut parts: Vec<String> = Vec::new();
        for node in path {
            match &node.kind {
                GrammarNodeKind::RuleStart(name) => parts.push(format!("RuleStart({})", name)),
                GrammarNodeKind::RuleEnd(name) => parts.push(format!("RuleEnd({})", name)),
                GrammarNodeKind::Token(tok) => parts.push(format!("Token({})", tok)),
                GrammarNodeKind::Normal => {}
            }
        }
        parts.join(" -> ")
    }
}

/// Convenience wrapper that tokenizes text and then parses it.
pub struct TextParser {
    parser: Parser,
    lexer_grammar: LexerGrammar,
}

impl TextParser {
    pub fn new(grammar: Grammar, lexer_grammar: LexerGrammar) -> Self {
        TextParser {
            parser: Parser::new(grammar),
            lexer_grammar,
        }
    }

    pub fn parse(&self, text: &str) -> Result<Ast, ParseError> {
        let mut lexer = Lexer::new(&self.lexer_grammar, text);
        self.parser.parse(&mut lexer)
    }

    pub fn parse_rule(&self, rule_name: &str, text: &str) -> Result<Ast, ParseError> {
        let mut lexer = Lexer::new(&self.lexer_grammar, text);
        self.parser.parse_rule(rule_name, &mut lexer)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AST construction
// ─────────────────────────────────────────────────────────────────────────────

fn build_ast(path: &[NodeRef], tokens: &[Token], grammar: &Grammar) -> Option<Ast> {
    let mut stack: Vec<Ast> = Vec::new();
    let mut result: Option<Ast> = None;
    let mut token_idx = 0usize;

    for node in path {
        match &node.kind {
            GrammarNodeKind::RuleStart(name) => {
                let mut ast_node = Ast::new(name.as_str());
                ast_node.id = node.id.clone();
                stack.push(ast_node);
            }
            GrammarNodeKind::RuleEnd(name) => {
                let mut completed = stack.pop().unwrap();
                if let Some(transformer) = grammar.get_ast_transformer(name.as_str()) {
                    let saved_id = completed.id.clone();
                    completed = transformer(completed);
                    completed.id = saved_id;
                }
                if let Some(parent) = stack.last_mut() {
                    parent.add_child(completed);
                } else {
                    result = Some(completed);
                }
            }
            GrammarNodeKind::Token(_) => {
                let token = &tokens[token_idx];
                token_idx += 1;
                let mut tok_ast = Ast::with_value(&token.token_type, &token.value);
                tok_ast.id = node.id.clone();
                if let Some(parent) = stack.last_mut() {
                    parent.add_child(tok_ast);
                }
            }
            GrammarNodeKind::Normal => {}
        }
    }

    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::{seq, tok};
    use crate::token::TokenStreamFromList;

    fn single_token_grammar() -> Grammar {
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| tok("NUM"));
        g
    }

    #[test]
    fn test_parse_single_token() {
        let g = single_token_grammar();
        let parser = Parser::new(g);
        let mut stream = TokenStreamFromList::new(vec![Token::new("NUM", "42", 1, 1)]);
        let ast = parser.parse(&mut stream).unwrap();
        assert_eq!(ast.name, "expr");
        assert_eq!(ast.children()[0].value.as_deref(), Some("42"));
    }

    #[test]
    fn test_unexpected_token_error() {
        let g = single_token_grammar();
        let parser = Parser::new(g);
        let mut stream = TokenStreamFromList::new(vec![Token::new("IDENT", "foo", 1, 1)]);
        let err = parser.parse(&mut stream).unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn test_incomplete_parse_error() {
        let mut g = Grammar::new();
        // seq([NUM, PLUS, NUM]) but we only provide NUM
        g.add_rule("expr", true, |_| {
            seq(vec![tok("NUM"), tok("PLUS"), tok("NUM")])
        });
        let parser = Parser::new(g);
        let mut stream = TokenStreamFromList::new(vec![Token::new("NUM", "1", 1, 1)]);
        let err = parser.parse(&mut stream).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteParse));
    }

    #[test]
    fn test_parse_sequence() {
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| {
            seq(vec![tok("NUM"), tok("PLUS"), tok("NUM")])
        });
        let parser = Parser::new(g);
        let tokens = vec![
            Token::new("NUM", "1", 1, 1),
            Token::new("PLUS", "+", 1, 2),
            Token::new("NUM", "2", 1, 3),
        ];
        let mut stream = TokenStreamFromList::new(tokens);
        let ast = parser.parse(&mut stream).unwrap();
        assert_eq!(ast.children().len(), 3);
    }
}
