use std::fmt::Display;

use abi_stable::std_types::RString;
use logos::Logos;

// ── logos inner token ────────────────────────────────────────────────

/// Intermediate token produced by the logos lexer.
/// Converted to the public `Token` enum by `tokenize_with_logos`.
#[derive(Logos, Debug, PartialEq)]
enum InnerToken {
    /// Single-quoted string — everything inside is literal.
    #[regex(r"'[^']*'", single_quoted)]
    /// Double-quoted string — `\` escapes `"`, `\`, `$`, `` ` ``, `\n`.
    #[regex(r#""([^"\\$`]|\\.)*""#, double_quoted)]
    Text(String),

    // ── multi-char operators (longest match wins) ──
    #[token("<<-")] DLessDash,
    #[token(">>")]  DGreat,
    #[token("<<")]  DLess,
    #[token("<>")]  LessGreat,
    #[token("<&")]  LessAnd,
    #[token(">&")]  GreatAnd,
    #[token(">|")]  Clobber,
    #[token("&&")]  AndIf,
    #[token("||")]  OrIf,

    // ── single-char operators ──
    #[token("|")]  Pipe,
    #[token(";")]  Semicolon,
    #[token("&")]  Background,
    #[token("<")]  Less,
    #[token(">")]  Great,
    #[token("(")]  OpenParen,
    #[token(")")]  CloseParen,

    // ── line continuation: `\<newline>` → skip ──
    #[regex(r"\\\n", logos::skip)]
    LineCont,

    // ── newline (after LineCont so `\<newline>` is consumed first) ──
    #[token("\n")]
    Newline,

    // ── unquoted word with backslash escapes ──
    // POSIX: `\X` preserves literal X; `\<newline>` is line continuation (handled above).
    #[regex(r#"([^\t\n |;&<>()'"\\]|\\[^\n])+"#, unescaped_word)]
    Word(String),

    // ── horizontal whitespace (tabs, spaces) ──
    #[regex(r"[ \t]+", logos::skip)]
    Whitespace,
}

// ── callbacks ────────────────────────────────────────────────────────

fn single_quoted(lex: &logos::Lexer<InnerToken>) -> String {
    let s = lex.slice();
    s[1..s.len() - 1].to_string()
}

fn double_quoted(lex: &logos::Lexer<InnerToken>) -> String {
    let s = lex.slice();
    let inner = &s[1..s.len() - 1];
    unescape_dq(inner)
}

fn unescape_dq(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(q @ '"') => result.push(q),
                Some(bs @ '\\') => result.push(bs),
                Some(d @ '$') => result.push(d),
                Some(b @ '`') => result.push(b),
                Some('n') => result.push('\n'),
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

/// POSIX unquoted-word backslash: `\X` preserves literal X.
fn unescaped_word(lex: &logos::Lexer<InnerToken>) -> String {
    let s = lex.slice();
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ── public token enum ────────────────────────────────────────────────

/// Token produced by the lexer, consumed by the parser.
#[derive(Debug, PartialEq)]
pub enum Token {
    Ident(String),
    Pipe,         // |
    Semicolon,    // ;
    Eof,
    // Logical operators
    AndIf,        // &&
    OrIf,         // ||
    // Background
    Background,   // &
    // Redirections
    Less,         // <
    Great,        // >
    DGreat,       // >>
    LessAnd,      // <&
    GreatAnd,     // >&
    LessGreat,    // <>
    DLess,        // <<
    DLessDash,    // <<-
    Clobber,      // >|
    // Grouping
    OpenParen,    // (
    CloseParen,   // )
    // Line separator
    Newline,      // \n
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Ident(s) => return write!(f, "{s}"),
            Self::Pipe => "|",
            Self::Semicolon => ";",
            Self::Eof => "EOF",
            Self::AndIf => "&&",
            Self::OrIf => "||",
            Self::Background => "&",
            Self::Less => "<",
            Self::Great => ">",
            Self::DGreat => ">>",
            Self::LessAnd => "<&",
            Self::GreatAnd => ">&",
            Self::LessGreat => "<>",
            Self::DLess => "<<",
            Self::DLessDash => "<<-",
            Self::Clobber => ">|",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::Newline => "<newline>",
        };
        write!(f, "{s}")
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

// ── conversion ───────────────────────────────────────────────────────

/// Run the logos lexer on `input` and produce a vector of public `Token`s,
/// with `Eof` appended.
pub(super) fn tokenize_with_logos(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = InnerToken::lexer(input)
        .filter_map(|t| match t {
            Ok(InnerToken::Text(s)) | Ok(InnerToken::Word(s)) => Some(Token::Ident(s)),
            Ok(InnerToken::Pipe) => Some(Token::Pipe),
            Ok(InnerToken::Semicolon) => Some(Token::Semicolon),
            Ok(InnerToken::AndIf) => Some(Token::AndIf),
            Ok(InnerToken::OrIf) => Some(Token::OrIf),
            Ok(InnerToken::Background) => Some(Token::Background),
            Ok(InnerToken::Less) => Some(Token::Less),
            Ok(InnerToken::Great) => Some(Token::Great),
            Ok(InnerToken::DGreat) => Some(Token::DGreat),
            Ok(InnerToken::LessAnd) => Some(Token::LessAnd),
            Ok(InnerToken::GreatAnd) => Some(Token::GreatAnd),
            Ok(InnerToken::LessGreat) => Some(Token::LessGreat),
            Ok(InnerToken::DLess) => Some(Token::DLess),
            Ok(InnerToken::DLessDash) => Some(Token::DLessDash),
            Ok(InnerToken::Clobber) => Some(Token::Clobber),
            Ok(InnerToken::OpenParen) => Some(Token::OpenParen),
            Ok(InnerToken::CloseParen) => Some(Token::CloseParen),
            Ok(InnerToken::Newline) => Some(Token::Newline),
            _ => None, // Whitespace / LineCont are skipped
        })
        .collect();
    tokens.push(Token::Eof);
    tokens
}

/// Keyword lookup (currently a no-op stub for future keyword support).
pub(super) fn get_keyword_token(_ident: &str) -> anyhow::Result<Token> {
    Err(anyhow::anyhow!("Not a keyword"))
}
