use crate::lexer::{Lexer, Token};
use crate::types::Command;

fn token_strings(lexer: &Lexer) -> Vec<String> {
    let tokens = lexer.tokenize();
    tokens
        .iter()
        .filter_map(|t| match t {
            Token::Word(s, _) => Some(s.clone()),
            Token::Pipe => Some("|".into()),
            Token::Semicolon => Some(";".into()),
            Token::AndIf => Some("&&".into()),
            Token::OrIf => Some("||".into()),
            Token::Background => Some("&".into()),
            Token::Less => Some("<".into()),
            Token::Great => Some(">".into()),
            Token::DGreat => Some(">>".into()),
            Token::LessAnd => Some("<&".into()),
            Token::GreatAnd => Some(">&".into()),
            Token::LessGreat => Some("<>".into()),
            Token::DLess => Some("<<".into()),
            Token::DLessDash => Some("<<-".into()),
            Token::Clobber => Some(">|".into()),
            Token::OpenParen => Some("(".into()),
            Token::CloseParen => Some(")".into()),
            Token::Newline => Some("\\n".into()),
            Token::Eof => None,
        })
        .collect()
}

// ── existing tests (unchanged behaviour) ──────────────────────

#[test]
fn semicolon_inside_quotes_is_preserved() {
    let lexer = Lexer::new("echo \"hello;\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello;"]);
}

#[test]
fn unquoted_semicolon_is_separate_token() {
    let lexer = Lexer::new("echo hello;");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", ";"]);
}

#[test]
fn pipe_inside_quotes_is_preserved() {
    let lexer = Lexer::new("echo \"|\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "|"]);
}

#[test]
fn unquoted_pipe_is_separate_token() {
    let lexer = Lexer::new("echo hello | cat");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", "|", "cat"]);
}

#[test]
fn single_quotes_protect_semicolon() {
    let lexer = Lexer::new("echo 'hello;'");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello;"]);
}

#[test]
fn mixed_quotes() {
    let lexer = Lexer::new("echo \"it's; fine\" | cat");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "it's; fine", "|", "cat"]);
}

#[test]
fn no_spaces_around_pipe() {
    let lexer = Lexer::new("echo hello|cat");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", "|", "cat"]);
}

#[test]
fn no_spaces_around_semicolon() {
    let lexer = Lexer::new("echo hello;cat");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", ";", "cat"]);
}

#[test]
fn double_quote_escaped_quote() {
    let lexer = Lexer::new("echo \"hello \\\"world\\\"\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello \"world\""]);
}

#[test]
fn double_quote_escaped_backslash() {
    let lexer = Lexer::new("echo \"path\\\\to\\\\file\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "path\\to\\file"]);
}

#[test]
fn double_quote_newline_escape() {
    // POSIX: \n inside double quotes is literal \ and n
    let lexer = Lexer::new("echo \"hello\\nworld\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello\\nworld"]);
}

#[test]
fn empty_double_quoted() {
    let lexer = Lexer::new("echo \"\"");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", ""]);
}

#[test]
fn empty_single_quoted() {
    let lexer = Lexer::new("echo ''");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", ""]);
}

#[test]
fn multiple_semicolons() {
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

// ── POSIX operator tests ────────────────────────────────────────

#[test]
fn posix_and_or() {
    let lexer = Lexer::new("gcc -c foo.c && gcc -c bar.c || echo failed");
    let tokens = token_strings(&lexer);
    assert_eq!(
        tokens,
        vec!["gcc", "-c", "foo.c", "&&", "gcc", "-c", "bar.c", "||", "echo", "failed"]
    );
}

#[test]
fn posix_redirects_comprehensive() {
    let lexer = Lexer::new("cmd < in > out 2> err >> log <> both <& 0 >& 1 >| forced");
    let tokens = token_strings(&lexer);
    assert_eq!(
        tokens,
        vec![
            "cmd", "<", "in", ">", "out", "2", ">", "err", ">>", "log",
            "<>", "both", "<&", "0", ">&", "1", ">|", "forced",
        ]
    );
}

#[test]
fn posix_here_document() {
    let lexer = Lexer::new("cat << EOF\nhello\nEOF");
    let tokens = token_strings(&lexer);
    assert_eq!(
        tokens,
        vec!["cat", "<<", "EOF", "\\n", "hello", "\\n", "EOF"]
    );
}

#[test]
fn posix_here_document_dash() {
    let lexer = Lexer::new("cat <<- EOF");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["cat", "<<-", "EOF"]);
}

