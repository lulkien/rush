use abi_stable::std_types::{RString, RVec};
use anyhow::anyhow;

use crate::lexer::token::tokenize_with_logos;
pub use crate::lexer::token::Token;
use crate::types::{Command, CommandPipe, CommandPipeList, Redirect};

pub mod token;
#[cfg(test)]
mod tests;

pub struct Lexer {
    input: String,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
        }
    }

    pub fn tokenize(&self) -> Vec<Token> {
        tokenize_with_logos(&self.input)
    }

    /// Parse a line (or multi-line input) into a `CommandPipeList`.
    ///
    /// POSIX grammar handled:
    ///   `cmd1 | cmd2`          — pipeline
    ///   `cmd1; cmd2`           — sequential
    ///   `cmd1 && cmd2`         — AND list (treated as sequential for now)
    ///   `cmd1 || cmd2`         — OR list  (treated as sequential for now)
    ///   `cmd &`                — background (flag set, treated as sequential)
    ///   `cmd < file`           — input redirect
    ///   `cmd > file` / `>>`    — output redirect
    ///   `cmd <& N` / `>& N`    — fd duplication
    ///
    /// Not yet handled in the executor (lexer recognises, parser stores):
    ///   subshells `(list)`, grouping `{ list; }`
    pub fn parse_line(&self) -> anyhow::Result<CommandPipeList> {
        let tokens = self.tokenize();

        let mut current_cmd: Option<RString> = None;
        let mut current_args: RVec<RString> = RVec::new();
        let mut current_redirects: Vec<Redirect> = Vec::new();
        let mut current_list = CommandPipeList::new();
        let mut current_pipe = CommandPipe::new();
        let mut pending_redirect_op: Option<crate::types::RedirectOp> = None;

        for token in tokens {
            log::debug!("Token: {token}");
            match token {
                // ── end of input ──────────────────────────────
                Token::Eof => {
                    flush_command(
                        &mut current_cmd,
                        &mut current_args,
                        &mut current_redirects,
                        false,
                        &mut current_pipe,
                    );
                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }
                    break;
                }

                // ── command separators ────────────────────────
                Token::Semicolon | Token::Newline => {
                    flush_command(
                        &mut current_cmd,
                        &mut current_args,
                        &mut current_redirects,
                        false,
                        &mut current_pipe,
                    );
                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }
                    current_pipe = CommandPipe::new();
                }

                Token::AndIf | Token::OrIf => {
                    // TODO: conditional execution
                    flush_command(
                        &mut current_cmd,
                        &mut current_args,
                        &mut current_redirects,
                        false,
                        &mut current_pipe,
                    );
                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }
                    current_pipe = CommandPipe::new();
                }

                Token::Background => {
                    flush_command(
                        &mut current_cmd,
                        &mut current_args,
                        &mut current_redirects,
                        true, // background
                        &mut current_pipe,
                    );
                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }
                    current_pipe = CommandPipe::new();
                }

                // ── pipeline ──────────────────────────────────
                Token::Pipe => {
                    if current_cmd.is_none() && current_pipe.is_empty() {
                        return Err(anyhow!("Syntax error: pipe without command"));
                    }
                    flush_command(
                        &mut current_cmd,
                        &mut current_args,
                        &mut current_redirects,
                        false,
                        &mut current_pipe,
                    );
                    // Don't flush the pipe — we're continuing this pipeline.
                }

                // ── redirect operators ────────────────────────
                Token::Less => pending_redirect_op = Some(crate::types::RedirectOp::Less),
                Token::Great => pending_redirect_op = Some(crate::types::RedirectOp::Great),
                Token::DGreat => pending_redirect_op = Some(crate::types::RedirectOp::DGreat),
                Token::LessAnd => pending_redirect_op = Some(crate::types::RedirectOp::LessAnd),
                Token::GreatAnd => pending_redirect_op = Some(crate::types::RedirectOp::GreatAnd),
                Token::LessGreat => pending_redirect_op = Some(crate::types::RedirectOp::LessGreat),
                Token::DLess => pending_redirect_op = Some(crate::types::RedirectOp::DLess),
                Token::DLessDash => pending_redirect_op = Some(crate::types::RedirectOp::DLessDash),
                Token::Clobber => pending_redirect_op = Some(crate::types::RedirectOp::Clobber),

                // ── grouping (not yet implemented) ────────────
                Token::OpenParen | Token::CloseParen => {
                    return Err(anyhow!(
                        "Syntax error: {} not yet supported",
                        token
                    ));
                }

                // ── words ─────────────────────────────────────
                Token::Ident(word) => {
                    if let Some(op) = pending_redirect_op.take() {
                        // This word is the target of the preceding redirect.
                        current_redirects.push(Redirect {
                            op,
                            target: word.into(),
                        });
                    } else if current_cmd.is_none() {
                        current_cmd = Some(word.into());
                    } else {
                        current_args.push(word.into());
                    }
                }
            }
        }

        Ok(current_list)
    }
}

/// Build a `Command` from the accumulated state and push it into the pipe.
fn flush_command(
    cmd: &mut Option<RString>,
    args: &mut RVec<RString>,
    redirects: &mut Vec<Redirect>,
    _background: bool,
    pipe: &mut CommandPipe,
) {
    if let Some(name) = cmd.take() {
        let command = Command {
            name,
            args: std::mem::take(args),
            redirects: std::mem::take(redirects),
            kind: Default::default(),
        };
        pipe.append_command(command);
    }
    args.clear();
    redirects.clear();
}

