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
    Call(Box<Expression<'src>>, Vec<Expression<'src>>),
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
    return_type: Type<'src>,
    parameters: Vec<(&'src str, Type<'src>)>,
    content: Box<Expression<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Plus,
    Minus,
    Mul,
    Div,
    Mod,
    Xor,
    And,
    Or,
    RShift,
    LShift,
    Ne, Lt, Gt, Leq, Geq, Eq,
    Assign,
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
    InferenceBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Binding {
    None,
    RAssign,
    LOr, ROr,
    LXor, RXor,
    LAnd, RAnd,
    LRela, RRela,
    LShift, RShift,
    LPlMin, RPlMin,
    LMulDiv, RMulDiv,
    LAssign,
    ANot,
    ANegate,
    PCall,
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
        
        let end = source.chars().count() - 1;
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
            Token::Colon => Some(Self::parse_type(lexer, end, false)?),
        );

        token_expect_consume!(lexer, end, Token::Eq);

        let expr = Self::parse_expr(lexer, end)?;

        token_expect_consume!(lexer, end, Token::Semi);

        Ok(Declaration { mutable: is_mut, name, given_type, value: expr })
    }

    fn parse_type(lexer: &mut Peekable<Lexer<'src>>, end: usize, block_inference: bool) -> Result<Type<'src>, Spanned<'src, CompileError>> {
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

                    args.push(Self::parse_type(lexer, end, block_inference)?);

                    token_expect_match!(lexer, end, span,
                        Token::ParenClose => break,
                        Token::Comma => (),
                    );
                }

                let ret = token_expect_match!(lexer, end, span,
                    Token::Pipe => if !block_inference { None } else {
                        Err(Spanned::new(ParserError::InferenceBlocked, span))?
                    },
                    Token::Colon => Some(Box::new(Self::parse_type(lexer, end, block_inference)?)),
                );

                Ok(Type::Function(args, ret))
            }
        )
    }

    fn parse_expr(lexer: &mut Peekable<Lexer<'src>>, end: usize) -> Result<Expression<'src>, Spanned<'src, CompileError>> {
        Self::parse_expr_bp(lexer, end, Binding::None)
    }

    fn parse_expr_bp(lexer: &mut Peekable<Lexer<'src>>, end: usize, bp: Binding) -> Result<Expression<'src>, Spanned<'src, CompileError>> {
        let mut lhs = token_expect_match!(lexer, end, span,
            Token::Ident(ident) => Expression::Ident(ident),
            Token::IntLit(lit) => Expression::IntLit(lit),
            Token::BraceOpen => {
                let block = Self::parse_block(lexer, end)?;
                
                token_expect_consume!(lexer, end, Token::BraceClose);

                Expression::Block(block)
            },
            Token::Fn => {
                token_expect_consume!(lexer, end, Token::ParenOpen);

                let mut args = Vec::new();

                loop {
                    let ident = token_expect_match!(lexer, end, span,
                        Token::ParenClose => break,
                        Token::Ident(ident) => ident,
                    );

                    token_expect_consume!(lexer, end, Token::Colon);

                    args.push((ident, Self::parse_type(lexer, end, true)?));

                    token_expect_match!(lexer, end, span,
                        Token::ParenClose => break,
                        Token::Comma => (),
                    );
                }

                let ret = token_expect_match!(lexer, end, span,
                    Token::Pipe => Err(Spanned::new(ParserError::InferenceBlocked, span))?,
                    Token::Colon => Self::parse_type(lexer, end, true)?,
                );

                token_expect_consume!(lexer, end, Token::Colon);

                let content = Box::new(Self::parse_expr_bp(lexer, end, Binding::None)?);

                Expression::Function(Function { return_type: ret, parameters: args, content })
            },
            t if t.prefix_bp().is_some() => Expression::UnaryOp(match t {
                Token::Not => UnOp::Not,
                Token::Minus => UnOp::Minus,
                _ => unreachable!("Not a valid prefix operator but t should be a prefix operator"),
            }, Box::new(Self::parse_expr_bp(lexer, end, Token::Not.prefix_bp().unwrap().1)?)),
        );

        loop {
            let op_token = token_expect_peek_match!(lexer, end, span,
                Token::Semi | Token::Comma | Token::BraceClose | Token::ParenClose => break,
                t if t.infix_bp().is_some() || t.postfix_bp().is_some() => t,
            );

            if let Some((l_bp, ())) = op_token.postfix_bp() {
                if l_bp < bp {
                    break;
                }

                lexer.next();

                lhs = match op_token {
                    Token::ParenOpen => {
                        let mut exprs = Vec::new();

                        loop {
                            token_expect_peek_match!(lexer, end, span,
                                Token::ParenClose => {
                                    lexer.next();
                                    break;
                                },
                                _ => (),
                            );

                            exprs.push(Self::parse_expr_bp(lexer, end, Binding::None)?);

                            token_expect_match!(lexer, end, span,
                                Token::ParenClose => break,
                                Token::Comma => (),
                            )
                        }

                        Expression::Call(Box::new(lhs), exprs)
                    },
                    _ => {
                        Expression::UnaryOp(match op_token {
                            _ => unreachable!("Not a valid postfix operator but op_token should be a postfix operator"),
                        }, Box::new(lhs))
                    },
                };

                continue;
            }

            let (l_bp, r_bp) = op_token.infix_bp().unwrap();

            if l_bp < bp {
                break;
            }

            lexer.next();

            lhs = Expression::BinaryOp(match op_token {
                Token::Plus => BinOp::Plus,
                Token::Minus => BinOp::Minus,
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Mod => BinOp::Mod,
                Token::Eq => BinOp::Assign,
                Token::Xor => BinOp::Xor,
                Token::And => BinOp::And,
                Token::Pipe => BinOp::Or,
                Token::RShift => BinOp::RShift,
                Token::LShift => BinOp::LShift,
                Token::Deq => BinOp::Eq,
                Token::Neq => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Leq => BinOp::Leq,
                Token::Geq => BinOp::Geq,
                _ => unreachable!("Not a valid infix operator but op_token should be an infix operator"),
            }, Box::new(lhs), Box::new(Self::parse_expr_bp(lexer, end, r_bp)?));
        }

        Ok(lhs)
    }

    fn parse_block(lexer: &mut Peekable<Lexer<'src>>, end: usize) -> Result<Block<'src>, Spanned<'src, CompileError>> {
        let mut content = Vec::new();

        loop {
            token_expect_peek_match!(lexer, end, span,
                Token::Const | Token::Mut => content.push(DeclOrExpr::Declaration(Self::parse_decl(lexer, end)?)),
                Token::BraceClose => return Ok(Block { content, last: None }),
                _ => {
                    let expr = Self::parse_expr(lexer, end)?;

                    token_expect_peek_match!(lexer, end, span,
                        Token::Semi => {
                            lexer.next();
                            content.push(DeclOrExpr::Expression(expr));
                        },
                        Token::BraceClose => {
                            return Ok(Block { content, last: Some(Box::new(expr)) })
                        },
                    );
                }
            );
        }
    }
}
