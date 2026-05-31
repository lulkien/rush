use abi_stable::std_types::{RString, RVec};
use anyhow::anyhow;

use crate::lexer::token::{Token, tokenize_with_logos};
use crate::types::{Command, CommandPipe, CommandPipeList};

mod token;

pub struct Lexer {
    input: String,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
        }
    }

    fn tokenize(&self) -> Vec<Token> {
        tokenize_with_logos(&self.input)
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
        // 'echo "hello;"' should produce tokens: echo, hello;
        let lexer = Lexer::new("echo \"hello;\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello;"]);
    }

    #[test]
    fn unquoted_semicolon_is_separate_token() {
        // 'echo hello;' should produce tokens: echo, hello, ;
        let lexer = Lexer::new("echo hello;");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", ";"]);
    }

    #[test]
    fn pipe_inside_quotes_is_preserved() {
        // 'echo "|"' should produce tokens: echo, |
        let lexer = Lexer::new("echo \"|\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "|"]);
    }

    #[test]
    fn unquoted_pipe_is_separate_token() {
        // 'echo hello | cat' should produce tokens: echo, hello, |, cat
        let lexer = Lexer::new("echo hello | cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", "|", "cat"]);
    }

    #[test]
    fn single_quotes_protect_semicolon() {
        // "echo 'hello;'" should produce tokens: echo, hello;
        let lexer = Lexer::new("echo 'hello;'");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello;"]);
    }

    #[test]
    fn mixed_quotes() {
        // 'echo "it's; fine" | cat' should produce tokens: echo, it's; fine, |, cat
        let lexer = Lexer::new("echo \"it's; fine\" | cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "it's; fine", "|", "cat"]);
    }

    /// Regression: no-spaces-around-pipe should still tokenise correctly.
    #[test]
    fn no_spaces_around_pipe() {
        let lexer = Lexer::new("echo hello|cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", "|", "cat"]);
    }

    /// Regression: no-spaces-around-semicolon should still tokenise correctly.
    #[test]
    fn no_spaces_around_semicolon() {
        let lexer = Lexer::new("echo hello;cat");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello", ";", "cat"]);
    }

    /// Double-quote escape: \" should produce a literal quote inside the token.
    #[test]
    fn double_quote_escaped_quote() {
        let lexer = Lexer::new("echo \"hello \\\"world\\\"\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello \"world\""]);
    }

    /// Double-quote escape: \\ should produce a literal backslash.
    #[test]
    fn double_quote_escaped_backslash() {
        let lexer = Lexer::new("echo \"path\\\\to\\\\file\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "path\\to\\file"]);
    }

    /// Newline inside double quotes via \\n.
    #[test]
    fn double_quote_newline_escape() {
        let lexer = Lexer::new("echo \"hello\\nworld\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", "hello\nworld"]);
    }

    /// Empty double-quoted string.
    #[test]
    fn empty_double_quoted() {
        let lexer = Lexer::new("echo \"\"");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", ""]);
    }

    /// Empty single-quoted string.
    #[test]
    fn empty_single_quoted() {
        let lexer = Lexer::new("echo ''");
        let tokens = token_strings(&lexer);
        assert_eq!(tokens, vec!["echo", ""]);
    }

    /// Bare semicolons should not produce phantom empty commands.
    #[test]
    fn multiple_semicolons() {
        // ";;" should produce no commands — just empty pipes.
        let lexer = Lexer::new("echo a;;echo b");
        let result = lexer.parse_line().unwrap();
        let pipes: Vec<Vec<String>> = result
            .into_iter()
            .map(|pipe| {
                pipe.into_iter()
                    .map(|cmd| {
                        let mut s = cmd.name.to_string();
                        for arg in cmd.args.iter() {
                            s.push(' ');
                            s.push_str(arg);
                        }
                        s
                    })
                    .collect()
            })
            .collect();
        assert_eq!(pipes, vec![vec!["echo a"], vec!["echo b"]]);
    }
}
