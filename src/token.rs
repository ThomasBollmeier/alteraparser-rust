/// A single lexical token produced by the [`crate::lexer::Lexer`].
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token classification (e.g. `"NUMBER"`, `"LPAREN"`).
    pub token_type: String,
    /// The matched text.
    pub value: String,
    /// 1-based line number in the source.
    pub line: usize,
    /// 1-based column number in the source.
    pub column: usize,
}

impl Token {
    pub fn new(token_type: impl Into<String>, value: impl Into<String>, line: usize, column: usize) -> Self {
        Token {
            token_type: token_type.into(),
            value: value.into(),
            line,
            column,
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token({}, {}, {}, {})", self.token_type, self.value, self.line, self.column)
    }
}

/// A source of [`Token`]s.  Returns `None` when exhausted.
pub trait TokenStream {
    fn advance(&mut self) -> Option<Token>;
}

/// A [`TokenStream`] backed by an in-memory list – useful for testing.
pub struct TokenStreamFromList {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenStreamFromList {
    pub fn new(tokens: Vec<Token>) -> Self {
        TokenStreamFromList { tokens, pos: 0 }
    }
}

impl TokenStream for TokenStreamFromList {
    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_stream_from_list() {
        let tokens = vec![
            Token::new("A", "a", 1, 1),
            Token::new("B", "b", 1, 2),
        ];
        let mut stream = TokenStreamFromList::new(tokens);
        let t = stream.advance().unwrap();
        assert_eq!(t.token_type, "A");
        assert_eq!(t.value, "a");
        let t = stream.advance().unwrap();
        assert_eq!(t.token_type, "B");
        assert!(stream.advance().is_none());
        assert!(stream.advance().is_none()); // exhausted
    }
}
