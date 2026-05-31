use std::fmt::Display;

use abi_stable::std_types::RString;
use logos::Logos;

/// Intermediate token from the logos lexer.
/// Converted to the public `Token` enum in `Lexer::tokenize`.
#[derive(Logos, Debug, PartialEq)]
enum InnerToken {
    /// Single-quoted string: `'...'` — everything inside is literal.
    #[regex(r"'[^']*'", single_quoted)]
    /// Double-quoted string: `"..."` — backslash escapes `"`, `\`, `n`, `t`.
    #[regex(r#""([^"\\]|\\.)*""#, double_quoted)]
    Text(String),

    #[token("|")]
    Pipe,

    #[token(";")]
    Semicolon,

    /// Unquoted word: any non-whitespace, non-separator, non-quote run.
    #[regex(r#"[^\s|;'"]+"#, |lex| lex.slice().to_string())]
    Ident(String),

    /// Whitespace — never emitted; skipped by the callback.
    #[regex(r"\s+", |_| logos::Skip)]
    Whitespace,
}

/// Strip outer single quotes.  `'hello'` → `hello`.
fn single_quoted(lex: &logos::Lexer<InnerToken>) -> String {
    let s = lex.slice();
    s[1..s.len() - 1].to_string()
}

/// Strip outer double quotes and process `\"`, `\\`, `\n`, `\t`.
fn double_quoted(lex: &logos::Lexer<InnerToken>) -> String {
    let s = lex.slice();
    // Strip the outer double-quote characters.
    let inner = &s[1..s.len() - 1];
    unescape_dq(inner)
}

fn unescape_dq(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[derive(Debug, PartialEq)]
pub enum Token {
    Ident(String),
    Pipe,
    Semicolon,
    Eof,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Pipe => write!(f, "PIPE"),
            Self::Semicolon => write!(f, "SEMICOLON"),
            Self::Eof => write!(f, "END OF FILE"),
        }
    }
}

impl From<Token> for RString {
    fn from(val: Token) -> Self {
        match val {
            Token::Ident(ident) => ident.into(),
            _ => RString::new(),
        }
    }
}

/// Convert a stream of `InnerToken` items into our public `Token` stream,
/// appending `Eof` at the end.
pub(super) fn tokenize_with_logos(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = InnerToken::lexer(input)
        .filter_map(|t| match t {
            Ok(InnerToken::Text(s)) | Ok(InnerToken::Ident(s)) => Some(Token::Ident(s)),
            Ok(InnerToken::Pipe) => Some(Token::Pipe),
            Ok(InnerToken::Semicolon) => Some(Token::Semicolon),
            _ => None, // Whitespace (never emitted) and errors
        })
        .collect();

    tokens.push(Token::Eof);
    tokens
}

/// Keyword lookup (currently a no-op stub for future keyword support).
pub(super) fn get_keyword_token(ident: &str) -> anyhow::Result<Token> {
    match ident {
        _ => Err(anyhow::anyhow!("Not a keyword")),
    }
}