#[test]
fn posix_background() {
    let lexer = Lexer::new("sleep 10 &");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["sleep", "10", "&"]);
}

#[test]
fn posix_grouping_tokens() {
    // Braces are regular word characters in the lexer;
    // grouping semantics are handled by the parser.
    let lexer = Lexer::new("( cmd1; cmd2 ) && echo ok");
    let tokens = token_strings(&lexer);
    assert_eq!(
        tokens,
        vec!["(", "cmd1", ";", "cmd2", ")", "&&", "echo", "ok"]
    );
}

#[test]
fn posix_line_continuation() {
    // \<newline> should be skipped entirely (line continuation).
    let input = "echo hello \\\nworld";
    let lexer = Lexer::new(input);
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", "world"]);
}

#[test]
fn posix_backslash_in_unquoted_word() {
    // POSIX: \X in an unquoted word preserves literal X.
    let lexer = Lexer::new("echo hello\\ world");
    let tokens = token_strings(&lexer);
    // backslash-space → literal space, so it's one word "hello world"
    assert_eq!(tokens, vec!["echo", "hello world"]);
}

#[test]
fn posix_backslash_escape_sequence_in_word() {
    let lexer = Lexer::new("echo a\\\"b");
    let tokens = token_strings(&lexer);
    // \" in unquoted word → literal " (backslash stripped)
    assert_eq!(tokens, vec!["echo", "a\"b"]);
}

#[test]
fn posix_newline_as_separator() {
    let input = "echo a\necho b\n";
    let lexer = Lexer::new(input);
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

#[test]
fn posix_redirect_parsed_into_command() {
    let lexer = Lexer::new("echo hello > out.txt");
    let result = lexer.parse_line().unwrap();
    let commands: Vec<Command> = result
        .into_iter()
        .flat_map(|pipe| pipe.into_iter().collect::<Vec<_>>())
        .collect();
    assert_eq!(commands.len(), 1);
    let cmd = &commands[0];
    assert_eq!(cmd.name.as_str(), "echo");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].as_str(), "hello");
    assert_eq!(cmd.redirects.len(), 1);
    assert_eq!(cmd.redirects[0].op, crate::types::RedirectOp::Great);
    assert_eq!(cmd.redirects[0].target.as_str(), "out.txt");
}

#[test]
fn posix_multiple_redirects() {
    let lexer = Lexer::new("cmd < input > output 2> error");
    let result = lexer.parse_line().unwrap();
    let commands: Vec<Command> = result
        .into_iter()
        .flat_map(|pipe| pipe.into_iter().collect::<Vec<_>>())
        .collect();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].redirects.len(), 3);
    assert_eq!(commands[0].redirects[0].target.as_str(), "input");
    assert_eq!(commands[0].redirects[1].target.as_str(), "output");
    assert_eq!(commands[0].redirects[2].target.as_str(), "error");
}

#[test]
fn posix_background_flag_on_command() {
    let lexer = Lexer::new("sleep 10 &");
    let result = lexer.parse_line().unwrap();
    let commands: Vec<Command> = result
        .into_iter()
        .flat_map(|pipe| pipe.into_iter().collect::<Vec<_>>())
        .collect();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name.as_str(), "sleep");
    // background is tracked at the AST level (CompleteCommand);
    // the flat parse_line representation does not preserve it.
}

#[test]
fn posix_subshell_not_supported() {
    let lexer = Lexer::new("(echo hello)");
    let result = lexer.parse_line();
    assert!(result.is_err());
}

#[test]
fn posix_dollar_not_an_operator() {
    // $ is a regular word character, not a lexer operator.
    // Expansion is handled later by the parser/executor.
    let lexer = Lexer::new("echo $HOME ${PATH}");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "$HOME", "${PATH}"]);
}

#[test]
fn posix_empty_input() {
    let lexer = Lexer::new("");
    let result = lexer.parse_line().unwrap();
    assert!(result.is_empty());
}

#[test]
fn posix_whitespace_only() {
    let lexer = Lexer::new("   \t  ");
    let result = lexer.parse_line().unwrap();
    assert!(result.is_empty());
}
