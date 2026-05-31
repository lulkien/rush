use crate::lexer::token::tokenize_with_logos;
pub use crate::lexer::token::Token;

pub mod token;
#[cfg(test)]
mod tests;

pub struct Lexer {
    input: String,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
        }
    }

    pub fn tokenize(&self) -> Vec<Token> {
        tokenize_with_logos(&self.input)
    }
}
