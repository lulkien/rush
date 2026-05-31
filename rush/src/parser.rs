//! Recursive-descent parser for POSIX shell grammar.
//!
//! Consumes the token stream produced by the logos lexer and emits a
//! [`Program`](crate::types::Program) AST.

use abi_stable::std_types::{RString, RVec};

use crate::lexer::token::Token;
use crate::types::*;

// ── entry point ──────────────────────────────────────────────────────

/// Parse an entire token stream into a `Program`.
pub fn parse(tokens: &[Token]) -> anyhow::Result<Program> {
    let mut p = Parser::new(tokens);
    let program = p.parse_program()?;
    Ok(program)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// When true, `skip_separators` stops before consuming `;;`.
    /// Set during case-item body parsing.
    stop_at_dsemi: bool,
}

/// Internal dispatch to avoid borrow conflicts in the command parser.
enum Action {
    Redirect(RedirectOp),
    Word(String),
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            stop_at_dsemi: false,
        }
    }

    // ── helpers ──────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn expect_ident(&mut self) -> anyhow::Result<String> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            Some(t) => Err(anyhow::anyhow!("expected word, found {t}")),
            None => Err(anyhow::anyhow!("unexpected end of input")),
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Eof))
    }

    /// True if the next token is a reserved word that closes a compound
    /// command body (fi, done, esac, then, else, elif, do, }).
    fn at_terminator(&self) -> bool {
        match self.peek() {
            Some(Token::CloseParen) => true,
            Some(Token::Ident(s)) => matches!(
                s.as_str(),
                "fi" | "done" | "esac" | "then" | "else" | "elif" | "do" | "}"
            ),
            _ => false,
        }
    }

    fn is_separator(&self) -> bool {
        matches!(self.peek(), Some(Token::Semicolon) | Some(Token::Newline))
    }

    /// Skip semicolons and newlines (line breaks).
    /// When `stop_at_dsemi` is true, stops before consuming a second
    /// consecutive semicolon (preserves `;;` for case-arm termination).
    fn skip_separators(&mut self) {
        while self.is_separator() {
            // If stopping at ;;, only consume single semicolons.
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
            // Normal mode: consume all separators
            self.advance();
        }
    }

    // ── reserved words ───────────────────────────────────────────

    fn is_reserved(s: &str) -> bool {
        matches!(
            s,
            "if" | "then" | "else" | "elif" | "fi"
                | "for" | "while" | "until" | "do" | "done"
                | "case" | "esac" | "in"
                | "{" | "}" | "!" | "function"
        )
    }

    fn peek_is_reserved(&self) -> bool {
        match self.peek() {
            Some(Token::Ident(s)) => Self::is_reserved(s),
            _ => false,
        }
    }

    /// Consume a reserved word and return it, or error.
    fn expect_reserved(&mut self, word: &str) -> anyhow::Result<()> {
        match self.advance() {
            Some(Token::Ident(s)) if s == word => Ok(()),
            Some(t) => Err(anyhow::anyhow!("expected '{word}', found {t}")),
            None => Err(anyhow::anyhow!("expected '{word}', found end of input")),
        }
    }

    // ── grammar: program ─────────────────────────────────────────

    fn parse_program(&mut self) -> anyhow::Result<Program> {
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

    // ── grammar: complete_command ────────────────────────────────

    fn parse_complete_command(&mut self) -> anyhow::Result<CompleteCommand> {
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

    // ── grammar: and_or ──────────────────────────────────────────

    fn parse_and_or(&mut self) -> anyhow::Result<AndOrList> {
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

    // ── grammar: pipeline ────────────────────────────────────────

    fn parse_pipeline(&mut self) -> anyhow::Result<Pipeline> {
        // Leading `!` negation
        let negation = match self.peek() {
            Some(Token::Ident(s)) if s == "!" => {
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

    // ── grammar: command ─────────────────────────────────────────

    fn parse_command(&mut self) -> anyhow::Result<Command> {
        if self.peek_is_reserved() {
            return self.parse_compound_command();
        }

        // Check for subshell by opening paren
        if let Some(Token::OpenParen) = self.peek() {
            return self.parse_compound_command();
        }

        self.parse_simple_command()
    }

    // ── grammar: simple_command ──────────────────────────────────

    fn parse_simple_command(&mut self) -> anyhow::Result<Command> {
        let mut name: Option<RString> = None;
        let mut args = RVec::new();
        let mut redirects: Vec<Redirect> = Vec::new();
        let mut pending_op: Option<RedirectOp> = None;

        loop {
            // Peek at the next token and decide what to do.
            // We clone the string early to avoid holding a borrow across advance().
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
                Some(Token::Ident(s)) => Some(Action::Word(s.clone())),
            };

            match action {
                None => break,
                Some(Action::Redirect(op)) => {
                    self.advance();
                    pending_op = Some(op);
                }
                Some(Action::Word(word)) => {
                    self.advance();
                    if let Some(op) = pending_op.take() {
                        redirects.push(Redirect {
                            op,
                            target: word.into(),
                        });
                    } else if name.is_none() {
                        name = Some(word.into());
                    } else {
                        args.push(word.into());
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

    // ── grammar: compound_command ────────────────────────────────

    fn parse_compound_command(&mut self) -> anyhow::Result<Command> {
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
            Some(Token::Ident(s)) => match s.as_str() {
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

    fn expect_close_paren(&mut self) -> anyhow::Result<()> {
        match self.advance() {
            Some(Token::CloseParen) => Ok(()),
            Some(t) => Err(anyhow::anyhow!("expected ')', found {t}")),
            None => Err(anyhow::anyhow!("expected ')', found end of input")),
        }
    }

    // ── if clause ────────────────────────────────────────────────

    fn parse_if_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("if")?;
        let condition = self.parse_program()?; // reads until `then`
        self.expect_reserved("then")?;
        let body = self.parse_program()?; // reads until `elif`, `else`, or `fi`

        let mut elifs = Vec::new();
        while self.peek_is_reserved() {
            match self.peek() {
                Some(Token::Ident(s)) if s == "elif" => {
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
                Some(Token::Ident(s)) if s == "else" => {
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

    // ── while / until ────────────────────────────────────────────

    fn parse_while_clause(&mut self) -> anyhow::Result<Command> {
        let _is_until = match self.peek() {
            Some(Token::Ident(s)) if s == "until" => {
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

    // ── for ──────────────────────────────────────────────────────

    fn parse_for_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("for")?;
        let name = RString::from(self.expect_ident()?.as_str());

        // Optional `in words...`
        let words = if self.peek_is_reserved() {
            match self.peek() {
                Some(Token::Ident(s)) if s == "in" => {
                    self.advance();
                    let mut w = Vec::new();
                    loop {
                        match self.peek() {
                            Some(Token::Ident(s))
                                if !Self::is_reserved(s) || s == "do" =>
                            {
                                // `do` terminates the word list, don't consume it
                                if s == "do" {
                                    break;
                                }
                                w.push(RString::from(s.as_str()));
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
            // `for name; do ...` — iterate over "$@"
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

    // ── case ─────────────────────────────────────────────────────

    fn parse_case_clause(&mut self) -> anyhow::Result<Command> {
        self.expect_reserved("case")?;
        let word = RString::from(self.expect_ident()?.as_str());
        self.expect_reserved("in")?;
        self.skip_separators();

        let mut arms = Vec::new();
        loop {
            if self.peek_is_reserved()
                && let Some(Token::Ident(s)) = self.peek()
                    && s == "esac" {
                        break;
                    }
            if self.is_at_end() {
                return Err(anyhow::anyhow!("expected 'esac', found end of input"));
            }

            // Parse pattern: one pattern or '(' pattern ... ')'
            let mut patterns = Vec::new();
            if let Some(Token::OpenParen) = self.peek() {
                self.advance(); // optional leading '('
            }
            loop {
                let pat = self.expect_ident()?;
                patterns.push(RString::from(pat.as_str()));
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

            // Body: parse commands until we hit `;;`
            let mut body_items = Vec::new();
            self.stop_at_dsemi = true;
            loop {
                // Check for `;;` terminator BEFORE parsing (so we don't
                // let parse_and_or consume it via skip_separators).
                if let Some(Token::Semicolon) = self.peek() {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance(); // consume second
                        break;
                    }
                    // Single `;` — this is a command separator.
                    // Our parser's `parse_and_or` would skip it anyway,
                    // so just continue to the next command.
                    continue;
                }
                // Stop on reserved words that end the case
                if self.peek_is_reserved()
                    && let Some(Token::Ident(s)) = self.peek()
                        && (s == "esac" || s == "fi" || s == "done" || s == "elif"
                            || s == "else" || s == "then" || s == "do")
                        {
                            break;
                        }
                if self.is_at_end() {
                    break;
                }
                // Parse one command.  parse_complete_command will call
                // parse_and_or, which calls skip_separators() after the
                // pipeline — this will eat any `;` separators BETWEEN
                // commands.  That's fine because we already checked for
                // `;;` above.
                let cmd = self.parse_complete_command()?;
                body_items.push(cmd);
                // After the command, skip only newlines (parse_and_or
                // already ate any leading `;` separators for us).
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

// ── tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_input(input: &str) -> Program {
        let lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        parse(&tokens).unwrap()
    }

    fn first_command(program: &Program) -> &Command {
        &program.items[0].list.first.commands[0]
    }

    // ── simple commands ──────────────────────────────────────────

    #[test]
    fn simple_command_bare() {
        let p = parse_input("echo hello");
        assert_eq!(p.items.len(), 1);
        let cmd = first_command(&p);
        assert_eq!(cmd.name.as_str(), "echo");
        assert_eq!(cmd.args.len(), 1);
        assert_eq!(cmd.args[0].as_str(), "hello");
    }

    #[test]
    fn pipeline_two_commands() {
        let p = parse_input("echo hello | cat");
        assert_eq!(p.items.len(), 1);
        let pipe = &p.items[0].list.first;
        assert_eq!(pipe.commands.len(), 2);
        assert_eq!(pipe.commands[0].name.as_str(), "echo");
        assert_eq!(pipe.commands[1].name.as_str(), "cat");
    }

    #[test]
    fn semicolon_separated() {
        let p = parse_input("echo a; echo b");
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].list.first.commands[0].name.as_str(), "echo");
        assert_eq!(p.items[1].list.first.commands[0].name.as_str(), "echo");
    }

    #[test]
    fn newline_separated() {
        let p = parse_input("echo a\necho b\n");
        assert_eq!(p.items.len(), 2);
    }

    #[test]
    fn background_command() {
        let p = parse_input("sleep 10 &");
        assert!(p.items[0].background);
    }

    // ── and-or lists ──────────────────────────────────────────────

    #[test]
    fn and_list() {
        let p = parse_input("gcc -c foo.c && gcc -c bar.c");
        assert_eq!(p.items.len(), 1);
        let list = &p.items[0].list;
        assert_eq!(list.first.commands[0].name.as_str(), "gcc");
        assert_eq!(list.rest.len(), 1);
    }

    #[test]
    fn or_list() {
        let p = parse_input("cmd1 || cmd2");
        let list = &p.items[0].list;
        assert_eq!(list.rest.len(), 1);
    }

    // ── compound commands ────────────────────────────────────────

    #[test]
    fn subshell() {
        let p = parse_input("(echo hello)");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::Subshell(inner) => {
                assert_eq!(inner.items.len(), 1);
            }
            _ => panic!("expected subshell"),
        }
    }

    #[test]
    fn if_clause_simple() {
        let p = parse_input("if true; then echo yes; fi");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::If(ifc) => {
                assert_eq!(ifc.condition.items.len(), 1);
                assert_eq!(ifc.body.items.len(), 1);
                assert!(ifc.elifs.is_empty());
                assert!(ifc.else_body.is_none());
            }
            _ => panic!("expected if"),
        }
    }

    #[test]
    fn if_clause_with_else() {
        let p = parse_input("if false; then echo no; else echo yes; fi");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::If(ifc) => {
                assert!(ifc.else_body.is_some());
            }
            _ => panic!("expected if"),
        }
    }

    #[test]
    fn if_clause_with_elif() {
        let p = parse_input("if false; then echo no; elif true; then echo yes; fi");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::If(ifc) => {
                assert_eq!(ifc.elifs.len(), 1);
            }
            _ => panic!("expected if"),
        }
    }

    #[test]
    fn while_loop() {
        let p = parse_input("while true; do echo hi; done");
        let cmd = first_command(&p);
        matches!(&cmd.kind, CommandKind::While(_));
    }

    #[test]
    fn for_loop() {
        let p = parse_input("for f in a b c; do echo $f; done");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::For(fc) => {
                assert_eq!(fc.name.as_str(), "f");
                assert_eq!(fc.words.len(), 3);
            }
            _ => panic!("expected for"),
        }
    }

    #[test]
    fn for_loop_multiline() {
        let p = parse_input("for f in a b c; do\necho $f\ndone");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::For(fc) => {
                assert_eq!(fc.name.as_str(), "f");
                assert_eq!(fc.words.len(), 3);
                assert_eq!(fc.body.items.len(), 1);
            }
            _ => panic!("expected for, got {:?}", cmd.kind),
        }
    }

    #[test]
    fn case_single_arm() {
        let p = parse_input("case x in a) echo hi;; esac");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::Case(cc) => {
                assert_eq!(cc.arms.len(), 1);
                assert_eq!(cc.arms[0].patterns.len(), 1);
                assert_eq!(cc.arms[0].patterns[0].as_str(), "a");
            }
            _ => panic!("expected case, got {:?}", cmd.kind),
        }
    }

    #[test]
    fn case_two_arms() {
        let p = parse_input("case $x in a) echo one;; b) echo two;; esac");
        let cmd = first_command(&p);
        match &cmd.kind {
            CommandKind::Case(cc) => {
                assert_eq!(cc.arms.len(), 2);
                assert_eq!(cc.arms[0].patterns.len(), 1);
            }
            _ => panic!("expected case"),
        }
    }

    #[test]
    fn pipeline_with_negation() {
        let p = parse_input("! grep foo");
        assert!(p.items[0].list.first.negation);
    }

    #[test]
    fn redirect_in_simple_command() {
        let p = parse_input("echo hello > out.txt");
        let cmd = first_command(&p);
        assert_eq!(cmd.redirects.len(), 1);
        assert_eq!(cmd.redirects[0].target.as_str(), "out.txt");
    }

    #[test]
    fn multiple_commands_with_newlines() {
        let p = parse_input("echo a\nif true; then echo b; fi\necho c");
        assert_eq!(p.items.len(), 3);
    }
}
