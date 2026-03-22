//! Alteraparser – a parser-combinator library.
//!
//! Pipeline: text → [`Lexer`] → token stream → [`Parser`] → [`Ast`]
//!
//! Or end-to-end with [`TextParser`] which combines both steps.
//!
//! # Quick start
//! ```rust,ignore
//! use alteraparser::prelude::*;
//!
//! let mut lg = LexerGrammar::new();
//! lg.add_rule("NUMBER", r"\d+", false, false);
//! lg.add_rule("PLUS",   r"\+",  false, false);
//! lg.add_rule("WS",     r"\s+", true,  false);
//!
//! let mut grammar = Grammar::new();
//! grammar.add_rule("expr", true, |g| {
//!     seq(vec![g.rule_ref("expr"), tok("PLUS"), g.rule_ref("expr")])
//! });
//!
//! let mut parser = TextParser::new(grammar, lg);
//! let ast = parser.parse_text("1 + 2").unwrap();
//! ```

pub mod ast;
pub mod error;
pub mod grammar;
pub mod lexer;
pub mod lexer_grammar;
pub mod parser;
pub mod token;

pub mod prelude {
    pub use crate::ast::{Ast, AstStrWriter};
    pub use crate::error::ParseError;
    pub use crate::grammar::{choice, many, one_or_more, opt, seq, tok, Grammar};
    pub use crate::lexer::Lexer;
    pub use crate::lexer_grammar::LexerGrammar;
    pub use crate::parser::{Parser, TextParser};
    pub use crate::token::{Token, TokenStream, TokenStreamFromList};
}
