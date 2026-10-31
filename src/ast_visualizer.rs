use graphviz_rust::{self, dot_structures::Graph};

use crate::ast::Ast;

pub struct AstVisualizer {
    next_node_id: usize,
}

impl AstVisualizer {
    
    pub fn new() -> Self {
        AstVisualizer { next_node_id: 0 }
    }

    pub fn to_dot_string(&mut self, ast: &Ast) -> String {
        let mut result = String::new();
        result.push_str("digraph AST {\n");
        result.push_str("\tnode [shape=box];\n");
        self.visit_ast(ast, None, &mut result);
        result.push_str("}\n");

        result
    }

    fn visit_ast(&mut self, ast: &Ast, node_id: Option<&str>, result: &mut String) {
        let node_id = match node_id {
            Some(id) => id.to_string(),
            None => self.next_node_id(),
        };
        let label = match ast.value {
            Some(ref value) => format!("{}: {}", &ast.name, value),
            None => format!("{}", &ast.name),
        };
        result.push_str(&format!("\t{node_id} [label=\"{label}\"];\n"));
        for child in ast.children() {
            let child_id = self.next_node_id();
            self.visit_ast(child, Some(&child_id), result);
            result.push_str(&format!("\t{node_id} -> {child_id};\n"));
        }
    }

    fn next_node_id(&mut self) -> String {
        let id = self.next_node_id;
        self.next_node_id += 1;
        format!("node_{}", id)
    }
}

impl From<Ast> for Graph {

    fn from(ast: Ast) -> Self {
        let mut visualizer = AstVisualizer::new();
        let dot_string = visualizer.to_dot_string(&ast);
        graphviz_rust::parse(&dot_string).expect("Could not parse AST")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Ast;

    #[test]
    fn test_to_graphviz_includes_root_and_leaf_nodes() {
        let mut root = Ast::new("add");
        root.add_child(Ast::with_value("NUMBER", "3"));
        root.add_child(Ast::with_value("NUMBER", "5"));

        let mut visualizer = AstVisualizer::new();
        let graph = visualizer.to_dot_string(&root);

        assert!(graph.starts_with("digraph AST {\n"));
        assert!(graph.contains("\tnode [shape=box];\n"));
        assert!(graph.contains("\tnode_0 [label=\"add\"];\n"));
        assert!(graph.contains("\tnode_1 [label=\"NUMBER: 3\"];\n"));
        assert!(graph.contains("\tnode_2 [label=\"NUMBER: 5\"];\n"));
        assert!(graph.contains("\tnode_0 -> node_1;\n"));
        assert!(graph.contains("\tnode_0 -> node_2;\n"));
        assert!(graph.ends_with("}\n"));
    }

    #[test]
    fn test_to_graphviz_handles_nested_children() {
        let mut root = Ast::new("root");

        let mut left = Ast::new("left");
        left.add_child(Ast::with_value("IDENT", "x"));

        let mut right = Ast::new("right");
        right.add_child(Ast::with_value("NUMBER", "7"));

        root.add_child(left);
        root.add_child(right);

        let mut visualizer = AstVisualizer::new();
        let graph = visualizer.to_dot_string(&root);

        assert!(graph.contains("\tnode_0 [label=\"root\"];\n"));
        assert!(graph.contains("\tnode_1 [label=\"left\"];\n"));
        assert!(graph.contains("\tnode_2 [label=\"IDENT: x\"];\n"));
        assert!(graph.contains("\tnode_3 [label=\"right\"];\n"));
        assert!(graph.contains("\tnode_4 [label=\"NUMBER: 7\"];\n"));
        assert!(graph.contains("\tnode_0 -> node_1;\n"));
        assert!(graph.contains("\tnode_1 -> node_2;\n"));
        assert!(graph.contains("\tnode_0 -> node_3;\n"));
        assert!(graph.contains("\tnode_3 -> node_4;\n"));
    }

    #[test]
    fn test_graph_creation_from_ast() {
        let mut root = Ast::new("expr");
        root.add_child(Ast::with_value("NUMBER", "3"));
        root.add_child(Ast::with_value("PLUS", "+"));
        root.add_child(Ast::with_value("NUMBER", "5"));

        let graph: Graph = root.into();

        let expected = graphviz_rust::parse(
            r#"digraph AST {
	node [shape=box];
	node_0 [label="expr"];
	node_1 [label="NUMBER: 3"];
	node_0 -> node_1;
	node_2 [label="PLUS: +"];
	node_0 -> node_2;
	node_3 [label="NUMBER: 5"];
	node_0 -> node_3;
}
"#,
        )
        .expect("Could not parse expected graph");

        assert_eq!(graph, expected);
    }
}