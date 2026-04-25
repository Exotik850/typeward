use std::marker::PhantomData;

use crate::{error::ParseResult, parse::Parse, prelude::And};

/// A wrapper parser that ignores the output of `T`.
///
/// This is useful for parsing things like delimiters or keywords that don't need to be retained in the output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Ignore<T>(PhantomData<T>);

/// Type alias for `Ignore<T>`, indicating that the output of `T` is intentionally discarded.
pub type Forget<T> = Ignore<T>;
/// A parser that matches `A` followed by `B`, but only retains the output of `A`.  
pub type Preceded<P, T> = And<Ignore<P>, T>;
/// A parser that matches `A` followed by `B`, but only retains the output of `B`.
pub type Terminated<T, S> = And<T, Ignore<S>>;
/// A parser that matches `A` followed by `B` followed by `C`, but only retains the output of `B`.
pub type Between<S, T, E> = And<Ignore<S>, And<T, Ignore<E>>>;

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
