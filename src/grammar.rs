//! Grammar combinator framework.
//!
//! # Architecture
//!
//! Each call to [`GrammarHandle::rule_ref`] creates **fresh** [`RuleStart`] /
//! [`RuleEnd`] nodes for that specific occurrence of the rule. The body is
//! expanded **lazily** the first time the start node's children are accessed
//! during traversal. This mirrors the Python implementation and correctly
//! handles mutually recursive grammars without introducing false ambiguity.
//!
//! ## Node types
//!
//! | [`GrammarNodeKind`]    | Meaning                          |
//! |------------------------|----------------------------------|
//! | `Normal`               | Structural / routing node        |
//! | `Token(type)`          | Terminal – must match a token    |
//! | `RuleStart(name)`      | Entry to a named rule            |
//! | `RuleEnd(name)`        | Exit from a named rule           |
//!
//! ## Combinator functions
//!
//! | Function         | Grammar notation    |
//! |-----------------|---------------------|
//! | [`tok`]         | terminal symbol     |
//! | [`seq`]         | A B C …             |
//! | [`choice`]      | A \| B \| C …       |
//! | [`opt`]         | A?                  |
//! | [`many`]        | A*                  |
//! | [`one_or_more`] | A+                  |

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use crate::ast::Ast;

// ─────────────────────────────────────────────────────────────────────────────
// Expander type alias
// ─────────────────────────────────────────────────────────────────────────────

type ExpanderFn = Rc<dyn Fn(&GrammarHandle) -> Box<dyn GrammarElement>>;

// ─────────────────────────────────────────────────────────────────────────────
// GrammarHandle – passed into every rule expander closure
// ─────────────────────────────────────────────────────────────────────────────

/// View into the grammar's rule registry used inside rule-definition closures.
///
/// ```rust,ignore
/// grammar.add_rule("sum", true, |g| {
///     seq(vec![g.rule_ref("term"),
///              many(seq(vec![tok("PLUS"), g.rule_ref("term")]))])
/// });
/// ```
#[derive(Clone)]
pub struct GrammarHandle {
    defs: Rc<HashMap<String, ExpanderFn>>,
}

impl GrammarHandle {
    /// Create a reference to a named rule. Each call produces **fresh** graph
    /// nodes so that multiple occurrences of the same rule in one definition
    /// remain independent (no shared `RuleEnd` edges).
    pub fn rule_ref(&self, name: &str) -> Box<dyn GrammarElement> {
        make_lazy_rule(name, "", Rc::clone(&self.defs))
    }

