# Alteraparser

> Audiatur et altera pars(er)  
>   
> *Unknown Roman Computer Scientist*
 
A library to create a parser from a grammar

## AST JSON serialization

`Ast` supports serialization and deserialization to/from JSON via `serde`:

```rust
use alteraparser::ast::Ast;

let mut ast = Ast::new("sum");
ast.add_child(Ast::with_value("NUMBER", "3"));

let json = ast.to_json_pretty().unwrap();
let restored = Ast::from_json(&json).unwrap();
assert_eq!(restored.name, "sum");
```
