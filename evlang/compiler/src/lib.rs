#![no_std]

use crate::{lexer::LexerError, span::Spanned};

extern crate alloc;

pub mod span;
pub mod lexer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileError {
    LexerError(LexerError),
}

impl<'src> From<Spanned<'src, LexerError>> for Spanned<'src, CompileError> {
    fn from(value: Spanned<'src, LexerError>) -> Self {
        value.map(|le| CompileError::LexerError(le))
    }
}
