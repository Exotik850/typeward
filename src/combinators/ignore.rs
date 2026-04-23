use std::marker::PhantomData;

use crate::{error::ParseResult, parse::Parse};

/// A wrapper parser that ignores the output of `T`.
/// 
/// This is useful for parsing things like delimiters or keywords that don't need to be retained in the output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Ignore<T>(PhantomData<T>);

pub type Forget<T> = Ignore<T>;

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
