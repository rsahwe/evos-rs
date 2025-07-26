use core::{marker::PhantomData, ops::{Add, AddAssign, Range, RangeInclusive}};

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

    /// Constructs a new span for a single character
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
        if lhs.end <= rhs.start {
            Some(Self::merge_unchecked(lhs, rhs))
        } else if rhs.end <= lhs.start {
            Some(Self::merge_unchecked(rhs, lhs))
        } else {
            None
        }
    }

    /// Merges any two spans together.
    pub const fn complete_merge(lhs: Self, rhs: Self) -> Self {
        Self::new_raw(
            if lhs.start < rhs.start { lhs.start } else { rhs.start },
            if lhs.end > rhs.end { lhs.end } else { rhs.end }
        )
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

/// A type T associated with a Span
#[derive(Clone, Debug, Hash)]
pub struct Spanned<'src, T> {
    /// The spanned T instance
    pub inner: T,
    pub span: Span<'src>,
}

impl<'src, T: Copy> Copy for Spanned<'src, T> {}

impl<'src, T: PartialEq> PartialEq for Spanned<'src, T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.span == other.span
    }
}

impl<'src, T: Eq> Eq for Spanned<'src, T> {}

impl<'src, T> Spanned<'src, T> {
    /// Adds a span to inner
    pub const fn new(inner: T, span: Span<'src>) -> Self {
        Spanned { inner, span }
    }
    
    /// Evaluates the span of T
    pub const fn span(&self) -> Span<'src> {
        self.span
    }

    /// Keeps the span while mapping the value
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<'src, U> {
        Spanned::new(f(self.inner), self.span)
    }

    /// Strips the span off of inner
    pub fn strip(self) -> T {
        self.inner
    }
}
