use alloc::boxed::Box;

use crate::lexer::{Keyword, Lexer, LexerError, Span, Spanned, Symbol, Token};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Expression<'src> {
    Unit(Span<'src>),
    Ident(Spanned<'src, &'src str>),
}

impl<'src> Expression<'src> {
    pub const fn span(&self) -> Span<'src> {
        match self {
            Expression::Unit(span) => *span,
            Expression::Ident(ident) => ident.span(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type<'src> {
    Unit,
    Bool,
    I8, U8, I16, U16, I32, U32, I64, U64,
    Reference(Box<Type<'src>>, bool),// LIFETIME?
    Array(Box<Type<'src>>, usize),
    Custom(&'src str),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Statement<'src> {
    Expression(Expression<'src>),
    Definition {
        name: Spanned<'src, &'src str>,
        typename: Option<Spanned<'src, Type<'src>>>,
        value: Expression<'src>,
    },
}

impl<'src> Statement<'src> {
    pub fn span(&self) -> Span<'src> {
        match self {
            Statement::Expression(expression) => expression.span(),
            Statement::Definition { name, value, .. } => name.span() + value.span(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Parser<'src> {
    inner: Lexer<'src>,
    complete: bool,//TODO: SET THIS
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ParserError {
    LexerError(LexerError),
    UnexpectedToken { expected: &'static str },
}

impl<'src> Iterator for Parser<'src> {
    type Item = Result<Statement<'src>, Spanned<'src, ParserError>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.complete {
            return None;
        }

        let res = match self.inner.peek() {
            Ok(next) => match next.inner {
                Token::Keyword(keyword) => match keyword {
                    Keyword::Decl => todo!(),
                    Keyword::Enum => todo!(),
                    Keyword::Struct => todo!(),
                    Keyword::Trait => todo!(),
                    _ => Some(self.expression().map(|expr| Statement::Expression(expr)).inspect_err(|_| self.complete = true)),
                },
                Token::Error(lexer_error) => {
                    self.complete = true;
                    Some(Err(Spanned::new(ParserError::LexerError(lexer_error), next.span())))
                },
                _ => Some(self.expression().map(|expr| Statement::Expression(expr)).inspect_err(|_| self.complete = true)),
            },
            Err(error) => {
                self.complete = true;
                Some(Err(error.map(|e| ParserError::LexerError(e))))
            },
        };

        let next = match self.inner.next().map_err(|e| e.map(|le| ParserError::LexerError(le))) {
            Ok(token) => token,
            Err(err) => {
                self.complete = true;
                return Some(Err(err))
            },
        };

        if !matches!(next.inner, Token::Symbol(Symbol::Semi)) {
            self.complete = true;
            return Some(Err(Spanned::new(ParserError::UnexpectedToken { expected: "';'" }, next.span())));
        }

        res
    }
}

impl<'src> Parser<'src> {
    pub fn new(lexer: Lexer<'src>) -> Self {
        Self { inner: lexer, complete: false }
    }

    fn expression(&mut self) -> Result<Expression<'src>, Spanned<'src, ParserError>> {
        match self.inner.peek() {
            Ok(next) => match next.inner {
                Token::Ident(ident) => {
                    let _ = self.inner.next();
                    Ok(Expression::Ident(next.map(|_| ident)))
                },
                Token::Keyword(_) => todo!("Expression"),
                Token::Symbol(_) => todo!("Expression"),
                Token::Error(_) => unreachable!(),// Since <Parser as Iterator>::next filters this
            },
            Err(_) => unreachable!(),// Since <Parser as Iterator>::next filters this
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ident_expr() {
        assert!(Parser::new(Lexer::new("test;")).eq([Ok(Statement::Expression(Expression::Ident(Spanned::new("test", Span::new_inclusive(0..=3))))), Err(Spanned::new(ParserError::LexerError(LexerError::UnexpectedEof), Span::new_single(5)))].into_iter()));
    }
}
