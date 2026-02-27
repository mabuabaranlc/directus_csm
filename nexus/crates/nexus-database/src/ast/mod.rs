// AST module — Query→AST→SQL pipeline
// Mirrors api/src/types/ast.ts and api/src/utils/get-ast-from-query/

use nexus_types::query::Query;

/// AST node types for the query pipeline
#[derive(Debug, Clone)]
pub enum AstNode {
    Root(RootNode),
    M2O(RelationalNode),
    O2M(RelationalNode),
    A2O(RelationalNode),
}

#[derive(Debug, Clone)]
pub struct RootNode {
    pub name: String,
    pub children: Vec<ChildNode>,
    pub query: Query,
}

#[derive(Debug, Clone)]
pub enum ChildNode {
    Field(FieldNode),
    FunctionField(FunctionFieldNode),
    Nested(Box<AstNode>),
}

#[derive(Debug, Clone)]
pub struct FieldNode {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionFieldNode {
    pub name: String,
    pub function: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RelationalNode {
    pub name: String,
    pub field_key: String,
    pub parent_key: String,
    pub relation_type: RelationType,
    pub children: Vec<ChildNode>,
    pub query: Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    ManyToOne,
    OneToMany,
    AnyToOne,
}
