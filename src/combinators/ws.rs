use crate::{error::ParseResult, input::Input, parse::Parse};

/// A wrapper parser that trims leading whitespace before parsing `T`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Ws<T>(pub T);

impl<T> Ws<T> {
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }

    #[must_use]
    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> std::ops::Deref for Ws<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PartialEq<T>> PartialEq<T> for Ws<T> {
    fn eq(&self, other: &T) -> bool {
        &self.0 == other
    }
}

impl<'a, I, T> Parse<'a, I> for Ws<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (value, rest) = T::parse_with_context(input.trim_start(), context)?;
        Ok((Self(value), rest))
    }
}

pub trait WsExt: Sized {
    type Whitespaced;

    fn parse_ws<'a, I>(input: I) -> ParseResult<(Self::Whitespaced, I)>
    where
        I: Input<'a>,
        Self::Whitespaced: Parse<'a, I>,
    {
        Self::Whitespaced::parse(input)
    }
}

impl<T> WsExt for T {
    type Whitespaced = Ws<T>;
}

#[cfg(test)]
mod tests {
    use crate::{combinators::ws::Ws, parse::Parse};

    #[test]
    fn ws_consumes_leading_whitespace() {
        let (parsed, rest) = Ws::<i64>::parse("   42 tail").unwrap();
        assert_eq!(parsed.into_inner(), 42);
        assert_eq!(rest, " tail");
    }
}
