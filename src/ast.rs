use std::collections::HashMap;

/// A value that can be stored as an AST node attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum AstAttr {
    Bool(bool),
    Str(String),
}

impl From<bool> for AstAttr {
    fn from(v: bool) -> Self { AstAttr::Bool(v) }
}
impl From<&str> for AstAttr {
    fn from(v: &str) -> Self { AstAttr::Str(v.to_string()) }
}
impl From<String> for AstAttr {
    fn from(v: String) -> Self { AstAttr::Str(v) }
}

/// A node in the Abstract Syntax Tree.
///
/// Rule nodes carry a `name` (the rule name) and no `value`.
/// Token nodes carry both a `name` (the token type) and a `value` (the matched text).
#[derive(Debug, Clone)]
pub struct Ast {
    pub name: String,
    pub value: Option<String>,
    pub id: String,
    children: Vec<Ast>,
    attributes: HashMap<String, AstAttr>,
}

impl Ast {
    pub fn new(name: impl Into<String>) -> Self {
        Ast {
            name: name.into(),
            value: None,
            id: String::new(),
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_value(name: impl Into<String>, value: impl Into<String>) -> Self {
        Ast {
            name: name.into(),
            value: Some(value.into()),
            id: String::new(),
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn add_child(&mut self, child: Ast) {
        self.children.push(child);
    }

    pub fn children(&self) -> &[Ast] {
        &self.children
    }

    pub fn nth_child(&self, n: usize) -> &Ast {
        &self.children[n]
    }

    pub fn children_by_name(&self, name: &str) -> Vec<&Ast> {
        self.children.iter().filter(|c| c.name == name).collect()
    }

    /// Find the first child with the given id. Clears the id on the returned node by default.
    pub fn child_by_id(&self, id: &str, clear_id: bool) -> Option<Ast> {
        for child in &self.children {
            if child.id == id {
                let mut cloned = child.clone();
                if clear_id {
                    cloned.id = String::new();
                }
                return Some(cloned);
            }
        }
        None
    }

    /// Collect all children with the given id. Clears id on returned nodes by default.
    pub fn children_by_id(&self, id: &str, clear_id: bool) -> Vec<Ast> {
        self.children
            .iter()
            .filter(|c| c.id == id)
            .map(|c| {
                let mut cloned = c.clone();
                if clear_id {
                    cloned.id = String::new();
                }
                cloned
            })
            .collect()
    }

    pub fn set_attr(&mut self, name: impl Into<String>, value: impl Into<AstAttr>) {
        self.attributes.insert(name.into(), value.into());
    }

    pub fn get_attr(&self, name: &str) -> Option<&AstAttr> {
        self.attributes.get(name)
    }

    pub fn has_attr(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }
}

impl std::ops::Index<usize> for Ast {
    type Output = Ast;
    fn index(&self, i: usize) -> &Ast {
        &self.children[i]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// AstStrWriter
// ──────────────────────────────────────────────────────────────────────────────

/// Renders an [`Ast`] as an XML-like string for debugging.
pub struct AstStrWriter {
    indent_size: usize,
}

impl AstStrWriter {
    pub fn new(indent_size: usize) -> Self {
        AstStrWriter { indent_size }
    }

    pub fn write_ast_to_str(&self, ast: &Ast) -> String {
        let mut out = String::new();
        self.write_node(ast, 0, &mut out);
        out
    }

    fn indent(&self, level: usize, out: &mut String) {
        for _ in 0..level * self.indent_size {
            out.push(' ');
        }
    }

    fn write_node(&self, ast: &Ast, level: usize, out: &mut String) {
        let id_str = if ast.id.is_empty() {
            String::new()
        } else {
            format!(" id=\"{}\"", ast.id)
        };

        match &ast.value {
            None => {
                if ast.children.is_empty() {
                    self.indent(level, out);
                    out.push_str(&format!("<{}{}/>\n", ast.name, id_str));
                } else {
                    self.indent(level, out);
                    out.push_str(&format!("<{}{}>\n", ast.name, id_str));
                    for child in &ast.children {
                        self.write_node(child, level + 1, out);
                    }
                    self.indent(level, out);
                    out.push_str(&format!("</{}>\n", ast.name));
                }
            }
            Some(val) => {
                if ast.children.is_empty() {
                    self.indent(level, out);
                    out.push_str(&format!("<{}{}> {} </{}>\n", ast.name, id_str, val, ast.name));
                } else {
                    self.indent(level, out);
                    out.push_str(&format!("<{}{}> {}\n", ast.name, id_str, val));
                    for child in &ast.children {
                        self.write_node(child, level + 1, out);
                    }
                    self.indent(level, out);
                    out.push_str(&format!("</{}>\n", ast.name));
                }
            }
        }
    }
}

impl Default for AstStrWriter {
    fn default() -> Self {
        AstStrWriter::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_children() {
        let mut root = Ast::new("sum");
        root.add_child(Ast::with_value("NUMBER", "3"));
        root.add_child(Ast::with_value("PLUS", "+"));
        root.add_child(Ast::with_value("NUMBER", "5"));
        assert_eq!(root.children().len(), 3);
        assert_eq!(root.nth_child(0).value.as_deref(), Some("3"));
    }

    #[test]
    fn test_children_by_name() {
        let mut root = Ast::new("call");
        root.add_child(Ast::new("callee"));
        root.add_child(Ast::new("arg"));
        root.add_child(Ast::new("arg"));
        let args = root.children_by_name("arg");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_child_by_id_clears_id() {
        let mut root = Ast::new("call");
        let child = Ast::new("NUMBER").with_id("callee");
        root.add_child(child);
        let found = root.child_by_id("callee", true).unwrap();
        assert_eq!(found.name, "NUMBER");
        assert_eq!(found.id, ""); // id cleared
    }

    #[test]
    fn test_children_by_id() {
        let mut root = Ast::new("call");
        root.add_child(Ast::new("NUMBER").with_id("arg"));
        root.add_child(Ast::new("IDENT").with_id("arg"));
        let args = root.children_by_id("arg", true);
        assert_eq!(args.len(), 2);
        assert!(args.iter().all(|a| a.id.is_empty()));
    }

    #[test]
    fn test_set_get_attr() {
        let mut node = Ast::new("x");
        node.set_attr("flag", true);
        assert!(node.has_attr("flag"));
        assert_eq!(node.get_attr("flag"), Some(&AstAttr::Bool(true)));
        assert!(!node.has_attr("missing"));
    }

    #[test]
    fn test_ast_str_writer() {
        let mut root = Ast::new("sum");
        root.add_child(Ast::with_value("NUMBER", "3"));
        let out = AstStrWriter::default().write_ast_to_str(&root);
        assert!(out.contains("<sum>"));
        assert!(out.contains("<NUMBER> 3 </NUMBER>"));
        assert!(out.contains("</sum>"));
    }
}