    /// Like [`rule_ref`] but with an id tag for AST retrieval.
    pub fn rule_ref_id(&self, name: &str, id: &str) -> Box<dyn GrammarElement> {
        make_lazy_rule(name, id, Rc::clone(&self.defs))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node
// ─────────────────────────────────────────────────────────────────────────────

/// Discriminates the four node roles in the syntax graph.
#[derive(Debug, Clone, PartialEq)]
pub enum GrammarNodeKind {
    Normal,
    Token(String),
    RuleStart(String),
    RuleEnd(String),
}

/// Lazy-expansion payload stored inside a `RuleStart` node.
struct LazyPayload {
    /// The matching `RuleEnd` node for this specific occurrence.
    rule_end: NodeRef,
    /// Expander closure (captures `GrammarHandle` by `Rc`).
    expander: ExpanderFn,
    /// Shared rule registry (needed to build sub-rule handles).
    defs: Rc<HashMap<String, ExpanderFn>>,
    /// Whether expansion has been triggered yet.
    is_expanded: Cell<bool>,
}

/// A node in the syntax graph.
///
/// `NodeRef = Rc<GrammarNode>` (no outer `RefCell`). Interior mutability is
/// achieved via `children: RefCell<…>` and `Cell<bool>` inside `LazyPayload`.
pub struct GrammarNode {
    pub kind: GrammarNodeKind,
    pub id: String,
    children: RefCell<Vec<NodeRef>>,
    lazy: Option<LazyPayload>,
}

/// Shared reference to a [`GrammarNode`].
pub type NodeRef = Rc<GrammarNode>;

impl GrammarNode {
    fn new(kind: GrammarNodeKind) -> NodeRef {
        Rc::new(GrammarNode {
            kind,
            id: String::new(),
            children: RefCell::new(Vec::new()),
            lazy: None,
        })
    }

    fn new_with_id(kind: GrammarNodeKind, id: &str) -> NodeRef {
        Rc::new(GrammarNode {
            kind,
            id: id.to_string(),
            children: RefCell::new(Vec::new()),
            lazy: None,
        })
    }

    fn new_lazy_start(rule_name: &str, id: &str, rule_end: NodeRef, defs: Rc<HashMap<String, ExpanderFn>>) -> NodeRef {
        let expander = Rc::clone(&defs[rule_name]);
        Rc::new(GrammarNode {
            kind: GrammarNodeKind::RuleStart(rule_name.to_string()),
            id: id.to_string(),
            children: RefCell::new(Vec::new()),
            lazy: Some(LazyPayload {
                rule_end,
                expander,
                defs,
                is_expanded: Cell::new(false),
            }),
        })
    }

    pub fn add_child(&self, child: NodeRef) {
        self.children.borrow_mut().push(child);
    }

    /// Get children, triggering lazy expansion first if this is a RuleStart node.
    pub fn get_children(&self) -> std::cell::Ref<'_, Vec<NodeRef>> {
        self.expand_if_needed();
        self.children.borrow()
    }

    fn expand_if_needed(&self) {
        if let Some(lazy) = &self.lazy {
            if !lazy.is_expanded.get() {
                // Mark expanded BEFORE calling expander to break recursion cycles.
                lazy.is_expanded.set(true);
                let handle = GrammarHandle { defs: Rc::clone(&lazy.defs) };
                let body = (lazy.expander)(&handle);
                self.children.borrow_mut().push(body.in_node());
                body.out_node().add_child(Rc::clone(&lazy.rule_end));
            }
        }
    }
}

fn make_lazy_rule(name: &str, id: &str, defs: Rc<HashMap<String, ExpanderFn>>) -> Box<dyn GrammarElement> {
    let end = GrammarNode::new(GrammarNodeKind::RuleEnd(name.to_string()));
    let start = GrammarNode::new_lazy_start(name, id, Rc::clone(&end), defs);
    Box::new(LazyRuleElement { start, end })
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph traversal helpers (used by the parser)
// ─────────────────────────────────────────────────────────────────────────────

/// Find all 1-token lookahead continuations reachable from `node`.
///
/// Returns `(token_type, path_including_token_node)` pairs. The path begins
/// with the first non-Normal node reachable from `node` and ends with the
/// token node itself.
pub fn find_follow_tokens(node: &NodeRef) -> Vec<(String, Vec<NodeRef>)> {
    let mut results = Vec::new();
    find_follow_tokens_rec(node, vec![], &mut results, false);
    results
}

fn find_follow_tokens_rec(
    node: &NodeRef,
    path: Vec<NodeRef>,
    results: &mut Vec<(String, Vec<NodeRef>)>,
    include_self: bool,
) {
    let mut path_so_far = path;

    // Append non-Normal nodes to the running path
    if !matches!(node.kind, GrammarNodeKind::Normal) {
        path_so_far.push(Rc::clone(node));
    }

    // If this is a Token, and we should include it, record and stop.
    if let GrammarNodeKind::Token(ref tt) = node.kind {
        if include_self {
            results.push((tt.clone(), path_so_far));
            return;
        }
    }

    // Recurse into children (triggers lazy expansion for RuleStart nodes).
    let children: Vec<NodeRef> = node.get_children().clone();
    for child in children {
        find_follow_tokens_rec(&child, path_so_far.clone(), results, true);
    }
}

/// Find an epsilon (token-free) path from `node` to a node satisfying `end_pred`.
pub fn find_epsilon_path<F>(node: &NodeRef, end_pred: &F) -> Option<Vec<NodeRef>>
where
    F: Fn(&NodeRef) -> bool,
{
    let mut visited: HashSet<*const GrammarNode> = HashSet::new();
    find_epsilon_path_rec(node, end_pred, vec![], &mut visited)
}

fn find_epsilon_path_rec<F>(
    node: &NodeRef,
    end_pred: &F,
    path: Vec<NodeRef>,
    visited: &mut HashSet<*const GrammarNode>,
) -> Option<Vec<NodeRef>>
where
    F: Fn(&NodeRef) -> bool,
{
    let ptr = Rc::as_ptr(node);
    if visited.contains(&ptr) {
        return None;
    }
    visited.insert(ptr);

    let mut path_so_far = path;
    if !matches!(node.kind, GrammarNodeKind::Normal) {
        path_so_far.push(Rc::clone(node));
    }

    if end_pred(node) {
        return Some(path_so_far);
    }

    // Recurse into children, but skip child Token nodes
    // (epsilon path = no *additional* tokens consumed; the starting node may
    // itself be a Token – that one was already consumed by the parser).
    let children: Vec<NodeRef> = node.get_children().clone();
    for child in children {
        if matches!(child.kind, GrammarNodeKind::Token(_)) {
            continue;
        }
        if let Some(p) = find_epsilon_path_rec(&child, end_pred, path_so_far.clone(), visited) {
            return Some(p);
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// GrammarElement trait
// ─────────────────────────────────────────────────────────────────────────────

/// A composable grammar piece with an entry node and an exit node.
pub trait GrammarElement {
    fn in_node(&self) -> NodeRef;
    fn out_node(&self) -> NodeRef;
    fn clone_element(&self) -> Box<dyn GrammarElement>;
    fn set_id(&mut self, id: &str);
    fn get_id(&self) -> &str;
}

fn connect(from: &dyn GrammarElement, to: &dyn GrammarElement) {
    from.out_node().add_child(to.in_node());
}

// ─────────────────────────────────────────────────────────────────────────────
// TokenElement
// ─────────────────────────────────────────────────────────────────────────────

/// Terminal element – matches a single token of the given type.
pub struct TokenElement {
    node: NodeRef,
}

impl TokenElement {
    pub fn new(token_type: &str, id: &str) -> Self {
        TokenElement {
            node: GrammarNode::new_with_id(GrammarNodeKind::Token(token_type.to_string()), id),
        }
    }
}

impl GrammarElement for TokenElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.node) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.node) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        let kind = self.node.kind.clone();
        let id = self.node.id.clone();
        let token_type = if let GrammarNodeKind::Token(ref t) = kind { t.as_str() } else { "" };
        Box::new(TokenElement::new(token_type, &id))
    }
    fn set_id(&mut self, id: &str) {
        // Token nodes are Rc; we can't mutate through Rc without RefCell.
        // Re-create the node with the new id.
        let kind = self.node.kind.clone();
        let token_type = if let GrammarNodeKind::Token(ref t) = kind { t.clone() } else { String::new() };
        self.node = GrammarNode::new_with_id(GrammarNodeKind::Token(token_type), id);
    }
    fn get_id(&self) -> &str { &self.node.id }
}

/*
// ─────────────────────────────────────────────────────────────────────────────
// NormalElement  (wraps a single Normal node)
// ─────────────────────────────────────────────────────────────────────────────

struct NormalElement {
    node: NodeRef,
}

impl NormalElement {
    fn new() -> Self { NormalElement { node: GrammarNode::new(GrammarNodeKind::Normal) } }
}

impl GrammarElement for NormalElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.node) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.node) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        // Sharing the same underlying node is fine for Normal elements
        // used only as anchors; callers that need isolation should use
        // a different element type.
        Box::new(NormalElement { node: Rc::clone(&self.node) })
    }
    fn set_id(&mut self, _: &str) {}
    fn get_id(&self) -> &str { "" }
}

 */

// ─────────────────────────────────────────────────────────────────────────────
// Sequence
// ─────────────────────────────────────────────────────────────────────────────

/// Matches all contained elements in order (A B C …).
pub struct Sequence {
    elements: Vec<Box<dyn GrammarElement>>,
}

impl Sequence {
    fn new(elements: Vec<Box<dyn GrammarElement>>) -> Self {
        let elems: Vec<Box<dyn GrammarElement>> =
            elements.into_iter().map(|e| e.clone_element()).collect();
        for i in 0..elems.len() - 1 {
            connect(elems[i].as_ref(), elems[i + 1].as_ref());
        }
        Sequence { elements: elems }
    }
}

impl GrammarElement for Sequence {
    fn in_node(&self) -> NodeRef { self.elements[0].in_node() }
    fn out_node(&self) -> NodeRef { self.elements.last().unwrap().out_node() }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        Box::new(Sequence::new(self.elements.iter().map(|e| e.clone_element()).collect()))
    }
    fn set_id(&mut self, _: &str) {}
    fn get_id(&self) -> &str { "" }
}

