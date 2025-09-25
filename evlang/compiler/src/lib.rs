#![no_std]

use crate::{lexer::LexerError, parser::ParserError, span::Spanned};

extern crate alloc;

pub mod span;
pub mod lexer;
pub mod parser;
pub mod checker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileError {
    LexerError(LexerError),
    ParserError(ParserError),
}

impl<'src> From<Spanned<'src, LexerError>> for Spanned<'src, CompileError> {
    fn from(value: Spanned<'src, LexerError>) -> Self {
        value.map(|le| CompileError::LexerError(le))
    }
}

impl<'src> From<Spanned<'src, ParserError>> for Spanned<'src, CompileError> {
    fn from(value: Spanned<'src, ParserError>) -> Self {
        value.map(|pe| CompileError::ParserError(pe))
    }
}
