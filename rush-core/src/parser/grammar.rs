//! Core grammar: program, complete_command, and_or, pipeline, command, simple_command.

use crate::lexer::token::Token;
use crate::types::*;

use super::{Action, Parser};

impl<'a> Parser<'a> {
    // ── grammar: program ─────────────────────────────────────────────

    pub(crate) fn parse_program(&mut self) -> anyhow::Result<Program> {
        let mut items = Vec::new();
        self.skip_separators();

        while !self.is_at_end() {
            // Stop at compound-command terminators (fi, done, esac, etc.)
            if self.at_terminator() {
                break;
            }
            let cmd = self.parse_complete_command()?;
            items.push(cmd);

            // Separators between commands
            if self.is_separator() {
                self.skip_separators();
            } else if !self.is_at_end() {
                // If not at end and no separator, the next token
                // starts a new command (no explicit separator needed
                // at EOF).
            }
        }

        Ok(Program { items })
    }

    // ── grammar: complete_command ────────────────────────────────────

    pub(crate) fn parse_complete_command(&mut self) -> anyhow::Result<CompleteCommand> {
        let list = self.parse_and_or()?;
        let background = match self.peek() {
            Some(Token::Background) => {
                self.advance();
                true
            }
            _ => false,
        };
        Ok(CompleteCommand { list, background })
    }

    // ── grammar: and_or ──────────────────────────────────────────────

    pub(crate) fn parse_and_or(&mut self) -> anyhow::Result<AndOrList> {
        let first = self.parse_pipeline()?;
        let mut rest = Vec::new();

        loop {
            self.skip_separators();
            match self.peek() {
                Some(Token::AndIf) => {
                    self.advance();
                    self.skip_separators();
                    let p = self.parse_pipeline()?;
                    rest.push((AndOr::And, p));
                }
                Some(Token::OrIf) => {
                    self.advance();
                    self.skip_separators();
                    let p = self.parse_pipeline()?;
                    rest.push((AndOr::Or, p));
                }
                _ => break,
            }
        }

        Ok(AndOrList { first, rest })
    }

    // ── grammar: pipeline ────────────────────────────────────────────

    pub(crate) fn parse_pipeline(&mut self) -> anyhow::Result<Pipeline> {
        // Leading `!` negation
        let negation = match self.peek() {
            Some(Token::Word(s, _)) if s == "!" => {
                self.advance();
                true
            }
            _ => false,
        };

        let mut commands = Vec::new();
        commands.push(self.parse_command()?);

        loop {
            self.skip_separators();
            match self.peek() {
                Some(Token::Pipe) => {
                    self.advance();
                    self.skip_separators();
                    commands.push(self.parse_command()?);
                }
                _ => break,
            }
        }

        Ok(Pipeline { negation, commands })
    }

    // ── grammar: command ─────────────────────────────────────────────

    pub(crate) fn parse_command(&mut self) -> anyhow::Result<Command> {
        if self.peek_is_reserved() {
            return self.parse_compound_command();
        }

        // Check for subshell by opening paren
        if let Some(Token::OpenParen) = self.peek() {
            return self.parse_compound_command();
        }

        self.parse_simple_command()
    }

    // ── grammar: simple_command ──────────────────────────────────────

    pub(crate) fn parse_simple_command(&mut self) -> anyhow::Result<Command> {
        let mut name: Option<String> = None;
        let mut args: Vec<String> = Vec::new();
        let mut redirects: Vec<Redirect> = Vec::new();
        let mut pending_op: Option<RedirectOp> = None;
        let mut pending_fd: Option<i32> = None;

        loop {
            // Peek at the next token and decide what to do.
            let action = match self.peek() {
                None | Some(Token::Eof) => None,
                Some(Token::Semicolon) | Some(Token::Newline) => None,
                Some(Token::Pipe) => None,
                Some(Token::AndIf) | Some(Token::OrIf) => None,
                Some(Token::Background) => None,
                Some(Token::OpenParen) | Some(Token::CloseParen) => None,

                Some(Token::Less) => Some(Action::Redirect(RedirectOp::Less)),
                Some(Token::Great) => Some(Action::Redirect(RedirectOp::Great)),
                Some(Token::DGreat) => Some(Action::Redirect(RedirectOp::DGreat)),
                Some(Token::LessAnd) => Some(Action::Redirect(RedirectOp::LessAnd)),
                Some(Token::GreatAnd) => Some(Action::Redirect(RedirectOp::GreatAnd)),
                Some(Token::LessGreat) => Some(Action::Redirect(RedirectOp::LessGreat)),
                Some(Token::DLess) => Some(Action::Redirect(RedirectOp::DLess)),
                Some(Token::DLessDash) => Some(Action::Redirect(RedirectOp::DLessDash)),
                Some(Token::Clobber) => Some(Action::Redirect(RedirectOp::Clobber)),
                Some(Token::Word(s, q)) => Some(Action::Word(s.clone(), *q)),
            };

            match action {
                None => break,
                Some(Action::Redirect(op)) => {
                    // If the previous word was a bare fd number, steal it.
                    // Check both the command name and the last argument.
                    let steal_fd = |s: &str| s.chars().all(|c| c.is_ascii_digit());
                    if let Some(ref n) = name
                        && steal_fd(n)
                    {
                        pending_fd = n.parse().ok();
                        name = None;
                    } else if let Some(last) = args.last()
                        && steal_fd(last)
                    {
                        pending_fd = last.parse().ok();
                        args.pop();
                    }
                    self.advance();
                    pending_op = Some(op);
                }
                Some(Action::Word(word, quote)) => {
                    self.advance();
                    // Single-quoted words get a \x01 prefix so the
                    // expansion pass can skip them.
                    let value: String = if quote == crate::lexer::token::QuoteKind::SingleQuoted {
                        format!("\x01{word}").to_string()
                    } else {
                        word.to_string()
                    };
                    if let Some(op) = pending_op.take() {
                        redirects.push(Redirect {
                            op,
                            src_fd: pending_fd.take(),
                            target: value,
                        });
                    } else if name.is_none() {
                        name = Some(value);
                    } else {
                        args.push(value);
                    }
                }
            }
        }

        Ok(Command {
            name: name.unwrap_or_default(),
            args,
            redirects,
            kind: CommandKind::Simple,
        })
    }
}
