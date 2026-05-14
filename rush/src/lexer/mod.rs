use abi_stable::std_types::{RString, RVec};
use anyhow::anyhow;
use shlex::Shlex;

use crate::types::{Command, CommandPipeList, CommandPipe};
use crate::lexer::token::{Token, get_keyword_token};

mod token;

/// Pre-process input: insert spaces around unquoted `;` and `|`
/// so that `shlex` naturally splits them as separate tokens.
/// Characters inside single or double quotes are left untouched.
fn preprocess_input(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 16);
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for ch in input.chars() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                result.push(ch);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                result.push(ch);
            }
            ';' | '|' if !in_single_quote && !in_double_quote => {
                result.push(' ');
                result.push(ch);
                result.push(' ');
            }
            _ => result.push(ch),
        }
    }

    result
}

pub struct Lexer {
    input: String,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: preprocess_input(input),
        }
    }

    fn tokenize(&self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut shlex = Shlex::new(&self.input);

        for token in shlex.by_ref() {
            match token.as_str() {
                ";" => tokens.push(Token::Semicolon),
                "|" => tokens.push(Token::Pipe),
                _ => match get_keyword_token(&token) {
                    Ok(token) => tokens.push(token),
                    Err(_) => tokens.push(Token::Ident(token.to_string())),
                },
            }
        }

        tokens.push(Token::Eof);
        tokens
    }

    pub fn parse_line(&self) -> anyhow::Result<CommandPipeList> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn token_strings(lexer: &Lexer) -> Vec<String> {
        let tokens = lexer.tokenize();
        tokens
            .iter()
            .filter_map(|t| match t {
                Token::Ident(s) => Some(s.clone()),
                Token::Pipe => Some("|".into()),
                Token::Semicolon => Some(";".into()),
                Token::Eof => None,
            })
            .collect()
    }

    #[test]
    fn semicolon_inside_quotes_is_preserved() {
        // 'echo "hello;"' → tokens should be: echo, hello;
        let lexer = Lexer::new("echo \"hello;\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello;"]);
    }

    #[test]
    fn unquoted_semicolon_is_separate_token() {
        // 'echo hello;' → tokens: echo, hello, ;
        let lexer = Lexer::new("echo hello;");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", ";"]);
    }

    #[test]
    fn pipe_inside_quotes_is_preserved() {
        // 'echo "|"' → tokens: echo, |
        let lexer = Lexer::new("echo \"|\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "|"]);
    }

    #[test]
    fn unquoted_pipe_is_separate_token() {
        // 'echo hello | cat' → tokens: echo, hello, |, cat
        let lexer = Lexer::new("echo hello | cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", "|", "cat"]);
    }

    #[test]
    fn single_quotes_protect_semicolon() {
        // 'echo 'hello;'' → tokens: echo, hello;
        let lexer = Lexer::new("echo 'hello;'");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello;"]);
    }

    #[test]
    fn mixed_quotes() {
        // 'echo "it's; fine" | cat' → tokens: echo, it's; fine, |, cat
        let lexer = Lexer::new("echo \"it's; fine\" | cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "it's; fine", "|", "cat"]);
    }
}