// ─────────────────────────────────────────────────────────────────────────────
// ChoiceElement
// ─────────────────────────────────────────────────────────────────────────────

/// Matches exactly one of the given branches (A | B | C …).
pub struct ChoiceElement {
    branches: Vec<Box<dyn GrammarElement>>,
    start: NodeRef,
    end: NodeRef,
}

impl ChoiceElement {
    fn new(branches: Vec<Box<dyn GrammarElement>>) -> Self {
        let start = GrammarNode::new(GrammarNodeKind::Normal);
        let end   = GrammarNode::new(GrammarNodeKind::Normal);
        let cloned: Vec<Box<dyn GrammarElement>> =
            branches.into_iter().map(|b| b.clone_element()).collect();
        for b in &cloned {
            start.add_child(b.in_node());
            b.out_node().add_child(Rc::clone(&end));
        }
        ChoiceElement { branches: cloned, start, end }
    }
}

impl GrammarElement for ChoiceElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.start) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.end) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        Box::new(ChoiceElement::new(self.branches.iter().map(|b| b.clone_element()).collect()))
    }
    fn set_id(&mut self, _: &str) {}
    fn get_id(&self) -> &str { "" }
}

// ─────────────────────────────────────────────────────────────────────────────
// OptionalElement
// ─────────────────────────────────────────────────────────────────────────────

