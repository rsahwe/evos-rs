use core::{fmt::Display, iter::Peekable, marker::PhantomData, ops::{Add, AddAssign, Range, RangeInclusive}, str::CharIndices};

/// Range in src (exclusive)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span<'src> {
    start: usize,
    end: usize,
    _phantom: PhantomData<&'src str>,
}

impl<'src> Span<'src> {
    /// Constructs a new raw span
    pub const fn new_raw(start: usize, end: usize) -> Self {
        Self { start, end, _phantom: PhantomData }
    }

    /// Constructs a new inclusive span
    pub const fn new_inclusive(range: RangeInclusive<usize>) -> Self {
        Self::new_raw(*range.start(), *range.end() + 1)
    }

    /// Constructs a new exclusive span
    pub const fn new_exclusive(range: Range<usize>) -> Self {
        Self::new_raw(range.start, range.end)
    }

    pub const fn new_single(position: usize) -> Self {
        Self::new_raw(position, position + 1)
    }

    /// Merges two spans where lhs is directly to the left of rhs.
    pub const fn merge_unchecked(lhs: Self, rhs: Self) -> Self {
        Self::new_raw(lhs.start, rhs.end)
    }

    /// Merges two adjacent spans.
    pub const fn merge(lhs: Self, rhs: Self) -> Option<Self> {
        if lhs.end == rhs.start {
            Some(Self::merge_unchecked(lhs, rhs))
        } else if rhs.end == lhs.start {
            Some(Self::merge_unchecked(rhs, lhs))
        } else {
            None
        }
    }

    /// Merges two possibly non-adjacent spans.
    pub const fn gap_merge(lhs: Self, rhs: Self) -> Option<Self> {
        match Self::merge(lhs, rhs) {
            Some(span) => Some(span),
            None => {
                if lhs.end < rhs.start {
                    Some(Self::merge_unchecked(lhs, rhs))
                } else if rhs.end < lhs.start {
                    Some(Self::merge_unchecked(rhs, lhs))
                } else {
                    None
                }
            },
        }
    }

    /// Merges any two spans together.
    pub const fn complete_merge(lhs: Self, rhs: Self) -> Self {
        match Self::gap_merge(lhs, rhs) {
            Some(span) => span,
            None => {
                Self::new_raw(
                    if lhs.start < rhs.start { lhs.start } else { rhs.start },
                    if lhs.end > rhs.end { lhs.end } else { rhs.end }
                )
            },
        }
    }

    /// Gets the slice from a span
    pub fn as_slice(&self, source: &'src str) -> &'src str {
        &source[self.start..self.end]
    }
}

impl<'src> Add for Span<'src> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::complete_merge(self, rhs)
    }
}

impl<'src> AddAssign for Span<'src> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

/// A span that can be displayed in a nice way in error messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DisplaySpan<'src> {
    slice: &'src str,
    source_name: &'src str,
}

impl<'src> DisplaySpan<'src> {
    /// Turns a span into a source
    pub fn from_source(_span: Span<'src>, _source: &'src str, _source_name: &'src str) -> Self {
        todo!()
    }
}

impl<'src> Display for DisplaySpan<'src> {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        todo!("Span display!!!")
    }
}

/// All possible basic elements of a source file
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token<'src> {
    /// Identifier: started by a..z|A..Z|_ and then same or 0..9
    Ident(&'src str),

    // Errors:
    Error(LexerError),
}

impl<'src> Token<'src> {
    pub const fn add_span(self, span: Span<'src>) -> SpannedToken<'src> {
        SpannedToken { token: self, span }
    }
}

/// Token + Span
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpannedToken<'src> {
    pub token: Token<'src>,
    pub span: Span<'src>,
}

/// Gives a reason for why the lexer stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LexerError {
    MalformedInput,
    UnexpectedEof,
}

impl<'src> LexerError {
    pub const fn add_span(self, span: Span<'src>) -> SpannedLexerError<'src> {
        SpannedLexerError { error: self, span }
    }
}

/// LexerError + Span
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpannedLexerError<'src> {
    pub error: LexerError,
    pub span: Span<'src>,
}

/// The iterator over tokens.
#[derive(Clone, Debug)]
pub struct RawLexer<'src> {
    source: &'src str,
    chars: Peekable<CharIndices<'src>>,
    complete: bool,
}

impl<'src> RawLexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, chars: source.char_indices().peekable(), complete: false }
    }
}

