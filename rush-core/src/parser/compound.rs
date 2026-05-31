//! Compound commands: subshell, brace group, if, while, for, case.



use crate::lexer::token::Token;
use crate::types::*;

use super::Parser;

impl<'a> Parser<'a> {
    // ── grammar: compound_command ────────────────────────────────────

    pub(crate) fn parse_compound_command(&mut self) -> anyhow::Result<Command> {
        match self.peek() {
            Some(Token::OpenParen) => {
                self.advance(); // consume '('
                let program = self.parse_program()?;
                self.expect_close_paren()?;
                Ok(Command {
                    kind: CommandKind::Subshell(program),
                    ..Default::default()
                })
            }
            Some(Token::Word(s, _)) => match s.as_str() {
                "{" => {
                    self.advance();
                    let program = self.parse_program()?;
                    self.expect_reserved("}")?;
                    Ok(Command {
                        kind: CommandKind::BraceGroup(program),
                        ..Default::default()
                    })
                }
                "if" => self.parse_if_clause(),
                "while" => self.parse_while_clause(),
                "until" => self.parse_while_clause(), // same grammar
                "for" => self.parse_for_clause(),
                "case" => self.parse_case_clause(),
                _ => Err(anyhow::anyhow!("unexpected reserved word: {s}")),
            },
            _ => Err(anyhow::anyhow!("expected compound command")),
        }
    }

    pub(crate) fn expect_close_paren(&mut self) -> anyhow::Result<()> {
        match self.advance() {
            Some(Token::CloseParen) => Ok(()),
            Some(t) => Err(anyhow::anyhow!("expected ')', found {t}")),
            None => Err(anyhow::anyhow!("expected ')', found end of input")),
        }
    }

    // ── if clause ────────────────────────────────────────────────────

    pub(crate) fn parse_if_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("if")?;
        let condition = self.parse_program()?; // reads until `then`
        self.expect_reserved("then")?;
        let body = self.parse_program()?; // reads until `elif`, `else`, or `fi`

        let mut elifs = Vec::new();
        while self.peek_is_reserved() {
            match self.peek() {
                Some(Token::Word(s, _)) if s == "elif" => {
                    self.advance();
                    let cond = self.parse_program()?;
                    self.expect_reserved("then")?;
                    let b = self.parse_program()?;
                    elifs.push((cond, b));
                }
                _ => break,
            }
        }

        let else_body = if self.peek_is_reserved() {
            match self.peek() {
                Some(Token::Word(s, _)) if s == "else" => {
                    self.advance();
                    Some(self.parse_program()?)
                }
                _ => None,
            }
        } else {
            None
        };

        self.expect_reserved("fi")?;

        Ok(Command {
            kind: CommandKind::If(IfClause {
                condition,
                body,
                elifs,
                else_body,
            }),
            ..Default::default()
        })
    }

    // ── while / until ────────────────────────────────────────────────

    pub(crate) fn parse_while_clause(&mut self) -> anyhow::Result<Command> {
        let _is_until = match self.peek() {
            Some(Token::Word(s, _)) if s == "until" => {
                self.advance();
                true
            }
            _ => {
                self.expect_reserved("while")?;
                false
            }
        };

        let condition = self.parse_program()?;
        self.expect_reserved("do")?;
        let body = self.parse_program()?;
        self.expect_reserved("done")?;

        let clause = WhileClause { condition, body };

        Ok(Command {
            kind: CommandKind::While(clause), // TODO: distinguish While vs Until
            ..Default::default()
        })
    }

    // ── for ──────────────────────────────────────────────────────────

    pub(crate) fn parse_for_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("for")?;
        let name = String::from(self.expect_ident()?.as_str());

        // Optional `in words...`
        let words = if self.peek_is_reserved() {
            match self.peek() {
                Some(Token::Word(s, _)) if s == "in" => {
                    self.advance();
                    let mut w = Vec::new();
                    loop {
                        match self.peek() {
                            Some(Token::Word(s, _))
                                if !Self::is_reserved(s) || s == "do" =>
                            {
                                if s == "do" {
                                    break;
                                }
                                w.push(String::from(s.as_str()));
                                self.advance();
                            }
                            _ => break,
                        }
                    }
                    w
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };

        self.skip_separators();
        self.expect_reserved("do")?;
        let body = self.parse_program()?;
        self.expect_reserved("done")?;

        Ok(Command {
            kind: CommandKind::For(ForClause {
                name,
                words,
                body,
            }),
            ..Default::default()
        })
    }

    // ── case ─────────────────────────────────────────────────────────

    pub(crate) fn parse_case_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("case")?;
        let word = String::from(self.expect_ident()?.as_str());
        self.expect_reserved("in")?;
        self.skip_separators();

        let mut arms = Vec::new();
        loop {
            if self.peek_is_reserved()
                && let Some(Token::Word(s, _)) = self.peek()
                && s == "esac"
            {
                break;
            }
            if self.is_at_end() {
                return Err(anyhow::anyhow!("expected 'esac', found end of input"));
            }

            // Parse pattern
            let mut patterns = Vec::new();
            if let Some(Token::OpenParen) = self.peek() {
                self.advance(); // optional leading '('
            }
            loop {
                let pat = self.expect_ident()?;
                patterns.push(String::from(pat.as_str()));
                match self.peek() {
                    Some(Token::Pipe) => {
                        self.advance();
                        continue;
                    }
                    _ => break,
                }
            }
            self.expect_close_paren()?;
            self.skip_separators();

            // Body: parse commands until `;;`
            let mut body_items = Vec::new();
            self.stop_at_dsemi = true;
            loop {
                if let Some(Token::Semicolon) = self.peek() {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance(); // consume second
                        break;
                    }
                    continue;
                }
                if self.peek_is_reserved()
                    && let Some(Token::Word(s, _)) = self.peek()
                    && (s == "esac" || s == "fi" || s == "done" || s == "elif"
                        || s == "else" || s == "then" || s == "do")
                {
                    break;
                }
                if self.is_at_end() {
                    break;
                }
                let cmd = self.parse_complete_command()?;
                body_items.push(cmd);
                while matches!(self.peek(), Some(Token::Newline)) {
                    self.advance();
                }
            }
            self.stop_at_dsemi = false;
            let body = Program { items: body_items };

            self.skip_separators();
            arms.push(CaseArm { patterns, body });
        }

        self.expect_reserved("esac")?;

        Ok(Command {
            kind: CommandKind::Case(CaseClause { word, arms }),
            ..Default::default()
        })
    }
}
