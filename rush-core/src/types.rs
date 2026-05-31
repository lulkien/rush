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

/// A redirect operator paired with its target word.
#[derive(Clone, Debug, Default)]
pub struct Redirect {
    pub op: RedirectOp,
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

// ── legacy helpers (used by executor) ────────────────────────────────

/// Return the flat list of simple commands from a pipeline.
/// Compound commands in the pipeline are ignored by the executor.
pub fn pipeline_commands(pipeline: &Pipeline) -> Vec<&Command> {
    pipeline.commands.iter().collect()
}

/// Walk an and-or list and call `f` for each simple pipeline.
pub fn walk_and_or(list: &AndOrList, mut f: impl FnMut(&Pipeline)) {
    f(&list.first);
    for (_, p) in &list.rest {
        f(p);
    }
}

// ── legacy flat types (used by pipe tests and executor) ──────────────

/// Flat list of pipe-separated commands (no AST nesting).
/// Kept for the integration-test harness; prefer `Pipeline` for new code.
#[derive(Clone, Debug, Default)]
pub struct CommandPipe(pub Vec<Command>);

impl CommandPipe {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn append_command(&mut self, command: Command) {
        self.0.push(command);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for CommandPipe {
    type Item = Command;
    type IntoIter = std::vec::IntoIter<Command>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Flat list of semicolon-separated pipe groups.
#[derive(Clone, Debug, Default)]
pub struct CommandPipeList(pub Vec<CommandPipe>);

impl CommandPipeList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn append_pipe(&mut self, pipe: CommandPipe) {
        self.0.push(pipe);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for CommandPipeList {
    type Item = CommandPipe;
    type IntoIter = std::vec::IntoIter<CommandPipe>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// ── trait ────────────────────────────────────────────────────────────

#[allow(unused)]
pub trait DashRegistry<R> {
    fn register(&self, name: &str, entry: Rc<RefCell<R>>);
    fn unregister(&self, name: &str);
    fn contains(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> anyhow::Result<Rc<RefCell<R>>>;
}