/// Makes an element optional (A?).
pub struct OptionalElement {
    inner: Box<dyn GrammarElement>,
    start: NodeRef,
    end: NodeRef,
}

impl OptionalElement {
    fn new(element: Box<dyn GrammarElement>) -> Self {
        let inner = element.clone_element();
        let start = GrammarNode::new(GrammarNodeKind::Normal);
        let end   = GrammarNode::new(GrammarNodeKind::Normal);
        start.add_child(inner.in_node());
        start.add_child(Rc::clone(&end));         // bypass path
        inner.out_node().add_child(Rc::clone(&end));
        OptionalElement { inner, start, end }
    }
}

impl GrammarElement for OptionalElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.start) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.end) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        Box::new(OptionalElement::new(self.inner.clone_element()))
    }
    fn set_id(&mut self, _: &str) {}
    fn get_id(&self) -> &str { "" }
}

// ─────────────────────────────────────────────────────────────────────────────
// ManyElement
// ─────────────────────────────────────────────────────────────────────────────

/// Matches zero or more repetitions (A*).
pub struct ManyElement {
    inner: Box<dyn GrammarElement>,
    start: NodeRef,
    end: NodeRef,
}

impl ManyElement {
    fn new(element: Box<dyn GrammarElement>) -> Self {
        let inner = element.clone_element();
        let start = GrammarNode::new(GrammarNodeKind::Normal);
        let end   = GrammarNode::new(GrammarNodeKind::Normal);
        start.add_child(inner.in_node());
        start.add_child(Rc::clone(&end));          // zero-matches bypass
        inner.out_node().add_child(Rc::clone(&start)); // loop back
        ManyElement { inner, start, end }
    }
}