impl<'src> Iterator for RawLexer<'src> {
    type Item = SpannedToken<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.chars.next();

        if next.is_none() {
            if self.complete {
                return None;
            }

            self.complete = true;

            return Some(
                Token::Error(LexerError::UnexpectedEof)
                    .add_span(
                        Span::new_single(self.source.len())
                    )
            );
        }

        let (pos, chr) = next.unwrap();

        match chr {
            c if c.is_whitespace() => {
                self.next()
            },
            c if c.is_alphabetic() || c == '_' => {
                let start = pos;

                let mut end = start;

                while let Some(next) = self.chars.next_if(|n| n.1.is_alphanumeric() || n.1 == '_') {
                    end = next.0;
                }

                //TODO: KEYWORDS

                Some(Token::Ident(&self.source[start..=end]).add_span(Span::new_inclusive(start..=end)))
            },
            _ => todo!("Lexer for `{}`!!!", self.source)
        }
    }
}

/// A peekable lexer that gives a LexerError instead 
#[derive(Clone, Debug)]
pub struct Lexer<'src> {
    inner: Peekable<RawLexer<'src>>,
    end_error: Option<SpannedLexerError<'src>>,
}

impl<'src> Lexer<'src> {
    pub fn next(&mut self) -> Result<SpannedToken<'src>, SpannedLexerError<'src>> {
        match self.end_error {
            Some(error) => Err(error),
            None => {
                match self.inner.next() {
                    Some(next) => {
                        match next.token {
                            Token::Error(error) => {
                                self.end_error = Some(error.add_span(next.span));
                                Err(error.add_span(next.span))
                            },
                            _ => Ok(next)
                        }
                    },
                    None => unreachable!("RawLexer did not return End reason!"),
                }
            },
        }
    }

    pub fn peek(&mut self) -> Result<SpannedToken<'src>, SpannedLexerError<'src>> {
        match self.end_error {
            Some(error) => Err(error),
            None => {
                match self.inner.peek() {
                    Some(next) => {
                        match next.token {
                            Token::Error(error) => {
                                self.end_error = Some(error.add_span(next.span));
                                Err(error.add_span(next.span))
                            },
                            _ => Ok(*next)
                        }
                    },
                    None => unreachable!("RawLexer did not return End reason!"),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::prelude::rust_2024::*;
    use std::*;

    const SPAN_TEST_SLICE: &str = "This is a test\nLine two\nEOF:";

    #[test]
    fn span_constructions() {
        assert_eq!(Span::new_single(0).as_slice(SPAN_TEST_SLICE), &SPAN_TEST_SLICE[0..=0]);
        assert_eq!(Span::new_raw(0, 2).as_slice(SPAN_TEST_SLICE), &SPAN_TEST_SLICE[0..2]);
        assert_eq!(Span::new_inclusive(0..=2).as_slice(SPAN_TEST_SLICE), &SPAN_TEST_SLICE[0..=2]);
        assert_eq!(Span::new_exclusive(0..2).as_slice(SPAN_TEST_SLICE), &SPAN_TEST_SLICE[0..2]);
    }

    #[test]
    fn span_merges() {
        // Normal
        assert_eq!(Span::new_exclusive(0..2) + Span::new_exclusive(2..3), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(0..1) + Span::new_exclusive(2..3), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(0..3) + Span::new_exclusive(2..3), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(0..1) + Span::new_exclusive(2..3), Span::new_exclusive(0..3));
        // Reversed
        assert_eq!(Span::new_exclusive(2..3) + Span::new_exclusive(0..2), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(2..3) + Span::new_exclusive(0..1), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(2..3) + Span::new_exclusive(0..3), Span::new_exclusive(0..3));
        assert_eq!(Span::new_exclusive(2..3) + Span::new_exclusive(0..1), Span::new_exclusive(0..3));
    }

    #[test]
    fn raw_lexer() {
        macro_rules! test_lexer_output {
            ($input:expr, $output:expr, $msg:expr) => {
                assert!(RawLexer::new($input).map(|st| dbg!(st.token)).eq($output), $msg)
            };
        }

        test_lexer_output!("  test\n ", [Token::Ident("test"), Token::Error(LexerError::UnexpectedEof)].into_iter(), "Simple ident test");
        test_lexer_output!("  te st\n ", [Token::Ident("te"), Token::Ident("st"), Token::Error(LexerError::UnexpectedEof)].into_iter(), "Simple ident test 2");
    }
}
