pub use crate::lexer::token::Token;
use crate::lexer::token::tokenize_with_logos;

#[cfg(test)]
mod tests;
pub mod token;

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