impl GrammarElement for ManyElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.start) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.end) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        Box::new(ManyElement::new(self.inner.clone_element()))
    }
    fn set_id(&mut self, _: &str) {}
    fn get_id(&self) -> &str { "" }
}

// ─────────────────────────────────────────────────────────────────────────────
// LazyRuleElement  (returned by GrammarHandle::rule_ref)
// ─────────────────────────────────────────────────────────────────────────────

/// A grammar element backed by a lazily-expanded named rule.
///
/// The start node holds the expansion payload; the body is connected the first
/// time the start node's children are accessed during traversal.
pub struct LazyRuleElement {
    pub start: NodeRef,
    pub end: NodeRef,
}

impl GrammarElement for LazyRuleElement {
    fn in_node(&self) -> NodeRef { Rc::clone(&self.start) }
    fn out_node(&self) -> NodeRef { Rc::clone(&self.end) }
    fn clone_element(&self) -> Box<dyn GrammarElement> {
        // Clone = create a new independent lazy instance for this rule.
        // Retrieve the rule name and defs from the start node's lazy payload.
        let rule_name = match &self.start.kind {
            GrammarNodeKind::RuleStart(n) => n.clone(),
            _ => panic!("LazyRuleElement start is not RuleStart"),
        };
        let id = self.start.id.clone();
        let defs = Rc::clone(
            &self.start.lazy.as_ref().expect("missing lazy payload").defs,
        );
        make_lazy_rule(&rule_name, &id, defs)
    }
    fn set_id(&mut self, id: &str) {
        // We can't mutate Rc<GrammarNode>.id directly; rebuild the start node.
        let rule_name = match &self.start.kind {
            GrammarNodeKind::RuleStart(n) => n.clone(),
            _ => panic!("LazyRuleElement start is not RuleStart"),
        };
        let defs = Rc::clone(
            &self.start.lazy.as_ref().expect("missing lazy payload").defs,
        );
        // Rebuild with the new id; drop any previous (unexpanded) children.
        self.start = GrammarNode::new_lazy_start(&rule_name, id, Rc::clone(&self.end), defs);
    }
    fn get_id(&self) -> &str { &self.start.id }
}

// ─────────────────────────────────────────────────────────────────────────────
// Combinator factory functions
// ─────────────────────────────────────────────────────────────────────────────

/// Terminal element matching a token of the given type.
pub fn tok(token_type: &str) -> Box<dyn GrammarElement> {
    Box::new(TokenElement::new(token_type, ""))
}

/// Like [`tok`] but with an id tag.
pub fn tok_id(token_type: &str, id: &str) -> Box<dyn GrammarElement> {
    Box::new(TokenElement::new(token_type, id))
}

/// Sequence combinator (A B C …).
pub fn seq(elements: Vec<Box<dyn GrammarElement>>) -> Box<dyn GrammarElement> {
    Box::new(Sequence::new(elements))
}

/// Choice combinator (A | B | C …).
pub fn choice(branches: Vec<Box<dyn GrammarElement>>) -> Box<dyn GrammarElement> {
    Box::new(ChoiceElement::new(branches))
}

/// Optional combinator (A?).
pub fn opt(element: Box<dyn GrammarElement>) -> Box<dyn GrammarElement> {
    Box::new(OptionalElement::new(element))
}

/// Zero-or-more repetition combinator (A*).
pub fn many(element: Box<dyn GrammarElement>) -> Box<dyn GrammarElement> {
    Box::new(ManyElement::new(element))
}

/// One-or-more repetition combinator (A+).  Sugar for `seq([e, many(e)])`.
pub fn one_or_more(element: Box<dyn GrammarElement>) -> Box<dyn GrammarElement> {
    let cloned = element.clone_element();
    seq(vec![element, many(cloned)])
}

