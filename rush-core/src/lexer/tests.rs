use crate::lexer::{Lexer, Token};

fn token_strings(lexer: &Lexer) -> Vec<String> {
    let tokens = lexer.tokenize();
    tokens
        .iter()
        .filter_map(|t| match t {
            Token::Word(s, _) => Some(s.clone()),
            Token::Pipe => Some("|".to_string()),
            Token::Semicolon => Some(";".to_string()),
            Token::AndIf => Some("&&".to_string()),
            Token::OrIf => Some("||".to_string()),
            Token::Background => Some("&".to_string()),
            Token::Less => Some("<".to_string()),
            Token::Great => Some(">".to_string()),
            Token::DGreat => Some(">>".to_string()),
            Token::LessAnd => Some("<&".to_string()),
            Token::GreatAnd => Some(">&".to_string()),
            Token::LessGreat => Some("<>".to_string()),
            Token::DLess => Some("<<".to_string()),
            Token::DLessDash => Some("<<-".to_string()),
            Token::Clobber => Some(">|".to_string()),
            Token::OpenParen => Some("(".to_string()),
            Token::CloseParen => Some(")".to_string()),
            Token::Newline => Some("\\n".to_string()),
            Token::Eof => None,
        })
        .collect()
}

// ── token-level tests ────────────────────────────────────────────

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

// ── POSIX operator token tests ───────────────────────────────────

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
    let lexer = Lexer::new("( cmd1; cmd2 ) && echo ok");
    let tokens = token_strings(&lexer);
    assert_eq!(
        tokens,
        vec!["(", "cmd1", ";", "cmd2", ")", "&&", "echo", "ok"]
    );
}

#[test]
fn posix_line_continuation() {
    let input = "echo hello \\\nworld";
    let lexer = Lexer::new(input);
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello", "world"]);
}

#[test]
fn posix_backslash_in_unquoted_word() {
    let lexer = Lexer::new("echo hello\\ world");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "hello world"]);
}

#[test]
fn posix_backslash_escape_sequence_in_word() {
    let lexer = Lexer::new("echo a\\\"b");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "a\"b"]);
}

#[test]
fn posix_dollar_not_an_operator() {
    let lexer = Lexer::new("echo $HOME ${PATH}");
    let tokens = token_strings(&lexer);
    assert_eq!(tokens, vec!["echo", "$HOME", "${PATH}"]);
}
