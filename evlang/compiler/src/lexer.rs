use core::{iter::Peekable, str::CharIndices};

use crate::span::{Span, Spanned};

/// All possible basic elements of a source file
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token<'src> {
    Ident(&'src str),
    IntLit(&'src str),
    Const,
    Mut,
    Fn,
    True,
    False,
    Pipe,
    Colon,
    ParenOpen,
    ParenClose,
    BraceOpen,
    BraceClose,
    Not,
    Plus,
    Minus,
    Star,
    Slash,
    Xor,
    And,
    LShift,
    RShift,
    Eq, Deq, Lt, Gt,
    Neq, Leq, Geq,
    Semi,
}

/// Gives a reason for why the lexer stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LexerError {
    /// Unlexable characters
    MalformedInput,
    /// Eof
    UnexpectedEof,
}

/// The lexer obviously
#[derive(Clone, Debug)]
pub struct Lexer<'src> {
    source: &'src str,
    chars: Peekable<CharIndices<'src>>,
    stopped: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, chars: source.char_indices().peekable(), stopped: false }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Spanned<'src, Token<'src>>, Spanned<'src, LexerError>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped {
            return None
        }

        match self.chars.next() {
            Some((idx, chr)) => {
                macro_rules! symbol {
                    ($first:expr $(, $chr:expr, $sym:expr)*) => {
                        match self.chars.peek().copied() {
                            Some((_np, nc)) => match nc {
                                $(
                                    $chr => {
                                        self.chars.next();
                                        Ok(Spanned::new($sym, Span::new_inclusive(idx..=_np)))
                                    },
                                )*
                                _ => Ok(Spanned::new($first, Span::new_single(idx))),
                            },
                            None => Ok(Spanned::new($first, Span::new_single(idx))),
                        }
                    };
                }

                Some(match chr {
                    '|' => symbol!(Token::Pipe),
                    ':' => symbol!(Token::Colon),
                    '(' => symbol!(Token::ParenOpen),
                    ')' => symbol!(Token::ParenClose),
                    '{' => symbol!(Token::BraceOpen),
                    '}' => symbol!(Token::BraceClose),
                    '+' => symbol!(Token::Plus),
                    '-' => symbol!(Token::Minus),
                    '*' => symbol!(Token::Star),
                    '/' => symbol!(Token::Slash),
                    '^' => symbol!(Token::Xor),
                    '&' => symbol!(Token::And),
                    '=' => symbol!(Token::Eq, '=', Token::Deq),
                    '>' => symbol!(Token::Gt, '=', Token::Geq, '>', Token::RShift),
                    '<' => symbol!(Token::Lt, '=', Token::Leq, '<', Token::LShift),
                    '!' => symbol!(Token::Not, '=', Token::Eq),
                    ';' => symbol!(Token::Semi),
                    c if c.is_whitespace() => {
                        self.next().unwrap()
                    },
                    c if c.is_alphabetic() || c == '_' => {
                        let start = idx;

                        let mut end = start;

                        while let Some(next) = self.chars.next_if(|n| n.1.is_alphanumeric() || n.1 == '_') {
                            end = next.0;
                        }

                        let slice = &self.source[start..=end];
                        let span = Span::new_inclusive(start..=end);

                        match slice {
                            "const" => Ok(Spanned::new(Token::Const, span)),
                            "mut" => Ok(Spanned::new(Token::Mut, span)),
                            "fn" => Ok(Spanned::new(Token::Fn, span)),
                            "true" => Ok(Spanned::new(Token::True, span)),
                            "false" => Ok(Spanned::new(Token::False, span)),
                            _ => Ok(Spanned::new(Token::Ident(slice), span)),
                        }
                    },
                    c if c.is_numeric() => {
                        let start = idx;

                        let mut end = start;

                        match self.chars.peek().copied() {
                            Some((ndx, nhr)) => match nhr {
                                '0'..'9' => {
                                    self.chars.next();
                                    end = ndx;
                                },
                                'x' | 'o' | 'b' => {
                                    self.chars.next();
                                    if c != '0' {
                                        self.stopped = true;
                                        return Some(Err(Spanned::new(LexerError::MalformedInput, Span::new_inclusive(idx..=ndx))));
                                    }
                                    match self.chars.next() {
                                        Some((tdx, thr)) => {
                                            if thr.is_ascii_hexdigit() {
                                                end = tdx;
                                            } else {
                                                self.stopped = true;
                                                return Some(Err(Spanned::new(LexerError::MalformedInput, Span::new_inclusive(idx..=tdx))));
                                            }
                                        },
                                        None => {
                                            self.stopped = true;
                                            return Some(Err(Spanned::new(LexerError::UnexpectedEof, Span::new_single(self.source.len()))))
                                        },
                                    }
                                },
                                _ => (),
                            },
                            None => (),
                        }

                        while let Some(next) = self.chars.next_if(|n| n.1.is_ascii_hexdigit() || n.1 == '_') {
                            end = next.0;
                        }

                        let slice = &self.source[start..=end];
                        let span = Span::new_inclusive(start..=end);

                        Ok(Spanned::new(Token::IntLit(slice), span))
                    },
                    _ => {
                        self.stopped = true;
                        Err(Spanned::new(LexerError::MalformedInput, Span::new_single(idx)))
                    },
                })
            },
            None => {
                self.stopped = true;
                None
            },
        }
    }
}
