use crate::error::ParseError;
use crate::lexer_grammar::LexerGrammar;
use crate::token::{Token, TokenStream};

/// Tokenizes a text string according to a [`LexerGrammar`].
///
/// Rules are tried in insertion order; the first match wins.
/// Tokens with `ignore = true` are silently consumed.
/// Raises [`ParseError::UnexpectedChar`] when no rule matches.
pub struct Lexer<'g> {
    grammar: &'g LexerGrammar,
    input: String,
    line: usize,
    column: usize,
}

impl<'g> Lexer<'g> {
    pub fn new(grammar: &'g LexerGrammar, text: &str) -> Self {
        Lexer {
            grammar,
            input: text.to_string(),
            line: 1,
            column: 1,
        }
    }

    pub fn set_input(&mut self, text: &str) {
        self.input = text.to_string();
        self.line = 1;
        self.column = 1;
    }

    /// Advance, returning `Err` on an unexpected character.
    pub fn next_token(&mut self) -> Result<Option<Token>, ParseError> {
        loop {
            if self.input.is_empty() {
                return Ok(None);
            }

            let mut matched = false;
            for rule in self.grammar.rules() {
                if let Ok(Some(m)) = rule.pattern.find(&self.input) {
                    let value = m.as_str().to_string();
                    let tok_line = self.line;
                    let tok_col = self.column;

                    // Update line/column tracking
                    let lines: Vec<&str> = value.split('\n').collect();
                    if lines.len() > 1 {
                        self.line += lines.len() - 1;
                        self.column = lines.last().unwrap().len() + 1;
                    } else {
                        self.column += value.len();
                    }
                    self.input = self.input[value.len()..].to_string();

                    if rule.ignore {
                        matched = true;
                        break; // continue outer loop
                    }
                    return Ok(Some(Token::new(&rule.token_type, value, tok_line, tok_col)));
                }
            }

            if !matched {
                let ch = self.input.chars().next().unwrap();
                return Err(ParseError::UnexpectedChar {
                    ch,
                    line: self.line,
                    col: self.column,
                });
            }
        }
    }
}

impl<'g> TokenStream for Lexer<'g> {
    fn advance(&mut self) -> Option<Token> {
        // Panic on lex error – mirrors Python's SyntaxError behaviour
        self.next_token().expect("Lex error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer_grammar::LexerGrammar;

    fn make_lg() -> LexerGrammar {
        let mut lg = LexerGrammar::new();
        lg.add_rule("WS", r"\s+", true, false);
        lg.add_rule("NUMBER", r"\d+", false, false);
        lg.add_rule("PLUS", r"\+", false, false);
        lg
    }

    #[test]
    fn test_simple_tokenization() {
        let lg = make_lg();
        let mut lexer = Lexer::new(&lg, "3 + 42");
        assert_eq!(lexer.advance().unwrap().token_type, "NUMBER");
        assert_eq!(lexer.advance().unwrap().token_type, "PLUS");
        let t = lexer.advance().unwrap();
        assert_eq!(t.token_type, "NUMBER");
        assert_eq!(t.value, "42");
        assert!(lexer.advance().is_none());
    }

    #[test]
    fn test_line_column_tracking() {
        let lg = make_lg();
        let mut lexer = Lexer::new(&lg, "3\n+");
        let t1 = lexer.next_token().unwrap().unwrap();
        assert_eq!(t1.line, 1);
        assert_eq!(t1.column, 1);
        let t2 = lexer.next_token().unwrap().unwrap();
        assert_eq!(t2.line, 2);
        assert_eq!(t2.column, 1);
    }

    #[test]
    fn test_unexpected_char_error() {
        let lg = make_lg();
        let mut lexer = Lexer::new(&lg, "?");
        let result = lexer.next_token();
        assert!(matches!(result, Err(ParseError::UnexpectedChar { ch: '?', .. })));
    }

    #[test]
    fn test_all_ignored() {
        let mut lg = LexerGrammar::new();
        lg.add_rule("WS", r"\s+", true, false);
        lg.add_rule("COMMENT", r"//.*", true, false);
        let mut lexer = Lexer::new(&lg, "// a comment\n   // another");
        assert!(lexer.next_token().unwrap().is_none());
    }
}
