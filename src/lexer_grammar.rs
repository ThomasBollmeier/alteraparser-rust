use fancy_regex::Regex;

/// A single tokenization rule: a token-type name, a compiled regex, and whether
/// matched tokens should be discarded (ignored).
pub struct LexerRule {
    pub token_type: String,
    pub pattern: Regex,
    pub ignore: bool,
}

/// Defines tokenization rules for the [`crate::lexer::Lexer`].
///
/// Rules are matched in insertion order; the first match wins.
pub struct LexerGrammar {
    rules: Vec<LexerRule>,
}

impl LexerGrammar {
    pub fn new() -> Self {
        LexerGrammar { rules: Vec::new() }
    }

    /// Register a tokenization rule.
    ///
    /// - `token_type`       – name used to classify matching text (e.g. `"NUMBER"`)
    /// - `pattern`          – regex pattern; `^` is prepended automatically
    /// - `ignore`           – when `true` matched tokens are silently discarded
    /// - `case_insensitive` – when `true` the regex is compiled case-insensitively
    pub fn add_rule(
        &mut self,
        token_type: impl Into<String>,
        pattern: &str,
        ignore: bool,
        case_insensitive: bool,
    ) -> &mut Self {
        let anchored = if pattern.starts_with('^') {
            pattern.to_string()
        } else {
            format!("^{pattern}")
        };
        let regex = if case_insensitive {
            Regex::new(&format!("(?i){anchored}")).expect("Invalid regex pattern")
        } else {
            Regex::new(&anchored).expect("Invalid regex pattern")
        };
        self.rules.push(LexerRule {
            token_type: token_type.into(),
            pattern: regex,
            ignore,
        });
        self
    }

    pub fn rules(&self) -> &[LexerRule] {
        &self.rules
    }

    /// Convenience: returns the token-type string (identity function, useful for
    /// readability when building grammars: `lg.token_type("NUMBER")`).
    pub fn token_type<'a>(&self, name: &'a str) -> &'a str {
        for rule in &self.rules {
            if rule.token_type == name {
                return name;
            }
        }
        panic!("Unrecognized token type: {}", name);
    }
}

impl Default for LexerGrammar {
    fn default() -> Self {
        LexerGrammar::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_rule_anchors_pattern() {
        let mut lg = LexerGrammar::new();
        lg.add_rule("NUM", r"\d+", false, false);
        let r = &lg.rules()[0];
        assert_eq!(r.token_type, "NUM");
        assert!(r.pattern.is_match("123abc").unwrap());
        assert!(!r.pattern.is_match("abc123").unwrap());
    }

    #[test]
    fn test_ignore_rule() {
        let mut lg = LexerGrammar::new();
        lg.add_rule("WS", r"\s+", true, false);
        assert!(lg.rules()[0].ignore);
    }

    #[test]
    fn test_case_insensitive() {
        let mut lg = LexerGrammar::new();
        lg.add_rule("KW", "if", false, true);
        assert!(lg.rules()[0].pattern.is_match("IF").unwrap());
        assert!(lg.rules()[0].pattern.is_match("If").unwrap());
    }
}
