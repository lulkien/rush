use std::fmt::Display;

use abi_stable::std_types::RString;

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
            Self::Ident(ident) => {
                write!(f, "{}", ident)
            }
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

pub(super) fn get_keyword_token(ident: &str) -> anyhow::Result<Token> {
    match ident {
        _ => Err(anyhow::anyhow!("Not a keyword")),
    }
}
