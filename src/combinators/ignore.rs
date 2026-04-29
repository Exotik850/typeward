use std::marker::PhantomData;

use crate::{
    error::ParseResult,
    parse::Parse,
    prelude::{And, WhitespaceStr},
};

/// A wrapper parser that ignores the output of `T`.
///
/// This is useful for parsing things like delimiters or keywords that don't need to be retained in the output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Ignore<T>(PhantomData<T>);

/// Type alias for `Ignore<T>`, indicating that the output of `T` is intentionally discarded.
pub type Forget<T> = Ignore<T>;
/// A parser that matches `A` followed by `B`, but only retains the output of `B`.  
pub type Preceded<P, T> = And<Ignore<P>, T>;
/// A parser that matches `A` followed by `B`, but only retains the output of `A`.
pub type Terminated<T, S> = And<T, Ignore<S>>;
/// A parser that matches `A` followed by `B` followed by `C`, but only retains the output of `B`.
pub type Between<S, T, E> = And<Ignore<S>, And<T, Ignore<E>>>;
/// A parser that matches `T` surrounded by optional whitespace, retaining only the output of `T`.
pub type Trim<T> =
    Between<Ignore<Option<WhitespaceStr<'static>>>, T, Ignore<Option<WhitespaceStr<'static>>>>;

impl<T> Trim<T> {
    pub fn value(&self) -> &T {
        self.right.left()
    }
}

impl<T> Ignore<T> {
    #[must_use]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, I, T> Parse<'a, I> for Ignore<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (_, input) = T::parse_with_context(input, context)?;
        Ok((Self::new(), input))
    }
}

/// A parser that repeatedly matches `T` and ignores all outputs
///
/// Fails if `T` fails with a fatal error, but otherwise succeeds even if `T` never matches at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct IgnoreMany<T>(PhantomData<T>);

impl<T> IgnoreMany<T> {
    #[must_use]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, I, T> Parse<'a, I> for IgnoreMany<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        mut input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        for res in super::iter::iter::<I, T>(input, context, "IgnoreMany") {
            let (_, next_input) = match res {
                Ok((item, next_input)) => (item, next_input),
                Err(err) if err.is_fatal() => return Err(err),
                Err(_) => break,
            };
            input = next_input;
        }
        Ok((Self::new(), input))
    }
}

/// A parser that repeatedly matches `T` and ignores all outputs, but requires at least one match
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct IgnoreMany1<T>(PhantomData<T>);

impl<T> IgnoreMany1<T> {
    #[must_use]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, I, T> Parse<'a, I> for IgnoreMany1<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        mut input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (_first, next_input) = T::parse_with_context(input, context)?;
        input = next_input;
        while let Ok((_, next_input)) = T::parse_with_context(input, context) {
            input = next_input;
        }
        Ok((Self::new(), input))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Count<T> {
    pub count: usize,
    _marker: PhantomData<T>,
}

impl<T> Count<T> {
    #[must_use]
    pub fn new(count: usize) -> Self {
        Self {
            count,
            _marker: PhantomData,
        }
    }
}

impl<'a, I, T> Parse<'a, I> for Count<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        mut input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let mut count = 0;
        while let Ok((_, next_input)) = T::parse_with_context(input, context) {
            input = next_input;
            count += 1;
        }
        Ok((Self::new(count), input))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_ignore() {
        let input = "abc";
        let result = Ignore::<Alpha>::parse(input).unwrap();
        assert_eq!(result.0, Ignore::new());
        assert_eq!(result.1, "");
    }

    #[test]
    fn test_ignore_many() {
        let input = "aaab";
        let result = IgnoreMany::<aChar>::parse(input).unwrap();
        assert_eq!(result.0, IgnoreMany::new());
        assert_eq!(result.1, "b");
    }

    #[test]
    fn test_ignore_many1() {
        let input = "aaab";
        let result = IgnoreMany1::<aChar>::parse(input).unwrap();
        assert_eq!(result.0, IgnoreMany1::new());
        assert_eq!(result.1, "b");

        let input = "b";
        let result = IgnoreMany1::<aChar>::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_trim() {
        let input = "  abc  ";
        let result = Trim::<Alpha>::parse(input).unwrap();
        assert_eq!(result.0.value(), &"abc");
        assert_eq!(result.1, "");
    }
}
