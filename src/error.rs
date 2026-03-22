use thiserror::Error;

/// Errors produced during lexing or parsing.
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    /// A character in the input text did not match any lexer rule.
    #[error("Unexpected character '{ch}' at line {line}, column {col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },

    /// A token in the token stream did not match the grammar at this point.
    #[error("Unexpected token '{found}' at line {line}, column {col}")]
    UnexpectedToken {
        found: String,
        line: usize,
        col: usize,
    },

    /// The grammar matched more than one valid parse path (ambiguous grammar).
    #[error("Ambiguous parse: multiple valid paths found")]
    AmbiguousGrammar,

    /// All tokens were consumed but no valid parse path reached the grammar end.
    #[error("Incomplete parse: cannot reach grammar end")]
    IncompleteParse,
}
