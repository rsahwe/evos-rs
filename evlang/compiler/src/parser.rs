use core::iter::Peekable;

use alloc::{boxed::Box, vec::Vec};

use crate::{lexer::{Lexer, Token}, span::{Span, Spanned}, CompileError};

#[derive(Debug, Clone)]
pub struct Ast<'src> {
    decls: Vec<Declaration<'src>>,
}

#[derive(Debug, Clone)]
pub struct Declaration<'src> {
    mutable: bool,
    name: &'src str,
    given_type: Option<Type<'src>>,
    value: Expression<'src>,
}

#[derive(Debug, Clone)]
pub enum Expression<'src> {
    BinaryOp(BinOp, Box<Expression<'src>>, Box<Expression<'src>>),
    UnaryOp(UnOp, Box<Expression<'src>>),
    Ident(&'src str),
    IntLit(&'src str),
    Block(Block<'src>),
    Function(Function<'src>),
}

#[derive(Debug, Clone)]
pub enum DeclOrExpr<'src> {
    Declaration(Declaration<'src>),
    Expression(Expression<'src>),
}

#[derive(Debug, Clone)]
pub struct Block<'src> {
    content: Vec<DeclOrExpr<'src>>,
    last: Option<Box<Expression<'src>>>,
}

#[derive(Debug, Clone)]
pub struct Function<'src> {
    name: &'src str,
    given_type: Option<Type<'src>>,
    parameters: Vec<&'src str>,
    content: Box<Expression<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Plus,
    Minus,
    Mul,
    Div,
    Xor,
    And,
    Or,
    RShift,
    LShift,
    Ne, Lt, Gt, Leq, Geq, Eq,
    Assign(Option<Box<BinOp>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Minus,
}

#[derive(Debug, Clone)]
pub enum Type<'src> {
    Simple(&'src str),
    Function(Vec<Type<'src>>, Option<Box<Type<'src>>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserError {
    MissingTokens,
    UnexpectedToken,
}

macro_rules! token_expect {
    ($lexer:ident, $end:ident) => {
        $lexer.next().ok_or(Spanned::new(ParserError::MissingTokens, Span::new_single($end)))??
    };
}

macro_rules! token_expect_peek {
    ($lexer:ident, $end:ident) => {
        (*$lexer.peek().ok_or(Spanned::new(ParserError::MissingTokens, Span::new_single($end)))?)?
    };
}

macro_rules! token_expect_match {
    ($lexer:ident, $end:ident, $span:ident, $($rest:tt)*) => {{
        let Spanned { inner, $span } = token_expect!($lexer, $end);
        match inner {
            $($rest)*
            _ => Err(Spanned::new(ParserError::UnexpectedToken, $span))?,
        }
    }};
}

macro_rules! token_expect_peek_match {
    ($lexer:ident, $end:ident, $span:ident, $($rest:tt)*) => {{
        let Spanned { inner, $span } = token_expect_peek!($lexer, $end);
        match inner {
            $($rest)*
            _ => Err(Spanned::new(ParserError::UnexpectedToken, $span))?,
        }
    }};
}

macro_rules! token_expect_consume {
    ($lexer:ident, $end:ident, $token:pat_param) => {
        token_expect_match!($lexer, $end, span,
            $token => (),
        )
    };
}

impl<'src> Ast<'src> {
    pub fn parse(source: &'src str) -> Result<Self, Spanned<'src, CompileError>> {
        let mut ast = Ast { decls: Vec::new() };
        
        if source.len() == 0 {
            return Ok(ast);
        }
        
        let end = source.len() - 1;
        let mut lexer = Lexer::new(source).peekable();

        while lexer.peek().is_some() {
            ast.decls.push(Self::parse_decl(&mut lexer, end)?);
        }

        Ok(ast)
    }

    fn parse_decl(lexer: &mut Peekable<Lexer<'src>>, end: usize) -> Result<Declaration<'src>, Spanned<'src, CompileError>> {
        let is_mut = token_expect_match!(lexer, end, span,
            Token::Const => false,
            Token::Mut => true,
        );

        let name = token_expect_match!(lexer, end, span,
            Token::Ident(ident) => ident,
        );

        let given_type = token_expect_match!(lexer, end, span,
            Token::Pipe => None,
            Token::Colon => Some(Self::parse_type(lexer, end)?),
        );

        token_expect_consume!(lexer, end, Token::Eq);

        let expr = Self::parse_expr(lexer, end)?;

        token_expect_consume!(lexer, end, Token::Semi);

        Ok(Declaration { mutable: is_mut, name, given_type, value: expr })
    }

    fn parse_type(lexer: &mut Peekable<Lexer<'src>>, end: usize) -> Result<Type<'src>, Spanned<'src, CompileError>> {
        token_expect_match!(lexer, end, span,
            Token::Ident(ident) => Ok(Type::Simple(ident)),
            Token::Fn => {
                token_expect_consume!(lexer, end, Token::ParenOpen);

                let mut args = Vec::new();

                loop {
                    token_expect_peek_match!(lexer, end, span,
                        Token::ParenClose => {
                            lexer.next();
                            break
                        },
                        Token::Ident(_) | Token::Fn => (),
                    );

                    args.push(Self::parse_type(lexer, end)?);

                    token_expect_match!(lexer, end, span,
                        Token::ParenClose => break,
                        Token::Comma => (),
                    );
                }

                let ret = token_expect_match!(lexer, end, span,
                    Token::Pipe => None,
                    Token::Colon => Some(Box::new(Self::parse_type(lexer, end)?)),
                );

                Ok(Type::Function(args, ret))
            }
        )
    }

    fn parse_expr(lexer: &mut Peekable<Lexer<'src>>, end: usize) -> Result<Expression<'src>, Spanned<'src, CompileError>> {
        todo!()
    }
}
