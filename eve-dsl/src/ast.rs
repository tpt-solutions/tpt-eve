/// One slot of a [`Triple`]: a bare identifier, a quoted text literal, or a
/// rule/query variable (uppercase-leading identifier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Ident(String),
    Literal(String),
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triple {
    pub subject: Term,
    pub predicate: Term,
    pub object: Term,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDef {
    pub name: Option<String>,
    pub conditions: Vec<Triple>,
    pub conclusion: Triple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDef {
    pub clauses: Vec<Triple>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Fact(Triple),
    Rule(RuleDef),
    Query(QueryDef),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}
