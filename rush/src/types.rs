use std::{cell::RefCell, rc::Rc};

use abi_stable::std_types::{RString, RVec};

/// POSIX redirection operator.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum RedirectOp {
    /// `<`  — redirect stdin from file
    #[default]
    Less,
    /// `>`  — redirect stdout to file (truncate)
    Great,
    /// `>>` — redirect stdout to file (append)
    DGreat,
    /// `<&` — duplicate input fd
    LessAnd,
    /// `>&` — duplicate output fd
    GreatAnd,
    /// `<>` — open file for read/write on stdin
    LessGreat,
    /// `<<` — here-document
    DLess,
    /// `<<-` — here-document with leading tab strip
    DLessDash,
    /// `>|` — clobber (force overwrite even with noclobber set)
    Clobber,
}

/// A redirect operator paired with its target word (filename or fd).
#[derive(Clone, Debug, Default)]
pub struct Redirect {
    pub op: RedirectOp,
    pub target: RString,
}

/// A single command: name, arguments, I/O redirects, and background flag.
#[derive(Clone, Debug, Default)]
pub struct Command {
    pub name: RString,
    pub args: RVec<RString>,
    pub redirects: Vec<Redirect>,
    pub background: bool,
}

impl Command {
    pub fn new(command_name: &str) -> Self {
        Self {
            name: command_name.into(),
            ..Default::default()
        }
    }

    pub fn new_with_args(command_name: &str, command_args: RVec<RString>) -> Self {
        Self {
            name: command_name.into(),
            args: command_args,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandPipeList(Vec<CommandPipe>);

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

#[derive(Clone, Debug, Default)]
pub struct CommandPipe(Vec<Command>);

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

#[allow(unused)]
pub trait DashRegistry<R> {
    fn register(&self, name: &str, entry: Rc<RefCell<R>>);
    fn unregister(&self, name: &str);
    fn contains(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> anyhow::Result<Rc<RefCell<R>>>;
}
