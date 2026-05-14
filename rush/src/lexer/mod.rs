use abi_stable::std_types::{RString, RVec};
use anyhow::anyhow;
use shlex::Shlex;

use crate::types::{Command, CommandPipeList, CommandPipe};
use crate::lexer::token::{Token, get_keyword_token};

mod token;

pub struct Lexer<'a>(Shlex<'a>);

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self(Shlex::new(input))
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        for token in self.0.by_ref() {
            match token.as_str() {
                ";" => tokens.push(Token::Semicolon),
                "|" => tokens.push(Token::Pipe),
                _ => match get_keyword_token(&token) {
                    Ok(token) => tokens.push(token),
                    Err(_) => {
                        if token.ends_with(';') {
                            tokens.push(Token::Ident(token.trim_end_matches(';').to_string()));
                            tokens.push(Token::Semicolon);
                        } else {
                            tokens.push(Token::Ident(token.to_string()));
                        }
                    }
                },
            }
        }

        tokens.push(Token::Eof);
        tokens
    }

    pub fn parse_line(&mut self) -> anyhow::Result<CommandPipeList> {
        let tokens = self.tokenize();

        let mut current_cmd: Option<RString> = None;
        let mut current_args: RVec<RString> = RVec::new();
        let mut current_list = CommandPipeList::new();
        let mut current_pipe = CommandPipe::new();

        for token in tokens {
            log::debug!("Token: {token}");
            match token {
                Token::Eof => {
                    if let Some(cmd) = current_cmd {
                        current_pipe.append_command(Command::new_with_args(
                            &cmd,
                            current_args.clone(),
                        ));
                    }
                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }
                    break;
                }
                Token::Semicolon => {
                    if current_cmd.is_none() {
                        continue;
                    }

                    current_pipe.append_command(Command::new_with_args(
                        &current_cmd.unwrap(),
                        current_args.clone(),
                    ));

                    if !current_pipe.is_empty() {
                        current_list.append_pipe(current_pipe);
                    }

                    current_pipe = CommandPipe::new();
                    current_cmd = None;
                    current_args.clear();
                }
                Token::Pipe => {
                    if current_cmd.is_none() {
                        return Err(anyhow!("Syntax error: pipe without command"));
                    }

                    current_pipe.append_command(Command::new_with_args(
                        &current_cmd.unwrap(),
                        current_args.clone(),
                    ));

                    current_cmd = None;
                    current_args.clear();
                }
                Token::Ident(ident) => {
                    if current_cmd.is_none() {
                        current_cmd = Some(ident.into());
                    } else {
                        current_args.push(ident.into());
                    }
                }
            }
        }

        Ok(current_list)
    }
}
