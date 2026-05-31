//! Recursive-descent parser for POSIX shell grammar.
//!
//! Consumes the token stream produced by the logos lexer and emits a
//! [`Program`](crate::types::Program) AST.

use crate::lexer::token::Token;
use crate::types::*;

mod compound;
mod grammar;
#[cfg(test)]
mod tests;

// ── entry point ──────────────────────────────────────────────────────

/// Parse an entire token stream into a `Program`.
pub fn parse(tokens: &[Token]) -> anyhow::Result<Program> {
    let mut p = Parser::new(tokens);
    let program = p.parse_program()?;
    Ok(program)
}

pub(crate) struct Parser<'a> {
    pub(crate) tokens: &'a [Token],
    pub(crate) pos: usize,
    /// When true, `skip_separators` stops before consuming `;;`.
    /// Set during case-item body parsing.
    pub(crate) stop_at_dsemi: bool,
}

/// Internal dispatch to avoid borrow conflicts in the command parser.
pub(crate) enum Action {
    Redirect(RedirectOp),
    Word(String, crate::lexer::token::QuoteKind),
}

impl<'a> Parser<'a> {
    pub(crate) fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            stop_at_dsemi: false,
        }
    }

    // ── helpers ──────────────────────────────────────────────────

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub(crate) fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    pub(crate) fn expect_ident(&mut self) -> anyhow::Result<String> {
        match self.advance() {
            Some(Token::Word(s, _)) => Ok(s.clone()),
            Some(t) => Err(anyhow::anyhow!("expected word, found {t}")),
            None => Err(anyhow::anyhow!("unexpected end of input")),
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Eof))
    }

    /// True if the next token is a reserved word that closes a compound
    /// command body (fi, done, esac, then, else, elif, do, }).
    pub(crate) fn at_terminator(&self) -> bool {
        match self.peek() {
            Some(Token::CloseParen) => true,
            Some(Token::Word(s, _)) => matches!(
                s.as_str(),
                "fi" | "done" | "esac" | "then" | "else" | "elif" | "do" | "}"
            ),
            _ => false,
        }
    }

    pub(crate) fn is_separator(&self) -> bool {
        matches!(self.peek(), Some(Token::Semicolon) | Some(Token::Newline))
    }

    /// Skip semicolons and newlines (line breaks).
    /// When `stop_at_dsemi` is true, stops before consuming a second
    /// consecutive semicolon (preserves `;;` for case-arm termination).
    pub(crate) fn skip_separators(&mut self) {
        while self.is_separator() {
            if self.stop_at_dsemi
                && let Some(Token::Semicolon) = self.peek()
            {
                self.advance();
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.pos -= 1;
                    break;
                }
                continue;
            }
            self.advance();
        }
    }

    // ── reserved words ───────────────────────────────────────────

    pub(crate) fn is_reserved(s: &str) -> bool {
        matches!(
            s,
            "if" | "then" | "else" | "elif" | "fi"
                | "for" | "while" | "until" | "do" | "done"
                | "case" | "esac" | "in"
                | "{" | "}" | "!" | "function"
        )
    }

    pub(crate) fn peek_is_reserved(&self) -> bool {
        match self.peek() {
            Some(Token::Word(s, _)) => Self::is_reserved(s),
            _ => false,
        }
    }

    /// Consume a reserved word and return it, or error.
    pub(crate) fn expect_reserved(&mut self, word: &str) -> anyhow::Result<()> {
        match self.advance() {
            Some(Token::Word(s, _)) if s == word => Ok(()),
            Some(t) => Err(anyhow::anyhow!("expected '{word}', found {t}")),
            None => Err(anyhow::anyhow!("expected '{word}', found end of input")),
        }
    }
}
