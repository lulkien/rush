use std::{cell::RefCell, rc::Rc};

// ── redirect types ───────────────────────────────────────────────────

/// POSIX redirection operator.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum RedirectOp {
    #[default]
    Less,
    Great,
    DGreat,
    LessAnd,
    GreatAnd,
    LessGreat,
    DLess,
    DLessDash,
    Clobber,
}

/// A redirect operator paired with its source fd and target word.
///
/// `src_fd` is None for the defaults: 0 for `<`, 1 for `>` and `>>`.
/// It is set when an explicit fd number precedes the operator (e.g. `2>`).
#[derive(Clone, Debug, Default)]
pub struct Redirect {
    pub op: RedirectOp,
    pub src_fd: Option<i32>,
    pub target: String,
}

// ── AST root ─────────────────────────────────────────────────────────

/// A complete shell program (script or multi-line input).
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub items: Vec<CompleteCommand>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }
}

// ── complete command ─────────────────────────────────────────────────

/// One complete command (possibly backgrounded): `list &` or `list`.
#[derive(Clone, Debug)]
pub struct CompleteCommand {
    pub list: AndOrList,
    pub background: bool,
}

// ── and-or list ──────────────────────────────────────────────────────

/// A list of pipelines connected by `&&` / `||`.
#[derive(Clone, Debug)]
pub struct AndOrList {
    pub first: Pipeline,
    pub rest: Vec<(AndOr, Pipeline)>,
}

#[derive(Clone, Debug)]
pub enum AndOr {
    And,
    Or,
}

// ── pipeline ─────────────────────────────────────────────────────────

/// A (possibly negated) pipeline: `! cmd1 | cmd2 | ...`
#[derive(Clone, Debug)]
pub struct Pipeline {
    pub negation: bool,
    pub commands: Vec<Command>,
}

// ── command ──────────────────────────────────────────────────────────

/// A command — either simple or compound.
///
/// The `name` and `args` fields are populated for simple commands and
/// are empty for compound commands.  The executor dispatches on `kind`.
#[derive(Clone, Debug, Default)]
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
    pub kind: CommandKind,
}

impl Command {
    pub fn new(command_name: &str) -> Self {
        Self {
            name: command_name.to_string(),
            ..Default::default()
        }
    }

    pub fn new_with_args(command_name: &str, command_args: Vec<String>) -> Self {
        Self {
            name: command_name.to_string(),
            args: command_args,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum CommandKind {
    #[default]
    Simple,
    Subshell(Program),
    BraceGroup(Program),
    If(IfClause),
    While(WhileClause),
    For(ForClause),
    Case(CaseClause),
}

// ── compound command payloads ────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct IfClause {
    pub condition: Program,
    pub body: Program,
    pub elifs: Vec<(Program, Program)>,
    pub else_body: Option<Program>,
}

#[derive(Clone, Debug)]
pub struct WhileClause {
    pub condition: Program,
    pub body: Program,
}

#[derive(Clone, Debug)]
pub struct ForClause {
    pub name: String,
    pub words: Vec<String>,
    pub body: Program,
}

#[derive(Clone, Debug)]
pub struct CaseClause {
    pub word: String,
    pub arms: Vec<CaseArm>,
}

#[derive(Clone, Debug)]
pub struct CaseArm {
    pub patterns: Vec<String>,
    pub body: Program,
}

// ── trait ────────────────────────────────────────────────────────────

#[allow(unused)]
pub trait DashRegistry<R> {
    fn register(&self, name: &str, entry: Rc<RefCell<R>>);
    fn unregister(&self, name: &str);
    fn contains(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> anyhow::Result<Rc<RefCell<R>>>;
}