/// Tag a grammar element with an id (for AST node retrieval).
pub fn with_id(mut element: Box<dyn GrammarElement>, id: &str) -> Box<dyn GrammarElement> {
    element.set_id(id);
    element
}

// ─────────────────────────────────────────────────────────────────────────────
// Grammar
// ─────────────────────────────────────────────────────────────────────────────

type AstTransformer = Box<dyn Fn(Ast) -> Ast>;

/// The complete grammar: rule definitions and optional AST transformers.
///
/// Call [`Grammar::compile`] once before parsing.
pub struct Grammar {
    rule_defs: HashMap<String, ExpanderFn>,
    start_rule: Option<String>,
    ast_transformers: HashMap<String, AstTransformer>,

    compiled: bool,
    global_start: Option<NodeRef>,
    global_end: Option<NodeRef>,
}

impl Grammar {
    pub fn new() -> Self {
        Grammar {
            rule_defs: HashMap::new(),
            start_rule: None,
            ast_transformers: HashMap::new(),
            compiled: false,
            global_start: None,
            global_end: None,
        }
    }

    /// Register a grammar rule.
    ///
    /// - `name`     – rule identifier used in AST node names
    /// - `is_start` – designate this as the grammar entry point
    /// - `expander` – closure receiving a [`GrammarHandle`] and returning a grammar element
    pub fn add_rule(
        &mut self,
        name: &str,
        is_start: bool,
        expander: impl Fn(&GrammarHandle) -> Box<dyn GrammarElement> + 'static,
    ) {
        self.rule_defs.insert(name.to_string(), Rc::new(expander));
        if is_start {
            self.start_rule = Some(name.to_string());
        }
    }

    /// Register an AST transformer for the named rule.
    pub fn add_ast_transformer(&mut self, rule_name: &str, f: impl Fn(Ast) -> Ast + 'static) {
        self.ast_transformers.insert(rule_name.to_string(), Box::new(f));
    }

    pub fn get_ast_transformer(&self, rule_name: &str) -> Option<&dyn Fn(Ast) -> Ast> {
        self.ast_transformers.get(rule_name).map(|f| f.as_ref())
    }

    /// Compile the grammar into a syntax graph.  Idempotent.
    pub fn compile(&mut self) {
        if self.compiled { return; }

        let start_rule = self.start_rule.clone().expect("No start rule set");

        // Build a shared registry of rule expanders so that each GrammarHandle
        // clone shares the same definitions (Rc, so no copying).
        let defs: Rc<HashMap<String, ExpanderFn>> = Rc::new(
            self.rule_defs
                .iter()
                .map(|(k, v)| (k.clone(), Rc::clone(v)))
                .collect(),
        );

        // Wrap the start rule in global start/end Normal nodes.
        let global_start = GrammarNode::new(GrammarNodeKind::Normal);
        let global_end   = GrammarNode::new(GrammarNodeKind::Normal);

        let start_elem = make_lazy_rule(&start_rule, "", Rc::clone(&defs));
        global_start.add_child(start_elem.in_node());
        start_elem.out_node().add_child(Rc::clone(&global_end));

        self.global_start = Some(global_start);
        self.global_end   = Some(global_end);
        self.compiled = true;
    }

    /// The global start node of the compiled syntax graph.
    pub fn start_node(&self) -> NodeRef {
        Rc::clone(self.global_start.as_ref().expect("Grammar not compiled"))
    }

    /// The global end node of the compiled syntax graph.
    pub fn end_node(&self) -> NodeRef {
        Rc::clone(self.global_end.as_ref().expect("Grammar not compiled"))
    }

    /// Find an epsilon path from `node` to the global end node.
    pub fn find_path_to_end(&self, node: &NodeRef) -> Option<Vec<NodeRef>> {
        let end = self.end_node();
        let end_ptr = Rc::as_ptr(&end);
        find_epsilon_path(node, &|n| Rc::as_ptr(n) == end_ptr)
    }
}

