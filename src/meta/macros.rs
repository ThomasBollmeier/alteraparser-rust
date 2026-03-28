use crate::prelude::Ast;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct MacroDefinition {
    parameters: Vec<String>,
    body: Ast,
}

pub struct MacroExpander {
    macros: HashMap<String, MacroDefinition>,
    arguments: Vec<HashMap<String, Ast>>,
}

impl MacroExpander {
    pub fn new() -> MacroExpander {
        MacroExpander {
            macros: HashMap::new(),
            arguments: Vec::new(),
        }
    }

    pub fn expand_macros(&mut self, ast: Ast) -> Ast {
        self.find_macro_definitions(&ast);
        self.eval_macro_calls(ast)
            .expect("Error evaluating macro calls")
    }

    fn eval_macro_calls(&mut self, ast: Ast) -> Option<Ast> {
        if ast.name == "macro" {
            return None;
        }

        let ret = match ast.name.as_str() {
            "macro_call" => self.eval_macro_call(&ast),
            "param_ref" => self.eval_param_ref(&ast),
            _ => self.eval_other_node(ast),
        };

        ret
    }

    fn eval_other_node(&mut self, mut ast: Ast) -> Option<Ast> {
        let evaluated_children = ast
            .children()
            .iter()
            .flat_map(|child| self.eval_macro_calls(child.clone()))
            .collect::<Vec<Ast>>();
        ast.children_mut().clear();
        evaluated_children.into_iter().for_each(|child| {
            ast.add_child(child);
        });

        Some(ast)
    }

    fn eval_macro_call(&mut self, macro_call: &Ast) -> Option<Ast> {
        self.push_call_arguments(macro_call);

        let macro_name = macro_call
            .children()
            .iter()
            .find(|c| c.name == "name")
            .and_then(|c| c.value.clone())
            .expect("Macro name not found");

        let macro_def = self
            .macros
            .get(&macro_name)
            .expect(&format!("Macro {} not found", macro_name));

        let body = macro_def.body.clone();
        let evaluated_body = self
            .eval_macro_calls(body)
            .expect("Error evaluating macro body");

        self.arguments.pop();

        Some(evaluated_body)
    }

    fn push_call_arguments(&mut self, macro_call: &Ast) {
        let macro_name = macro_call
            .children()
            .iter()
            .find(|c| c.name == "name")
            .and_then(|c| c.value.clone())
            .expect("Macro name not found");

        let arguments_node = macro_call
            .children()
            .iter()
            .find(|c| c.name == "arguments")
            .expect("arguments not found");
        let arguments: Vec<Ast> = arguments_node
            .children()
            .iter()
            .flat_map(|c| self.eval_macro_calls(c.clone()))
            .collect();

        let parameters = &self
            .macros
            .get(&macro_name)
            .expect("Macro not found")
            .parameters;

        if parameters.len() != arguments.len() {
            panic!("Bad number of arguments in macro call {}", macro_name);
        }

        let mut arguments_map = HashMap::new();

        for (param, arg) in parameters.iter().zip(arguments.iter()) {
            arguments_map.insert(param.clone(), arg.clone());
        }

        self.arguments.push(arguments_map);
    }

    fn eval_param_ref(&mut self, ast: &Ast) -> Option<Ast> {
        let mut param_name = ast.value.clone()?;
        let mut idx: i32 = self.arguments.len() as i32 - 1;
        let mut ret: Option<Ast> = None;

        while idx >= 0 {
            let arguments = &self.arguments[idx as usize];
            if !arguments.contains_key(&param_name) {
                break;
            }
            let value = arguments.get(&param_name).unwrap();
            if value.name != "param_ref" {
                ret = Some(value.clone());
                break;
            }
            param_name = value.value.clone()?;
            idx -= 1;
        }

        ret
    }

    fn find_macro_definitions(&mut self, ast: &Ast) {
        self.macros.clear();
        self.find_macro_defs(ast);
    }

    fn find_macro_defs(&mut self, ast: &Ast) {
        if ast.name == "macro" {
            let children = ast.children();
            let name = children
                .iter()
                .find(|c| c.name == "name")
                .and_then(|c| c.value.clone())
                .expect("Macro name not found");

            let params_node = children
                .iter()
                .find(|c| c.name == "parameters")
                .expect("parameters not found");
            let parameters: Vec<String> = params_node
                .children()
                .iter()
                .map(|c| c.value.clone().unwrap())
                .collect();

            let body = children[2].clone();

            self.macros
                .insert(name, MacroDefinition { parameters, body });
        }

        for child in ast.children() {
            self.find_macro_defs(child);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::AstStrWriter;
    use crate::lexer::Lexer;
    use crate::meta::grammar::make_grammar;
    use crate::meta::lexer_grammar::make_lexer_grammar;
    use crate::meta::macros::MacroExpander;
    use crate::parser::Parser;

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

    #[test]
    fn test_grammar_with_nested_macro() {
        let grammar_code = r#"
        @start
        s -> a*;
        a -> group<b>;
        b -> NUMBER;
        group<content> -> LPAREN double<double<content>> RPAREN;
        double<data> -> data COMMA data;
        "#;

        run(grammar_code);
    }

    #[test]
    fn test_recursive_macro_calls() {
        let grammar_code = r#"
        @start
        s -> a*;
        a -> group<group<b>>;
        b -> NUMBER;
        group<content> -> LPAREN content RPAREN;
        "#;

        run(grammar_code);
    }

    fn run(grammar_code: &str) {
        let lexer_grammar = make_lexer_grammar();
        let mut lexer = Lexer::new(&lexer_grammar, grammar_code);

        let grammar = make_grammar();
        let parser = Parser::new(grammar);

        let ast = parser.parse(&mut lexer).unwrap();

        let mut expander = MacroExpander::new();
        let expanded = expander.expand_macros(ast);

        let writer = AstStrWriter::new(2);

        println!("{}", writer.write_ast_to_str(&expanded));
    }
}
