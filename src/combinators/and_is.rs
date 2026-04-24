use crate::{error::{ParseError, ParseResult}, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default)]
pub struct AndIs<A, B> {
    pub value: A,
    _marker: std::marker::PhantomData<B>,
}

impl<A, B> AndIs<A, B> {
    pub fn new(value: A) -> Self {
        Self {
            value,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn into_inner(self) -> A {
        self.value
    }

    pub fn inner(&self) -> &A {
        &self.value
    }

    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.value
    }
}

impl<A, B> std::ops::Deref for AndIs<A, B> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<A, B> std::ops::DerefMut for AndIs<A, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, I, A, B> Parse<'a, I> for AndIs<A, B>
where
    I: crate::input::Input<'a>,
    A: Parse<'a, I>,
    B: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (value, remaining) = A::parse_with_context(input, context)?;
        let parsed = input.slice_to(remaining)?;
        let (_, parsed_remaining) = B::parse_with_context(parsed, context)?;
        if !parsed_remaining.is_empty() {
            return Err(ParseError::custom(
                "AndIs validator parser did not consume the full parsed segment",
            ));
        }
        Ok((AndIs::new(value), remaining))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        combinators::and_is::AndIs,
        parse::Parse,
        prelude::{AlphaNumStr, AlphaStr},
    };

    #[test]
    fn test_and_is() {
        let input = "abc123";
        let result = AndIs::<&str, AlphaNumStr>::parse(input);
        assert!(result.is_ok());
        let (and_is, remaining) = result.unwrap();
        assert_eq!(and_is.into_inner(), "abc123");
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_and_is_fails() {
        let input = "abc123";
        let result = AndIs::<&str, AlphaStr>::parse(input);
        assert!(result.is_err());
    }
}