impl Default for Grammar {
    fn default() -> Self { Grammar::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tok_single_child() {
        let t = tok("NUM");
        // A token element has no children (it IS the terminal).
        let follows = find_follow_tokens(&t.in_node());
        assert_eq!(follows.len(), 0);
    }

    #[test]
    fn test_seq_follow() {
        // From the global start (Normal) node, seq([tok("A"), tok("B")]) → first reachable token is "A"
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| seq(vec![tok("A"), tok("B")]));
        g.compile();
        let follows = find_follow_tokens(&g.start_node());
        assert_eq!(follows.len(), 1);
        assert_eq!(follows[0].0, "A");
    }

    #[test]
    fn test_choice_follow() {
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| choice(vec![tok("A"), tok("B")]));
        g.compile();
        let mut types: Vec<String> = find_follow_tokens(&g.start_node())
            .into_iter().map(|(t, _)| t).collect();
        types.sort();
        assert_eq!(types, vec!["A", "B"]);
    }

    #[test]
    fn test_opt_follow() {
        // opt(tok("A")) followed by tok("B") → can see "A" or "B" from start
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| seq(vec![opt(tok("A")), tok("B")]));
        g.compile();
        let mut types: Vec<String> = find_follow_tokens(&g.start_node())
            .into_iter().map(|(t, _)| t).collect();
        types.sort();
        assert_eq!(types, vec!["A", "B"]);
    }

    #[test]
    fn test_many_follow() {
        // many(tok("A")) followed by tok("B")
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| seq(vec![many(tok("A")), tok("B")]));
        g.compile();
        let mut types: Vec<String> = find_follow_tokens(&g.start_node())
            .into_iter().map(|(t, _)| t).collect();
        types.sort();
        assert_eq!(types, vec!["A", "B"]);
    }

    #[test]
    fn test_grammar_compile() {
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| tok("NUM"));
        g.compile();
        let start = g.start_node();
        assert!(!start.get_children().is_empty());
    }

    #[test]
    fn test_grammar_find_path_to_end() {
        let mut g = Grammar::new();
        g.add_rule("expr", true, |_| tok("NUM"));
        g.compile();
        let start = g.start_node();
        // Cannot reach end without consuming the NUM token.
        assert!(g.find_path_to_end(&start).is_none());
    }

    #[test]
    fn test_grammar_multi_rule() {
        // sum = term (('+') term)*    term = tok("NUM")
        let mut g = Grammar::new();
        g.add_rule("term", false, |_| tok("NUM"));
        g.add_rule("sum", true, |h| {
            seq(vec![
                h.rule_ref("term"),
                many(seq(vec![tok("PLUS"), h.rule_ref("term")])),
            ])
        });
        g.compile();
        let follows = find_follow_tokens(&g.start_node());
        let types: Vec<String> = follows.into_iter().map(|(t, _)| t).collect();
        assert_eq!(types, vec!["NUM"]);
    }

    #[test]
    fn test_rule_ref_independent_ends() {
        // term = factor (* factor)*    factor = NUM
        // After parsing the first factor, the continuation must be many.start (not another factor).
        let mut g = Grammar::new();
        g.add_rule("factor", false, |_| tok("NUM"));
        g.add_rule("term", true, |h| {
            seq(vec![
                h.rule_ref("factor"),
                many(seq(vec![tok("MUL"), h.rule_ref("factor")])),
            ])
        });
        g.compile();
        // We should be able to parse "NUM" (single factor, no multiplication).
        use crate::parser::Parser;
        use crate::token::{Token, TokenStreamFromList};
        let parser = Parser::new(g);
        let mut stream = TokenStreamFromList::new(vec![Token::new("NUM", "1", 1, 1)]);
        let result = parser.parse(&mut stream);
        assert!(result.is_ok(), "should parse without ambiguity: {:?}", result);
    }
}
