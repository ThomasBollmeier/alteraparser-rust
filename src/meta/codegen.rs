use crate::ast::Ast;
use crate::error::ParseError;
use crate::lexer::Lexer;
use crate::meta::grammar::make_grammar;
use crate::meta::lexer_grammar::make_lexer_grammar;
use crate::parser::Parser;

pub struct CodeGeneratorBuilder {
    indent_size: usize,
    top_comment: String,
    function_name: String,
}

impl CodeGeneratorBuilder {
    pub fn new() -> Self {
        Self {
            indent_size: 2,
            top_comment: "".to_string(),
            function_name: "make_grammar".to_string(),
        }
    }

    pub fn indent_size(&mut self, size: usize) -> &mut Self {
        self.indent_size = size;
        self
    }

    pub fn top_comment(&mut self, comment: &str) -> &mut Self {
        self.top_comment = comment.to_string();
        self
    }

    pub fn function_name(&mut self, name: &str) -> &mut Self {
        self.function_name = name.to_string();
        self
    }

    pub fn build(&self) -> CodeGenerator {
        CodeGenerator::new(
            self.indent_size,
            self.top_comment.clone(),
            self.function_name.clone(),
        )
    }
}

pub struct CodeGenerator {
    indent_size: usize,
    top_comment: String,
    function_name: String,
}

impl CodeGenerator {
    pub fn new(indent_size: usize, top_comment: String, function_name: String) -> Self {
        Self {
            indent_size,
            top_comment,
            function_name,
        }
    }

    pub fn generate_code(&self, grammar_definition: &str) -> Result<String, ParseError> {
        let lexer_grammar = make_lexer_grammar();
        let mut lexer = Lexer::new(&lexer_grammar, grammar_definition);

        let grammar = make_grammar();
        let parser = Parser::new(grammar);

        let ast = parser.parse(&mut lexer)?;

        let mut lines = Lines::new(self.indent_size);

        self.generate_grammar(&ast, &mut lines);

        Ok(lines.get_content())
    }

    fn generate_grammar(&self, grammar: &Ast, lines: &mut Lines) {
        if !self.top_comment.is_empty() {
            lines.writeln(&format!("// {}", self.top_comment));
            lines.writeln("");
        }
        lines.writeln("#![allow(unused)]");
        lines.writeln("use alteraparser::prelude::*;");
        lines.writeln("");
        lines.writeln(&format!("pub fn {}() -> Grammar {{", self.function_name));
        lines.indent();
        lines.writeln("let mut grammar = Grammar::new();");
        lines.writeln("");

        for rule in grammar.children() {
            self.generate_rule(rule, lines);
        }

        lines.writeln("grammar");
        lines.dedent();
        lines.writeln("}");
    }

    fn generate_rule(&self, rule: &Ast, lines: &mut Lines) {
        let is_start_rule = rule.name == "start_rule";
        let children = rule.children();
        let rule_name = children.get(0).unwrap().value.as_ref().unwrap();
        let body = children.get(1).unwrap();

        lines.writeln(&format!(
            "grammar.add_rule(\"{}\", {}, |g| {{",
            rule_name, is_start_rule
        ));

        lines.indent();
        self.generate_body(body, lines, true);
        lines.dedent();

        lines.writeln("});");
        lines.writeln("");
    }

    fn generate_body(&self, body: &Ast, lines: &mut Lines, end_with_line_break: bool) {
        let name = body.name.as_str();
        match name {
            "seq" | "choice" => self.generate_seq_or_choice(body, lines, end_with_line_break),
            "opt" | "one_or_more" | "many" => self.generate_mult(body, lines, end_with_line_break),
            "rule_ref" => self.generate_simple("g.rule_ref", body, lines, end_with_line_break),
            "token" => self.generate_simple("tok", body, lines, end_with_line_break),
            _ => {}
        }
    }

    fn generate_simple(
        &self,
        name: &str,
        simple: &Ast,
        lines: &mut Lines,
        end_with_line_break: bool,
    ) {
        let value = simple.value.as_ref().unwrap();
        let id_nodes = simple.children_by_name("id");
        let id_node_opt = id_nodes.get(0);

        let text = match id_node_opt {
            Some(id) => {
                let id = id.value.as_ref().unwrap();
                &format!("{}_id(\"{}\", \"{}\")", name, value, id)
            }
            None => &format!("{}(\"{}\")", name, value),
        };

        if end_with_line_break {
            lines.writeln(text);
        } else {
            lines.write(text);
        }
    }

    fn generate_mult(&self, mult: &Ast, lines: &mut Lines, end_with_line_break: bool) {
        lines.writeln(&format!("{}(", mult.name));
        lines.indent();
        self.generate_body(mult.children().get(0).unwrap(), lines, true);
        lines.dedent();
        if end_with_line_break {
            lines.writeln(")");
        } else {
            lines.write(")");
        }
    }

    fn generate_seq_or_choice(
        &self,
        seq_or_choice: &Ast,
        lines: &mut Lines,
        end_with_line_break: bool,
    ) {
        lines.writeln(&format!("{}(vec![", seq_or_choice.name));
        lines.indent();

        for child in seq_or_choice.children() {
            self.generate_body(child, lines, false);
            lines.writeln(",");
        }

        lines.dedent();

        if end_with_line_break {
            lines.writeln("])");
        } else {
            lines.write("])");
        }
    }
}

struct Lines {
    lines: Vec<String>,
    indent_level: usize,
    indent_size: usize,
    current_line: String,
}

impl Lines {
    pub fn new(indent_size: usize) -> Lines {
        Lines {
            lines: vec![],
            indent_level: 0,
            indent_size,
            current_line: String::new(),
        }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        self.indent_level -= 1;
    }

    pub fn write(&mut self, text: &str) {
        if self.current_line.is_empty() {
            let indentation = " ".repeat(self.indent_size * self.indent_level);
            self.current_line.push_str(&indentation);
        }
        self.current_line.push_str(text);
    }

    pub fn writeln(&mut self, text: &str) {
        self.write(text);
        self.lines.push(self.current_line.clone());
        self.current_line = String::new();
    }

    pub fn get_content(&self) -> String {
        let mut ret = self.lines.join("\n");
        if !self.current_line.is_empty() {
            ret.push_str("\n");
            ret.push_str(&self.current_line);
        }
        ret.push_str("\n");
        ret
    }
}

#[cfg(test)]
mod test {
    use crate::meta::codegen::CodeGeneratorBuilder;

    #[test]
    fn test_codegen() {
        let grammar_definition = r#"
        -- Expression grammar
        @start
        sum -> term ((op#ADD | op#SUB) term)*;

        term -> product ((op#MUL | op#DIV) product)*;

        product -> NUMBER | IDENTIFIER | group;

        group -> LPAREN sum RPAREN;
        "#;

        let code_gen = CodeGeneratorBuilder::new()
            .indent_size(4)
            .top_comment("Generated grammar code")
            .function_name("make_expression_grammar")
            .build();

        println!("{}", code_gen.generate_code(&grammar_definition).unwrap());
    }
}
